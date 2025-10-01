use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

use crate::auth::{extract_bearer_token, AuthError, AuthService, UserRole, UserSession};

/// JWT authentication middleware
pub async fn jwt_auth_middleware(
    State(auth_service): State<AuthService>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // Get authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or(AuthError::MissingAuthHeader)?;

    // Extract bearer token
    let token = extract_bearer_token(auth_header)?;

    // Validate session
    let session = auth_service.validate_session(token).await?;

    // Add user session to request extensions
    request.extensions_mut().insert(session);

    Ok(next.run(request).await)
}

/// Role-based authorization middleware
pub fn require_role(required_role: UserRole) -> impl Fn(Request, Next) -> futures::future::BoxFuture<'static, Result<Response, AuthError>> + Clone {
    move |request: Request, next: Next| {
        let required_role = required_role.clone();
        Box::pin(async move {
            // Get user session from request extensions
            let session = request
                .extensions()
                .get::<UserSession>()
                .ok_or(AuthError::InsufficientPermissions)?;

            // Check if user has required role
            if !session.role.can_access(&required_role) {
                return Err(AuthError::InsufficientPermissions);
            }

            Ok(next.run(request).await)
        })
    }
}

/// Admin-only middleware
pub async fn admin_only_middleware(
    request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let session = request
        .extensions()
        .get::<UserSession>()
        .ok_or(AuthError::InsufficientPermissions)?;

    if session.role != UserRole::Admin {
        return Err(AuthError::InsufficientPermissions);
    }

    Ok(next.run(request).await)
}

/// Coach or Admin middleware
pub async fn coach_or_admin_middleware(
    request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let session = request
        .extensions()
        .get::<UserSession>()
        .ok_or(AuthError::InsufficientPermissions)?;

    if !matches!(session.role, UserRole::Coach | UserRole::Admin) {
        return Err(AuthError::InsufficientPermissions);
    }

    Ok(next.run(request).await)
}

/// Extract user session from request (for use in handlers)
pub fn extract_user_session(request: &Request) -> Result<&UserSession, AuthError> {
    request
        .extensions()
        .get::<UserSession>()
        .ok_or(AuthError::InsufficientPermissions)
}

/// CORS configuration for authentication endpoints
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_credentials(true)
}

/// Security headers middleware
pub fn security_headers_layer() -> tower_http::set_header::SetResponseHeaderLayer<&'static str> {
    tower_http::set_header::SetResponseHeaderLayer::overriding(
        axum::http::header::HeaderName::from_static("x-content-type-options"),
        "nosniff",
    )
}

/// Rate limiting middleware (simple in-memory implementation)
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    pub fn check_rate_limit(&self, key: &str) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();

        // Get or create entry for this key
        let entry = requests.entry(key.to_string()).or_insert_with(Vec::new);

        // Remove old requests outside the window
        entry.retain(|&time| now.duration_since(time) < self.window);

        // Check if we've exceeded the limit
        if entry.len() >= self.max_requests {
            return false;
        }

        // Add current request
        entry.push(now);
        true
    }
}

/// Rate limiting middleware function
pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
    rate_limiter: RateLimiter,
) -> Result<Response, StatusCode> {
    // Extract client IP (or use a default for testing)
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .or_else(|| request.headers().get("x-real-ip"))
        .and_then(|header| header.to_str().ok())
        .unwrap_or("unknown");

    // Check rate limit
    if !rate_limiter.check_rate_limit(client_ip) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Create a service builder with common middleware
pub fn create_middleware_stack() -> impl Clone {
    ServiceBuilder::new()
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(security_headers_layer())
        .layer(cors_layer())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));

        // First 3 requests should succeed
        assert!(limiter.check_rate_limit("client1"));
        assert!(limiter.check_rate_limit("client1"));
        assert!(limiter.check_rate_limit("client1"));

        // 4th request should fail
        assert!(!limiter.check_rate_limit("client1"));

        // Different client should succeed
        assert!(limiter.check_rate_limit("client2"));
    }

    #[test]
    fn test_user_role_permissions() {
        let admin = UserRole::Admin;
        let coach = UserRole::Coach;
        let athlete = UserRole::Athlete;

        // Admin can access everything
        assert!(admin.can_access(&admin));
        assert!(admin.can_access(&coach));
        assert!(admin.can_access(&athlete));

        // Coach can access coach and athlete
        assert!(coach.can_access(&coach));
        assert!(coach.can_access(&athlete));
        assert!(!coach.can_access(&admin));

        // Athlete can only access athlete
        assert!(athlete.can_access(&athlete));
        assert!(!athlete.can_access(&coach));
        assert!(!athlete.can_access(&admin));
    }
}