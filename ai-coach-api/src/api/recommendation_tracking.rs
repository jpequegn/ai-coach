use crate::auth::{Claims, AuthService, jwt_auth_middleware};
use crate::models::{
    CompleteRecommendationRequest, HistoryFilter, RateRecommendationRequest,
    SkipRecommendationRequest, UserRecommendation, UserRecommendationWithTemplate,
};
use crate::services::RecommendationTrackingService;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Json},
    routing::{get, post},
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Complete a recommendation
pub async fn complete_recommendation(
    State(service): State<Arc<RecommendationTrackingService>>,
    Extension(claims): Extension<Claims>,
    Path(recommendation_id): Path<Uuid>,
    Json(request): Json<CompleteRecommendationRequest>,
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

    match service
        .complete_recommendation(recommendation_id, user_id, request)
        .await
    {
        Ok(recommendation) => (StatusCode::OK, Json(recommendation)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Skip a recommendation
pub async fn skip_recommendation(
    State(service): State<Arc<RecommendationTrackingService>>,
    Extension(claims): Extension<Claims>,
    Path(recommendation_id): Path<Uuid>,
    Json(request): Json<SkipRecommendationRequest>,
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

    match service
        .skip_recommendation(recommendation_id, user_id, request)
        .await
    {
        Ok(recommendation) => (StatusCode::OK, Json(recommendation)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Rate a recommendation
pub async fn rate_recommendation(
    State(service): State<Arc<RecommendationTrackingService>>,
    Extension(claims): Extension<Claims>,
    Path(recommendation_id): Path<Uuid>,
    Json(request): Json<RateRecommendationRequest>,
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

    match service
        .rate_recommendation(recommendation_id, user_id, request)
        .await
    {
        Ok(recommendation) => (StatusCode::OK, Json(recommendation)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Get user recommendation history
pub async fn get_recommendation_history(
    State(service): State<Arc<RecommendationTrackingService>>,
    Extension(claims): Extension<Claims>,
    Query(filters): Query<HistoryFilter>,
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

    match service.get_user_history(user_id, filters).await {
        Ok(history) => (StatusCode::OK, Json(history)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Create recommendation tracking routes
pub fn recommendation_tracking_routes(db: PgPool, auth_service: Arc<AuthService>) -> Router {
    let service = Arc::new(RecommendationTrackingService::new(db));

    Router::new()
        .route("/:id/complete", post(complete_recommendation))
        .route("/:id/skip", post(skip_recommendation))
        .route("/:id/rate", post(rate_recommendation))
        .route("/history", get(get_recommendation_history))
        .layer(middleware::from_fn_with_state(
            auth_service.clone(),
            jwt_auth_middleware,
        ))
        .with_state(service)
}
