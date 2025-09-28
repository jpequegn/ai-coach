use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Event {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub event_type: EventType,
    pub sport: Sport,
    pub event_date: NaiveDate,
    pub event_time: Option<NaiveTime>,
    pub location: Option<String>,
    pub distance: Option<f64>,
    pub distance_unit: Option<String>,
    pub elevation_gain: Option<f64>,
    pub expected_duration: Option<i32>, // minutes
    pub registration_deadline: Option<NaiveDate>,
    pub cost: Option<f64>,
    pub website_url: Option<String>,
    pub notes: Option<String>,
    pub status: EventStatus,
    pub priority: EventPriority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "event_type", rename_all = "snake_case")]
pub enum EventType {
    Race,
    Competition,
    Training,
    GroupRide,
    Clinic,
    Workshop,
    Social,
    Volunteer,
    Personal, // Personal milestone/test
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "sport", rename_all = "snake_case")]
pub enum Sport {
    Cycling,
    Running,
    Swimming,
    Triathlon,
    Duathlon,
    CrossTraining,
    Strength,
    Yoga,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "event_status", rename_all = "snake_case")]
pub enum EventStatus {
    Planned,
    Registered,
    Confirmed,
    InProgress,
    Completed,
    Cancelled,
    Missed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "event_priority", rename_all = "snake_case")]
pub enum EventPriority {
    Low,     // Fun events, social rides
    Medium,  // Regular training events
    High,    // Important races
    Critical, // A-priority races
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventRequest {
    pub name: String,
    pub description: Option<String>,
    pub event_type: EventType,
    pub sport: Sport,
    pub event_date: NaiveDate,
    pub event_time: Option<NaiveTime>,
    pub location: Option<String>,
    pub distance: Option<f64>,
    pub distance_unit: Option<String>,
    pub elevation_gain: Option<f64>,
    pub expected_duration: Option<i32>,
    pub registration_deadline: Option<NaiveDate>,
    pub cost: Option<f64>,
    pub website_url: Option<String>,
    pub notes: Option<String>,
    pub priority: EventPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEventRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub event_date: Option<NaiveDate>,
    pub event_time: Option<NaiveTime>,
    pub location: Option<String>,
    pub distance: Option<f64>,
    pub distance_unit: Option<String>,
    pub elevation_gain: Option<f64>,
    pub expected_duration: Option<i32>,
    pub registration_deadline: Option<NaiveDate>,
    pub cost: Option<f64>,
    pub website_url: Option<String>,
    pub notes: Option<String>,
    pub status: Option<EventStatus>,
    pub priority: Option<EventPriority>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EventPlan {
    pub id: Uuid,
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub training_phases: serde_json::Value, // JSON structure for periodization
    pub peak_date: NaiveDate,
    pub taper_start_date: NaiveDate,
    pub base_training_weeks: i32,
    pub build_training_weeks: i32,
    pub peak_training_weeks: i32,
    pub taper_weeks: i32,
    pub recovery_weeks: i32,
    pub travel_considerations: Option<String>,
    pub logistics_notes: Option<String>,
    pub equipment_checklist: Option<serde_json::Value>,
    pub nutrition_plan: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventPlanRequest {
    pub peak_date: NaiveDate,
    pub base_training_weeks: i32,
    pub build_training_weeks: i32,
    pub peak_training_weeks: i32,
    pub taper_weeks: i32,
    pub recovery_weeks: i32,
    pub travel_considerations: Option<String>,
    pub logistics_notes: Option<String>,
    pub equipment_checklist: Option<serde_json::Value>,
    pub nutrition_plan: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPhase {
    pub phase_name: String,
    pub phase_type: PhaseType,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub weeks: i32,
    pub weekly_volume_range: (f64, f64), // min, max hours or TSS
    pub intensity_distribution: IntensityDistribution,
    pub focus_areas: Vec<String>,
    pub key_workouts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "phase_type", rename_all = "snake_case")]
pub enum PhaseType {
    Base,
    Build,
    Peak,
    Taper,
    Recovery,
    Transition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntensityDistribution {
    pub zone1_percentage: f64, // Easy/Recovery
    pub zone2_percentage: f64, // Aerobic base
    pub zone3_percentage: f64, // Tempo
    pub zone4_percentage: f64, // Lactate threshold
    pub zone5_percentage: f64, // VO2 max
    pub zone6_percentage: f64, // Neuromuscular power
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCalendar {
    pub events: Vec<Event>,
    pub event_plans: Vec<EventPlan>,
    pub conflicts: Vec<EventConflict>,
    pub recommendations: Vec<EventRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventConflict {
    pub event1_id: Uuid,
    pub event2_id: Uuid,
    pub conflict_type: ConflictType,
    pub severity: ConflictSeverity,
    pub description: String,
    pub suggested_resolution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "conflict_type", rename_all = "snake_case")]
pub enum ConflictType {
    DateOverlap,
    TooClose,        // Events too close together
    TrainingConflict, // Conflicts with training plan
    RecoveryNeeded,   // Insufficient recovery time
    TravelConflict,   // Travel schedule conflicts
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "conflict_severity", rename_all = "snake_case")]
pub enum ConflictSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecommendation {
    pub event_id: Uuid,
    pub recommendation_type: EventRecommendationType,
    pub title: String,
    pub description: String,
    pub priority: EventPriority,
    pub action_required: bool,
    pub deadline: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[sqlx(type_name = "event_recommendation_type", rename_all = "snake_case")]
pub enum EventRecommendationType {
    RegisterSoon,
    AdjustTraining,
    BookTravel,
    CheckEquipment,
    NutritionPlan,
    TaperStart,
    RecoveryPlan,
    ConflictResolution,
}