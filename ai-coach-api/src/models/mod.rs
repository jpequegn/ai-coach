// Data models and ML structures

pub mod user;
pub mod athlete_profile;
pub mod training_session;
pub mod training_metrics;
pub mod coaching_recommendation;
pub mod training_plan;
pub mod model_prediction;
pub mod training_features;
pub mod workout_recommendation;
pub mod performance_insights;
pub mod notification;
pub mod goal;
pub mod event;
pub mod plan_generation;
pub mod vision_analysis;
pub mod validation;
pub mod keypoint;
pub mod recovery_data;
pub mod recovery_analysis;
pub mod training_recovery_settings;

pub use user::*;
pub use athlete_profile::*;
pub use training_session::*;
pub use training_metrics::*;
pub use coaching_recommendation::*;
pub use training_plan::*;
pub use model_prediction::*;
pub use training_features::*;
pub use workout_recommendation::*;
pub use performance_insights::*;
pub use notification::*;
pub use goal::*;
pub use event::*;
pub use plan_generation::*;
pub use vision_analysis::*;
pub use validation::*;
// Don't glob re-export keypoint to avoid conflict with vision_analysis::Keypoint
// Import specific types as needed
pub use keypoint::{
    CocoKeypoint, JointAngle, NormalizationMethod, NormalizationParams, PoseFrame,
    SmoothingConfig,
};
pub use recovery_data::*;
pub use recovery_analysis::*;
pub use training_recovery_settings::*;