use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AthleteProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub sport: String,
    pub ftp: Option<i32>,
    pub lthr: Option<i32>,
    pub max_heart_rate: Option<i32>,
    pub threshold_pace: Option<f64>,
    pub zones: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAthleteProfile {
    pub user_id: Uuid,
    pub sport: String,
    pub ftp: Option<i32>,
    pub lthr: Option<i32>,
    pub max_heart_rate: Option<i32>,
    pub threshold_pace: Option<f64>,
    pub zones: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAthleteProfile {
    pub sport: Option<String>,
    pub ftp: Option<i32>,
    pub lthr: Option<i32>,
    pub max_heart_rate: Option<i32>,
    pub threshold_pace: Option<f64>,
    pub zones: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZoneData {
    pub zone_1_min: Option<i32>,
    pub zone_1_max: Option<i32>,
    pub zone_2_min: Option<i32>,
    pub zone_2_max: Option<i32>,
    pub zone_3_min: Option<i32>,
    pub zone_3_max: Option<i32>,
    pub zone_4_min: Option<i32>,
    pub zone_4_max: Option<i32>,
    pub zone_5_min: Option<i32>,
    pub zone_5_max: Option<i32>,
}