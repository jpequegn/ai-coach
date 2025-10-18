// ============================================================================
// API routes and handlers - MVP ONLY
// ============================================================================

// MVP modules - working and tested
pub mod response;  // Standardized API responses
pub mod health;
pub mod routes;
pub mod auth;
pub mod user_profile;

// Recommendation system modules
pub mod recommendation_tracking;
pub mod recommendation_engine;
pub mod progression;

// Disabled modules - too many compilation errors for MVP
// pub mod training;
// pub mod ml_predictions;
// pub mod workout_recommendations;
// pub mod performance_insights;
// pub mod goals;  // Blocked by SQLite DateTime TEXT compatibility - see docs/sqlite-compatibility-notes.md
// pub mod analytics;
// pub mod coaching;
// pub mod notifications;
// pub mod events;
// pub mod plan_generation;
// pub mod vision;
// pub mod docs;
// pub mod recovery;
// pub mod recovery_analysis;
// pub mod recovery_profile;
// pub mod oura_wearable;
// pub mod training_adjustment;
// pub mod validation;
// pub mod recommendation_tracking;
// pub mod recommendation_engine;
// pub mod recommendation_effectiveness;
// pub mod job_admin;
// pub mod job_admin_routes;
// pub mod daily_recovery_job_admin;
// pub mod daily_recovery_job_admin_routes;
// pub mod alert_delivery_admin;
// pub mod alert_delivery_admin_routes;
// pub mod data_quality_admin;
// pub mod data_quality_admin_routes;