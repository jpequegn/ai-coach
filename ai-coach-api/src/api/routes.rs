use axum::{routing::get, Router};
use sqlx::PgPool;

use super::auth::{admin_routes, auth_routes};
use super::health::{health_check, health_check_detailed, HealthState};
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
use super::recovery_profile::recovery_profile_routes;
use super::oura_wearable::oura_wearable_routes;
use super::training_adjustment::training_adjustment_routes;
use super::validation::validation_routes;
use super::recommendation_tracking::recommendation_tracking_routes;
use super::recommendation_engine::recommendation_engine_routes;
use super::recommendation_effectiveness::recommendation_effectiveness_routes;
use super::job_admin_routes::job_admin_routes;
use super::daily_recovery_job_admin_routes::daily_recovery_job_admin_routes;
use crate::auth::AuthService;
use crate::config::AppConfig;
use crate::services::{DailyRecoveryCalculationJob, RecoveryJobScheduler};
use std::sync::Arc;

pub fn create_routes(
    db: PgPool,
    jwt_secret: &str,
    app_config: &AppConfig,
    scheduler: Option<Arc<RecoveryJobScheduler>>,
    recovery_job: Option<Arc<DailyRecoveryCalculationJob>>,
) -> Router {
    let auth_service = AuthService::new(db.clone(), jwt_secret);

    // Create v1 API routes
    let mut api_v1 = Router::new()
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
        .nest("/recovery/profile", recovery_profile_routes(db.clone(), auth_service.clone()))
        .nest("/recovery/recommendations", recommendation_tracking_routes(db.clone(), auth_service.clone()))
        .nest("/recovery/recommendations", recommendation_engine_routes(db.clone(), auth_service.clone()))
        .nest("/recovery/recommendations", recommendation_effectiveness_routes(db.clone(), auth_service.clone()))
        .nest("/training/adjustment", training_adjustment_routes(db.clone(), auth_service.clone()))
        .nest("/validation", validation_routes(db.clone(), auth_service.clone()));

    // Add Oura wearable routes if credentials are configured
    if let (Some(client_id), Some(client_secret), Some(redirect_uri)) = (
        &app_config.oura_client_id,
        &app_config.oura_client_secret,
        &app_config.oura_redirect_uri,
    ) {
        api_v1 = api_v1.nest(
            "/recovery/wearables/oura",
            oura_wearable_routes(
                db.clone(),
                auth_service.clone(),
                client_id.clone(),
                client_secret.clone(),
                redirect_uri.clone(),
            ),
        );
        tracing::info!("Oura wearable integration enabled");
    } else {
        tracing::warn!("Oura wearable integration disabled (missing OAuth credentials)");
    }

    // Add job admin routes if scheduler is provided
    if let Some(scheduler) = scheduler {
        api_v1 = api_v1.nest("/admin/jobs", job_admin_routes(scheduler, auth_service.clone()));
        tracing::info!("Job administration endpoints enabled at /api/v1/admin/jobs");
    }

    // Add recovery job admin routes if job is provided
    if let Some(job) = recovery_job {
        api_v1 = api_v1.nest(
            "/admin/jobs/daily-recovery",
            daily_recovery_job_admin_routes(job, auth_service.clone()),
        );
        tracing::info!("Daily recovery job administration endpoints enabled at /api/v1/admin/jobs/daily-recovery");
    }

    api_v1 = api_v1
        // Documentation routes
        .merge(docs_routes())
        // Legacy routes for backward compatibility
        .nest("/ml", ml_prediction_routes(db.clone(), auth_service.clone()))
        .nest("/workouts", workout_recommendation_routes(db.clone(), auth_service.clone()))
        .nest("/performance", performance_insights_routes(db.clone(), auth_service.clone()));

    // Create health state
    let health_state = Arc::new(HealthState {
        scheduler: scheduler.clone(),
    });

    Router::new()
        .route("/health", get(health_check))
        .route(
            "/health/detailed",
            get(health_check_detailed).with_state(health_state),
        )
        .nest("/api/v1", api_v1)
        // Maintain backward compatibility with existing routes
        .nest("/api/auth", auth_routes(auth_service.clone()))
        .nest("/api/admin", admin_routes(auth_service.clone()))
        .nest("/api/training", training_routes(db.clone(), auth_service.clone()))
        .nest("/api/ml", ml_prediction_routes(db.clone(), auth_service.clone()))
        .nest("/api/workouts", workout_recommendation_routes(db.clone(), auth_service.clone()))
        .nest("/api/performance", performance_insights_routes(db.clone(), auth_service.clone()))
}