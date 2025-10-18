use crate::auth::{AuthService, Claims, jwt_auth_middleware};
use crate::models::recommendation::{
    CurrentRecommendationsQuery, CurrentRecommendationsResponse, RecommendationContext,
};
use crate::models::RecoveryScore;
use crate::services::{RecommendationEngine, ProgressionService};
use axum::{
    extract::{Query, State},
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

/// Get current personalized recommendations
pub async fn get_current_recommendations(
    State(engine): State<Arc<RecommendationEngine>>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<CurrentRecommendationsQuery>,
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

    // Get latest recovery score for the user
    let recovery_score = match get_latest_recovery_score(&engine.db, user_id).await {
        Ok(Some(score)) => score,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "No recovery data available. Please sync your wearable device or manually log recovery metrics."})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    // Create default context (can be enhanced with user preferences later)
    let context = RecommendationContext::default();

    // Generate recommendations
    match engine
        .generate_recommendations(
            user_id,
            &recovery_score,
            context,
            params.limit,
            params.category,
        )
        .await
    {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Helper function to get latest recovery score
async fn get_latest_recovery_score(
    db: &SqlitePool,
    user_id: uuid::Uuid,
) -> Result<Option<RecoveryScore>, sqlx::Error> {
    sqlx::query_as::<_, RecoveryScore>(
        r#"
        SELECT * FROM recovery_scores
        WHERE user_id = ?1
        ORDER BY score_date DESC
        LIMIT 1
        "#,
    )
    .bind(user_id.to_string())
    .fetch_optional(db)
    .await
}

/// Create recommendation engine routes
pub fn recommendation_engine_routes(db: SqlitePool, auth_service: Arc<AuthService>) -> Router {
    let progression_service = Arc::new(ProgressionService::new(db.clone()));
    let engine = Arc::new(RecommendationEngine::new(db.clone(), progression_service));

    Router::new()
        .route("/current", get(get_current_recommendations))
        .layer(middleware::from_fn_with_state(
            auth_service.clone(),
            jwt_auth_middleware,
        ))
        .with_state(engine)
}
