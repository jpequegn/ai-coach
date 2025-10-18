use crate::auth::{AuthService, Claims, jwt_auth_middleware};
use crate::models::UserProgressionResponse;
use crate::services::ProgressionService;
use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    response::{IntoResponse, Json},
    routing::get,
    Extension, Router,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

/// Get user's progression data (experience level, category masteries, requirements)
pub async fn get_user_progression(
    State(service): State<Arc<ProgressionService>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid user ID format"})),
            )
            .into_response()
        }
    };

    match service.get_user_progression(user_id).await {
        Ok(progression) => (StatusCode::OK, Json(progression)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Create progression routes
pub fn progression_routes(db: SqlitePool, auth_service: Arc<AuthService>) -> Router {
    let service = Arc::new(ProgressionService::new(db));

    Router::new()
        .route("/progression", get(get_user_progression))
        .layer(middleware::from_fn_with_state(
            auth_service.clone(),
            jwt_auth_middleware,
        ))
        .with_state(service)
}
