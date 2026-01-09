//! Document editing handlers (OnlyOffice integration)
//!
//! Implements document editing session management for OnlyOffice integration

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use dashmap::DashMap;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::LazyLock;
use tokio::fs;

use crate::entity::file_info;
use crate::handlers::recent::record_file_access;
use crate::middleware::auth::CurrentUser;
use crate::middleware::DbConn;
use crate::state::AppState;

/// Global editing sessions storage
static EDITING_SESSIONS: LazyLock<DashMap<String, EditingSession>> =
    LazyLock::new(DashMap::new);

/// Editing session information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditingSession {
    pub session_id: String,
    pub created_at: i64,
    pub file_path: String,
    #[serde(skip)]
    pub abs_file_path: PathBuf,
    #[serde(skip)]
    pub file_size: i64,
    pub content_type: String,
    pub token: String,
    #[serde(skip)]
    pub user_id: i64,
    pub user_name: String,
    pub full_name: String,
    pub display_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub doc_server_url: String,
    pub datadisk_url: String,
}

/// Create session request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    pub file_path: String,
}

/// Document status from OnlyOffice callback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[repr(i32)]
pub enum DocStatus {
    Edited = 1,
    ReadyForSave = 2,
    SaveWithError = 3,
    ClosedNoChanges = 4,
    BeingEditedSaved = 6,
    ForceSaveWithError = 7,
}

/// Action from OnlyOffice callback
#[derive(Debug, Deserialize)]
pub struct Action {
    #[serde(rename = "type")]
    pub action_type: i32,
    #[serde(rename = "userid")]
    pub user_id: String,
}

/// OnlyOffice callback request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallbackRequest {
    pub key: String,
    pub status: i32,
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub users: Vec<String>,
    #[serde(default)]
    pub changes_url: String,
    #[serde(default)]
    pub file_type: String,
    #[serde(default)]
    pub forcesave_type: i32,
    #[serde(default)]
    pub token: String,
}

/// JWT claims for OnlyOffice token (matches Go version - no exp field)
#[derive(Debug, Serialize, Deserialize)]
pub struct DocJwtClaims {
    pub document: DocumentClaims,
    #[serde(rename = "editorConfig")]
    pub editor_config: EditorConfigClaims,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentClaims {
    pub key: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorConfigClaims {
    pub callback_url: String,
    pub mode: String,
}

/// Query session request
#[derive(Debug, Deserialize)]
pub struct QuerySessionRequest {
    pub session: String,
}

/// Generate consistent session ID based on file path
fn generate_session_id(abs_file_path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(abs_file_path.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..16]) // Use first 16 bytes (32 hex chars)
}

/// Get content type based on file extension
fn get_content_type(file_path: &str) -> String {
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext.to_lowercase().as_str() {
        "doc" | "docx" => "application/msword".to_string(),
        "xls" | "xlsx" => "application/vnd.ms-excel".to_string(),
        "ppt" | "pptx" => "application/vnd.ms-powerpoint".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

/// Choose display name (prefer full name over username)
fn choose_display_name(full_name: &str, user_name: &str) -> String {
    if !full_name.is_empty() {
        full_name.to_string()
    } else {
        user_name.to_string()
    }
}

/// Get user's file storage path
fn get_user_path(config: &crate::config::Config, username: &str) -> PathBuf {
    config.root_dir.join(username)
}

/// Resolve file ID from path
async fn resolve_file_id(
    db: &sea_orm::DatabaseConnection,
    username: &str,
    file_path: &str,
) -> i64 {
    let cleaned = file_path.trim_matches('/');
    if cleaned.is_empty() {
        return 0;
    }

    let parts: Vec<&str> = cleaned.split('/').collect();
    let mut parent_id: i64 = -1;
    let mut file_id: i64 = 0;

    for (idx, part) in parts.iter().enumerate() {
        let file = file_info::Entity::find()
            .filter(file_info::Column::ParentId.eq(parent_id))
            .filter(file_info::Column::Username.eq(username))
            .filter(file_info::Column::Name.eq(*part))
            .one(db)
            .await;

        match file {
            Ok(Some(f)) => {
                if idx < parts.len() - 1 {
                    if !f.is_directory {
                        return 0;
                    }
                    parent_id = f.id;
                } else {
                    file_id = f.id;
                }
            }
            _ => return 0,
        }
    }

    file_id
}

/// Sign JWT token for OnlyOffice
fn sign_jwt(claims: &DocJwtClaims, secret: &str) -> Result<String, String> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| format!("Failed to sign JWT: {}", e))
}

/// Verify JWT token from OnlyOffice
fn verify_jwt(token: &str, secret: &str) -> Result<(), String> {
    // Handle "Bearer " prefix
    let token = if token.starts_with("Bearer ") {
        &token[7..]
    } else {
        token
    };

    // Use HS256 algorithm and disable exp validation
    // Parse as generic Value since OnlyOffice token structure varies between requests
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = false;
    validation.required_spec_claims.clear();

    decode::<serde_json::Value>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|_| ())
    .map_err(|e| format!("Failed to verify JWT: {}", e))
}

