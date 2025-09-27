use axum::{routing::get, Router};
use sqlx::PgPool;

use super::auth::{admin_routes, auth_routes};
use super::health::health_check;
use super::training::training_routes;
use super::ml_predictions::ml_prediction_routes;
use crate::auth::AuthService;

pub fn create_routes(db: PgPool, jwt_secret: &str) -> Router {
    let auth_service = AuthService::new(db.clone(), jwt_secret);

    Router::new()
        .route("/health", get(health_check))
        .nest("/api/auth", auth_routes(auth_service.clone()))
        .nest("/api/admin", admin_routes(auth_service.clone()))
        .nest("/api/training", training_routes(db.clone(), auth_service.clone()))
        .nest("/api/ml", ml_prediction_routes(db, auth_service))
}