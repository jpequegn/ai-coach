use crate::auth::{jwt_auth_middleware, AuthService, Claims};
use crate::models::user_recovery_profile::*;
use crate::services::UserRecoveryProfileService;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Json},
    routing::{get, patch},
    Extension, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Get user recovery profile
pub async fn get_recovery_profile(
    State(service): State<Arc<UserRecoveryProfileService>>,
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

    match service.get_profile(user_id).await {
        Ok(profile) => {
            let response: ProfileResponse = profile.into();
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Update user recovery profile
pub async fn update_recovery_profile(
    State(service): State<Arc<UserRecoveryProfileService>>,
    Extension(claims): Extension<Claims>,
    Json(updates): Json<UpdateProfileRequest>,
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

    match service.update_profile(user_id, updates).await {
        Ok(profile) => {
            let response: ProfileResponse = profile.into();
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Query parameters for effective techniques
#[derive(Debug, Deserialize)]
pub struct EffectiveTechniquesQuery {
    pub limit: Option<i32>,
}

/// Get effective techniques for user
pub async fn get_effective_techniques(
    State(service): State<Arc<UserRecoveryProfileService>>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<EffectiveTechniquesQuery>,
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

    match service.get_effective_techniques(user_id, params.limit).await {
        Ok(techniques) => {
            let response = EffectiveTechniquesResponse {
                total: techniques.len(),
                techniques,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Get profile insights and patterns
pub async fn get_profile_insights(
    State(service): State<Arc<UserRecoveryProfileService>>,
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

    match service.get_insights(user_id).await {
        Ok(insights) => (StatusCode::OK, Json(insights)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Create recovery profile routes
pub fn recovery_profile_routes(db: PgPool, auth_service: Arc<AuthService>) -> Router {
    let service = Arc::new(UserRecoveryProfileService::new(db));

    Router::new()
        .route("/", get(get_recovery_profile).patch(update_recovery_profile))
        .route("/effective-techniques", get(get_effective_techniques))
        .route("/insights", get(get_profile_insights))
        .layer(middleware::from_fn_with_state(
            auth_service.clone(),
            jwt_auth_middleware,
        ))
        .with_state(service)
}
