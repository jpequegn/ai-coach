use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

use super::goal::{Goal, GoalType, GoalCategory};
use super::event::{Event, EventPriority};
use super::training_plan::{TrainingPlan, PlanStatus};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GeneratedPlan {
    pub id: Uuid,
    pub user_id: Uuid,
    pub goal_id: Option<Uuid>,
    pub event_id: Option<Uuid>,
    pub plan_name: String,
    pub plan_type: PlanType,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub total_weeks: i32,
    pub plan_structure: serde_json::Value, // JSON structure containing phases, weeks, workouts
    pub generation_parameters: serde_json::Value, // Parameters used for generation
    pub adaptation_history: serde_json::Value, // Track plan adjustments
    pub status: PlanStatus,
    pub confidence_score: Option<f64>, // AI confidence in plan effectiveness
    pub success_prediction: Option<f64>, // Predicted success probability
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "plan_type", rename_all = "snake_case")]
pub enum PlanType {
    GoalBased,       // Generated from specific goal
    EventBased,      // Generated for specific event
    Progressive,     // General progressive training plan
    Maintenance,     // Fitness maintenance plan
    Recovery,        // Recovery and base building
    Custom,          // Custom user-defined plan
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanGenerationRequest {
    pub goals: Vec<Uuid>,           // Goal IDs to base plan on
    pub events: Vec<Uuid>,          // Event IDs to target
    pub start_date: NaiveDate,
    pub preferences: UserTrainingPreferences,
    pub constraints: TrainingConstraints,
    pub plan_duration_weeks: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTrainingPreferences {
    pub available_days_per_week: i32,
    pub preferred_workout_duration: i32, // minutes
    pub max_workout_duration: i32,       // minutes
    pub intensity_preference: IntensityPreference,
    pub preferred_training_times: Vec<String>, // e.g., ["morning", "evening"]
    pub equipment_available: Vec<Equipment>,
    pub training_location: TrainingLocation,
    pub experience_level: ExperienceLevel,
    pub injury_history: Vec<String>,
    pub recovery_needs: RecoveryLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "intensity_preference", rename_all = "snake_case")]
pub enum IntensityPreference {
    LowIntensityHighVolume,
    ModerateIntensityModerateVolume,
    HighIntensityLowVolume,
    Polarized,    // Mix of very easy and very hard
    Pyramidal,    // Mostly moderate with some easy and hard
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "equipment", rename_all = "snake_case")]
pub enum Equipment {
    Road,
    Mountain,
    Gravel,
    Tt,
    Trainer,
    PowerMeter,
    HeartRateMonitor,
    Cadence,
    SmartTrainer,
    Rollers,
    Gym,
    Pool,
    Track,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "training_location", rename_all = "snake_case")]
pub enum TrainingLocation {
    Outdoor,
    Indoor,
    Mixed,
    Gym,
    Home,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "experience_level", rename_all = "snake_case")]
pub enum ExperienceLevel {
    Beginner,    // 0-1 years
    Intermediate, // 1-3 years
    Advanced,    // 3-5 years
    Expert,      // 5+ years
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "recovery_level", rename_all = "snake_case")]
pub enum RecoveryLevel {
    Fast,     // Young, no health issues
    Normal,   // Average recovery
    Slow,     // Older, health considerations
    Variable, // Depends on life stress
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConstraints {
    pub max_weekly_hours: f64,
    pub min_weekly_hours: f64,
    pub max_consecutive_hard_days: i32,
    pub required_rest_days: i32,
    pub travel_dates: Vec<DateRange>,
    pub blackout_dates: Vec<DateRange>, // Dates when training isn't possible
    pub priority_dates: Vec<DateRange>, // Dates when training is especially important
    pub equipment_limitations: Vec<String>,
    pub health_considerations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAdaptation {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub adaptation_type: AdaptationType,
    pub trigger_reason: String,
    pub changes_made: serde_json::Value,
    pub effectiveness_score: Option<f64>,
    pub applied_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "adaptation_type", rename_all = "snake_case")]
pub enum AdaptationType {
    VolumeIncrease,
    VolumeDecrease,
    IntensityIncrease,
    IntensityDecrease,
    FrequencyChange,
    RecoveryIncrease,
    GoalAdjustment,
    EventRescheduling,
    InjuryAccommodation,
    ProgressAcceleration,
    ProgressDeceleration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanWeekStructure {
    pub week_number: i32,
    pub phase_name: String,
    pub weekly_volume: f64,        // Hours or TSS
    pub weekly_intensity: f64,     // Average intensity
    pub workout_days: Vec<WorkoutDay>,
    pub rest_days: Vec<i32>,       // Day numbers (1=Monday)
    pub week_goals: Vec<String>,
    pub key_sessions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutDay {
    pub day_of_week: i32,
    pub workout_type: WorkoutType,
    pub duration_minutes: i32,
    pub intensity_zone: IntensityZone,
    pub workout_description: String,
    pub power_targets: Option<PowerTargets>,
    pub heart_rate_targets: Option<HeartRateTargets>,
    pub pace_targets: Option<PaceTargets>,
    pub equipment_needed: Vec<Equipment>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "workout_type", rename_all = "snake_case")]
pub enum WorkoutType {
    Recovery,
    Endurance,
    Tempo,
    SweetSpot,
    Threshold,
    Vo2Max,
    Neuromuscular,
    Strength,
    CrossTrain,
    Test,
    Race,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "intensity_zone", rename_all = "snake_case")]
pub enum IntensityZone {
    Zone1, // Active recovery
    Zone2, // Aerobic base
    Zone3, // Tempo
    Zone4, // Lactate threshold
    Zone5, // VO2 max
    Zone6, // Neuromuscular power
    Mixed, // Multiple zones
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerTargets {
    pub ftp_percentage_low: f64,
    pub ftp_percentage_high: f64,
    pub average_watts: Option<f64>,
    pub normalized_power: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartRateTargets {
    pub hr_percentage_low: f64,
    pub hr_percentage_high: f64,
    pub average_hr: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaceTargets {
    pub pace_per_km_low: String,  // e.g., "4:30"
    pub pace_per_km_high: String, // e.g., "5:00"
    pub pace_per_mile_low: Option<String>,
    pub pace_per_mile_high: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAlternative {
    pub id: Uuid,
    pub original_plan_id: Uuid,
    pub alternative_name: String,
    pub alternative_description: String,
    pub differences: serde_json::Value,
    pub estimated_effectiveness: f64,
    pub suitability_score: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachingInsight {
    pub plan_id: Uuid,
    pub insight_type: InsightType,
    pub title: String,
    pub description: String,
    pub recommended_action: Option<String>,
    pub importance: ImportanceLevel,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "insight_type", rename_all = "snake_case")]
pub enum InsightType {
    ProgressOptimization,
    RecoveryRecommendation,
    VolumeAdjustment,
    IntensityAdjustment,
    GoalAlignment,
    RiskMitigation,
    OpportunityHighlight,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "importance_level", rename_all = "snake_case")]
pub enum ImportanceLevel {
    Low,
    Medium,
    High,
    Critical,
}