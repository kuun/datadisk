//! Task Manager implementation
//!
//! Manages background tasks for file operations

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, watch, RwLock};

/// Global task manager instance
pub static TASK_MANAGER: std::sync::LazyLock<TaskManager> =
    std::sync::LazyLock::new(TaskManager::new);

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Starting,
    Running,
    Suspended,
    Completed,
    Cancelled,
    Failed,
}

/// Task type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Copy,
    Move,
}

/// Conflict policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictPolicy {
    Ask,
    Abort,
    Skip,
    Rename,
    Overwrite,
}

impl Default for ConflictPolicy {
    fn default() -> Self {
        ConflictPolicy::Ask
    }
}

/// File info for conflict display
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConflictFileInfo {
    pub name: String,
    pub size: i64,
    #[serde(rename = "modifyTime")]
    pub modify_time: i64,
    #[serde(rename = "isDirectory")]
    pub is_directory: bool,
}

/// Conflict info
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConflictInfo {
    #[serde(rename = "needConfirm")]
    pub need_confirm: bool,
    #[serde(rename = "conflictPolicy")]
    pub conflict_policy: ConflictPolicy,
    #[serde(rename = "srcFile")]
    pub src_file: ConflictFileInfo,
    #[serde(rename = "dstFile")]
    pub dst_file: ConflictFileInfo,
}

/// Base task information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub agent: String,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "startedAt")]
    pub started_at: i64,
    #[serde(rename = "updatedAt")]
    pub updated_at: i64,
    pub status: TaskStatus,
    #[serde(rename = "type")]
    pub task_type: TaskType,
    #[serde(rename = "userId")]
    pub user_id: i64,
    pub error: Option<String>,
    // Copy task specific fields
    #[serde(rename = "isCopy")]
    pub is_copy: bool,
    pub source: String,
    pub target: String,
    pub files: Vec<String>,
    #[serde(rename = "conflictInfo")]
    pub conflict_info: ConflictInfo,
    // Progress fields
    #[serde(rename = "currentFile")]
    pub current_file: String,
    #[serde(rename = "currentFileSize")]
    pub current_file_size: i64,
    #[serde(rename = "currentFileCopiedSize")]
    pub current_file_copied_size: i64,
    #[serde(rename = "totalFiles")]
    pub total_files: i64,
    #[serde(rename = "copiedFiles")]
    pub copied_files: i64,
    #[serde(rename = "totalSize")]
    pub total_size: i64,
    #[serde(rename = "copiedSize")]
    pub copied_size: i64,
}

impl TaskInfo {
    pub fn new(user_id: i64, agent: &str, task_type: TaskType) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            agent: agent.to_string(),
            created_at: now,
            started_at: 0,
            updated_at: now,
            status: TaskStatus::Pending,
            task_type,
            user_id,
            error: None,
            is_copy: matches!(task_type, TaskType::Copy),
            source: String::new(),
            target: String::new(),
            files: Vec::new(),
            conflict_info: ConflictInfo::default(),
            current_file: String::new(),
            current_file_size: 0,
            current_file_copied_size: 0,
            total_files: 0,
            copied_files: 0,
            total_size: 0,
            copied_size: 0,
        }
    }
}

/// Task trait for all task implementations
pub trait Task: Send + Sync {
    fn info(&self) -> TaskInfo;
    fn id(&self) -> String;
    fn start(self: Arc<Self>);
    fn cancel(&self);
    fn suspend(&self);
    fn resume(&self);
    fn resolve_conflict(&self, policy: ConflictPolicy);
}

/// Copy task implementation
pub struct CopyTask {
    info: RwLock<TaskInfo>,
    user_dir: PathBuf,
    cancel_tx: watch::Sender<bool>,
    suspend_tx: watch::Sender<bool>,
    conflict_tx: tokio::sync::mpsc::Sender<ConflictPolicy>,
    conflict_rx: RwLock<Option<tokio::sync::mpsc::Receiver<ConflictPolicy>>>,
    notify_tx: broadcast::Sender<TaskNotification>,
}

