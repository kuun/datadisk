//! File handlers
//!
//! Implements file CRUD operations, upload, download, and preview

use axum::{
    body::Body,
    extract::{Multipart, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
    Extension,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

use crate::entity::{file_access, file_info};
use crate::handlers::audit::service::log_operation;
use crate::handlers::recent::record_file_access;
use crate::middleware::auth::CurrentUser;
use crate::middleware::DbConn;
use crate::routes::ApiResponse;
use crate::state::AppState;

/// Check if a path is safe (no .. or traversal)
fn is_safe_path(path: &str) -> bool {
    let path = path.trim_start_matches('/');
    if path.is_empty() {
        return true;
    }
    std::path::Path::new(path)
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)))
}

/// Check if a filename is safe (no path separators)
fn is_safe_filename(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Windows restricted characters
    if name.chars().any(|c| "<>:\"/\\|?*".contains(c)) {
        return false;
    }

    // Control characters
    if name.chars().any(|c| c.is_control()) {
        return false;
    }

    // Ban names consisting entirely of dots (., .., ..., ....)
    if name.chars().all(|c| c == '.') {
        return false;
    }

    let path = std::path::Path::new(name);
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(c)), None) => c == name,
        _ => false,
    }
}

/// Operation types (matching Go version)
mod op_type {
    pub const MKDIR: &str = "创建目录";
    pub const OPEN_FILE: &str = "访问目录/文件";
    pub const DELETE: &str = "删除";
    pub const RENAME: &str = "重命名";
    pub const COPY: &str = "复制";
    pub const MOVE: &str = "移动";
    pub const UPLOAD: &str = "上传";
    pub const DOWNLOAD: &str = "下载";
}

const OP_SUCCESS: &str = "成功";

/// Download info storage
static DOWNLOAD_MAP: std::sync::LazyLock<Mutex<HashMap<String, DownloadInfo>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone)]
struct DownloadInfo {
    files: Vec<String>,
    parent_dir: String,
}

/// Mkdir request
#[derive(Debug, Deserialize)]
pub struct MkdirRequest {
    pub path: Option<String>,
    pub name: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<i64>,
    #[serde(rename = "parentPath")]
    pub parent_path: Option<String>,
}

/// Delete file request
#[derive(Debug, Deserialize)]
pub struct DeleteFileRequest {
    pub ids: Vec<i64>,
    #[serde(rename = "parentPath")]
    pub parent_path: String,
}

/// File query parameters
#[derive(Debug, Deserialize)]
pub struct FileQuery {
    #[serde(rename = "parentId")]
    pub parent_id: i64,
}

/// Download pre request
#[derive(Debug, Deserialize)]
pub struct DownloadPreRequest {
    pub files: Vec<String>,
    #[serde(rename = "parentDir")]
    pub parent_dir: String,
}

/// Download query
#[derive(Debug, Deserialize)]
pub struct DownloadQuery {
    pub guid: String,
}

/// Download pre response
#[derive(Debug, Serialize)]
pub struct DownloadPreResponse {
    pub result: bool,
    pub guid: String,
}

/// Rename request
#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    #[serde(rename = "oldPath")]
    pub old_path: String,
    #[serde(rename = "newName")]
    pub new_name: String,
}

/// Delete files request (new API)
#[derive(Debug, Deserialize)]
pub struct DeleteFilesRequest {
    pub files: Vec<String>,
    #[serde(rename = "parentDir")]
    pub parent_dir: String,
}

/// Create directory request (new API)
#[derive(Debug, Deserialize)]
pub struct CreateDirRequest {
    pub path: String,
    pub name: String,
}

/// Path query
#[derive(Debug, Deserialize)]
pub struct PathQuery {
    pub path: String,
}

/// File info response
#[derive(Debug, Serialize)]
pub struct FileInfoResponse {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub file_type: String,
    pub size: i64,
    #[serde(rename = "isDirectory")]
    pub is_directory: bool,
    #[serde(rename = "createTime")]
    pub create_time: i64,
    #[serde(rename = "modifyTime")]
    pub modify_time: i64,
    #[serde(rename = "parentId")]
    pub parent_id: i64,
    pub username: String,
}

impl From<file_info::Model> for FileInfoResponse {
    fn from(m: file_info::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            file_type: m.file_type.clone(),
            size: m.size,
            is_directory: m.is_directory,
            create_time: m.create_time,
            modify_time: m.modify_time,
            parent_id: m.parent_id,
            username: m.username,
        }
    }
}

/// Directory listing item (for new API)
#[derive(Debug, Serialize)]
pub struct DirectoryItem {
    pub basename: String,
    pub filename: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub size: i64,
    pub lastmod: String,
    pub mime: String,
}

/// Get user path from config and username
/// Path format: {root_dir}/{username} (matching Go version)
pub fn get_user_path(config: &crate::config::Config, username: &str) -> PathBuf {
    config.root_dir.join(username)
}

/// Resolve directory ID from path
async fn resolve_dir_id(
    db: &sea_orm::DatabaseConnection,
    username: &str,
    path: &str,
) -> i64 {
    if path.is_empty() || path == "/" {
        return -1;
    }

    let cleaned = path.trim_matches('/');
    if cleaned.is_empty() {
        return -1;
    }

    let parts: Vec<&str> = cleaned.split('/').collect();
    let mut parent_id: i64 = -1;

    for part in parts {
        let file_info = file_info::Entity::find()
            .filter(file_info::Column::ParentId.eq(parent_id))
            .filter(file_info::Column::Username.eq(username))
            .filter(file_info::Column::Name.eq(part))
            .one(db)
            .await;

        match file_info {
            Ok(Some(f)) => {
                if !f.is_directory {
                    return 0;
                }
                parent_id = f.id;
            }
            _ => return 0,
        }
    }

    parent_id
}

