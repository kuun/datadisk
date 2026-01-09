use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    middleware,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde::Serialize;
use tower_http::{
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tower_sessions::{MemoryStore, SessionManagerLayer};

use crate::handlers;
use crate::middleware::auth_layer;
use crate::state::AppState;
use crate::ws;

pub mod health;

/// API response wrapper
#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: true,
            message: "success".to_string(),
            data: Some(data),
        }
    }

    pub fn error(_code: i32, message: impl Into<String>) -> Self {
        Self {
            code: false,
            message: message.into(),
            data: None,
        }
    }
}

impl ApiResponse<()> {
    pub fn success_msg(message: impl Into<String>) -> Self {
        Self {
            code: true,
            message: message.into(),
            data: None,
        }
    }
}

/// Create the main router
pub fn create_router(state: AppState) -> Router {
    // Session store (in-memory for now)
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // Set to true in production with HTTPS
        .with_http_only(true);

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // API routes
    let api_routes = Router::new()
        // Health check
        .route("/health", get(health::health_check))
        // Setup routes
        .route("/setup/status", get(health::setup_status))
        .route("/setup/test-db", post(handlers::setup::test_db_connection))
        .route("/setup/init/db", post(handlers::setup::init_db))
        .route("/setup/init/user", post(handlers::setup::init_user))
        // Auth routes
        .route("/login", post(handlers::auth::login))
        .route("/logout", post(handlers::auth::logout))
        .route("/user/current", get(handlers::auth::current_user))
        // Config routes
        .route("/config", get(handlers::config::get_config))
        // Department routes
        .route("/departments/add", post(handlers::department::add_department))
        .route("/department/delete", post(handlers::department::delete_department))
        .route("/department/update", post(handlers::department::update_department))
        .route("/department/query", get(handlers::department::get_departments))
        .route("/department/query/all", get(handlers::department::get_dept_and_users))
        // User routes
        .route("/user/add", post(handlers::user::add_user))
        .route("/user/delete", post(handlers::user::delete_user))
        .route("/user/update", post(handlers::user::update_user))
        .route("/user/info", get(handlers::user::get_user_by_username))
        .route("/user/query", get(handlers::user::get_users_by_dept))
        .route("/user/enable", post(handlers::user::enable_user))
        .route("/user/disable", post(handlers::user::disable_user))
        .route("/user/change-password", post(handlers::user::change_password))
        .route("/user/reset-password", post(handlers::user::reset_password))
        // Avatar routes
        .route("/user/avatar/:username", get(handlers::user::get_user_avatar))
        .route("/user/upload/avatar", post(handlers::user::upload_user_avatar))
        .route("/user/avatar/:username", delete(handlers::user::delete_user_avatar))
        // Group routes
        .route("/group/add", post(handlers::group::add_group))
        .route("/group/delete", post(handlers::group::delete_group))
        .route("/group/query", get(handlers::group::get_groups))
        .route("/group/addUsers", post(handlers::group::add_users_to_group))
        .route("/group/deleteUsers", post(handlers::group::delete_users_from_group))
        .route("/group/query/users", get(handlers::group::get_group_users))
        // Role routes
        .route("/role/add", post(handlers::role::add_role))
        .route("/role/delete", post(handlers::role::delete_role))
        .route("/role/update", post(handlers::role::update_role))
        .route("/role/list", get(handlers::role::get_roles))
        .route("/role/permissions", get(handlers::role::get_available_permissions))
        // File routes
        .route("/file/mkdir", post(handlers::file::mkdir))
        .route("/file/remove/file", post(handlers::file::remove_file))
        .route("/file/query/files", get(handlers::file::get_files))
        .route(
            "/file/upload",
            post(handlers::file::upload_file)
                .layer(DefaultBodyLimit::max(state.config.max_upload_size)),
        )
        .route("/file/download", get(handlers::file::download_file))
        .route("/file/download/pre", post(handlers::file::download_pre))
        .route("/file/list", get(handlers::file::list_directory))
        .route("/file/rename", post(handlers::file::rename_file))
        .route("/file/content", get(handlers::file::get_file_content))
        .route("/file/delete", post(handlers::file::delete_files))
        .route("/file/download/single", get(handlers::file::download_single_file))
        .route("/file/preview/single", get(handlers::file::preview_single_file))
        .route("/file/copy", post(handlers::file::copy_move_file))
        .route("/file/resolve-conflict", post(handlers::file::resolve_conflict))
        // Archive preview
        .route("/archive/preview", get(handlers::archive_preview::archive_preview))
        // Recent files routes
        .route("/file/recent", get(handlers::recent::get_recent_files))
        .route("/file/recent", delete(handlers::recent::clear_recent_files))
        .route("/file/recent/:id", delete(handlers::recent::delete_recent_file))
        // Task routes
        .route("/task/query", get(handlers::task::get_tasks))
        .route("/task/cancel", post(handlers::task::cancel_task))
        .route("/task/suspend", post(handlers::task::suspend_task))
        .route("/task/resume", post(handlers::task::resume_task))
        .route("/task/delete", delete(handlers::task::delete_task))
        // Audit log routes
        .route("/oplog/query", get(handlers::audit::query_oplog))
        .route("/oplog/delete", post(handlers::audit::delete_oplog))
        // Document editing routes (OnlyOffice integration)
        .route("/editing/create", post(handlers::editing::create_editing_session))
        .route("/editing/save/:sessionId", post(handlers::editing::save_editing_session))
        .route("/editing/download/:sessionId", get(handlers::editing::get_editing_session))
        .route("/editing/query", get(handlers::editing::get_editing_session_info))
        // WebSocket
        .route("/ws", get(ws::serve_ws));

    // Note: upload route has custom DefaultBodyLimit from config.max_upload_size
    // The upload handler streams large files and returns user-friendly error messages

    // Static file service for frontend
    // Serves files from webapp/dist, falls back to index.html for SPA routing
    let static_dir = "webapp/dist";
    let index_file = format!("{}/index.html", static_dir);
    let serve_dir = ServeDir::new(static_dir)
        .not_found_service(ServeFile::new(&index_file));

    Router::new()
        .nest("/api", api_routes)
        .fallback_service(serve_dir)
        .layer(middleware::from_fn_with_state(state.clone(), auth_layer))
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

/// Fallback handler for 404
pub async fn fallback() -> (StatusCode, Json<ApiResponse<()>>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiResponse::error(404, "Not Found")),
    )
}
