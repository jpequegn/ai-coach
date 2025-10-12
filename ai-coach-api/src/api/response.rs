//! Standardized API response types and builders
//!
//! This module provides a consistent response structure across all API endpoints
//! with support for success responses, errors, pagination, and metadata.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Standard API response envelope
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Response data (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    /// Error information (present on error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,

    /// Response metadata (pagination, timing, etc.)
    pub meta: ResponseMeta,
}

/// Error information in API responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    /// Error code for programmatic handling
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Response metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMeta {
    /// Whether the request was successful
    pub success: bool,

    /// HTTP status code
    pub status: u16,

    /// Pagination information (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationMeta>,

    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Pagination metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationMeta {
    /// Current page number (1-indexed)
    pub page: u32,

    /// Items per page
    pub per_page: u32,

    /// Total number of items
    pub total: u64,

    /// Total number of pages
    pub total_pages: u32,

    /// Whether there is a next page
    pub has_next: bool,

    /// Whether there is a previous page
    pub has_prev: bool,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a successful response with data
    pub fn ok(data: T) -> Self {
        Self {
            data: Some(data),
            error: None,
            meta: ResponseMeta::success(StatusCode::OK),
        }
    }

    /// Create a successful response with custom status code
    pub fn ok_with_status(data: T, status: StatusCode) -> Self {
        Self {
            data: Some(data),
            error: None,
            meta: ResponseMeta::success(status),
        }
    }

    /// Create a paginated response
    pub fn paginated(data: T, page: u32, per_page: u32, total: u64) -> Self {
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;

        Self {
            data: Some(data),
            error: None,
            meta: ResponseMeta {
                success: true,
                status: StatusCode::OK.as_u16(),
                pagination: Some(PaginationMeta {
                    page,
                    per_page,
                    total,
                    total_pages,
                    has_next: page < total_pages,
                    has_prev: page > 1,
                }),
                timestamp: chrono::Utc::now(),
            },
        }
    }

    /// Create an error response
    pub fn error(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
                details: None,
            }),
            meta: ResponseMeta::error(status),
        }
    }

    /// Create an error response with details
    pub fn error_with_details(
        status: StatusCode,
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
                details: Some(details),
            }),
            meta: ResponseMeta::error(status),
        }
    }
}

impl ResponseMeta {
    /// Create success metadata
    pub fn success(status: StatusCode) -> Self {
        Self {
            success: true,
            status: status.as_u16(),
            pagination: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create error metadata
    pub fn error(status: StatusCode) -> Self {
        Self {
            success: false,
            status: status.as_u16(),
            pagination: None,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.meta.status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        (status, Json(self)).into_response()
    }
}

impl ApiError {
    /// Create a new API error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Add details to the error
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

/// Common error codes
pub mod error_codes {
    pub const VALIDATION_ERROR: &str = "VALIDATION_ERROR";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const UNAUTHORIZED: &str = "UNAUTHORIZED";
    pub const FORBIDDEN: &str = "FORBIDDEN";
    pub const CONFLICT: &str = "CONFLICT";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const BAD_REQUEST: &str = "BAD_REQUEST";
    pub const SERVICE_UNAVAILABLE: &str = "SERVICE_UNAVAILABLE";
}

/// Helper trait for converting errors to API responses
pub trait IntoApiError {
    fn into_api_error(self) -> (StatusCode, ApiError);
}

impl IntoApiError for anyhow::Error {
    fn into_api_error(self) -> (StatusCode, ApiError) {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::new(error_codes::INTERNAL_ERROR, self.to_string()),
        )
    }
}

impl IntoApiError for sqlx::Error {
    fn into_api_error(self) -> (StatusCode, ApiError) {
        match self {
            sqlx::Error::RowNotFound => (
                StatusCode::NOT_FOUND,
                ApiError::new(error_codes::NOT_FOUND, "Resource not found"),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::new(error_codes::INTERNAL_ERROR, "Database error"),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct TestData {
        id: i32,
        name: String,
    }

    #[test]
    fn test_ok_response() {
        let data = TestData {
            id: 1,
            name: "Test".to_string(),
        };
        let response = ApiResponse::ok(data);

        assert!(response.meta.success);
        assert_eq!(response.meta.status, 200);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_error_response() {
        let response: ApiResponse<()> = ApiResponse::error(
            StatusCode::BAD_REQUEST,
            error_codes::VALIDATION_ERROR,
            "Invalid input",
        );

        assert!(!response.meta.success);
        assert_eq!(response.meta.status, 400);
        assert!(response.data.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, error_codes::VALIDATION_ERROR);
        assert_eq!(error.message, "Invalid input");
    }

    #[test]
    fn test_paginated_response() {
        let data = vec![
            TestData {
                id: 1,
                name: "Test1".to_string(),
            },
            TestData {
                id: 2,
                name: "Test2".to_string(),
            },
        ];

        let response = ApiResponse::paginated(data, 1, 10, 25);

        assert!(response.meta.success);
        assert!(response.meta.pagination.is_some());

        let pagination = response.meta.pagination.unwrap();
        assert_eq!(pagination.page, 1);
        assert_eq!(pagination.per_page, 10);
        assert_eq!(pagination.total, 25);
        assert_eq!(pagination.total_pages, 3);
        assert!(pagination.has_next);
        assert!(!pagination.has_prev);
    }

    #[test]
    fn test_pagination_math() {
        // Test edge cases
        let response = ApiResponse::paginated(vec![1, 2, 3], 3, 10, 25);
        let pagination = response.meta.pagination.unwrap();
        assert!(pagination.has_prev);
        assert!(!pagination.has_next);

        // Test exact page boundary
        let response = ApiResponse::paginated(vec![1], 2, 10, 20);
        let pagination = response.meta.pagination.unwrap();
        assert_eq!(pagination.total_pages, 2);
        assert!(!pagination.has_next);
    }
}
