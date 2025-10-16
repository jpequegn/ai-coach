use axum::{middleware, routing::get, Router};
use sqlx::SqlitePool;
use tower_http::request_id::{PropagateRequestIdLayer, SetRequestIdLayer};

// ============================================================================
// MVP Routes - Core functionality only
// ============================================================================
use super::auth::{admin_routes, auth_routes};
use super::health::{health_check, health_check_detailed, HealthState};
use super::user_profile::user_profile_routes;

// ============================================================================
// Disabled Routes - Comment out for MVP
// ============================================================================
// use super::training::training_routes;
// use super::ml_predictions::ml_prediction_routes;
// use super::workout_recommendations::workout_recommendation_routes;
// use super::performance_insights::performance_insights_routes;
// use super::goals::goals_routes;  // Blocked by SQLite DateTime TEXT compatibility
// use super::analytics::analytics_routes;
// use super::coaching::coaching_routes;
// use super::notifications::notification_routes;
// use super::events::events_routes;
// use super::plan_generation::plan_generation_routes;
// use super::vision::vision_routes;
// use super::docs::docs_routes;
// use super::recovery::recovery_routes;
// use super::recovery_analysis::recovery_analysis_routes;
// use super::recovery_profile::recovery_profile_routes;
// use super::oura_wearable::oura_wearable_routes;
// use super::training_adjustment::training_adjustment_routes;
// use super::validation::validation_routes;
// use super::recommendation_tracking::recommendation_tracking_routes;
// use super::recommendation_engine::recommendation_engine_routes;
// use super::recommendation_effectiveness::recommendation_effectiveness_routes;
// use super::job_admin_routes::job_admin_routes;
// use super::daily_recovery_job_admin_routes::daily_recovery_job_admin_routes;
// use super::alert_delivery_admin_routes::alert_delivery_admin_routes;
// use super::data_quality_admin_routes::data_quality_admin_routes;
use crate::auth::{AuthService, middleware::cors_layer};
use crate::config::AppConfig;
use crate::middleware::{UuidRequestIdGenerator, logging_middleware};
// Disabled for MVP - admin/job services not needed
// use crate::services::{
//     AlertDeliveryQueueService, DailyRecoveryCalculationJob, NotificationService,
//     RecoveryJobScheduler,
// };
use std::sync::Arc;

pub fn create_routes(
    db: SqlitePool,
    jwt_secret: &str,
    app_config: &AppConfig,
    _scheduler: Option<Arc<()>>,  // Disabled - type changed to avoid importing RecoveryJobScheduler
    _recovery_job: Option<Arc<()>>,  // Disabled - type changed to avoid importing DailyRecoveryCalculationJob
) -> Router {
    let auth_service = AuthService::new(db.clone(), jwt_secret);

    // ========================================================================
    // MVP API Routes - Minimal functionality
    // ========================================================================
    let api_v1 = Router::new()
        .nest("/auth", auth_routes(auth_service.clone()))
        .nest("/admin", admin_routes(auth_service.clone()))
        .nest("/user", user_profile_routes(db.clone(), auth_service.clone()));
        // .nest("/goals", goals_routes(db.clone(), auth_service.clone()));  // Blocked by SQLite DateTime

    // Disabled for MVP - too many compilation errors
    // .nest("/training", training_routes(db.clone(), auth_service.clone()))
    // .nest("/coaching", coaching_routes(db.clone(), auth_service.clone()))
    // .nest("/analytics", analytics_routes(db.clone(), auth_service.clone()))
    // .nest("/notifications", notification_routes(db.clone(), auth_service.clone()))
    // .nest("/events", events_routes(db.clone(), auth_service.clone()))
    // .nest("/plans", plan_generation_routes(db.clone(), auth_service.clone()))
    // .nest("/vision", vision_routes(db.clone(), auth_service.clone()))
    // .nest("/recovery", recovery_routes(db.clone(), auth_service.clone()))
    // .nest("/recovery/analysis", recovery_analysis_routes(db.clone(), auth_service.clone()))
    // .nest("/recovery/profile", recovery_profile_routes(db.clone(), auth_service_arc.clone()))
    // .nest("/recovery/recommendations", recommendation_tracking_routes(db.clone(), auth_service_arc.clone()))
    // .nest("/recovery/recommendations", recommendation_engine_routes(db.clone(), auth_service_arc.clone()))
    // .nest("/recovery/recommendations", recommendation_effectiveness_routes(db.clone(), auth_service_arc.clone()))
    // .nest("/training/adjustment", training_adjustment_routes(db.clone(), auth_service.clone()))
    // .nest("/validation", validation_routes(db.clone(), auth_service.clone()));

    tracing::info!("✅ MVP API initialized with: auth, admin, user profile");
    tracing::warn!("⚠️  Disabled for MVP: goals (SQLite blocker), training, ML, recovery, notifications, analytics");

    // Create minimal health state
    let health_state = Arc::new(HealthState {
        scheduler: None,  // No scheduler in MVP
        db: db.clone(),
    });

    Router::new()
        .route("/health", get(health_check))
        .route(
            "/health/detailed",
            get(health_check_detailed).with_state(health_state),
        )
        .nest("/api/v1", api_v1)
        // Maintain backward compatibility with existing auth routes
        .nest("/api/auth", auth_routes(auth_service.clone()))
        .nest("/api/admin", admin_routes(auth_service.clone()))
        // Add CORS middleware with configured origins
        .layer(cors_layer(
            app_config.allowed_origins.clone(),
            app_config.is_development(),
        ))
        // Add request ID generation and propagation
        .layer(SetRequestIdLayer::new(
            axum::http::header::HeaderName::from_static("x-request-id"),
            UuidRequestIdGenerator::default(),
        ))
        .layer(PropagateRequestIdLayer::new(
            axum::http::header::HeaderName::from_static("x-request-id"),
        ))
        // Add logging middleware
        .layer(middleware::from_fn(logging_middleware))
}