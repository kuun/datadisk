use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// Application error types
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication required")]
    Unauthorized,

    #[error("Access forbidden")]
    Forbidden,

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Payload too large: {0}")]
    PayloadTooLarge(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

/// Error response body
#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message, details) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized", None),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden", None),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "Not Found", Some(msg.clone())),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "Bad Request", Some(msg.clone())),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "Conflict", Some(msg.clone())),
            AppError::PayloadTooLarge(msg) => {
                (StatusCode::PAYLOAD_TOO_LARGE, msg.as_str(), None)
            }
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error", None)
            }
            AppError::Database(err) => {
                tracing::error!("Database error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database Error", None)
            }
            AppError::Io(err) => {
                tracing::error!("IO error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "IO Error", None)
            }
            AppError::Json(err) => {
                (StatusCode::BAD_REQUEST, "Invalid JSON", Some(err.to_string()))
            }
            AppError::Config(msg) => {
                tracing::error!("Config error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Configuration Error", None)
            }
            AppError::Validation(msg) => {
                (StatusCode::BAD_REQUEST, "Validation Error", Some(msg.clone()))
            }
        };

        let body = ErrorResponse {
            code: status.as_u16(),
            message: message.to_string(),
            details,
        };

        (status, Json(body)).into_response()
    }
}

/// Result type alias for application
pub type AppResult<T> = Result<T, AppError>;

/// Helper trait for converting Option to AppError::NotFound
pub trait OptionExt<T> {
    fn ok_or_not_found(self, msg: impl Into<String>) -> AppResult<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_not_found(self, msg: impl Into<String>) -> AppResult<T> {
        self.ok_or_else(|| AppError::NotFound(msg.into()))
    }
}

/// Helper to convert anyhow errors to AppError
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response() {
        let err = AppError::NotFound("User not found".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_option_ext() {
        let opt: Option<i32> = None;
        let result = opt.ok_or_not_found("Item not found");
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
