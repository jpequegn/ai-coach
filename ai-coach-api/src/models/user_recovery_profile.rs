use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// Database Models
// ============================================================================

/// User recovery profile with preferences and learned patterns
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRecoveryProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub preferred_recovery_activities: JsonValue,
    pub available_equipment: JsonValue,
    pub dietary_preferences: JsonValue,
    pub sleep_schedule: JsonValue,
    pub stress_triggers: JsonValue,
    pub effective_techniques: JsonValue,
    pub completion_stats: JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Request DTOs
// ============================================================================

/// Request to update user recovery profile
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub preferred_recovery_activities: Option<Vec<String>>,
    pub available_equipment: Option<Vec<String>>,
    pub dietary_preferences: Option<DietaryPreferences>,
    pub sleep_schedule: Option<SleepSchedule>,
    pub stress_triggers: Option<Vec<String>>,
}

/// Dietary preferences structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DietaryPreferences {
    pub vegetarian: Option<bool>,
    pub vegan: Option<bool>,
    pub allergies: Option<Vec<String>>,
    pub restrictions: Option<Vec<String>>,
    pub supplements: Option<Vec<String>>,
}

/// Sleep schedule structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepSchedule {
    pub bedtime: Option<String>, // Format: "HH:MM"
    pub wake_time: Option<String>, // Format: "HH:MM"
    pub target_duration_hours: Option<f64>,
    pub nap_preference: Option<bool>,
}

/// Effective technique record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveTechnique {
    pub template_id: Uuid,
    pub category: String,
    pub title: String,
    pub completion_count: i32,
    pub total_shown: i32,
    pub completion_rate: f64,
    pub average_rating: f64,
    pub total_ratings: i32,
    pub last_completed: Option<DateTime<Utc>>,
    pub effectiveness_score: f64,
}

/// Completion statistics per category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStats {
    pub category: String,
    pub completed: i32,
    pub shown: i32,
    pub completion_rate: f64,
    pub average_rating: Option<f64>,
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Complete recovery profile response
#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub preferred_recovery_activities: Vec<String>,
    pub available_equipment: Vec<String>,
    pub dietary_preferences: DietaryPreferences,
    pub sleep_schedule: SleepSchedule,
    pub stress_triggers: Vec<String>,
    pub effective_techniques: Vec<EffectiveTechnique>,
    pub completion_stats: Vec<CategoryStats>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UserRecoveryProfile> for ProfileResponse {
    fn from(profile: UserRecoveryProfile) -> Self {
        Self {
            id: profile.id,
            user_id: profile.user_id,
            preferred_recovery_activities: serde_json::from_value(
                profile.preferred_recovery_activities.clone(),
            )
            .unwrap_or_default(),
            available_equipment: serde_json::from_value(profile.available_equipment.clone())
                .unwrap_or_default(),
            dietary_preferences: serde_json::from_value(profile.dietary_preferences.clone())
                .unwrap_or_default(),
            sleep_schedule: serde_json::from_value(profile.sleep_schedule.clone())
                .unwrap_or_default(),
            stress_triggers: serde_json::from_value(profile.stress_triggers.clone())
                .unwrap_or_default(),
            effective_techniques: serde_json::from_value(profile.effective_techniques.clone())
                .unwrap_or_default(),
            completion_stats: extract_completion_stats(&profile.completion_stats),
            created_at: profile.created_at,
            updated_at: profile.updated_at,
        }
    }
}

/// Helper to extract completion stats from JSON
fn extract_completion_stats(stats_json: &JsonValue) -> Vec<CategoryStats> {
    if let Some(stats_obj) = stats_json.as_object() {
        stats_obj
            .iter()
            .filter_map(|(category, stats)| {
                let completed = stats.get("completed")?.as_i64()? as i32;
                let shown = stats.get("shown")?.as_i64()? as i32;
                let completion_rate = if shown > 0 {
                    completed as f64 / shown as f64
                } else {
                    0.0
                };
                let average_rating = stats
                    .get("average_rating")
                    .and_then(|v| v.as_f64());

                Some(CategoryStats {
                    category: category.clone(),
                    completed,
                    shown,
                    completion_rate,
                    average_rating,
                })
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Response for effective techniques endpoint
#[derive(Debug, Serialize)]
pub struct EffectiveTechniquesResponse {
    pub techniques: Vec<EffectiveTechnique>,
    pub total: usize,
}

/// Profile insights response
#[derive(Debug, Serialize)]
pub struct ProfileInsightsResponse {
    pub insights: Vec<RecoveryInsight>,
    pub patterns: Vec<RecoveryPattern>,
    pub recommendations_summary: RecommendationsSummary,
}

/// Individual recovery insight
#[derive(Debug, Serialize)]
pub struct RecoveryInsight {
    pub insight_type: String,
    pub title: String,
    pub description: String,
    pub confidence: f64,
    pub actionable: bool,
}

/// Identified recovery pattern
#[derive(Debug, Serialize)]
pub struct RecoveryPattern {
    pub pattern_type: String,
    pub description: String,
    pub frequency: String,
    pub suggestions: Vec<String>,
}

/// Summary of recommendation interactions
#[derive(Debug, Serialize)]
pub struct RecommendationsSummary {
    pub total_shown: i32,
    pub total_completed: i32,
    pub total_skipped: i32,
    pub overall_completion_rate: f64,
    pub most_effective_category: Option<String>,
    pub preferred_time_of_day: Option<String>,
    pub average_rating: Option<f64>,
}

// ============================================================================
// Default Implementations
// ============================================================================

impl Default for DietaryPreferences {
    fn default() -> Self {
        Self {
            vegetarian: Some(false),
            vegan: Some(false),
            allergies: Some(Vec::new()),
            restrictions: Some(Vec::new()),
            supplements: Some(Vec::new()),
        }
    }
}

impl Default for SleepSchedule {
    fn default() -> Self {
        Self {
            bedtime: Some("22:00".to_string()),
            wake_time: Some("06:00".to_string()),
            target_duration_hours: Some(8.0),
            nap_preference: Some(false),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate effectiveness score from technique data
pub fn calculate_effectiveness_score(
    completion_rate: f64,
    average_rating: f64,
    total_interactions: i32,
) -> f64 {
    // Weight factors
    const COMPLETION_WEIGHT: f64 = 0.4;
    const RATING_WEIGHT: f64 = 0.4;
    const FREQUENCY_WEIGHT: f64 = 0.2;

    // Normalize rating from 1-5 scale to 0-1 scale
    let normalized_rating = (average_rating - 1.0) / 4.0;

    // Frequency score: More interactions = higher confidence (capped at 20 interactions)
    let frequency_score = (total_interactions as f64 / 20.0).min(1.0);

    // Calculate weighted score
    (completion_rate * COMPLETION_WEIGHT)
        + (normalized_rating * RATING_WEIGHT)
        + (frequency_score * FREQUENCY_WEIGHT)
}