impl CopyTask {
    pub fn new(
        user_id: i64,
        _username: &str,
        agent: &str,
        is_copy: bool,
        source: String,
        target: String,
        files: Vec<String>,
        user_dir: PathBuf,
        notify_tx: broadcast::Sender<TaskNotification>,
    ) -> Self {
        let task_type = if is_copy { TaskType::Copy } else { TaskType::Move };
        let mut info = TaskInfo::new(user_id, agent, task_type);
        info.is_copy = is_copy;
        info.source = source.clone();
        info.target = target.clone();
        info.files = files.clone();
        info.total_files = files.len() as i64;

        // Auto-apply rename policy if source and target are the same
        if source == target {
            info.conflict_info.conflict_policy = ConflictPolicy::Rename;
        }

        let (cancel_tx, _) = watch::channel(false);
        let (suspend_tx, _) = watch::channel(false);
        let (conflict_tx, conflict_rx) = tokio::sync::mpsc::channel(1);

        Self {
            info: RwLock::new(info),
            user_dir,
            cancel_tx,
            suspend_tx,
            conflict_tx,
            conflict_rx: RwLock::new(Some(conflict_rx)),
            notify_tx,
        }
    }

    fn notify(&self, info: &TaskInfo) {
        let _ = self.notify_tx.send(TaskNotification::TaskInfo(info.clone()));
    }

    /// Join user path safely
    fn join_user_path(&self, paths: &[&str]) -> Result<PathBuf, String> {
        // Get the canonical user directory first
        let user_dir_canonical = self.user_dir.canonicalize()
            .map_err(|e| format!("failed to canonicalize user_dir: {}", e))?;

        let mut full = user_dir_canonical.clone();
        for p in paths {
            let trimmed = p.trim_start_matches('/');
            if !trimmed.is_empty() {
                full = full.join(trimmed);
            }
        }

        // Try to canonicalize, but if path doesn't exist yet, normalize manually
        let full = if full.exists() {
            full.canonicalize().unwrap_or(full)
        } else {
            // For non-existent paths, normalize while keeping as absolute path
            Self::normalize_path(&full)
        };

        // Check if path is still under user_dir
        if !full.starts_with(&user_dir_canonical) {
            tracing::warn!("Path check failed: full={:?}, user_dir_canonical={:?}",
                full, user_dir_canonical);
            return Err("accessing path outside user directory".to_string());
        }
        Ok(full)
    }

