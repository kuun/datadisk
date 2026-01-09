use axum::{extract::State, response::Json};
use serde::Serialize;

use crate::state::AppState;
use super::ApiResponse;

#[derive(Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct SetupStatus {
    pub initialized: bool,
}

/// Health check endpoint
pub async fn health_check() -> Json<ApiResponse<HealthStatus>> {
    Json(ApiResponse::success(HealthStatus {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

/// Check if system is initialized
/// Returns {"initialized": bool} directly (no ApiResponse wrapper, matching Go behavior)
/// Note: We check the file directly instead of state.config.initialized because
/// the config is loaded at startup and won't reflect runtime changes during setup
pub async fn setup_status(State(state): State<AppState>) -> Json<SetupStatus> {
    // Check sys_inited file directly to reflect runtime changes
    let inited_path = state.config.config_dir.join("sys_inited");
    Json(SetupStatus {
        initialized: inited_path.exists(),
    })
}
