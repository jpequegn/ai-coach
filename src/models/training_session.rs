use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrainingSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub date: NaiveDate,
    pub trainrs_data: Option<Value>,
    pub uploaded_file_path: Option<String>,
    pub session_type: Option<String>,
    pub duration_seconds: Option<i32>,
    pub distance_meters: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTrainingSession {
    pub user_id: Uuid,
    pub date: NaiveDate,
    pub trainrs_data: Option<Value>,
    pub uploaded_file_path: Option<String>,
    pub session_type: Option<String>,
    pub duration_seconds: Option<i32>,
    pub distance_meters: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTrainingSession {
    pub date: Option<NaiveDate>,
    pub trainrs_data: Option<Value>,
    pub uploaded_file_path: Option<String>,
    pub session_type: Option<String>,
    pub duration_seconds: Option<i32>,
    pub distance_meters: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionSummary {
    pub total_sessions: i64,
    pub total_duration: Option<i64>,
    pub total_distance: Option<f64>,
    pub average_duration: Option<f64>,
    pub session_types: Vec<String>,
}