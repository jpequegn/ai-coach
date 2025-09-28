use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Goal {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: String,
    pub goal_type: GoalType,
    pub goal_category: GoalCategory,
    pub target_value: Option<f64>,
    pub current_value: Option<f64>,
    pub unit: Option<String>,
    pub target_date: Option<NaiveDate>,
    pub status: GoalStatus,
    pub priority: GoalPriority,
    pub event_id: Option<Uuid>, // Link to specific event
    pub parent_goal_id: Option<Uuid>, // For sub-goals
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "goal_type", rename_all = "snake_case")]
pub enum GoalType {
    // Performance Goals
    Power,           // Target FTP, power output
    Pace,            // Target pace per distance
    RaceTime,        // Target race completion time
    Distance,        // Target distance achievement
    HeartRate,       // Target heart rate zones

    // Process Goals
    Consistency,     // Training consistency percentage
    WeeklyTss,       // Weekly Training Stress Score targets
    WeeklyVolume,    // Weekly training volume targets
    RecoveryMetrics, // Recovery and sleep targets

    // Event-Specific Goals
    EventPreparation, // Race/event preparation milestones
    PeakPerformance,  // Peak performance timing
    TaperExecution,   // Successful taper execution

    // Health and Fitness Goals
    Weight,          // Weight management
    BodyComposition, // Body composition targets
    Strength,        // Strength training goals
    Flexibility,     // Mobility and flexibility

    // Custom Goals
    Custom,          // User-defined custom goals
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "goal_category", rename_all = "snake_case")]
pub enum GoalCategory {
    Performance,     // Performance-based goals
    Process,         // Process and habit goals
    Event,           // Event-specific goals
    Health,          // Health and wellness goals
    Training,        // Training methodology goals
    Competition,     // Competition and racing goals
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "goal_status", rename_all = "snake_case")]
pub enum GoalStatus {
    Draft,           // Goal is being planned
    Active,          // Goal is actively being pursued
    OnTrack,         // Goal is progressing well
    AtRisk,          // Goal progress is concerning
    Completed,       // Goal has been achieved
    Failed,          // Goal was not achieved
    Paused,          // Goal is temporarily paused
    Cancelled,       // Goal has been cancelled
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "goal_priority", rename_all = "snake_case")]
pub enum GoalPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGoalRequest {
    pub title: String,
    pub description: String,
    pub goal_type: GoalType,
    pub goal_category: GoalCategory,
    pub target_value: Option<f64>,
    pub unit: Option<String>,
    pub target_date: Option<NaiveDate>,
    pub priority: GoalPriority,
    pub event_id: Option<Uuid>,
    pub parent_goal_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGoalRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub target_value: Option<f64>,
    pub current_value: Option<f64>,
    pub target_date: Option<NaiveDate>,
    pub status: Option<GoalStatus>,
    pub priority: Option<GoalPriority>,
    pub event_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GoalProgress {
    pub id: Uuid,
    pub goal_id: Uuid,
    pub value: f64,
    pub date: NaiveDate,
    pub note: Option<String>,
    pub milestone_achieved: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGoalProgressRequest {
    pub value: f64,
    pub date: Option<NaiveDate>,
    pub note: Option<String>,
    pub milestone_achieved: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalProgressSummary {
    pub goal_id: Uuid,
    pub progress_percentage: Option<f64>,
    pub trend_direction: TrendDirection,
    pub projected_completion_date: Option<NaiveDate>,
    pub recent_entries: Vec<GoalProgress>,
    pub milestones_achieved: Vec<String>,
    pub success_probability: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "trend_direction", rename_all = "snake_case")]
pub enum TrendDirection {
    Improving,
    Stable,
    Declining,
    Insufficient, // Not enough data
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalRecommendation {
    pub goal_id: Uuid,
    pub recommendation_type: RecommendationType,
    pub title: String,
    pub description: String,
    pub priority: GoalPriority,
    pub suggested_actions: Vec<String>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "recommendation_type", rename_all = "snake_case")]
pub enum RecommendationType {
    AdjustTarget,    // Suggest adjusting goal target
    ExtendDeadline,  // Suggest extending deadline
    IncreaseEffort,  // Suggest increasing training effort
    ChangeStrategy,  // Suggest changing approach
    Celebration,     // Acknowledge achievement
    Warning,         // Warn about potential failure
}