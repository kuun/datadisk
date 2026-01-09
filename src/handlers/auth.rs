//! Authentication handlers
//!
//! Implements login, logout, and current user endpoints

use axum::{
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::entity::user;
use crate::handlers::audit::service::log_operation;
use crate::middleware::auth::{CurrentUser, SESSION_USER_KEY, SESSION_TIMESTAMP_KEY};
use crate::middleware::DbConn;
use crate::routes::ApiResponse;

/// Operation types for auth
const OP_LOGIN: &str = "登录";
const OP_LOGOUT: &str = "登出";
const OP_SUCCESS: &str = "成功";
const OP_FAILED: &str = "失败";

/// Login request body
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub message: String,
}

/// Login error response (matching Go version)
#[derive(Debug, Serialize)]
pub struct LoginErrorResponse {
    pub error: String,
}

/// Current user response
#[derive(Debug, Serialize)]
pub struct CurrentUserResponse {
    pub id: i64,
    pub username: String,
    pub full_name: String,
    pub department_id: i64,
    pub dept_name: String,
    pub status: i32,
}

/// POST /api/login
pub async fn login(
    Extension(db): Extension<DbConn>,
    session: Session,
    Json(req): Json<LoginRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Validate input
    if req.username.is_empty() || req.password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "bad request"})),
        );
    }

    // Find user in database
    let db = &*db;
    let user_result = user::Entity::find()
        .filter(user::Column::Username.eq(&req.username))
        .one(db)
        .await;

    let db_user = match user_result {
        Ok(Some(user)) => user,
        Ok(None) => {
            tracing::warn!("Login failed: user not found - {}", req.username);
            log_operation(&req.username, OP_LOGIN, "用户不存在", OP_FAILED, None);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "username or password error"})),
            );
        }
        Err(e) => {
            tracing::error!("Database error during login: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal error"})),
            );
        }
    };

    // Verify password using bcrypt
    let password_valid = bcrypt::verify(&req.password, &db_user.password).unwrap_or(false);
    if !password_valid {
        tracing::warn!("Login failed: wrong password - {}", req.username);
        log_operation(&req.username, OP_LOGIN, "密码错误", OP_FAILED, None);
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "username or password error"})),
        );
    }

    // Check user status (2 = disabled)
    if db_user.status == 2 {
        tracing::warn!("Login failed: user disabled - {}", req.username);
        log_operation(&req.username, OP_LOGIN, "用户已禁用", OP_FAILED, None);
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "user is disabled"})),
        );
    }

    // Update last login time
    let now = chrono::Utc::now().timestamp() as i32;
    let mut active_model: user::ActiveModel = db_user.into();
    active_model.last_login = Set(now);
    active_model.status = Set(1); // Set status to active
    if let Err(e) = active_model.update(db).await {
        tracing::error!("Failed to update last login: {}", e);
    }

    // Save session
    if let Err(e) = session.insert(SESSION_USER_KEY, &req.username).await {
        tracing::error!("Failed to save session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "internal error"})),
        );
    }
    if let Err(e) = session.insert(SESSION_TIMESTAMP_KEY, chrono::Utc::now().timestamp()).await {
        tracing::error!("Failed to save session timestamp: {}", e);
    }

    tracing::info!("User logged in: {}", req.username);
    log_operation(&req.username, OP_LOGIN, "", OP_SUCCESS, None);

    (
        StatusCode::OK,
        Json(serde_json::json!({"message": "login success"})),
    )
}

/// POST /api/logout
pub async fn logout(
    session: Session,
    Extension(current_user): Extension<CurrentUser>,
) -> impl IntoResponse {
    let username = current_user.username.clone();

    if let Err(e) = session.flush().await {
        tracing::error!("Failed to flush session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(500, "internal error")),
        );
    }

    log_operation(&username, OP_LOGOUT, "", OP_SUCCESS, None);

    (
        StatusCode::OK,
        Json(ApiResponse::success_msg("logout success")),
    )
}

/// GET /api/user/current
/// Returns user object directly (no ApiResponse wrapper, matching Go behavior)
pub async fn current_user(
    Extension(user): Extension<CurrentUser>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "username": user.username,
        "permissions": user.permissions_string()
    }))
}
