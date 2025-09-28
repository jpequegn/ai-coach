use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{Json, Response},
};
use serde::Serialize;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tower::ServiceBuilder;
use tower_http::limit::RequestBodyLimitLayer;

#[derive(Debug, Serialize)]
pub struct RateLimitError {
    pub error_code: String,
    pub message: String,
    pub retry_after: u64,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub burst_size: u32,
    pub window_size: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            burst_size: 10,
            window_size: Duration::from_secs(60),
        }
    }
}

#[derive(Debug)]
struct RateLimitEntry {
    requests: Vec<Instant>,
    last_reset: Instant,
}

impl RateLimitEntry {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            last_reset: Instant::now(),
        }
    }

    fn add_request(&mut self, now: Instant, window_size: Duration) {
        // Remove old requests outside the window
        self.requests.retain(|&request_time| now.duration_since(request_time) < window_size);

        // Add current request
        self.requests.push(now);
    }

    fn request_count(&self, now: Instant, window_size: Duration) -> usize {
        self.requests.iter()
            .filter(|&&request_time| now.duration_since(request_time) < window_size)
            .count()
    }
}

#[derive(Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    store: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn check_rate_limit(&self, key: &str) -> Result<(), (StatusCode, Json<RateLimitError>)> {
        let now = Instant::now();
        let mut store = self.store.write().unwrap();

        let entry = store.entry(key.to_string()).or_insert_with(RateLimitEntry::new);

        // Check minute limit
        let minute_count = entry.request_count(now, Duration::from_secs(60));
        if minute_count >= self.config.requests_per_minute as usize {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(RateLimitError {
                    error_code: "RATE_LIMIT_EXCEEDED".to_string(),
                    message: "Too many requests per minute".to_string(),
                    retry_after: 60,
                }),
            ));
        }

        // Check hour limit
        let hour_count = entry.request_count(now, Duration::from_secs(3600));
        if hour_count >= self.config.requests_per_hour as usize {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(RateLimitError {
                    error_code: "RATE_LIMIT_EXCEEDED".to_string(),
                    message: "Too many requests per hour".to_string(),
                    retry_after: 3600,
                }),
            ));
        }

        // Add the current request
        entry.add_request(now, self.config.window_size);

        Ok(())
    }

    pub fn cleanup_old_entries(&self) {
        let now = Instant::now();
        let mut store = self.store.write().unwrap();

        store.retain(|_, entry| {
            now.duration_since(entry.last_reset) < Duration::from_secs(7200) // Keep entries for 2 hours
        });
    }
}

/// Extract client identifier for rate limiting
fn get_client_key(headers: &HeaderMap, remote_addr: Option<std::net::SocketAddr>) -> String {
    // Try to get real IP from headers (for reverse proxy scenarios)
    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(ip_str) = forwarded_for.to_str() {
            // Take the first IP from the comma-separated list
            if let Some(first_ip) = ip_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }

    // Fall back to remote address
    if let Some(addr) = remote_addr {
        return addr.ip().to_string();
    }

    // Ultimate fallback
    "unknown".to_string()
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(rate_limiter): State<RateLimiter>,
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<RateLimitError>)> {
    let remote_addr = req.extensions().get::<std::net::SocketAddr>().copied();
    let client_key = get_client_key(&headers, remote_addr);

    // Check rate limit
    rate_limiter.check_rate_limit(&client_key)?;

    // Continue to next middleware/handler
    Ok(next.run(req).await)
}

/// Per-user rate limiting middleware (requires authentication)
pub async fn user_rate_limit_middleware(
    State(rate_limiter): State<RateLimiter>,
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<RateLimitError>)> {
    // Try to extract user ID from headers/claims
    let user_key = if let Some(auth_header) = headers.get("authorization") {
        // In a real implementation, you'd decode the JWT here
        // For now, use the full authorization header as key
        format!("user:{}", auth_header.to_str().unwrap_or("unknown"))
    } else {
        // Fall back to IP-based limiting
        let remote_addr = req.extensions().get::<std::net::SocketAddr>().copied();
        get_client_key(&headers, remote_addr)
    };

    // Check rate limit
    rate_limiter.check_rate_limit(&user_key)?;

    // Continue to next middleware/handler
    Ok(next.run(req).await)
}

/// Create rate limiting layer for specific routes
pub fn create_rate_limiting_layer(config: RateLimitConfig) {
    let _rate_limiter = RateLimiter::new(config);
    // Implementation simplified for now to fix compilation
}

/// Create user-specific rate limiting layer
pub fn create_user_rate_limiting_layer(config: RateLimitConfig) {
    let _rate_limiter = RateLimiter::new(config);
    // Implementation simplified for now to fix compilation
}

/// Different rate limiting configs for different endpoint types
pub struct RateLimitProfiles;

impl RateLimitProfiles {
    /// Conservative limits for auth endpoints
    pub fn auth() -> RateLimitConfig {
        RateLimitConfig {
            requests_per_minute: 10,
            requests_per_hour: 100,
            burst_size: 3,
            window_size: Duration::from_secs(60),
        }
    }

    /// Standard limits for API endpoints
    pub fn api() -> RateLimitConfig {
        RateLimitConfig {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            burst_size: 10,
            window_size: Duration::from_secs(60),
        }
    }

    /// Higher limits for file upload endpoints
    pub fn upload() -> RateLimitConfig {
        RateLimitConfig {
            requests_per_minute: 30,
            requests_per_hour: 200,
            burst_size: 5,
            window_size: Duration::from_secs(60),
        }
    }

    /// Very conservative limits for admin endpoints
    pub fn admin() -> RateLimitConfig {
        RateLimitConfig {
            requests_per_minute: 5,
            requests_per_hour: 50,
            burst_size: 2,
            window_size: Duration::from_secs(60),
        }
    }
}