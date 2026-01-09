//! Task handlers
//!
//! Implements task management API endpoints

use axum::{
    extract::Query,
    response::Json,
    Extension,
};
use serde::Deserialize;

use crate::middleware::auth::CurrentUser;
use crate::routes::ApiResponse;
use crate::task::{TaskStatus, TASK_MANAGER};

/// Task ID query
#[derive(Debug, Deserialize)]
pub struct TaskIdQuery {
    pub id: Option<String>,
}

/// GET /api/task/query
/// Returns task array directly (no ApiResponse wrapper, matching Go behavior)
pub async fn get_tasks(
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<TaskIdQuery>,
) -> Json<serde_json::Value> {
    if let Some(id) = query.id {
        // Get specific task
        match TASK_MANAGER.get_task(current_user.id, &id) {
            Some(task) => {
                let info = task.info();
                Json(serde_json::to_value(info).unwrap_or_default())
            }
            None => Json(serde_json::json!({"error": "Task is not found"})),
        }
    } else {
        // Get all tasks - return array directly
        let tasks = TASK_MANAGER.get_tasks(current_user.id);
        Json(serde_json::to_value(tasks).unwrap_or(serde_json::json!([])))
    }
}

/// POST /api/task/cancel
pub async fn cancel_task(
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<TaskIdQuery>,
) -> Json<ApiResponse<()>> {
    let id = match query.id {
        Some(id) => id,
        None => return Json(ApiResponse::error(400, "Task ID is required")),
    };

    match TASK_MANAGER.get_task(current_user.id, &id) {
        Some(task) => {
            task.cancel();
            TASK_MANAGER.remove_task(current_user.id, &id);
            Json(ApiResponse::success_msg("Task is cancelled"))
        }
        None => Json(ApiResponse::error(404, "Task is not found")),
    }
}

/// POST /api/task/suspend
pub async fn suspend_task(
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<TaskIdQuery>,
) -> Json<ApiResponse<()>> {
    let id = match query.id {
        Some(id) => id,
        None => return Json(ApiResponse::error(400, "Task ID is required")),
    };

    match TASK_MANAGER.get_task(current_user.id, &id) {
        Some(task) => {
            task.suspend();
            Json(ApiResponse::success_msg("Task is suspended"))
        }
        None => Json(ApiResponse::error(404, "Task is not found")),
    }
}

/// POST /api/task/resume
pub async fn resume_task(
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<TaskIdQuery>,
) -> Json<ApiResponse<()>> {
    let id = match query.id {
        Some(id) => id,
        None => return Json(ApiResponse::error(400, "Task ID is required")),
    };

    match TASK_MANAGER.get_task(current_user.id, &id) {
        Some(task) => {
            task.resume();
            Json(ApiResponse::success_msg("Task is resumed"))
        }
        None => Json(ApiResponse::error(404, "Task is not found")),
    }
}

/// DELETE /api/task/delete
pub async fn delete_task(
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<TaskIdQuery>,
) -> Json<ApiResponse<()>> {
    let id = match query.id {
        Some(id) => id,
        None => return Json(ApiResponse::error(400, "Task ID is required")),
    };

    match TASK_MANAGER.get_task(current_user.id, &id) {
        Some(task) => {
            let info = task.info();
            // Only allow deleting completed, failed, or cancelled tasks
            match info.status {
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                    TASK_MANAGER.remove_task(current_user.id, &id);
                    Json(ApiResponse::success_msg("任务已删除"))
                }
                _ => Json(ApiResponse::error(400, "只能删除已完成、失败或取消的任务")),
            }
        }
        None => Json(ApiResponse::error(404, "Task is not found")),
    }
}
