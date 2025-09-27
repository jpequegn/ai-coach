use axum::{routing::get, Router};
use sqlx::PgPool;

use super::auth::{admin_routes, auth_routes};
use super::health::health_check;
use crate::auth::AuthService;

pub fn create_routes(db: PgPool, jwt_secret: &str) -> Router {
    let auth_service = AuthService::new(db, jwt_secret);

    Router::new()
        .route("/health", get(health_check))
        .nest("/api/auth", auth_routes(auth_service.clone()))
        .nest("/api/admin", admin_routes(auth_service))
}