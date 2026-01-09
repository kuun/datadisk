//! Audit log handlers
//!
//! Implements operation log query and management

use axum::{
    extract::Query,
    response::Json,
    Extension,
};
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};

use crate::entity::op_log;
use crate::middleware::auth::CurrentUser;
use crate::middleware::DbConn;
use crate::routes::ApiResponse;

/// Query parameters for log pagination
#[derive(Debug, Deserialize)]
pub struct LogQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(rename = "pageSize", default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    10
}

/// Log response
#[derive(Debug, Serialize)]
pub struct LogResponse {
    pub id: i64,
    #[serde(rename = "opTime")]
    pub op_time: i64,
    pub username: String,
    #[serde(rename = "opType")]
    pub op_type: String,
    #[serde(rename = "opDesc")]
    pub op_desc: String,
    #[serde(rename = "oldValue")]
    pub old_value: String,
    pub result: String,
    pub ip: String,
}

impl From<op_log::Model> for LogResponse {
    fn from(m: op_log::Model) -> Self {
        Self {
            id: m.id,
            op_time: m.op_time,
            username: m.username,
            op_type: m.op_type,
            op_desc: m.op_desc,
            old_value: m.old_value.unwrap_or_default(),
            result: m.result,
            ip: m.ip.unwrap_or_default(),
        }
    }
}

/// Query response with pagination
#[derive(Debug, Serialize)]
pub struct LogQueryResponse {
    pub logs: Vec<LogResponse>,
    pub total: u64,
}

/// Check if user has audit permission
fn can_view_audit(user: &CurrentUser) -> bool {
    user.can_audit()
}

/// GET /api/oplog/query
pub async fn query_oplog(
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<LogQuery>,
) -> Json<LogQueryResponse> {
    // Permission check: only admin can view audit logs
    if !can_view_audit(&current_user) {
        return Json(LogQueryResponse { logs: vec![], total: 0 });
    }

    let db = &*db;
    let page = query.page.max(1) as u64;
    let page_size = query.page_size.max(1).min(100) as u64;
    let offset = (page - 1) * page_size;

    // Query logs with pagination
    let result = op_log::Entity::find()
        .order_by_desc(op_log::Column::Id)
        .offset(offset)
        .limit(page_size)
        .all(db)
        .await;

    let logs = match result {
        Ok(logs) => logs.into_iter().map(|l| l.into()).collect(),
        Err(e) => {
            tracing::error!("Failed to query logs: {}", e);
            return Json(LogQueryResponse { logs: vec![], total: 0 });
        }
    };

    // Get total count
    let total = match op_log::Entity::find().count(db).await {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to count logs: {}", e);
            return Json(LogQueryResponse { logs, total: 0 });
        }
    };

    Json(LogQueryResponse { logs, total })
}

/// POST /api/oplog/delete
pub async fn delete_oplog(
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(ids): Json<Vec<i64>>,
) -> Json<ApiResponse<()>> {
    // Permission check: only admin can delete audit logs
    if !can_view_audit(&current_user) {
        return Json(ApiResponse::error(403, "权限不足，仅管理员可删除审计日志"));
    }

    if ids.is_empty() {
        return Json(ApiResponse::error(400, "No IDs provided"));
    }

    // Delete logs
    let result = op_log::Entity::delete_many()
        .filter(op_log::Column::Id.is_in(ids))
        .exec(&*db)
        .await;

    match result {
        Ok(res) => {
            let message = format!("成功删除{}条日志", res.rows_affected);
            Json(ApiResponse::success_msg(message))
        }
        Err(e) => {
            tracing::error!("Failed to delete logs: {}", e);
            Json(ApiResponse::error(500, "Failed to delete logs"))
        }
    }
}

/// Service for adding operation logs
pub mod service {
    use sea_orm::{ActiveModelTrait, Set};
    use tokio::sync::mpsc;

    use crate::entity::op_log;

    /// Log entry to be added
    #[derive(Debug, Clone)]
    pub struct LogEntry {
        pub username: String,
        pub op_type: String,
        pub op_desc: String,
        pub old_value: Option<String>,
        pub result: String,
        pub ip: Option<String>,
    }

    /// Global log channel
    static LOG_TX: std::sync::OnceLock<mpsc::Sender<LogEntry>> = std::sync::OnceLock::new();

    /// Initialize the audit log service
    /// This function is idempotent - calling it multiple times is safe
    pub fn init(db: sea_orm::DatabaseConnection) {
        // If already initialized, skip
        if LOG_TX.get().is_some() {
            tracing::debug!("Audit log service already initialized, skipping");
            return;
        }

        let (tx, mut rx) = mpsc::channel::<LogEntry>(200);
        if LOG_TX.set(tx).is_err() {
            // Another thread initialized it first, that's fine
            tracing::debug!("Audit log service initialized by another thread");
            return;
        }

        // Spawn background task to process log entries
        tokio::spawn(async move {
            while let Some(entry) = rx.recv().await {
                let now = chrono::Utc::now().timestamp();
                let log = op_log::ActiveModel {
                    op_time: Set(now),
                    username: Set(entry.username),
                    op_type: Set(entry.op_type),
                    op_desc: Set(entry.op_desc),
                    old_value: Set(entry.old_value),
                    result: Set(entry.result),
                    ip: Set(entry.ip),
                    ..Default::default()
                };

                if let Err(e) = log.insert(&db).await {
                    tracing::error!("Failed to log operation: {}", e);
                }
            }
        });
    }

    /// Add an operation log entry
    pub fn add_log(entry: LogEntry) {
        if let Some(tx) = LOG_TX.get() {
            if tx.try_send(entry).is_err() {
                tracing::warn!("Log channel is full, operation log dropped");
            }
        } else {
            tracing::warn!("Audit log service not initialized, log dropped: {} - {}", entry.op_type, entry.op_desc);
        }
    }

    /// Helper function to create a log entry from request context
    pub fn log_operation(
        username: &str,
        op_type: &str,
        op_desc: &str,
        result: &str,
        ip: Option<&str>,
    ) {
        add_log(LogEntry {
            username: username.to_string(),
            op_type: op_type.to_string(),
            op_desc: op_desc.to_string(),
            old_value: None,
            result: result.to_string(),
            ip: ip.map(|s| s.to_string()),
        });
    }
}