/// Resolve file info from path (returns file_id and file_name)
async fn resolve_file_info(
    db: &sea_orm::DatabaseConnection,
    username: &str,
    path: &str,
) -> Option<(i64, String)> {
    if path.is_empty() || path == "/" {
        return None;
    }

    let cleaned = path.trim_matches('/');
    if cleaned.is_empty() {
        return None;
    }

    let parts: Vec<&str> = cleaned.split('/').collect();
    let mut parent_id: i64 = -1;
    let mut last_file: Option<file_info::Model> = None;

    for part in parts {
        let file = file_info::Entity::find()
            .filter(file_info::Column::ParentId.eq(parent_id))
            .filter(file_info::Column::Username.eq(username))
            .filter(file_info::Column::Name.eq(part))
            .one(db)
            .await;

        match file {
            Ok(Some(f)) => {
                parent_id = f.id;
                last_file = Some(f);
            }
            _ => return None,
        }
    }

    last_file.map(|f| (f.id, f.name))
}

/// POST /api/file/mkdir
pub async fn mkdir(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<MkdirRequest>,
) -> Json<ApiResponse<()>> {
    if req.name.is_empty() || !is_safe_filename(&req.name) {
        return Json(ApiResponse::error(400, "文件夹名称无效"));
    }

    let user_path = get_user_path(&state.config, &current_user.username);
    let parent_path = req.parent_path.clone().or(req.path.clone()).unwrap_or_default();
    if !is_safe_path(&parent_path) {
        return Json(ApiResponse::error(400, "invalid parent path"));
    }
    let parent_path = parent_path.trim_start_matches('/').to_string();

    // Resolve parent ID
    let parent_id = if let Some(pid) = req.parent_id {
        if pid > 0 { pid } else { -1 }
    } else {
        resolve_dir_id(&*db, &current_user.username, &parent_path).await
    };

    // Check if parent exists (if parent_id > 0)
    if parent_id > 0 {
        let count = file_info::Entity::find()
            .filter(file_info::Column::Id.eq(parent_id))
            .filter(file_info::Column::Username.eq(&current_user.username))
            .count(&*db)
            .await;

        match count {
            Ok(0) => return Json(ApiResponse::error(500, "parent_dir_not_exists")),
            Err(e) => {
                tracing::error!("Database error: {}", e);
                return Json(ApiResponse::error(500, "internal error"));
            }
            _ => {}
        }
    } else if parent_id == 0 {
        return Json(ApiResponse::error(400, "parent_dir_not_exists"));
    }

    // Create directory in transaction
    let now = chrono::Utc::now().timestamp();
    let dir_name = req.name.clone();
    let parent_path_for_log = parent_path.clone();
    let username_for_log = current_user.username.clone();
    let result = (&*db)
        .transaction::<_, (), sea_orm::DbErr>(|txn| {
            Box::pin(async move {
                // Insert into database
                let new_dir = file_info::ActiveModel {
                    username: Set(current_user.username.clone()),
                    file_type: Set("dir".to_string()),
                    name: Set(req.name.clone()),
                    parent_id: Set(parent_id),
                    create_time: Set(now),
                    modify_time: Set(now),
                    is_directory: Set(true),
                    size: Set(0),
                    ..Default::default()
                };
                new_dir.insert(txn).await?;

                // Create directory on filesystem
                let dir_path = user_path.join(&parent_path).join(&req.name);
                tokio::fs::create_dir_all(&dir_path)
                    .await
                    .map_err(|e: std::io::Error| sea_orm::DbErr::Custom(e.to_string()))?;

                Ok(())
            })
        })
        .await;

    match result {
        Ok(_) => {
            let op_desc = format!("{}/{}", parent_path_for_log, dir_name);
            log_operation(&username_for_log, op_type::MKDIR, &op_desc, OP_SUCCESS, None);
            Json(ApiResponse::success_msg("success"))
        }
        Err(e) => {
            tracing::error!("Failed to create directory: {}", e);
            Json(ApiResponse::error(500, "create_dir_error"))
        }
    }
}

