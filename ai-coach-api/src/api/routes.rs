use axum::{routing::get, Router};
use sqlx::PgPool;

use super::auth::{admin_routes, auth_routes};
use super::health::health_check;
use super::training::training_routes;
use super::ml_predictions::ml_prediction_routes;
use super::workout_recommendations::workout_recommendation_routes;
use super::performance_insights::performance_insights_routes;
use super::goals::goals_routes;
use super::user_profile::user_profile_routes;
use super::analytics::analytics_routes;
use super::coaching::coaching_routes;
use super::notifications::notification_routes;
use super::events::events_routes;
use super::plan_generation::plan_generation_routes;
use super::vision::vision_routes;
use super::docs::docs_routes;
use super::recovery::recovery_routes;
use super::recovery_analysis::recovery_analysis_routes;
use crate::auth::AuthService;

pub fn create_routes(db: PgPool, jwt_secret: &str) -> Router {
    let auth_service = AuthService::new(db.clone(), jwt_secret);

    // Create v1 API routes
    let api_v1 = Router::new()
        .nest("/auth", auth_routes(auth_service.clone()))
        .nest("/admin", admin_routes(auth_service.clone()))
        .nest("/training", training_routes(db.clone(), auth_service.clone()))
        .nest("/coaching", coaching_routes(db.clone(), auth_service.clone()))
        .nest("/goals", goals_routes(db.clone(), auth_service.clone()))
        .nest("/analytics", analytics_routes(db.clone(), auth_service.clone()))
        .nest("/user", user_profile_routes(db.clone(), auth_service.clone()))
        .nest("/notifications", notification_routes(db.clone(), auth_service.clone()))
        .nest("/events", events_routes(db.clone(), auth_service.clone()))
        .nest("/plans", plan_generation_routes(db.clone(), auth_service.clone()))
        .nest("/vision", vision_routes(db.clone(), auth_service.clone()))
        .nest("/recovery", recovery_routes(db.clone(), auth_service.clone()))
        .nest("/recovery/analysis", recovery_analysis_routes(db.clone(), auth_service.clone()))
        // Documentation routes
        .merge(docs_routes())
        // Legacy routes for backward compatibility
        .nest("/ml", ml_prediction_routes(db.clone(), auth_service.clone()))
        .nest("/workouts", workout_recommendation_routes(db.clone(), auth_service.clone()))
        .nest("/performance", performance_insights_routes(db.clone(), auth_service.clone()));

    Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", api_v1)
        // Maintain backward compatibility with existing routes
        .nest("/api/auth", auth_routes(auth_service.clone()))
        .nest("/api/admin", admin_routes(auth_service.clone()))
        .nest("/api/training", training_routes(db.clone(), auth_service.clone()))
        .nest("/api/ml", ml_prediction_routes(db.clone(), auth_service.clone()))
        .nest("/api/workouts", workout_recommendation_routes(db.clone(), auth_service.clone()))
        .nest("/api/performance", performance_insights_routes(db.clone(), auth_service.clone()))
}