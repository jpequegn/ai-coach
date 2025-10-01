use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrainingPlan {
    pub id: Uuid,
    pub user_id: Uuid,
    pub goal: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub plan_data: Value,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTrainingPlan {
    pub user_id: Uuid,
    pub goal: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub plan_data: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTrainingPlan {
    pub goal: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub plan_data: Option<Value>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PlanStatus {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

impl ToString for PlanStatus {
    fn to_string(&self) -> String {
        match self {
            PlanStatus::Draft => "draft".to_string(),
            PlanStatus::Active => "active".to_string(),
            PlanStatus::Completed => "completed".to_string(),
            PlanStatus::Cancelled => "cancelled".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlanWeek {
    pub week_number: i32,
    pub weekly_volume: Option<f64>,
    pub weekly_intensity: Option<f64>,
    pub workouts: Vec<PlannedWorkout>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlannedWorkout {
    pub day_of_week: i32,
    pub workout_type: String,
    pub duration_minutes: Option<i32>,
    pub distance_meters: Option<f64>,
    pub intensity: Option<String>,
    pub description: Option<String>,
}