/// GET /api/file/query/files
pub async fn get_files(
    State(_state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<FileQuery>,
) -> Json<ApiResponse<Vec<FileInfoResponse>>> {
    let result = file_info::Entity::find()
        .filter(file_info::Column::ParentId.eq(query.parent_id))
        .filter(file_info::Column::Username.eq(&current_user.username))
        .order_by_asc(file_info::Column::CreateTime)
        .all(&*db)
        .await;

    match result {
        Ok(files) => {
            let response: Vec<FileInfoResponse> = files.into_iter().map(|f| f.into()).collect();
            Json(ApiResponse::success(response))
        }
        Err(e) => {
            tracing::error!("Failed to get files: {}", e);
            Json(ApiResponse::error(500, e.to_string()))
        }
    }
}

/// POST /api/file/remove/file
pub async fn remove_file(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<DeleteFileRequest>,
) -> Json<ApiResponse<()>> {
    if !is_safe_path(&req.parent_path) {
        return Json(ApiResponse::error(400, "invalid parent path"));
    }
    let user_path = get_user_path(&state.config, &current_user.username);
    let parent_path = req.parent_path.trim_start_matches('/');
    let mut success_count = 0;
    let mut error_count = 0;

    for id in req.ids {
        // Get file info
        let file_info = file_info::Entity::find_by_id(id)
            .filter(file_info::Column::Username.eq(&current_user.username))
            .one(&*db)
            .await;

        let file = match file_info {
            Ok(Some(f)) => f,
            Ok(None) => {
                tracing::error!("File not found: {}", id);
                error_count += 1;
                continue;
            }
            Err(e) => {
                tracing::error!("Database error: {}", e);
                error_count += 1;
                continue;
            }
        };

        let file_path = user_path.join(parent_path).join(&file.name);

        if file.is_directory {
            // Delete children recursively
            delete_children(&*db, id, &current_user.username).await;

            // Delete directory from filesystem
            if let Err(e) = fs::remove_dir_all(&file_path).await {
                tracing::error!("Failed to delete directory: {}", e);
                error_count += 1;
                continue;
            }
        } else {
            // Delete file from database
            if let Err(e) = file_info::Entity::delete_by_id(id)
                .exec(&*db)
                .await
            {
                tracing::error!("Failed to delete file from database: {}", e);
                error_count += 1;
                continue;
            }

            // Delete file from filesystem
            if let Err(e) = fs::remove_file(&file_path).await {
                tracing::error!("Failed to delete file from filesystem: {}", e);
            }
        }

        // Audit log
        let op_desc = if parent_path == "/" {
            format!("/{}", file.name)
        } else {
            format!("{}/{}", parent_path, file.name)
        };
        log_operation(&current_user.username, op_type::DELETE, &op_desc, OP_SUCCESS, None);
        success_count += 1;
    }

    let message = format!(
        "删除成功{}个文件，失败{}个文件",
        success_count, error_count
    );
    Json(ApiResponse::success_msg(message))
}

/// Delete children recursively
async fn delete_children(db: &sea_orm::DatabaseConnection, parent_id: i64, username: &str) {
    let children = file_info::Entity::find()
        .filter(file_info::Column::ParentId.eq(parent_id))
        .filter(file_info::Column::Username.eq(username))
        .all(db)
        .await;

    if let Ok(children) = children {
        for child in children {
            if child.is_directory {
                Box::pin(delete_children(db, child.id, username)).await;
            }
            let _ = file_info::Entity::delete_by_id(child.id).exec(db).await;
        }
    }

    let _ = file_info::Entity::delete_by_id(parent_id).exec(db).await;
}

/// POST /api/file/download/pre
pub async fn download_pre(
    Extension(_current_user): Extension<CurrentUser>,
    Json(req): Json<DownloadPreRequest>,
) -> Json<DownloadPreResponse> {
    if req.files.is_empty() || req.parent_dir.is_empty() {
        return Json(DownloadPreResponse {
            result: false,
            guid: String::new(),
        });
    }

    if !is_safe_path(&req.parent_dir) {
        return Json(DownloadPreResponse {
            result: false,
            guid: String::new(),
        });
    }

    for file in &req.files {
        if !is_safe_filename(file) {
             return Json(DownloadPreResponse {
                result: false,
                guid: String::new(),
            });
        }
    }

    let guid = uuid::Uuid::new_v4().to_string();

    let download_info = DownloadInfo {
        files: req.files,
        parent_dir: req.parent_dir,
    };

    DOWNLOAD_MAP.lock().unwrap().insert(guid.clone(), download_info);

    Json(DownloadPreResponse {
        result: true,
        guid,
    })
}

/// GET /api/file/download
pub async fn download_file(
    State(state): State<AppState>,
    Extension(_db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<DownloadQuery>,
) -> impl IntoResponse {
    let download_info = {
        let mut map = DOWNLOAD_MAP.lock().unwrap();
        map.remove(&query.guid)
    };

    let download_info = match download_info {
        Some(info) => info,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                [(header::CONTENT_TYPE, "application/json")],
                Body::from(r#"{"error": "invalid guid"}"#),
            )
                .into_response();
        }
    };

    let user_path = get_user_path(&state.config, &current_user.username);
    let base_dir = user_path.join(download_info.parent_dir.trim_start_matches('/'));
    let username = current_user.username.clone();

    // Create a channel for streaming zip data
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Vec<u8>, std::io::Error>>(32);

    // Spawn a task to write zip data
    let base_dir_clone = base_dir.clone();
    let files = download_info.files.clone();
    let parent_dir = download_info.parent_dir.clone();

    tokio::task::spawn_blocking(move || {
        // Use a custom Write implementation that sends to the channel
        let writer = ChannelWriter::new(tx.clone());
        // Use new_stream for non-seekable writer (zip 7.0+)
        let mut zip = zip::ZipWriter::new_stream(writer);
        // Use Stored (no compression) for faster download speed
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for file_name in &files {
            let file_path = base_dir_clone.join(file_name);

            if let Err(e) = add_to_zip_streaming(&mut zip, &base_dir_clone, &file_path, &options, &username, &parent_dir) {
                tracing::error!("Failed to add file to zip: {}", e);
            }
        }

        if let Err(e) = zip.finish() {
            tracing::error!("Failed to finish zip: {}", e);
        }
    });

    // Convert receiver to stream
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    let body = Body::from_stream(stream);

    // Return streaming response
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=download.zip",
        )
        .header(header::TRANSFER_ENCODING, "chunked")
        .body(body)
        .unwrap()
}

/// Channel-based writer for streaming zip
struct ChannelWriter {
    tx: tokio::sync::mpsc::Sender<Result<Vec<u8>, std::io::Error>>,
    buffer: Vec<u8>,
}

const CHANNEL_BUFFER_SIZE: usize = 1024 * 1024; // 1MB buffer for better throughput