    /// Normalize path by resolving . and ..
    fn normalize_path(path: &Path) -> PathBuf {
        let mut components = Vec::new();
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::CurDir => {}
                c => components.push(c),
            }
        }
        components.iter().collect()
    }

    /// Calculate source files total size and count
    async fn calc_source(&self) -> Result<(), String> {
        let info = self.info.read().await;
        let files = info.files.clone();
        let source = info.source.clone();
        drop(info);

        let mut total_files: i64 = 0;
        let mut total_size: i64 = 0;

        for file in &files {
            let full_path = self.join_user_path(&[&source, file])?;

            let metadata = tokio::fs::metadata(&full_path).await
                .map_err(|e| format!("failed to stat source file: {}", e))?;

            if metadata.is_dir() {
                // Walk directory
                let mut stack = vec![full_path];
                while let Some(dir) = stack.pop() {
                    let mut entries = tokio::fs::read_dir(&dir).await
                        .map_err(|e| format!("failed to read directory: {}", e))?;

                    while let Some(entry) = entries.next_entry().await
                        .map_err(|e| format!("failed to read entry: {}", e))?
                    {
                        let meta = entry.metadata().await
                            .map_err(|e| format!("failed to get metadata: {}", e))?;
                        if meta.is_dir() {
                            stack.push(entry.path());
                        } else {
                            total_files += 1;
                            total_size += meta.len() as i64;
                        }
                    }
                }
            } else {
                total_files += 1;
                total_size += metadata.len() as i64;
            }
        }

        let mut info = self.info.write().await;
        info.total_files = total_files;
        info.total_size = total_size;
        Ok(())
    }

    /// Check target directory exists
    async fn check_target(&self) -> Result<(), String> {
        let info = self.info.read().await;
        let target = info.target.clone();
        drop(info);

        let full_path = self.join_user_path(&[&target])?;

        let metadata = tokio::fs::metadata(&full_path).await
            .map_err(|_| "target path does not exist".to_string())?;

        if !metadata.is_dir() {
            return Err("target path is not a directory".to_string());
        }
        Ok(())
    }

    /// Generate unique path for rename policy
    fn generate_unique_path(path: &Path) -> PathBuf {
        let parent = path.parent().unwrap_or(Path::new(""));
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        for i in 1.. {
            let new_name = if ext.is_empty() {
                format!("{}({})", stem, i)
            } else {
                format!("{}({}).{}", stem, i, ext)
            };
            let new_path = parent.join(new_name);
            if !new_path.exists() {
                return new_path;
            }
        }
        path.to_path_buf()
    }

    /// Copy or move files
    async fn copy_or_move(&self) -> Result<(), String> {
        // Take the conflict receiver
        let mut conflict_rx = self.conflict_rx.write().await.take()
            .ok_or("conflict receiver already taken")?;

        let info = self.info.read().await;
        let files = info.files.clone();
        let source = info.source.clone();
        let target = info.target.clone();
        let is_copy = info.is_copy;
        let mut conflict_policy = info.conflict_info.conflict_policy;
        drop(info);

        for file in &files {
            // Check cancelled
            if *self.cancel_tx.borrow() {
                return Err("task cancelled".to_string());
            }

            let src_path = self.join_user_path(&[&source, file])?;
            let mut dst_path = self.join_user_path(&[&target, file])?;

            // Create parent directories
            if let Some(parent) = dst_path.parent() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| format!("failed to create target directories: {}", e))?;
            }

            // Check for conflict
            if dst_path.exists() {
                match conflict_policy {
                    ConflictPolicy::Abort => {
                        return Err("conflict detected, aborting".to_string());
                    }
                    ConflictPolicy::Skip => {
                        continue;
                    }
                    ConflictPolicy::Rename => {
                        dst_path = Self::generate_unique_path(&dst_path);
                    }
                    ConflictPolicy::Overwrite => {
                        // Proceed to overwrite
                    }
                    ConflictPolicy::Ask => {
                        // Get conflict info
                        let src_meta = tokio::fs::metadata(&src_path).await
                            .map_err(|e| format!("failed to stat source: {}", e))?;
                        let dst_meta = tokio::fs::metadata(&dst_path).await
                            .map_err(|e| format!("failed to stat dest: {}", e))?;

                        {
                            let mut info = self.info.write().await;
                            info.conflict_info.need_confirm = true;
                            info.conflict_info.src_file = ConflictFileInfo {
                                name: src_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string(),
                                size: src_meta.len() as i64,
                                modify_time: src_meta.modified()
                                    .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
                                    .unwrap_or(0),
                                is_directory: src_meta.is_dir(),
                            };
                            info.conflict_info.dst_file = ConflictFileInfo {
                                name: dst_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string(),
                                size: dst_meta.len() as i64,
                                modify_time: dst_meta.modified()
                                    .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
                                    .unwrap_or(0),
                                is_directory: dst_meta.is_dir(),
                            };
                            info.updated_at = chrono::Utc::now().timestamp();
                            self.notify(&info);
                        }

                        // Wait for conflict resolution
                        let policy = conflict_rx.recv().await
                            .ok_or("conflict channel closed")?;

                        // Clear conflict info
                        {
                            let mut info = self.info.write().await;
                            info.conflict_info.need_confirm = false;
                            info.conflict_info.src_file = ConflictFileInfo::default();
                            info.conflict_info.dst_file = ConflictFileInfo::default();
                            // Remember the policy for subsequent conflicts
                            info.conflict_info.conflict_policy = policy;
                        }
                        conflict_policy = policy;

                        // Check cancelled after waiting
                        if *self.cancel_tx.borrow() {
                            return Err("task cancelled".to_string());
                        }

                        match policy {
                            ConflictPolicy::Abort => {
                                return Err("conflict detected, aborting".to_string());
                            }
                            ConflictPolicy::Skip => {
                                continue;
                            }
                            ConflictPolicy::Rename => {
                                dst_path = Self::generate_unique_path(&dst_path);
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Get source metadata
            let src_meta = tokio::fs::metadata(&src_path).await
                .map_err(|e| format!("failed to stat source: {}", e))?;

            // Update current file info
            {
                let mut info = self.info.write().await;
                info.current_file = file.clone();
                info.current_file_size = src_meta.len() as i64;
                info.current_file_copied_size = 0;
            }

            if is_copy {
                self.copy_file(&src_path, &dst_path).await?;
            } else {
                // Move: try rename first, fall back to copy+delete
                if tokio::fs::rename(&src_path, &dst_path).await.is_err() {
                    self.copy_file(&src_path, &dst_path).await?;
                    if src_meta.is_dir() {
                        tokio::fs::remove_dir_all(&src_path).await
                            .map_err(|e| format!("failed to remove source dir: {}", e))?;
                    } else {
                        tokio::fs::remove_file(&src_path).await
                            .map_err(|e| format!("failed to remove source file: {}", e))?;
                    }
                }

                // Update progress for move
                let mut info = self.info.write().await;
                info.copied_files += 1;
                info.copied_size += info.current_file_size;
                info.updated_at = chrono::Utc::now().timestamp();
                self.notify(&info);
            }
        }

        Ok(())
    }

    /// Copy a file or directory
    fn copy_file<'a>(&'a self, src: &'a Path, dst: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            let metadata = tokio::fs::metadata(src).await
                .map_err(|e| format!("failed to stat source: {}", e))?;

            if metadata.is_dir() {
                return self.copy_dir(src, dst).await;
            }

        // Copy file with progress tracking
        let mut src_file = tokio::fs::File::open(src).await
            .map_err(|e| format!("failed to open source: {}", e))?;
        let mut dst_file = tokio::fs::File::create(dst).await
            .map_err(|e| format!("failed to create dest: {}", e))?;

        let mut buf = vec![0u8; 1024 * 1024]; // 1MB buffer
        let mut copied: i64 = 0;

        loop {
            // Check cancelled
            if *self.cancel_tx.borrow() {
                return Err("task cancelled".to_string());
            }

            // Check suspended
            while *self.suspend_tx.borrow() {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if *self.cancel_tx.borrow() {
                    return Err("task cancelled".to_string());
                }
            }

            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let n = src_file.read(&mut buf).await
                .map_err(|e| format!("failed to read: {}", e))?;

            if n == 0 {
                break;
            }

            dst_file.write_all(&buf[..n]).await
                .map_err(|e| format!("failed to write: {}", e))?;

            copied += n as i64;

            // Update progress
            let mut info = self.info.write().await;
            info.current_file_copied_size = copied;
            info.updated_at = chrono::Utc::now().timestamp();
            self.notify(&info);
        }

        dst_file.flush().await
            .map_err(|e| format!("failed to flush: {}", e))?;

            // Update copied count
            let mut info = self.info.write().await;
            info.copied_files += 1;
            info.copied_size += copied;
            info.updated_at = chrono::Utc::now().timestamp();
            self.notify(&info);

            Ok(())
        })
    }

    /// Copy directory recursively
    fn copy_dir<'a>(&'a self, src: &'a Path, dst: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            tokio::fs::create_dir_all(dst).await
                .map_err(|e| format!("failed to create dest dir: {}", e))?;

            let mut entries = tokio::fs::read_dir(src).await
                .map_err(|e| format!("failed to read dir: {}", e))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| format!("failed to read entry: {}", e))?
            {
                // Check cancelled
                if *self.cancel_tx.borrow() {
                    return Err("task cancelled".to_string());
                }

                // Check suspended
                while *self.suspend_tx.borrow() {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }

                let src_path = entry.path();
                let dst_path = dst.join(entry.file_name());

                let meta = entry.metadata().await
                    .map_err(|e| format!("failed to get metadata: {}", e))?;

                // Update current file
                {
                    let mut info = self.info.write().await;
                    info.current_file = entry.file_name().to_string_lossy().to_string();
                    info.current_file_size = meta.len() as i64;
                    info.current_file_copied_size = 0;
                }

                self.copy_file(&src_path, &dst_path).await?;
            }

            Ok(())
        })
    }

    /// Run the copy task
    async fn run_async(&self) {
        // Update status to starting
        {
            let mut info = self.info.write().await;
            info.status = TaskStatus::Starting;
            info.started_at = chrono::Utc::now().timestamp();
            info.updated_at = info.started_at;
            self.notify(&info);
        }

        // Calculate source
        if let Err(e) = self.calc_source().await {
            let mut info = self.info.write().await;
            info.status = TaskStatus::Failed;
            info.error = Some(e);
            info.updated_at = chrono::Utc::now().timestamp();
            self.notify(&info);
            return;
        }

        // Check target
        if let Err(e) = self.check_target().await {
            let mut info = self.info.write().await;
            info.status = TaskStatus::Failed;
            info.error = Some(e);
            info.updated_at = chrono::Utc::now().timestamp();
            self.notify(&info);
            return;
        }

        // Update status to running
        {
            let mut info = self.info.write().await;
            info.status = TaskStatus::Running;
            info.updated_at = chrono::Utc::now().timestamp();
            self.notify(&info);
        }

        // Copy or move
        if let Err(e) = self.copy_or_move().await {
            let mut info = self.info.write().await;
            info.status = TaskStatus::Failed;
            info.error = Some(e);
            info.updated_at = chrono::Utc::now().timestamp();
            self.notify(&info);
            return;
        }

        // Update status to completed
        {
            let mut info = self.info.write().await;
            info.status = TaskStatus::Completed;
            info.updated_at = chrono::Utc::now().timestamp();
            self.notify(&info);
        }
    }
}