/// POST /api/editing/create
/// Creates a new editing session or returns existing one
pub async fn create_editing_session(
    State(state): State<AppState>,
    Extension(db): Extension<DbConn>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let user_path = get_user_path(&state.config, &current_user.username);
    let abs_file_path = user_path.join(req.file_path.trim_start_matches('/'));

    // Check if file exists
    let file_info = match fs::metadata(&abs_file_path).await {
        Ok(info) => info,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "file not found"})),
            )
                .into_response();
        }
    };

    // Generate consistent session ID based on absolute path for collaboration
    let session_id = generate_session_id(&abs_file_path.to_string_lossy());

    // Record file access for document editing
    let file_name = std::path::Path::new(&req.file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let file_id = resolve_file_id(&*db, &current_user.username, &req.file_path).await;
    let clean_path = format!("/{}", req.file_path.trim_start_matches('/'));

    if file_id > 0 {
        record_file_access(
            &*db,
            current_user.id,
            file_id,
            &clean_path,
            &file_name,
            "edit",
            false,
        ).await;
    }

    // Check if session already exists
    if let Some(existing) = EDITING_SESSIONS.get(&session_id) {
        tracing::info!(
            "Returning existing session: {} for file: {} by user: {}",
            session_id,
            req.file_path,
            current_user.username
        );
        return Json(existing.clone()).into_response();
    }

    // Create JWT token for OnlyOffice
    let doc_config = &state.config.doc;
    tracing::info!(
        "Doc config: doc_server_url={}, datadisk_url={}",
        doc_config.doc_server_url,
        doc_config.datadisk_url
    );

    if doc_config.doc_server_url.is_empty() || doc_config.datadisk_url.is_empty() {
        tracing::warn!("Doc config is not properly configured");
    }

    let claims = DocJwtClaims {
        document: DocumentClaims {
            key: session_id.clone(),
            url: format!("{}/api/editing/download/{}", doc_config.datadisk_url, session_id),
        },
        editor_config: EditorConfigClaims {
            callback_url: format!("{}/api/editing/save/{}", doc_config.datadisk_url, session_id),
            mode: "edit".to_string(),
        },
    };

    let token = match sign_jwt(&claims, &doc_config.doc_secret) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to generate token: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "failed to create session"})),
            )
                .into_response();
        }
    };

    let session = EditingSession {
        session_id: session_id.clone(),
        created_at: chrono::Utc::now().timestamp(),
        file_path: req.file_path.clone(),
        abs_file_path: abs_file_path.clone(),
        file_size: file_info.len() as i64,
        content_type: get_content_type(&req.file_path),
        token,
        user_id: current_user.id,
        user_name: current_user.username.clone(),
        full_name: current_user.full_name.clone(),
        display_name: choose_display_name(&current_user.full_name, &current_user.username),
        first_name: String::new(),
        last_name: String::new(),
        email: current_user.email.clone(),
        doc_server_url: doc_config.doc_server_url.clone(),
        datadisk_url: doc_config.datadisk_url.clone(),
    };

    EDITING_SESSIONS.insert(session_id.clone(), session.clone());

    tracing::info!(
        "Created editing session: {} for file: {} by user: {}",
        session_id,
        req.file_path,
        current_user.username
    );

    Json(session).into_response()
}

