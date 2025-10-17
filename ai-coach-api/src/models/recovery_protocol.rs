use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

use super::recommendation::RecommendationCategory;

/// Protocol sequence type enum
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum ProtocolSequence {
    #[serde(rename = "parallel")]
    Parallel,    // All recommendations at once
    #[serde(rename = "sequential")]
    Sequential,  // One after another
    #[serde(rename = "phased")]
    Phased,      // Spread over multiple days
}

impl std::fmt::Display for ProtocolSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parallel => write!(f, "parallel"),
            Self::Sequential => write!(f, "sequential"),
            Self::Phased => write!(f, "phased"),
        }
    }
}

/// Protocol status enum
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum ProtocolStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "abandoned")]
    Abandoned,
}

impl std::fmt::Display for ProtocolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Completed => write!(f, "completed"),
            Self::Abandoned => write!(f, "abandoned"),
        }
    }
}

/// Recovery protocol template model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecoveryProtocol {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub duration_days: i32,
    pub sequence_type: ProtocolSequence,
    pub category: String,
    pub target_scenarios: String, // JSON array stored as string in SQLite
    pub effectiveness_score: f64,
    pub times_activated: i32,
    pub times_completed: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Protocol recommendation link model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProtocolRecommendation {
    pub id: Uuid,
    pub protocol_id: Uuid,
    pub recommendation_template_id: Uuid,
    pub sequence_order: i32,
    pub day_number: Option<i32>,
    pub is_required: bool,
    pub created_at: DateTime<Utc>,
}

/// User protocol activation model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserProtocolActivation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub protocol_id: Uuid,
    pub status: ProtocolStatus,
    pub progress: String, // JSON stored as string in SQLite
    pub activated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub abandoned_at: Option<DateTime<Utc>>,
    pub abandonment_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Request to create a new recovery protocol
#[derive(Debug, Deserialize)]
pub struct CreateRecoveryProtocolRequest {
    pub name: String,
    pub description: String,
    pub duration_days: i32,
    pub sequence_type: ProtocolSequence,
    pub category: String,
    pub target_scenarios: Vec<String>,
}

/// Request to activate a protocol
#[derive(Debug, Deserialize)]
pub struct ActivateProtocolRequest {
    pub protocol_id: Uuid,
}

/// Request to update protocol progress
#[derive(Debug, Deserialize)]
pub struct UpdateProtocolProgressRequest {
    pub recommendation_id: Uuid,
    pub completed: bool,
}

/// Request to abandon a protocol
#[derive(Debug, Deserialize)]
pub struct AbandonProtocolRequest {
    pub reason: Option<String>,
}

/// Query parameters for protocol listing
#[derive(Debug, Deserialize)]
pub struct ProtocolQuery {
    pub scenario: Option<String>,
    pub duration_max: Option<i32>,
    pub category: Option<String>,
}

/// Protocol response with recommendations
#[derive(Debug, Serialize)]
pub struct ProtocolResponse {
    #[serde(flatten)]
    pub protocol: RecoveryProtocol,
    pub recommendations: Vec<ProtocolRecommendationDetail>,
    pub target_scenarios_parsed: Vec<String>,
}

/// Protocol recommendation detail for responses
#[derive(Debug, Serialize)]
pub struct ProtocolRecommendationDetail {
    pub recommendation_id: Uuid,
    pub sequence_order: i32,
    pub day_number: Option<i32>,
    pub is_required: bool,
    pub title: String,
    pub description: String,
    pub action: String,
}

/// Active protocol with progress
#[derive(Debug, Serialize)]
pub struct ActiveProtocolResponse {
    pub activation_id: Uuid,
    #[serde(flatten)]
    pub protocol: RecoveryProtocol,
    pub status: ProtocolStatus,
    pub progress: ProtocolProgress,
    pub activated_at: DateTime<Utc>,
    pub recommendations: Vec<ProtocolRecommendationProgress>,
}

/// Protocol progress summary
#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolProgress {
    pub total_recommendations: i32,
    pub completed_count: i32,
    pub completion_percentage: f64,
}

/// Individual recommendation progress in protocol
#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolRecommendationProgress {
    pub recommendation_id: Uuid,
    pub title: String,
    pub sequence_order: i32,
    pub day_number: Option<i32>,
    pub is_required: bool,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Protocol activation response
#[derive(Debug, Serialize)]
pub struct ProtocolActivationResponse {
    pub activation_id: Uuid,
    pub protocol: RecoveryProtocol,
    pub status: ProtocolStatus,
    pub next_recommendations: Vec<ProtocolRecommendationDetail>,
    pub message: String,
}