impl Task for CopyTask {
    fn info(&self) -> TaskInfo {
        futures::executor::block_on(async { self.info.read().await.clone() })
    }

    fn id(&self) -> String {
        futures::executor::block_on(async { self.info.read().await.id.clone() })
    }

    fn start(self: Arc<Self>) {
        tokio::spawn(async move {
            self.run_async().await;
        });
    }

    fn cancel(&self) {
        let _ = self.cancel_tx.send(true);
        futures::executor::block_on(async {
            let mut info = self.info.write().await;
            info.status = TaskStatus::Cancelled;
            info.updated_at = chrono::Utc::now().timestamp();
            self.notify(&info);
        });
    }

    fn suspend(&self) {
        let _ = self.suspend_tx.send(true);
        futures::executor::block_on(async {
            let mut info = self.info.write().await;
            if info.status == TaskStatus::Running {
                info.status = TaskStatus::Suspended;
                info.updated_at = chrono::Utc::now().timestamp();
                self.notify(&info);
            }
        });
    }

    fn resume(&self) {
        let _ = self.suspend_tx.send(false);
        futures::executor::block_on(async {
            let mut info = self.info.write().await;
            if info.status == TaskStatus::Suspended {
                info.status = TaskStatus::Running;
                info.updated_at = chrono::Utc::now().timestamp();
                self.notify(&info);
            }
        });
    }