impl ChannelWriter {
    fn new(tx: tokio::sync::mpsc::Sender<Result<Vec<u8>, std::io::Error>>) -> Self {
        Self {
            tx,
            buffer: Vec::with_capacity(CHANNEL_BUFFER_SIZE),
        }
    }

    fn flush_buffer(&mut self) -> std::io::Result<()> {
        if !self.buffer.is_empty() {
            let data = std::mem::take(&mut self.buffer);
            self.tx.blocking_send(Ok(data))
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "channel closed"))?;
        }
        Ok(())
    }
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(buf);

        // Flush when buffer reaches threshold
        if self.buffer.len() >= CHANNEL_BUFFER_SIZE {
            self.flush_buffer()?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flush_buffer()
    }
}

impl Drop for ChannelWriter {
    fn drop(&mut self) {
        let _ = self.flush_buffer();
    }
}

/// Add file or directory to zip with streaming and audit logging
fn add_to_zip_streaming<W: Write>(
    zip: &mut zip::ZipWriter<zip::write::StreamWriter<W>>,
    base_dir: &PathBuf,
    path: &PathBuf,
    options: &zip::write::FileOptions<()>,
    username: &str,
    parent_dir: &str,
) -> std::io::Result<()> {
    if path.is_dir() {
        let entries: Vec<_> = std::fs::read_dir(path)?.collect();

        // If directory is empty, add directory entry to zip
        if entries.is_empty() {
            let dir_name = path
                .strip_prefix(base_dir)
                .map(|p| format!("{}/", p.to_string_lossy()))
                .unwrap_or_else(|_| format!("{}/", path.file_name().unwrap().to_string_lossy()));
            zip.add_directory(&dir_name, options.clone())?;
        } else {
            for entry in entries {
                let entry = entry?;
                add_to_zip_streaming(zip, base_dir, &entry.path(), options, username, parent_dir)?;
            }
        }
    } else if path.is_file() {
        let name = path
            .strip_prefix(base_dir)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.file_name().unwrap().to_string_lossy().to_string());

        zip.start_file(&name, options.clone())?;
        let mut file = std::fs::File::open(path)?;
        let mut buffer = vec![0u8; 1024 * 1024]; // 1MB read buffer for better throughput
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            zip.write_all(&buffer[..n])?;
        }

        // Audit log for each downloaded file
        let log_path = format!("{}/{}", parent_dir, name).replace("//", "/");
        log_operation(username, op_type::DOWNLOAD, &log_path, OP_SUCCESS, None);
    }
    Ok(())
}

/// GET /api/file/list - List directory contents (new API)
/// Returns array directly (no ApiResponse wrapper, matching Go behavior)
pub async fn list_directory(
    State(state): State<AppState>,
    Extension(_db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<PathQuery>,
) -> impl IntoResponse {
    if !is_safe_path(&query.path) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid path"})),
        ).into_response();
    }
    let user_path = get_user_path(&state.config, &current_user.username);
    let path = if query.path.is_empty() { "/" } else { &query.path };
    let full_path = user_path.join(path.trim_start_matches('/'));

    // Ensure user root directory exists (create if not)
    if !user_path.exists() {
        if let Err(e) = fs::create_dir_all(&user_path).await {
            tracing::error!("Failed to create user directory: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "failed to create user directory"})),
            ).into_response();
        }
    }

    // Check if path exists
    if !full_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "path not found"})),
        ).into_response();
    }

    // Read directory
    let entries = match fs::read_dir(&full_path).await {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Failed to read directory: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "failed to read directory"})),
            ).into_response();
        }
    };

    let mut items = Vec::new();
    let mut entries = entries;

    while let Some(entry) = entries.next_entry().await.ok().flatten() {
        let metadata = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };

        let basename = entry.file_name().to_string_lossy().to_string();
        let filename = format!("{}/{}", path.trim_end_matches('/'), basename);

        let (item_type, mime) = if metadata.is_dir() {
            ("directory".to_string(), String::new())
        } else {
            let mime = get_mime_type(&basename);
            ("file".to_string(), mime)
        };

        let lastmod = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| {
                chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        items.push(DirectoryItem {
            basename,
            filename,
            item_type,
            size: metadata.len() as i64,
            lastmod,
            mime,
        });
    }

    // Audit log for directory access
    let clean_path = if path == "/" { "/".to_string() } else { format!("/{}", path.trim_matches('/')) };
    log_operation(&current_user.username, op_type::OPEN_FILE, &clean_path, OP_SUCCESS, None);

    // Return array directly (matching Go behavior)
    Json(items).into_response()
}

