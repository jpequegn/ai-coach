use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Alert delivery queue entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertDeliveryQueue {
    pub id: Uuid,
    pub alert_id: Uuid,
    pub delivery_method: DeliveryMethod,
    pub recipient_id: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub status: DeliveryQueueStatus,
    pub error_message: Option<String>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Delivery method for alerts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum DeliveryMethod {
    #[serde(rename = "push")]
    #[sqlx(rename = "push")]
    Push,
    #[serde(rename = "email")]
    #[sqlx(rename = "email")]
    Email,
    #[serde(rename = "sms")]
    #[sqlx(rename = "sms")]
    Sms,
}

/// Delivery queue status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum DeliveryQueueStatus {
    #[serde(rename = "pending")]
    #[sqlx(rename = "pending")]
    Pending,
    #[serde(rename = "delivered")]
    #[sqlx(rename = "delivered")]
    Delivered,
    #[serde(rename = "failed")]
    #[sqlx(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    #[sqlx(rename = "cancelled")]
    Cancelled,
}

/// Delivery statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryStats {
    pub total_attempted: usize,
    pub successful: usize,
    pub failed: usize,
    pub retrying: usize,
    pub delivery_time_avg_ms: u64,
    pub by_method: std::collections::HashMap<String, MethodStats>,
}

/// Statistics per delivery method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodStats {
    pub attempted: usize,
    pub successful: usize,
    pub failed: usize,
    pub success_rate: f64,
}

impl Default for DeliveryStats {
    fn default() -> Self {
        Self {
            total_attempted: 0,
            successful: 0,
            failed: 0,
            retrying: 0,
            delivery_time_avg_ms: 0,
            by_method: std::collections::HashMap::new(),
        }
    }
}
