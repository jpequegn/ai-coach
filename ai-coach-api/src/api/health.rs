use axum::{http::StatusCode, response::Json};
use serde_json::{json, Value};

pub async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "status": "healthy",
        "service": "ai-coach",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}