/// GET /api/editing/download/:sessionId
/// Download file for OnlyOffice document server
pub async fn get_editing_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // Verify JWT token from Authorization header
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Err(e) = verify_jwt(auth_header, &state.config.doc.doc_secret) {
        tracing::error!("JWT verification failed: {}", e);
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "unauthorized"})),
        )
            .into_response();
    }

    // Get session
    let session = match EDITING_SESSIONS.get(&session_id) {
        Some(s) => s.clone(),
        None => {
            tracing::error!("Session not found: {}", session_id);
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "unauthorized"})),
            )
                .into_response();
        }
    };

    // Read file
    let file_content = match fs::read(&session.abs_file_path).await {
        Ok(content) => content,
        Err(e) => {
            tracing::error!("Failed to read file: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "failed to read file"})),
            )
                .into_response();
        }
    };

    let filename = session.abs_file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &session.content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", filename),
        )
        .body(Body::from(file_content))
        .unwrap()
        .into_response()
}

/// POST /api/editing/save/:sessionId
/// OnlyOffice callback for saving document
pub async fn save_editing_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(callback): Json<CallbackRequest>,
) -> impl IntoResponse {
    // Verify JWT token from Authorization header (same as Go version)
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Err(e) = verify_jwt(auth_header, &state.config.doc.doc_secret) {
        tracing::error!("JWT verification failed: {}", e);
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "unauthorized"})),
        );
    }

    // Get session
    let session = match EDITING_SESSIONS.get(&session_id) {
        Some(s) => s.clone(),
        None => {
            tracing::error!("Session not found: {}", session_id);
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "unauthorized"})),
            );
        }
    };

    tracing::debug!(
        "Handle file {} save callback request: {:?}",
        session.abs_file_path.display(),
        callback
    );

    // Handle save based on status
    let status = callback.status;
    if status == 2 || status == 6 || status == 3 || status == 7 {
        // ReadyForSave, BeingEditedSaved, SaveWithError, ForceSaveWithError
        if let Err(e) = on_save(&callback, &session).await {
            tracing::error!("Failed to save file: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("failed to save file: {}", e)})),
            );
        }
    }

    (StatusCode::OK, Json(serde_json::json!({"error": 0})))
}

/// Handle document save from OnlyOffice
async fn on_save(callback: &CallbackRequest, session: &EditingSession) -> Result<(), String> {
    if callback.url.is_empty() {
        return Err("No download URL provided".to_string());
    }

    // Create temp directory
    let tmp_dir = std::env::temp_dir().join("datadisk_editing");
    fs::create_dir_all(&tmp_dir).await.map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let tmp_path = tmp_dir.join(&callback.key);

    // Download file from OnlyOffice
    let response = reqwest::get(&callback.url)
        .await
        .map_err(|e| format!("Failed to download file: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // Write to temp file
    fs::write(&tmp_path, &bytes)
        .await
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    // Move temp file to target location
    fs::rename(&tmp_path, &session.abs_file_path)
        .await
        .map_err(|e| format!("Failed to save file: {}", e))?;

    tracing::info!("Successfully saved file {}", session.abs_file_path.display());
    Ok(())
}

/// GET /api/editing/query
/// Query editing session info
pub async fn get_editing_session_info(
    Query(query): Query<QuerySessionRequest>,
) -> impl IntoResponse {
    match EDITING_SESSIONS.get(&query.session) {
        Some(session) => (StatusCode::OK, Json(serde_json::to_value(session.clone()).unwrap())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "session not found"})),
        ),
    }
}
