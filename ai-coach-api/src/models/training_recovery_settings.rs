use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

// ============================================================================
// Database Models
// ============================================================================

/// Training recovery settings for user
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrainingRecoverySettings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub auto_adjust_enabled: bool,
    pub adjustment_aggressiveness: String, // conservative, moderate, aggressive
    pub min_rest_days_per_week: i32,
    pub max_consecutive_training_days: i32,
    pub allow_intensity_reduction: bool,
    pub allow_volume_reduction: bool,
    pub allow_workout_swap: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Request DTOs
// ============================================================================

/// Request to update training recovery settings
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateTrainingRecoverySettingsRequest {
    pub auto_adjust_enabled: Option<bool>,

    #[validate(custom = "validate_aggressiveness")]
    pub adjustment_aggressiveness: Option<String>,

    #[validate(range(min = 0, max = 7, message = "Min rest days must be between 0 and 7"))]
    pub min_rest_days_per_week: Option<i32>,

    #[validate(range(min = 1, max = 14, message = "Max consecutive training days must be between 1 and 14"))]
    pub max_consecutive_training_days: Option<i32>,

    pub allow_intensity_reduction: Option<bool>,
    pub allow_volume_reduction: Option<bool>,
    pub allow_workout_swap: Option<bool>,
}

fn validate_aggressiveness(value: &str) -> Result<(), validator::ValidationError> {
    if !["conservative", "moderate", "aggressive"].contains(&value) {
        return Err(validator::ValidationError::new("invalid_aggressiveness"));
    }
    Ok(())
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Response for training recovery settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRecoverySettingsResponse {
    pub id: Uuid,
    pub auto_adjust_enabled: bool,
    pub adjustment_aggressiveness: String,
    pub min_rest_days_per_week: i32,
    pub max_consecutive_training_days: i32,
    pub allow_intensity_reduction: bool,
    pub allow_volume_reduction: bool,
    pub allow_workout_swap: bool,
}

// ============================================================================
// Conversion Implementations
// ============================================================================

impl From<TrainingRecoverySettings> for TrainingRecoverySettingsResponse {
    fn from(settings: TrainingRecoverySettings) -> Self {
        Self {
            id: settings.id,
            auto_adjust_enabled: settings.auto_adjust_enabled,
            adjustment_aggressiveness: settings.adjustment_aggressiveness,
            min_rest_days_per_week: settings.min_rest_days_per_week,
            max_consecutive_training_days: settings.max_consecutive_training_days,
            allow_intensity_reduction: settings.allow_intensity_reduction,
            allow_volume_reduction: settings.allow_volume_reduction,
            allow_workout_swap: settings.allow_workout_swap,
        }
    }
}

impl Default for TrainingRecoverySettings {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            auto_adjust_enabled: false,
            adjustment_aggressiveness: "moderate".to_string(),
            min_rest_days_per_week: 1,
            max_consecutive_training_days: 6,
            allow_intensity_reduction: true,
            allow_volume_reduction: true,
            allow_workout_swap: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
