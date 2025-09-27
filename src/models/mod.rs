// Data models and ML structures

pub mod user;
pub mod athlete_profile;
pub mod training_session;
pub mod training_metrics;
pub mod coaching_recommendation;
pub mod training_plan;
pub mod model_prediction;
pub mod validation;

pub use user::*;
pub use athlete_profile::*;
pub use training_session::*;
pub use training_metrics::*;
pub use coaching_recommendation::*;
pub use training_plan::*;
pub use model_prediction::*;
pub use validation::*;