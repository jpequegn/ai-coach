use axum::{routing::get, Router};

use super::health::health_check;

pub fn create_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
}