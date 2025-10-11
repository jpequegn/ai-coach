use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Data quality metrics for a user
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DataQualityMetrics {
    pub id: Uuid,
    pub user_id: Uuid,
    pub completeness_score: f64,
    pub consistency_score: f64,
    pub reliability_score: f64,
    pub overall_score: f64,
    pub days_analyzed: i32,
    pub missing_hrv_days: i32,
    pub missing_sleep_days: i32,
    pub missing_rhr_days: i32,
    pub last_hrv_reading: Option<DateTime<Utc>>,
    pub last_sleep_reading: Option<DateTime<Utc>>,
    pub last_rhr_reading: Option<DateTime<Utc>>,
    pub wearable_connected: bool,
    pub reminder_sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Missing data report for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingDataReport {
    pub user_id: Uuid,
    pub missing_hrv_days: i32,
    pub missing_sleep_days: i32,
    pub missing_rhr_days: i32,
    pub last_hrv_reading: Option<DateTime<Utc>>,
    pub last_sleep_reading: Option<DateTime<Utc>>,
    pub last_rhr_reading: Option<DateTime<Utc>>,
    pub wearable_connected: bool,
    pub days_since_last_reading: i32,
    pub urgency_level: UrgencyLevel,
}

/// Urgency level for data quality reminders
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UrgencyLevel {
    None,      // All data current
    Low,       // 1-2 days missing
    Medium,    // 3-5 days missing
    High,      // 6-10 days missing
    Critical,  // 10+ days missing
}

impl UrgencyLevel {
    pub fn from_days_missing(days: i32) -> Self {
        match days {
            0 => Self::None,
            1..=2 => Self::Low,
            3..=5 => Self::Medium,
            6..=10 => Self::High,
            _ => Self::Critical,
        }
    }
}

/// Data source type for reliability calculation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DataSource {
    Manual,          // User manual entry
    Wearable,        // Wearable device sync
    ApiIntegration,  // External API (Oura, etc.)
}

impl DataSource {
    /// Get reliability score for this data source
    pub fn reliability_score(&self) -> f64 {
        match self {
            Self::Manual => 0.70,
            Self::Wearable => 0.90,
            Self::ApiIntegration => 0.95,
        }
    }
}
