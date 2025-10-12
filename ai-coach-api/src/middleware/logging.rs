//! Structured logging middleware with request tracking and performance metrics
//!
//! This module provides comprehensive logging capabilities including:
//! - Request ID tracking across the entire request lifecycle
//! - User context injection for authenticated requests
//! - Performance metrics (request duration)
//! - Error context with detailed stacktraces
//! - Standardized log format

use axum::{
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tower_http::request_id::{MakeRequestId, RequestId};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Custom request ID generator using UUIDs
#[derive(Clone, Default)]
pub struct UuidRequestIdGenerator;

impl MakeRequestId for UuidRequestIdGenerator {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let request_id = Uuid::new_v4().to_string();
        Some(RequestId::new(request_id.parse().ok()?))
    }
}

/// Extract user ID from request headers (set by auth middleware)
fn extract_user_id(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// Extract request ID from headers
fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Logging middleware that adds request tracking and performance metrics
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let start_time = Instant::now();

    // Extract request metadata
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers();
    let request_id = extract_request_id(headers)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let user_id = extract_user_id(headers);

    // Create tracing span with request context
    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        method = %method,
        uri = %uri.path(),
        user_id = user_id.as_ref().map(|id| id.to_string()).as_deref(),
    );

    let _enter = span.enter();

    // Log request start
    if let Some(uid) = user_id {
        info!(
            user_id = %uid,
            "Request started"
        );
    } else {
        info!("Request started");
    }

    // Process request
    let response = next.run(request).await;

    // Calculate duration
    let duration = start_time.elapsed();
    let status = response.status();

    // Log response with appropriate level based on status
    match status.as_u16() {
        200..=299 => {
            info!(
                status = %status,
                duration_ms = %duration.as_millis(),
                "Request completed successfully"
            );
        }
        300..=399 => {
            info!(
                status = %status,
                duration_ms = %duration.as_millis(),
                "Request redirected"
            );
        }
        400..=499 => {
            warn!(
                status = %status,
                duration_ms = %duration.as_millis(),
                "Request failed (client error)"
            );
        }
        500..=599 => {
            error!(
                status = %status,
                duration_ms = %duration.as_millis(),
                "Request failed (server error)"
            );
        }
        _ => {
            warn!(
                status = %status,
                duration_ms = %duration.as_millis(),
                "Request completed with unusual status"
            );
        }
    }

    // Add performance warning for slow requests
    if duration.as_secs() > 1 {
        warn!(
            duration_ms = %duration.as_millis(),
            "Slow request detected (>1s)"
        );
    }

    response
}

/// Error logging helper with context
pub fn log_error_with_context(
    error: &anyhow::Error,
    context: &str,
    user_id: Option<Uuid>,
) {
    error!(
        error = %error,
        context = context,
        user_id = user_id.as_ref().map(|id| id.to_string()).as_deref(),
        "Operation failed"
    );

    // Log error chain for debugging
    for (i, cause) in error.chain().enumerate() {
        error!(
            error_chain_index = i,
            cause = %cause,
            "Error cause"
        );
    }
}

/// Performance logging helper for long-running operations
#[derive(Debug)]
pub struct PerformanceLogger {
    operation: String,
    start_time: Instant,
    user_id: Option<Uuid>,
}

impl PerformanceLogger {
    /// Create a new performance logger
    pub fn new(operation: impl Into<String>, user_id: Option<Uuid>) -> Self {
        let operation = operation.into();
        info!(
            operation = %operation,
            user_id = user_id.as_ref().map(|id| id.to_string()).as_deref(),
            "Operation started"
        );

        Self {
            operation,
            start_time: Instant::now(),
            user_id,
        }
    }

    /// Log completion with duration
    pub fn complete(self) {
        let duration = self.start_time.elapsed();
        info!(
            operation = %self.operation,
            duration_ms = %duration.as_millis(),
            user_id = self.user_id.as_ref().map(|id| id.to_string()).as_deref(),
            "Operation completed"
        );

        if duration.as_secs() > 5 {
            warn!(
                operation = %self.operation,
                duration_ms = %duration.as_millis(),
                "Slow operation detected (>5s)"
            );
        }
    }

    /// Log completion with custom message
    pub fn complete_with_message(self, message: &str) {
        let duration = self.start_time.elapsed();
        info!(
            operation = %self.operation,
            duration_ms = %duration.as_millis(),
            user_id = self.user_id.as_ref().map(|id| id.to_string()).as_deref(),
            message = message,
            "Operation completed"
        );
    }

    /// Log failure with error
    pub fn fail(self, error: &anyhow::Error) {
        let duration = self.start_time.elapsed();
        error!(
            operation = %self.operation,
            duration_ms = %duration.as_millis(),
            user_id = self.user_id.as_ref().map(|id| id.to_string()).as_deref(),
            error = %error,
            "Operation failed"
        );

        // Log error chain
        for (i, cause) in error.chain().enumerate() {
            error!(
                error_chain_index = i,
                cause = %cause,
                "Error cause"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_generation() {
        let mut generator = UuidRequestIdGenerator;
        let request = Request::builder().body(()).unwrap();

        let id1 = generator.make_request_id(&request);
        let id2 = generator.make_request_id(&request);

        assert!(id1.is_some());
        assert!(id2.is_some());
        // UUIDs should be different
        assert_ne!(format!("{:?}", id1), format!("{:?}", id2));
    }

    #[test]
    fn test_extract_user_id() {
        let mut headers = HeaderMap::new();
        let user_id = Uuid::new_v4();
        headers.insert("x-user-id", user_id.to_string().parse().unwrap());

        let extracted = extract_user_id(&headers);
        assert_eq!(extracted, Some(user_id));
    }

    #[test]
    fn test_extract_user_id_invalid() {
        let mut headers = HeaderMap::new();
        headers.insert("x-user-id", "invalid-uuid".parse().unwrap());

        let extracted = extract_user_id(&headers);
        assert_eq!(extracted, None);
    }

    #[test]
    fn test_extract_request_id() {
        let mut headers = HeaderMap::new();
        let request_id = "test-request-123";
        headers.insert("x-request-id", request_id.parse().unwrap());

        let extracted = extract_request_id(&headers);
        assert_eq!(extracted, Some(request_id.to_string()));
    }
}