/// Get MIME type from file extension
fn get_mime_type(filename: &str) -> String {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "txt" | "md" => "text/plain",
        "json" => "application/json",
        "js" => "application/javascript",
        "css" => "text/css",
        "html" => "text/html",
        "xml" => "application/xml",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "zip" => "application/zip",
        "rar" => "application/vnd.rar",
        "7z" => "application/x-7z-compressed",
        "tar" => "application/x-tar",
        "gz" | "tgz" => "application/gzip",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// POST /api/file/rename
pub async fn rename_file(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<RenameRequest>,
) -> Json<ApiResponse<()>> {
    if !is_safe_path(&req.old_path) {
        return Json(ApiResponse::error(400, "invalid old path"));
    }
    if req.old_path == "/" || req.old_path.trim().is_empty() {
        return Json(ApiResponse::error(400, "invalid old path"));
    }
    if !is_safe_filename(&req.new_name) {
        return Json(ApiResponse::error(400, "invalid new name"));
    }

    let user_path = get_user_path(&state.config, &current_user.username);
    let old_path = user_path.join(req.old_path.trim_start_matches('/'));
    let new_path = old_path.parent().unwrap().join(&req.new_name);

    // Check if old file exists
    if !old_path.exists() {
        return Json(ApiResponse::error(404, "file not found"));
    }

    // Check if new name already exists
    if new_path.exists() {
        return Json(ApiResponse::error(409, "file with new name already exists"));
    }

    // Rename the file
    if let Err(e) = fs::rename(&old_path, &new_path).await {
        tracing::error!("Failed to rename file: {}", e);
        return Json(ApiResponse::error(500, "failed to rename file"));
    }

    // Resolve parent_id from old_path's parent directory
    let parent_path = std::path::Path::new(&req.old_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let parent_id = resolve_dir_id(&*db, &current_user.username, &parent_path).await;

    // Update database (with correct parent_id to avoid updating same-name files in other dirs)
    let old_name = match old_path.file_name().and_then(|n| n.to_str()) {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => return Json(ApiResponse::error(400, "invalid old path")),
    };
    let db_result = file_info::Entity::update_many()
        .col_expr(file_info::Column::Name, sea_orm::sea_query::Expr::value(&req.new_name))
        .filter(file_info::Column::Username.eq(&current_user.username))
        .filter(file_info::Column::ParentId.eq(parent_id))
        .filter(file_info::Column::Name.eq(&old_name))
        .exec(&*db)
        .await;

    if let Err(e) = db_result {
        tracing::error!("Failed to update database during rename: {}", e);
        // Try to rollback filesystem change
        if let Err(re) = fs::rename(&new_path, &old_path).await {
            tracing::error!("Failed to rollback file rename: {}", re);
        }
        return Json(ApiResponse::error(500, "database error"));
    }

    // Audit log
    let op_desc = format!("{} => {}", req.old_path, req.new_name);
    log_operation(&current_user.username, op_type::RENAME, &op_desc, OP_SUCCESS, None);
    Json(ApiResponse::success_msg("file renamed successfully"))
}

/// GET /api/file/content
pub async fn get_file_content(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<PathQuery>,
) -> impl IntoResponse {
    if !is_safe_path(&query.path) {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            Body::from(r#"{"error": "invalid path"}"#),
        ).into_response();
    }
    let user_path = get_user_path(&state.config, &current_user.username);
    let file_path = user_path.join(query.path.trim_start_matches('/'));

    // Check if file exists
    let metadata = match fs::metadata(&file_path).await {
        Ok(m) => m,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/json")],
                Body::from(r#"{"error": "file not found"}"#),
            )
                .into_response();
        }
    };

    // Check if it's a file
    if metadata.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            Body::from(r#"{"error": "cannot preview directory"}"#),
        )
            .into_response();
    }

    // Read file content (limit to 10MB to prevent OOM)
    let content = match tokio::fs::File::open(&file_path).await {
        Ok(mut file) => {
            let mut buffer = Vec::new();
            let limit = 10 * 1024 * 1024; // 10MB limit
            let mut handle = tokio::io::AsyncReadExt::take(&mut file, limit);
            if let Err(e) = tokio::io::AsyncReadExt::read_to_end(&mut handle, &mut buffer).await {
                tracing::error!("Failed to read file: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "application/json")],
                    Body::from(r#"{"error": "failed to read file"}"#),
                ).into_response();
            }
            buffer
        }
        Err(e) => {
            tracing::error!("Failed to open file: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Body::from(r#"{"error": "failed to open file"}"#),
            )
                .into_response();
        }
    };

    // Determine content type
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let content_type = match ext {
        "json" => "application/json",
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "xml" => "application/xml",
        _ => "text/plain",
    };

    // Record file access for recent files
    let clean_path = format!("/{}", query.path.trim_start_matches('/'));
    if let Some((file_id, file_name)) = resolve_file_info(&*db, &current_user.username, &query.path).await {
        record_file_access(
            &*db,
            current_user.id,
            file_id,
            &clean_path,
            &file_name,
            "preview",
            false,
        ).await;
    }

    // Audit log
    log_operation(&current_user.username, op_type::OPEN_FILE, &clean_path, OP_SUCCESS, None);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(content))
        .unwrap()
}

