//! Recent files handlers
//!
//! Implements recently accessed files management

use axum::{
    extract::{Path, Query},
    response::Json,
    Extension,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, ActiveModelTrait, Set, PaginatorTrait};
use serde::{Deserialize, Serialize};

use crate::entity::{file_access, file_info};
use crate::middleware::auth::CurrentUser;
use crate::middleware::DbConn;

/// Recent files query parameters
#[derive(Debug, Deserialize)]
pub struct RecentQuery {
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    20
}

/// Recent file item response (matching Go structure)
#[derive(Debug, Serialize)]
pub struct RecentFileItem {
    pub id: i64,
    #[serde(rename = "fileId")]
    pub file_id: i64,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "accessTime")]
    pub access_time: i64,
    #[serde(rename = "accessType")]
    pub access_type: String,
    #[serde(rename = "isDir")]
    pub is_dir: bool,
    #[serde(rename = "fileInfo")]
    pub file_info: Option<FileInfoResponse>,
}

/// File info for recent file response
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
            file_type: m.file_type,
            size: m.size,
            is_directory: m.is_directory,
            create_time: m.create_time,
            modify_time: m.modify_time,
            parent_id: m.parent_id,
            username: m.username,
        }
    }
}

/// Recent files list response
#[derive(Debug, Serialize)]
pub struct RecentFilesResponse {
    pub files: Vec<RecentFileItem>,
}

/// GET /api/file/recent - Get recently accessed files
pub async fn get_recent_files(
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<RecentQuery>,
) -> Json<RecentFilesResponse> {
    let db = &*db;

    // Cap limit at 100
    let limit = query.limit.min(100);

    // Get recent file access records for the user
    let recent_access = file_access::Entity::find()
        .filter(file_access::Column::UserId.eq(current_user.id))
        .order_by_desc(file_access::Column::AccessTime)
        .all(db)
        .await;

    let recent_access = match recent_access {
        Ok(records) => records,
        Err(e) => {
            tracing::error!("Failed to get recent files: {}", e);
            return Json(RecentFilesResponse { files: vec![] });
        }
    };

    let mut result = Vec::new();

    // Build response with file details
    for access in recent_access.into_iter().take(limit as usize) {
        // Verify file still exists in database
        let file_info = file_info::Entity::find_by_id(access.file_id)
            .one(db)
            .await;

        if let Ok(Some(file)) = file_info {
            result.push(RecentFileItem {
                id: access.id,
                file_id: access.file_id,
                file_path: access.file_path,
                file_name: access.file_name,
                access_time: access.access_time,
                access_type: access.access_type,
                is_dir: access.is_dir,
                file_info: Some(file.into()),
            });
        }
    }

    Json(RecentFilesResponse { files: result })
}

/// DELETE /api/file/recent - Clear all recent files for current user
pub async fn clear_recent_files(
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
) -> Json<serde_json::Value> {
    let db = &*db;

    let result = file_access::Entity::delete_many()
        .filter(file_access::Column::UserId.eq(current_user.id))
        .exec(db)
        .await;

    match result {
        Ok(_) => Json(serde_json::json!({"message": "recent files cleared"})),
        Err(e) => {
            tracing::error!("Failed to clear recent files: {}", e);
            Json(serde_json::json!({"error": "failed to clear recent files"}))
        }
    }
}

/// DELETE /api/file/recent/:id - Delete a specific recent file record
pub async fn delete_recent_file(
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> Json<serde_json::Value> {
    let db = &*db;

    // Verify the record exists and belongs to the current user
    let access = file_access::Entity::find_by_id(id)
        .filter(file_access::Column::UserId.eq(current_user.id))
        .one(db)
        .await;

    match access {
        Ok(Some(_)) => {
            // Delete the record
            let result = file_access::Entity::delete_by_id(id)
                .exec(db)
                .await;

            match result {
                Ok(_) => Json(serde_json::json!({"message": "recent file record deleted"})),
                Err(e) => {
                    tracing::error!("Failed to delete recent file record: {}", e);
                    Json(serde_json::json!({"error": "failed to delete recent file record"}))
                }
            }
        }
        Ok(None) => Json(serde_json::json!({"error": "recent file record not found"})),
        Err(e) => {
            tracing::error!("Failed to find recent file record: {}", e);
            Json(serde_json::json!({"error": "failed to find recent file record"}))
        }
    }
}

/// Record file access (for download/preview/edit)
pub async fn record_file_access(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    file_id: i64,
    file_path: &str,
    file_name: &str,
    access_type: &str,
    is_dir: bool,
) {
    // Only record downloads, previews, and edits
    if access_type != "download" && access_type != "preview" && access_type != "edit" {
        return;
    }

    let now = chrono::Utc::now().timestamp();

    // Check if this file already exists in recent access list
    let existing = file_access::Entity::find()
        .filter(file_access::Column::UserId.eq(user_id))
        .filter(file_access::Column::FileId.eq(file_id))
        .one(db)
        .await;

    match existing {
        Ok(Some(record)) => {
            // Update existing record's access time
            let mut active: file_access::ActiveModel = record.into();
            active.access_time = Set(now);
            active.access_type = Set(access_type.to_string());
            if let Err(e) = active.update(db).await {
                tracing::error!("Failed to update file access record: {}", e);
            }
        }
        Ok(None) => {
            // Create new record
            let access = file_access::ActiveModel {
                user_id: Set(user_id),
                file_id: Set(file_id),
                file_path: Set(file_path.to_string()),
                file_name: Set(file_name.to_string()),
                access_time: Set(now),
                access_type: Set(access_type.to_string()),
                is_dir: Set(is_dir),
                ..Default::default()
            };

            if let Err(e) = access.insert(db).await {
                tracing::error!("Failed to record file access: {}", e);
                return;
            }

            // Check if user has more than 50 recent access records
            let count = file_access::Entity::find()
                .filter(file_access::Column::UserId.eq(user_id))
                .count(db)
                .await;

            if let Ok(count) = count {
                if count > 50 {
                    // Find and delete the oldest records to keep only 50
                    let oldest_records = file_access::Entity::find()
                        .filter(file_access::Column::UserId.eq(user_id))
                        .order_by_asc(file_access::Column::AccessTime)
                        .all(db)
                        .await;

                    if let Ok(records) = oldest_records {
                        let to_delete = count - 50;
                        for record in records.into_iter().take(to_delete as usize) {
                            if let Err(e) = file_access::Entity::delete_by_id(record.id)
                                .exec(db)
                                .await
                            {
                                tracing::error!("Failed to delete old access record: {}", e);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to check existing file access: {}", e);
        }
    }
}