    fn resolve_conflict(&self, policy: ConflictPolicy) {
        let _ = self.conflict_tx.try_send(policy);
    }
}

/// Task notification for WebSocket
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum TaskNotification {
    #[serde(rename = "taskInfo")]
    TaskInfo(TaskInfo),
    #[serde(rename = "taskDeleted")]
    TaskDeleted(String),
}

/// Task Manager
pub struct TaskManager {
    /// Tasks by user ID
    tasks: DashMap<i64, Vec<Arc<dyn Task>>>,
    /// Notification channel
    notify_tx: broadcast::Sender<TaskNotification>,
}

impl TaskManager {
    pub fn new() -> Self {
        let (notify_tx, _) = broadcast::channel(100);
        Self {
            tasks: DashMap::new(),
            notify_tx,
        }
    }

    /// Add a task
    pub fn add_task(&self, task: Arc<dyn Task>) {
        let info = task.info();
        let user_id = info.user_id;

        self.tasks
            .entry(user_id)
            .or_insert_with(Vec::new)
            .push(task.clone());

        // Notify about new task
        let _ = self.notify_tx.send(TaskNotification::TaskInfo(info));

        // Start task in background
        task.start();
    }

    /// Create and add a copy task
    pub fn create_copy_task(
        &self,
        user_id: i64,
        username: &str,
        agent: &str,
        is_copy: bool,
        source: String,
        target: String,
        files: Vec<String>,
        user_dir: PathBuf,
    ) -> TaskInfo {
        let task = Arc::new(CopyTask::new(
            user_id,
            username,
            agent,
            is_copy,
            source,
            target,
            files,
            user_dir,
            self.notify_tx.clone(),
        ));

        let info = task.info();
        self.add_task(task);
        info
    }

    /// Get a specific task
    pub fn get_task(&self, user_id: i64, task_id: &str) -> Option<Arc<dyn Task>> {
        self.tasks.get(&user_id).and_then(|tasks| {
            tasks
                .iter()
                .find(|t| t.id() == task_id)
                .cloned()
        })
    }

    /// Get all tasks for a user
    pub fn get_tasks(&self, user_id: i64) -> Vec<TaskInfo> {
        self.tasks
            .get(&user_id)
            .map(|tasks| tasks.iter().map(|t| t.info()).collect())
            .unwrap_or_default()
    }

    /// Remove a task
    pub fn remove_task(&self, user_id: i64, task_id: &str) {
        if let Some(mut tasks) = self.tasks.get_mut(&user_id) {
            tasks.retain(|t| t.id() != task_id);
        }

        // Notify about task deletion
        let _ = self
            .notify_tx
            .send(TaskNotification::TaskDeleted(task_id.to_string()));
    }

    /// Get notification receiver
    pub fn subscribe(&self) -> broadcast::Receiver<TaskNotification> {
        self.notify_tx.subscribe()
    }

    /// Get notification sender (for creating tasks)
    pub fn notify_sender(&self) -> broadcast::Sender<TaskNotification> {
        self.notify_tx.clone()
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}