/// POST /api/file/delete (new API)
pub async fn delete_files(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<DeleteFilesRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    if !is_safe_path(&req.parent_dir) {
        return Json(ApiResponse::error(400, "invalid parent directory"));
    }
    for file in &req.files {
        if !is_safe_filename(file) {
            return Json(ApiResponse::error(400, "invalid file name"));
        }
    }

    let user_path = get_user_path(&state.config, &current_user.username);
    let parent_dir = req.parent_dir.trim_start_matches('/');

    // Resolve parent_id from parent_dir path
    let parent_id = resolve_dir_id(&*db, &current_user.username, parent_dir).await;
    if parent_id == 0 {
        return Json(ApiResponse::error(400, "parent_dir_not_exists"));
    }

    let mut success = 0;
    let mut failed = 0;

    for file_name in &req.files {
        let file_path = user_path.join(parent_dir).join(file_name);

        // Check if file exists
        let metadata = match fs::metadata(&file_path).await {
            Ok(m) => m,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        // Delete from filesystem
        let result = if metadata.is_dir() {
            fs::remove_dir_all(&file_path).await
        } else {
            fs::remove_file(&file_path).await
        };

        if let Err(e) = result {
            tracing::error!("Failed to delete file {}: {}", file_name, e);
            failed += 1;
            continue;
        }

        // Delete from database (with correct parent_id to avoid deleting same-name files in other dirs)
        // First, get file_id to delete from recent access
        let file_record = file_info::Entity::find()
            .filter(file_info::Column::Username.eq(&current_user.username))
            .filter(file_info::Column::ParentId.eq(parent_id))
            .filter(file_info::Column::Name.eq(file_name))
            .one(&*db)
            .await;

        if let Ok(Some(file)) = file_record {
            // Delete from recent access records
            let _ = file_access::Entity::delete_many()
                .filter(file_access::Column::UserId.eq(current_user.id))
                .filter(file_access::Column::FileId.eq(file.id))
                .exec(&*db)
                .await;
        }

        // Delete file info
        let _ = file_info::Entity::delete_many()
            .filter(file_info::Column::Username.eq(&current_user.username))
            .filter(file_info::Column::ParentId.eq(parent_id))
            .filter(file_info::Column::Name.eq(file_name))
            .exec(&*db)
            .await;

        // Audit log
        let op_desc = if req.parent_dir == "/" {
            format!("/{}", file_name)
        } else {
            format!("{}/{}", req.parent_dir, file_name)
        };
        log_operation(&current_user.username, op_type::DELETE, &op_desc, OP_SUCCESS, None);
        success += 1;
    }

    let message = format!("删除成功{}个文件，失败{}个文件", success, failed);
    Json(ApiResponse::success(serde_json::json!({
        "message": message,
        "success": success,
        "failed": failed
    })))
}

/// GET /api/file/download/single
pub async fn download_single_file(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<PathQuery>,
) -> impl IntoResponse {
    if !is_safe_path(&query.path) {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            Body::from(r#"{"error": "invalid path"}"#),
        ).into_response();
    }
    let user_path = get_user_path(&state.config, &current_user.username);
    let file_path = user_path.join(query.path.trim_start_matches('/'));

    // Check if file exists
    let metadata = match fs::metadata(&file_path).await {
        Ok(m) => m,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/json")],
                Body::from(r#"{"error": "file not found"}"#),
            )
                .into_response();
        }
    };

    // Check if it's a file
    if metadata.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            Body::from(r#"{"error": "cannot download directory"}"#),
        )
            .into_response();
    }

    // Read file
    let file = match tokio::fs::File::open(&file_path).await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to open file: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Body::from(r#"{"error": "failed to open file"}"#),
            )
                .into_response();
        }
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download");

    // Record file access for recent files
    let clean_path = format!("/{}", query.path.trim_start_matches('/'));
    if let Some((file_id, file_name)) = resolve_file_info(&*db, &current_user.username, &query.path).await {
        record_file_access(
            &*db,
            current_user.id,
            file_id,
            &clean_path,
            &file_name,
            "download",
            false,
        ).await;
    }

    // Audit log
    log_operation(&current_user.username, op_type::DOWNLOAD, &clean_path, OP_SUCCESS, None);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(body)
        .unwrap()
}

/// GET /api/file/preview/single
pub async fn preview_single_file(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<PathQuery>,
) -> impl IntoResponse {
    if !is_safe_path(&query.path) {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            Body::from(r#"{"error": "invalid path"}"#),
        ).into_response();
    }
    let user_path = get_user_path(&state.config, &current_user.username);
    let file_path = user_path.join(query.path.trim_start_matches('/'));

    // Check if file exists
    let metadata = match fs::metadata(&file_path).await {
        Ok(m) => m,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/json")],
                Body::from(r#"{"error": "file not found"}"#),
            )
                .into_response();
        }
    };

    // Check if it's a file
    if metadata.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            Body::from(r#"{"error": "cannot preview directory"}"#),
        )
            .into_response();
    }

    // Read file
    let file = match tokio::fs::File::open(&file_path).await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to open file: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Body::from(r#"{"error": "failed to open file"}"#),
            )
                .into_response();
        }
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("preview");

    let content_type = get_mime_type(filename);

    // Record file access for recent files
    let clean_path = format!("/{}", query.path.trim_start_matches('/'));
    if let Some((file_id, file_name)) = resolve_file_info(&*db, &current_user.username, &query.path).await {
        record_file_access(
            &*db,
            current_user.id,
            file_id,
            &clean_path,
            &file_name,
            "preview",
            false,
        ).await;
    }

    // Audit log
    log_operation(&current_user.username, op_type::OPEN_FILE, &clean_path, OP_SUCCESS, None);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", filename),
        )
        .body(body)
        .unwrap()
}

/// Upload response matching Go version format
#[derive(Serialize)]
struct UploadResponse {
    result: bool,
    message: String,
}

