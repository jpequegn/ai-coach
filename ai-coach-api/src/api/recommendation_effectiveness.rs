use crate::auth::{AuthService, Claims, RoleGuard, jwt_auth_middleware};
use crate::models::{EffectivenessFilter, ProcessOutcomesResult};
use crate::services::RecommendationEffectivenessService;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Json},
    routing::{get, post},
    Extension, Router,
};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;

/// Get effectiveness analytics for recommendations
///
/// Returns effectiveness metrics filtered by template, category, or date range.
/// Available to all authenticated users for transparency.
pub async fn get_effectiveness_analytics(
    State(service): State<Arc<RecommendationEffectivenessService>>,
    Extension(_claims): Extension<Claims>,
    Query(filter): Query<EffectivenessFilter>,
) -> impl IntoResponse {
    match service.get_effectiveness_analytics(filter).await {
        Ok(analytics) => (StatusCode::OK, Json(analytics)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Get system-wide recommendation analytics
///
/// Returns comprehensive analytics including category performance,
/// top performers, and underperformers. Available to all users.
pub async fn get_system_analytics(
    State(service): State<Arc<RecommendationEffectivenessService>>,
    Extension(_claims): Extension<Claims>,
) -> impl IntoResponse {
    match service.get_system_analytics().await {
        Ok(analytics) => (StatusCode::OK, Json(analytics)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Manually trigger daily outcome processing
///
/// Admin-only endpoint to manually trigger the background job that processes
/// outcomes from 24 hours ago. Normally runs automatically via cron job.
pub async fn process_daily_outcomes(
    State(service): State<Arc<RecommendationEffectivenessService>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    // Admin role check
    if claims.role != "admin" {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin access required"})),
        )
            .into_response();
    }

    match service.process_daily_outcomes().await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Create router for recommendation effectiveness endpoints
pub fn recommendation_effectiveness_routes(
    db: PgPool,
    auth_service: Arc<AuthService>,
) -> Router {
    let service = Arc::new(RecommendationEffectivenessService::new(db));

    Router::new()
        // Public analytics endpoints (require auth but no role restriction)
        .route("/effectiveness", get(get_effectiveness_analytics))
        .route("/analytics", get(get_system_analytics))
        // Admin-only endpoint for manual processing
        .route("/process-outcomes", post(process_daily_outcomes))
        .layer(middleware::from_fn_with_state(
            auth_service.clone(),
            jwt_auth_middleware,
        ))
        .with_state(service)
}
