// ============================================================================
// MVP Models - Minimal functionality
// ============================================================================
pub mod user;
pub mod athlete_profile;
pub mod user_recovery_profile;  // Needed for DietaryPreferences and SleepSchedule used by User model
// pub mod goal;  // Blocked by SQLite DateTime TEXT compatibility - see docs/sqlite-compatibility-notes.md

// Disabled for MVP - too many compilation errors or not needed
// pub mod training_session;
// pub mod training_metrics;
// pub mod coaching_recommendation;
// pub mod training_plan;
// pub mod model_prediction;
// pub mod training_features;
// pub mod workout_recommendation;
// pub mod performance_insights;
// pub mod notification;
// pub mod event;
// pub mod plan_generation;
// pub mod vision_analysis;
// pub mod validation;
// pub mod keypoint;
// pub mod recovery_data;  // Uses sqlx::types::Json which is PostgreSQL-only
// pub mod recovery_analysis;  // Uses sqlx::types::Json which is PostgreSQL-only
// pub mod training_recovery_settings;
// pub mod validation_framework;
pub mod recommendation;
// pub mod user_recovery_profile;
pub mod recommendation_outcome;
pub mod progression;
// pub mod job_execution;
// pub mod alert_delivery;
// pub mod data_quality;
// pub mod recovery_protocol;  // Depends on recommendation module - Issue #178

pub use user::*;
pub use athlete_profile::*;
pub use user_recovery_profile::{DietaryPreferences, SleepSchedule};
// pub use goal::*;  // Blocked by SQLite DateTime TEXT compatibility

// Disabled for MVP
// pub use training_session::*;
// pub use training_metrics::*;
// pub use coaching_recommendation::*;
// pub use training_plan::*;
// pub use model_prediction::*;
// pub use training_features::*;
// pub use workout_recommendation::*;
// pub use performance_insights::*;
// pub use notification::*;
// pub use event::*;
// pub use plan_generation::*;
// pub use vision_analysis::*;
// pub use validation::*;
// pub use keypoint::{...};
// pub use recovery_data::*;
// pub use recovery_analysis::*;
// pub use training_recovery_settings::*;
// pub use validation_framework::*;
pub use recommendation::*;
// pub use user_recovery_profile::*;
pub use recommendation_outcome::*;
pub use progression::*;
// pub use job_execution::*;
// pub use alert_delivery::*;
// pub use data_quality::*;