/// POST /api/file/upload
/// Supports streaming upload for large files - data is written directly to disk
/// without loading the entire file into memory.
pub async fn upload_file(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut parent_id: Option<i64> = None;
    let mut parent_path = String::new();
    let mut file_name = String::new();
    let mut content_type = String::new();
    let mut file_written = false;
    let mut actual_size: i64 = 0;

    let user_path = get_user_path(&state.config, &current_user.username);
    let mut tmp_file: Option<tokio::fs::File> = None;
    let mut tmp_file_path: Option<PathBuf> = None;

    // Parse multipart form data with streaming
    while let Some(field) = multipart.next_field().await.ok().flatten() {
        let name = field.name().unwrap_or("").to_string();
        tracing::debug!("Parsing field: {}", name);

        match name.as_str() {
            "parentId" => {
                if let Ok(text) = field.text().await {
                    parent_id = text.parse().ok();
                }
            }
            "parentPath" => {
                if let Ok(text) = field.text().await {
                    if !is_safe_path(&text) {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(UploadResponse { result: false, message: "invalid parent path".to_string() })
                        );
                    }
                    parent_path = text;
                }
            }
            "file" => {
                file_name = field.file_name().unwrap_or("").to_string();
                if !is_safe_filename(&file_name) {
                     return (
                        StatusCode::BAD_REQUEST,
                        Json(UploadResponse { result: false, message: "invalid file name".to_string() })
                    );
                }
                content_type = field.content_type().unwrap_or("application/octet-stream").to_string();

                // Use a unique temp file to avoid collisions/issues if parentPath comes late
                // We'll rename it to the correct path after the upload is complete
                let uuid_name = uuid::Uuid::new_v4().to_string();
                let temp_path = user_path.join(&uuid_name).with_extension("uploading");

                // Ensure user root directory exists
                if !user_path.exists() {
                    if let Err(e) = fs::create_dir_all(&user_path).await {
                        tracing::error!("Failed to create user directory: {}", e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(UploadResponse { result: false, message: "上传文件失败".to_string() })
                        );
                    }
                }

                // Open temp file for streaming write
                let file = match tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&temp_path)
                    .await
                {
                    Ok(f) => f,
                    Err(e) => {
                        tracing::error!("Failed to open temp file: {}, path: {:?}", e, temp_path);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(UploadResponse { result: false, message: "上传文件失败".to_string() })
                        );
                    }
                };

                tmp_file = Some(file);
                tmp_file_path = Some(temp_path);

                // Get max upload size from config for validation
                let max_size = state.config.max_upload_size as i64;

                // Stream the file data directly to disk
                let file_ref = tmp_file.as_mut().unwrap();
                let mut field = field;
                
                loop {
                    match field.chunk().await {
                        Ok(Some(chunk)) => {
                            actual_size += chunk.len() as i64;
                            
                            // Check if file size exceeds limit
                            if actual_size > max_size {
                                tracing::warn!("Upload rejected: file size {} exceeds limit {}", actual_size, max_size);
                                // Clean up temp file
                                if let Some(ref path) = tmp_file_path {
                                    let _ = fs::remove_file(path).await;
                                }
                                let max_size_mb = max_size / (1024 * 1024);
                                return (
                                    StatusCode::PAYLOAD_TOO_LARGE,
                                    Json(UploadResponse { 
                                        result: false, 
                                        message: format!("文件大小超过限制，最大允许 {}MB", max_size_mb) 
                                    })
                                );
                            }
                            
                            if let Err(e) = file_ref.write_all(&chunk).await {
                                tracing::error!("Failed to write chunk: {}", e);
                                // Clean up temp file
                                if let Some(ref path) = tmp_file_path {
                                    let _ = fs::remove_file(path).await;
                                }
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(UploadResponse { result: false, message: "上传文件失败".to_string() })
                                );
                            }
                        }
                        Ok(None) => {
                            // End of stream
                            file_written = true;
                            break;
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            tracing::error!("Failed to read chunk: {}", error_msg);
                            // Clean up temp file
                            if let Some(ref path) = tmp_file_path {
                                let _ = fs::remove_file(path).await;
                            }

                            // Check if it's a body limit or multipart parsing error
                            let error_msg_lower = error_msg.to_lowercase();
                            let is_size_error = error_msg_lower.contains("body limit")
                                || error_msg_lower.contains("length limit")
                                || error_msg_lower.contains("payload too large")
                                || error_msg_lower.contains("multipart/form-data")
                                || error_msg_lower.contains("content-length");

                            let (status, response_msg) = if is_size_error {
                                let max_size_mb = max_size / (1024 * 1024);
                                (StatusCode::PAYLOAD_TOO_LARGE, format!("文件大小超过限制，最大允许 {}MB", max_size_mb))
                            } else {
                                (StatusCode::INTERNAL_SERVER_ERROR, "上传文件失败，请检查网络连接后重试".to_string())
                            };

                            return (
                                status,
                                Json(UploadResponse { result: false, message: response_msg })
                            );
                        }
                    }
                }

                // Flush the file
                if let Err(e) = file_ref.flush().await {
                    tracing::error!("Failed to flush file: {}", e);
                }
            }
            _ => {}
        }
    }

    if !file_written {
        tracing::error!("No file data received. file_name={}", file_name);
        return (
            StatusCode::BAD_REQUEST,
            Json(UploadResponse { result: false, message: "no file data".to_string() })
        );
    }

    tracing::debug!("Upload streaming complete: file_name={}, actual_size={}", file_name, actual_size);

    // Close the file handle before renaming
    drop(tmp_file);

    let tmp_path = tmp_file_path.unwrap();

    // Recalculate destination path to ensure we use the latest parent_path
    // This fixes the issue where "file" field appears before "parentPath" field
    let clean_parent_path = parent_path.trim_start_matches('/');
    let final_dest_path = user_path.join(clean_parent_path).join(&file_name);

    // Ensure parent directory exists for the final destination
    if let Some(parent) = final_dest_path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent).await {
                tracing::error!("Failed to create parent directory: {}", e);
                let _ = fs::remove_file(&tmp_path).await;
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(UploadResponse { result: false, message: "上传文件失败".to_string() })
                );
            }
        }
    }

    // Rename temp file to final file
    if let Err(e) = fs::rename(&tmp_path, &final_dest_path).await {
        tracing::error!("Failed to rename temp file: {}", e);
        let _ = fs::remove_file(&tmp_path).await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(UploadResponse { result: false, message: "上传文件失败".to_string() })
        );
    }

    // Resolve parent_id from parentPath if not provided or is root
    let resolved_parent_id = match parent_id {
        Some(id) if id > 0 => id,
        _ => {
            if !clean_parent_path.is_empty() {
                resolve_dir_id(&*db, &current_user.username, clean_parent_path).await
            } else {
                -1
            }
        }
    };
    if resolved_parent_id == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(UploadResponse { result: false, message: "parent_dir_not_exists".to_string() })
        );
    }

    // Save to database
    let now = chrono::Utc::now().timestamp();
    let file_info = file_info::ActiveModel {
        username: Set(current_user.username.clone()),
        name: Set(file_name.clone()),
        file_type: Set(content_type),
        size: Set(actual_size),
        parent_id: Set(resolved_parent_id),
        create_time: Set(now),
        modify_time: Set(now),
        is_directory: Set(false),
        ..Default::default()
    };

    if let Err(e) = file_info.insert(&*db).await {
        tracing::error!("Failed to save file info: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(UploadResponse { result: false, message: "上传文件失败".to_string() })
        );
    }

    // Audit log
    let log_path = format!("/{}/{}", clean_parent_path, file_name);
    let log_path = log_path.replace("//", "/");
    log_operation(&current_user.username, op_type::UPLOAD, &log_path, OP_SUCCESS, None);

    (
        StatusCode::OK,
        Json(UploadResponse { result: true, message: "上传文件成功".to_string() })
    )
}

