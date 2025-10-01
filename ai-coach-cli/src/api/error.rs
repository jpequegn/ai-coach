use reqwest::StatusCode;
use thiserror::Error;

/// API-specific errors
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Not authorized: {0}")]
    Unauthorized(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl ApiError {
    pub fn from_status(status: StatusCode, message: String) -> Self {
        let msg = if message.is_empty() {
            status.canonical_reason().unwrap_or("Unknown error").to_string()
        } else {
            message
        };

        match status {
            StatusCode::UNAUTHORIZED => ApiError::Unauthorized(msg),
            StatusCode::FORBIDDEN => ApiError::Unauthorized(msg),
            StatusCode::NOT_FOUND => ApiError::NotFound(msg),
            StatusCode::BAD_REQUEST => ApiError::BadRequest(msg),
            status if status.is_server_error() => ApiError::ServerError(msg),
            status if status.is_client_error() => ApiError::BadRequest(msg),
            _ => ApiError::Unknown(msg),
        }
    }
}
