use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// Database Models
// ============================================================================

/// Daily calculated recovery score
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecoveryScore {
    pub id: Uuid,
    pub user_id: Uuid,
    pub score_date: NaiveDate,
    pub readiness_score: f64,
    pub hrv_trend: String,
    pub hrv_deviation: Option<f64>,
    pub sleep_quality_score: Option<f64>,
    pub recovery_adequacy: Option<f64>,
    pub rhr_deviation: Option<f64>,
    pub training_strain: Option<f64>,
    pub recovery_status: String,
    pub recommended_tss_adjustment: Option<f64>,
    pub calculated_at: DateTime<Utc>,
    pub model_version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Recovery alert
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecoveryAlert {
    pub id: Uuid,
    pub user_id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub recovery_score_id: Option<Uuid>,
    pub message: String,
    pub recommendations: Option<sqlx::types::Json<Vec<AlertRecommendation>>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRecommendation {
    pub priority: String,
    pub category: String,
    pub message: String,
    pub action: String,
}

// ============================================================================
// Response DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStatusResponse {
    pub date: NaiveDate,
    pub readiness_score: f64,
    pub recovery_status: String,
    pub hrv_trend: String,
    pub hrv_deviation: Option<f64>,
    pub sleep_quality: Option<f64>,
    pub recovery_adequacy: Option<f64>,
    pub rhr_deviation: Option<f64>,
    pub recommended_tss_adjustment: Option<f64>,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub priority: String,
    pub category: String,
    pub message: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryTrendsResponse {
    pub period_days: i32,
    pub average_readiness: f64,
    pub trend_direction: String, // improving, stable, declining
    pub data_points: Vec<RecoveryDataPoint>,
    pub patterns: Vec<RecoveryPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryDataPoint {
    pub date: NaiveDate,
    pub readiness_score: f64,
    pub recovery_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPattern {
    pub pattern_type: String,
    pub description: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryHistoryResponse {
    pub scores: Vec<RecoveryScoreResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryScoreResponse {
    pub id: Uuid,
    pub date: NaiveDate,
    pub readiness_score: f64,
    pub recovery_status: String,
    pub hrv_trend: String,
    pub sleep_quality: Option<f64>,
    pub recovery_adequacy: Option<f64>,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryInsightsResponse {
    pub insights: Vec<RecoveryInsight>,
    pub key_factors: Vec<KeyFactor>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryInsight {
    pub category: String,
    pub title: String,
    pub description: String,
    pub impact: String, // positive, negative, neutral
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFactor {
    pub factor: String,
    pub current_value: f64,
    pub baseline_value: f64,
    pub deviation_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertResponse {
    pub id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub recommendations: Vec<AlertRecommendation>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSettingsResponse {
    pub poor_recovery_enabled: bool,
    pub poor_recovery_threshold: f64,
    pub declining_hrv_enabled: bool,
    pub declining_hrv_days: i32,
    pub critical_recovery_enabled: bool,
    pub critical_recovery_threshold: f64,
    pub high_strain_enabled: bool,
    pub high_strain_threshold: f64,
}

// ============================================================================
// Request DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct RecoveryHistoryQuery {
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub limit: Option<i64>,
    pub page: Option<i64>,
}

impl Default for RecoveryHistoryQuery {
    fn default() -> Self {
        Self {
            from_date: None,
            to_date: None,
            limit: Some(30),
            page: Some(1),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAlertSettings {
    pub poor_recovery_enabled: Option<bool>,
    pub poor_recovery_threshold: Option<f64>,
    pub declining_hrv_enabled: Option<bool>,
    pub declining_hrv_days: Option<i32>,
    pub critical_recovery_enabled: Option<bool>,
    pub critical_recovery_threshold: Option<f64>,
    pub high_strain_enabled: Option<bool>,
    pub high_strain_threshold: Option<f64>,
}

// ============================================================================
// Enums
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HrvTrend {
    Improving,
    Stable,
    Declining,
    InsufficientData,
}

impl HrvTrend {
    pub fn as_str(&self) -> &str {
        match self {
            HrvTrend::Improving => "improving",
            HrvTrend::Stable => "stable",
            HrvTrend::Declining => "declining",
            HrvTrend::InsufficientData => "insufficient_data",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryStatus {
    Optimal,
    Good,
    Fair,
    Poor,
    Critical,
}

impl RecoveryStatus {
    pub fn as_str(&self) -> &str {
        match self {
            RecoveryStatus::Optimal => "optimal",
            RecoveryStatus::Good => "good",
            RecoveryStatus::Fair => "fair",
            RecoveryStatus::Poor => "poor",
            RecoveryStatus::Critical => "critical",
        }
    }

    pub fn from_score(score: f64) -> Self {
        if score >= 85.0 {
            RecoveryStatus::Optimal
        } else if score >= 70.0 {
            RecoveryStatus::Good
        } else if score >= 50.0 {
            RecoveryStatus::Fair
        } else if score >= 30.0 {
            RecoveryStatus::Poor
        } else {
            RecoveryStatus::Critical
        }
    }
}

// ============================================================================
// Conversion Implementations
// ============================================================================

impl From<RecoveryScore> for RecoveryScoreResponse {
    fn from(score: RecoveryScore) -> Self {
        Self {
            id: score.id,
            date: score.score_date,
            readiness_score: score.readiness_score,
            recovery_status: score.recovery_status,
            hrv_trend: score.hrv_trend,
            sleep_quality: score.sleep_quality_score,
            recovery_adequacy: score.recovery_adequacy,
            calculated_at: score.calculated_at,
        }
    }
}

impl From<RecoveryAlert> for AlertResponse {
    fn from(alert: RecoveryAlert) -> Self {
        Self {
            id: alert.id,
            alert_type: alert.alert_type,
            severity: alert.severity,
            message: alert.message,
            recommendations: alert.recommendations.map(|r| r.0).unwrap_or_default(),
            acknowledged_at: alert.acknowledged_at,
            created_at: alert.created_at,
        }
    }
}
