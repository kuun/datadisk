//! Authentication middleware
//!
//! Provides session-based authentication for API routes

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use serde_json::json;
use std::ops::Deref;
use tower_sessions::Session;

use crate::entity::user;
use crate::state::AppState;

/// Session key for storing username
pub const SESSION_USER_KEY: &str = "user";
pub const SESSION_TIMESTAMP_KEY: &str = "timestamp";

/// Database connection wrapper for use in handlers via Extension
#[derive(Clone)]
pub struct DbConn(pub DatabaseConnection);

impl Deref for DbConn {
    type Target = DatabaseConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub use crate::permission::perm;

/// Extension to store current user in request
#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub id: i64,
    pub username: String,
    pub full_name: String,
    pub email: String,
    pub department_id: i64,
    pub dept_name: String,
    pub status: i32,
    /// Permissions loaded from Casbin (comma-separated for API compatibility)
    pub permissions: Vec<String>,
}

impl CurrentUser {
    /// Check if the user has a specific permission
    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions.iter().any(|p| p == perm)
    }

    /// Check if the user has file permission
    pub fn can_file(&self) -> bool {
        self.has_permission(perm::FILE)
    }

    /// Check if the user has contacts permission
    pub fn can_contacts(&self) -> bool {
        self.has_permission(perm::CONTACTS)
    }

    /// Check if the user has group permission
    pub fn can_group(&self) -> bool {
        self.has_permission(perm::GROUP)
    }

    /// Check if the user has audit permission
    pub fn can_audit(&self) -> bool {
        self.has_permission(perm::AUDIT)
    }

    /// Check if the user has all permissions
    pub fn has_all_permissions(&self) -> bool {
        perm::ALL.iter().all(|p: &&str| self.permissions.contains(&p.to_string()))
    }

    /// Get permissions as comma-separated string (for API response)
    pub fn permissions_string(&self) -> String {
        self.permissions.join(",")
    }
}

/// Paths that don't require authentication
fn is_public_path(path: &str) -> bool {
    // Only authenticate API routes (except public ones)
    // All non-API routes are static files and should be public
    if !path.starts_with("/api") {
        return true;
    }

    // Public API endpoints
    if path == "/api/login" || path == "/api/logout" {
        return true;
    }
    // Setup endpoints
    if path == "/api/setup/status"
        || path == "/api/setup/test-db"
        || path == "/api/setup/init/db"
        || path == "/api/setup/init/user" {
        return true;
    }
    // Health check
    if path == "/api/health" {
        return true;
    }
    // OnlyOffice editing callbacks
    if path.starts_with("/api/editing/save/") || path.starts_with("/api/editing/download/") {
        return true;
    }
    false
}

/// Authentication middleware
pub async fn auth_layer(
    State(state): State<AppState>,
    session: Session,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();

    // Try to get database connection and add to extensions if available
    // This allows all handlers to access db via Extension<DbConn>
    if let Some(db) = state.get_db().await {
        request.extensions_mut().insert(DbConn(db.clone()));
    }

    // Skip auth for public paths
    if is_public_path(&path) {
        return next.run(request).await;
    }

    // Get username from session
    let username: Option<String> = session.get(SESSION_USER_KEY).await.unwrap_or(None);

    let Some(username) = username else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "unauthorized"})),
        ).into_response();
    };

    // Check if database is initialized (get from extension we just set)
    let Some(db_conn) = request.extensions().get::<DbConn>() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "system_not_initialized"})),
        ).into_response();
    };

    // Look up user in database
    let user_result = user::Entity::find()
        .filter(user::Column::Username.eq(&username))
        .one(&**db_conn)
        .await;

    match user_result {
        Ok(Some(user_model)) => {
            // Get user permissions from Casbin
            let permissions = if let Some(perm_enforcer) = state.get_perm().await.as_ref() {
                perm_enforcer.get_user_permissions(&user_model.username).await
            } else {
                Vec::new()
            };

            // Create CurrentUser extension
            let current_user = CurrentUser {
                id: user_model.id,
                username: user_model.username,
                full_name: user_model.full_name,
                email: user_model.email.unwrap_or_default(),
                department_id: user_model.department_id,
                dept_name: user_model.dept_name,
                status: user_model.status,
                permissions,
            };

            // Insert into request extensions
            request.extensions_mut().insert(current_user);

            next.run(request).await
        }
        Ok(None) => {
            tracing::warn!("User not found in database: {}", username);
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "invalid_session"})),
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Database error during auth: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "internal error"})),
            ).into_response()
        }
    }
}