/// Copy/Move request
#[derive(Debug, Deserialize)]
pub struct CopyMoveRequest {
    #[serde(rename = "isCopy")]
    pub is_copy: bool,
    pub source: String,
    pub target: String,
    pub files: Vec<String>,
}

/// POST /api/file/copy
pub async fn copy_move_file(
    State(state): State<AppState>,
    Extension(_db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<CopyMoveRequest>,
) -> Json<ApiResponse<()>> {
    use crate::task::TASK_MANAGER;

    if !is_safe_path(&req.source) {
        return Json(ApiResponse::error(400, "invalid source path"));
    }
    if !is_safe_path(&req.target) {
        return Json(ApiResponse::error(400, "invalid target path"));
    }
    for file in &req.files {
        if !is_safe_filename(file) {
            return Json(ApiResponse::error(400, "invalid file name"));
        }
    }

    let user_path = get_user_path(&state.config, &current_user.username);

    // Create and add task
    let _task_info = TASK_MANAGER.create_copy_task(
        current_user.id,
        &current_user.username,
        "web", // agent
        req.is_copy,
        req.source.clone(),
        req.target.clone(),
        req.files.clone(),
        user_path,
    );

    // Audit log - one entry per file/directory
    let op_type_str = if req.is_copy { op_type::COPY } else { op_type::MOVE };
    for file in &req.files {
        let src_path = if req.source == "/" {
            format!("/{}", file)
        } else {
            format!("{}/{}", req.source, file)
        };
        let op_desc = format!("{} => {}", src_path, req.target);
        log_operation(&current_user.username, op_type_str, &op_desc, OP_SUCCESS, None);
    }

    Json(ApiResponse::success_msg("任务添加成功, 请查看任务列表"))
}

/// Conflict resolution request
#[derive(Debug, Deserialize)]
pub struct ResolveConflictRequest {
    #[serde(rename = "taskId")]
    pub task_id: String,
    pub policy: String,
    #[serde(default)]
    pub remember: bool,
}

/// POST /api/file/resolve-conflict
pub async fn resolve_conflict(
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<ResolveConflictRequest>,
) -> Json<ApiResponse<()>> {
    use crate::task::{ConflictPolicy, TASK_MANAGER};

    let policy = match req.policy.as_str() {
        "abort" => ConflictPolicy::Abort,
        "skip" => ConflictPolicy::Skip,
        "rename" => ConflictPolicy::Rename,
        "overwrite" => ConflictPolicy::Overwrite,
        _ => return Json(ApiResponse::error(400, "invalid policy")),
    };

    match TASK_MANAGER.get_task(current_user.id, &req.task_id) {
        Some(task) => {
            task.resolve_conflict(policy);
            Json(ApiResponse::success_msg("policy accepted"))
        }
        None => Json(ApiResponse::error(404, "task not found")),
    }
}

#[cfg(test)]
mod tests {
    use super::{get_mime_type, is_safe_filename, is_safe_path};

    #[test]
    fn safe_path_allows_root_and_normal_segments() {
        assert!(is_safe_path(""));
        assert!(is_safe_path("/"));
        assert!(is_safe_path("dir/subdir"));
        assert!(is_safe_path("/dir/subdir"));
    }

    #[test]
    fn safe_path_rejects_traversal_or_dot_segments() {
        assert!(!is_safe_path("../dir"));
        assert!(!is_safe_path("dir/../other"));
        assert!(!is_safe_path("/../dir"));
        assert!(!is_safe_path("dir/./file"));
    }

    #[test]
    fn safe_filename_rejects_invalid_names() {
        assert!(!is_safe_filename(""));
        assert!(!is_safe_filename("."));
        assert!(!is_safe_filename(".."));
        assert!(!is_safe_filename("..."));
        assert!(!is_safe_filename("a/b"));
        assert!(!is_safe_filename(r"a\b"));
        assert!(!is_safe_filename("a:"));
        assert!(!is_safe_filename("a\nb"));
    }

    #[test]
    fn safe_filename_allows_basic_names() {
        assert!(is_safe_filename("file.txt"));
        assert!(is_safe_filename("a-b_c 1.txt"));
    }

    #[test]
    fn mime_type_from_extension() {
        assert_eq!(get_mime_type("photo.jpg"), "image/jpeg");
        assert_eq!(get_mime_type("doc.pdf"), "application/pdf");
        assert_eq!(get_mime_type("unknown.bin"), "application/octet-stream");
    }
}
