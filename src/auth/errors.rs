use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("User not found")]
    UserNotFound,
    #[error("Email already exists")]
    EmailAlreadyExists,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token expired")]
    TokenExpired,
    #[error("Missing authorization header")]
    MissingAuthHeader,
    #[error("Invalid authorization header format")]
    InvalidAuthHeaderFormat,
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    #[error("Account locked")]
    AccountLocked,
    #[error("Password reset required")]
    PasswordResetRequired,
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Password validation failed: {0}")]
    PasswordValidation(String),
    #[error("Email validation failed: {0}")]
    EmailValidation(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("Password hashing error: {0}")]
    PasswordHashing(#[from] crate::auth::password::PasswordError),
    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials"),
            AuthError::UserNotFound => (StatusCode::NOT_FOUND, "User not found"),
            AuthError::EmailAlreadyExists => (StatusCode::CONFLICT, "Email already exists"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token expired"),
            AuthError::MissingAuthHeader => (StatusCode::UNAUTHORIZED, "Missing authorization header"),
            AuthError::InvalidAuthHeaderFormat => (StatusCode::UNAUTHORIZED, "Invalid authorization header format"),
            AuthError::InsufficientPermissions => (StatusCode::FORBIDDEN, "Insufficient permissions"),
            AuthError::AccountLocked => (StatusCode::FORBIDDEN, "Account locked"),
            AuthError::PasswordResetRequired => (StatusCode::FORBIDDEN, "Password reset required"),
            AuthError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded"),
            AuthError::PasswordValidation(_) => (StatusCode::BAD_REQUEST, "Password validation failed"),
            AuthError::EmailValidation(_) => (StatusCode::BAD_REQUEST, "Email validation failed"),
            AuthError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            AuthError::Jwt(_) => (StatusCode::UNAUTHORIZED, "Token error"),
            AuthError::PasswordHashing(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Password processing error"),
            AuthError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        };

        let body = Json(json!({
            "error": error_message,
            "message": self.to_string(),
        }));

        (status, body).into_response()
    }
}