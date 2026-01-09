//! Configuration handlers
//!
//! Returns public configuration settings to the frontend

use axum::{extract::State, response::Json};
use serde::Serialize;

use crate::state::AppState;

/// Public configuration response
#[derive(Debug, Serialize)]
pub struct PublicConfig {
    /// Maximum upload file size in bytes
    #[serde(rename = "maxUploadSize")]
    pub max_upload_size: usize,
}

/// GET /api/config
/// Returns public configuration settings
pub async fn get_config(State(state): State<AppState>) -> Json<PublicConfig> {
    Json(PublicConfig {
        max_upload_size: state.config.max_upload_size,
    })
}
