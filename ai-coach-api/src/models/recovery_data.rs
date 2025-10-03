use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

// ============================================================================
// Database Models
// ============================================================================

/// Heart Rate Variability reading from wearable devices or manual input
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct HrvReading {
    pub id: Uuid,
    pub user_id: Uuid,
    pub measurement_date: NaiveDate,
    pub measurement_timestamp: DateTime<Utc>,
    /// Root Mean Square of Successive Differences (ms) - primary HRV metric
    pub rmssd: f64,
    /// Standard Deviation of NN intervals (ms) - optional
    pub sdnn: Option<f64>,
    /// Percentage of successive NN intervals differing >50ms (%) - optional
    pub pnn50: Option<f64>,
    /// Data source: oura, whoop, manual, apple_health, garmin, polar, fitbit
    pub source: String,
    /// Additional metadata in JSON format
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// Sleep data including stages and efficiency
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SleepData {
    pub id: Uuid,
    pub user_id: Uuid,
    pub sleep_date: NaiveDate,
    pub total_sleep_hours: f64,
    pub deep_sleep_hours: Option<f64>,
    pub rem_sleep_hours: Option<f64>,
    pub light_sleep_hours: Option<f64>,
    pub awake_hours: Option<f64>,
    /// Sleep efficiency percentage (0-100)
    pub sleep_efficiency: Option<f64>,
    /// Time to fall asleep (minutes)
    pub sleep_latency_minutes: Option<i32>,
    pub bedtime: Option<DateTime<Utc>>,
    pub wake_time: Option<DateTime<Utc>>,
    /// Data source: oura, whoop, manual, apple_health, garmin, polar, fitbit
    pub source: String,
    /// Additional metadata in JSON format
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// Resting heart rate measurements
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RestingHrData {
    pub id: Uuid,
    pub user_id: Uuid,
    pub measurement_date: NaiveDate,
    pub measurement_timestamp: DateTime<Utc>,
    pub resting_hr: f64,
    /// Data source: oura, whoop, manual, apple_health, garmin, polar, fitbit
    pub source: String,
    /// Additional metadata in JSON format
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
}

/// Calculated baseline values for recovery metrics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecoveryBaseline {
    pub id: Uuid,
    pub user_id: Uuid,
    /// Baseline RMSSD value (30-day average)
    pub hrv_baseline_rmssd: Option<f64>,
    /// Baseline resting heart rate (30-day average)
    pub rhr_baseline: Option<f64>,
    /// Typical sleep duration (30-day average)
    pub typical_sleep_hours: Option<f64>,
    pub calculated_at: DateTime<Utc>,
    /// Number of data points used for calculation
    pub data_points_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// OAuth connection to wearable device platform
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WearableConnection {
    pub id: Uuid,
    pub user_id: Uuid,
    /// Provider: oura, whoop, apple_health, garmin, polar, fitbit
    pub provider: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub provider_user_id: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub connected_at: DateTime<Utc>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    /// Additional metadata in JSON format
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Request DTOs
// ============================================================================

/// Request to create HRV reading
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateHrvReadingRequest {
    #[validate(range(min = 0.0, max = 200.0, message = "RMSSD must be between 0 and 200 ms"))]
    pub rmssd: f64,

    #[validate(range(min = 0.0, max = 200.0, message = "SDNN must be between 0 and 200 ms"))]
    pub sdnn: Option<f64>,

    #[validate(range(min = 0.0, max = 100.0, message = "pNN50 must be between 0 and 100%"))]
    pub pnn50: Option<f64>,

    pub measurement_timestamp: Option<DateTime<Utc>>,

    pub metadata: Option<serde_json::Value>,
}

/// Request to create sleep data
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateSleepDataRequest {
    #[validate(range(min = 0.0, max = 24.0, message = "Total sleep must be between 0 and 24 hours"))]
    pub total_sleep_hours: f64,

    #[validate(range(min = 0.0, max = 24.0, message = "Deep sleep must be between 0 and 24 hours"))]
    pub deep_sleep_hours: Option<f64>,

    #[validate(range(min = 0.0, max = 24.0, message = "REM sleep must be between 0 and 24 hours"))]
    pub rem_sleep_hours: Option<f64>,

    #[validate(range(min = 0.0, max = 24.0, message = "Light sleep must be between 0 and 24 hours"))]
    pub light_sleep_hours: Option<f64>,

    #[validate(range(min = 0.0, max = 24.0, message = "Awake time must be between 0 and 24 hours"))]
    pub awake_hours: Option<f64>,

    #[validate(range(min = 0.0, max = 100.0, message = "Sleep efficiency must be between 0 and 100%"))]
    pub sleep_efficiency: Option<f64>,

    #[validate(range(min = 0, message = "Sleep latency must be positive"))]
    pub sleep_latency_minutes: Option<i32>,

    pub bedtime: Option<DateTime<Utc>>,
    pub wake_time: Option<DateTime<Utc>>,
    pub sleep_date: Option<NaiveDate>,
    pub metadata: Option<serde_json::Value>,
}

/// Request to create resting heart rate reading
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateRestingHrRequest {
    #[validate(range(min = 30.0, max = 150.0, message = "Resting HR must be between 30 and 150 bpm"))]
    pub resting_hr: f64,

    pub measurement_timestamp: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Response for HRV reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrvReadingResponse {
    pub id: Uuid,
    pub measurement_date: NaiveDate,
    pub measurement_timestamp: DateTime<Utc>,
    pub rmssd: f64,
    pub sdnn: Option<f64>,
    pub pnn50: Option<f64>,
    pub source: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Response for sleep data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepDataResponse {
    pub id: Uuid,
    pub sleep_date: NaiveDate,
    pub total_sleep_hours: f64,
    pub deep_sleep_hours: Option<f64>,
    pub rem_sleep_hours: Option<f64>,
    pub light_sleep_hours: Option<f64>,
    pub awake_hours: Option<f64>,
    pub sleep_efficiency: Option<f64>,
    pub sleep_latency_minutes: Option<i32>,
    pub bedtime: Option<DateTime<Utc>>,
    pub wake_time: Option<DateTime<Utc>>,
    pub source: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Response for resting heart rate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestingHrResponse {
    pub id: Uuid,
    pub measurement_date: NaiveDate,
    pub measurement_timestamp: DateTime<Utc>,
    pub resting_hr: f64,
    pub source: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Response for recovery baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryBaselineResponse {
    pub id: Uuid,
    pub hrv_baseline_rmssd: Option<f64>,
    pub rhr_baseline: Option<f64>,
    pub typical_sleep_hours: Option<f64>,
    pub calculated_at: DateTime<Utc>,
    pub data_points_count: i32,
}

/// Response for wearable connection (without sensitive tokens)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WearableConnectionResponse {
    pub id: Uuid,
    pub provider: String,
    pub provider_user_id: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub connected_at: DateTime<Utc>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// List of HRV readings with pagination info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrvReadingsListResponse {
    pub readings: Vec<HrvReadingResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// List of sleep data with pagination info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepDataListResponse {
    pub sleep_records: Vec<SleepDataResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// List of resting HR with pagination info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestingHrListResponse {
    pub readings: Vec<RestingHrResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

// ============================================================================
// Conversion Implementations
// ============================================================================

impl From<HrvReading> for HrvReadingResponse {
    fn from(reading: HrvReading) -> Self {
        Self {
            id: reading.id,
            measurement_date: reading.measurement_date,
            measurement_timestamp: reading.measurement_timestamp,
            rmssd: reading.rmssd,
            sdnn: reading.sdnn,
            pnn50: reading.pnn50,
            source: reading.source,
            metadata: reading.metadata.map(|v| v.0),
            created_at: reading.created_at,
        }
    }
}

impl From<SleepData> for SleepDataResponse {
    fn from(data: SleepData) -> Self {
        Self {
            id: data.id,
            sleep_date: data.sleep_date,
            total_sleep_hours: data.total_sleep_hours,
            deep_sleep_hours: data.deep_sleep_hours,
            rem_sleep_hours: data.rem_sleep_hours,
            light_sleep_hours: data.light_sleep_hours,
            awake_hours: data.awake_hours,
            sleep_efficiency: data.sleep_efficiency,
            sleep_latency_minutes: data.sleep_latency_minutes,
            bedtime: data.bedtime,
            wake_time: data.wake_time,
            source: data.source,
            metadata: data.metadata.map(|v| v.0),
            created_at: data.created_at,
        }
    }
}

impl From<RestingHrData> for RestingHrResponse {
    fn from(data: RestingHrData) -> Self {
        Self {
            id: data.id,
            measurement_date: data.measurement_date,
            measurement_timestamp: data.measurement_timestamp,
            resting_hr: data.resting_hr,
            source: data.source,
            metadata: data.metadata.map(|v| v.0),
            created_at: data.created_at,
        }
    }
}

impl From<RecoveryBaseline> for RecoveryBaselineResponse {
    fn from(baseline: RecoveryBaseline) -> Self {
        Self {
            id: baseline.id,
            hrv_baseline_rmssd: baseline.hrv_baseline_rmssd,
            rhr_baseline: baseline.rhr_baseline,
            typical_sleep_hours: baseline.typical_sleep_hours,
            calculated_at: baseline.calculated_at,
            data_points_count: baseline.data_points_count,
        }
    }
}

impl From<WearableConnection> for WearableConnectionResponse {
    fn from(conn: WearableConnection) -> Self {
        Self {
            id: conn.id,
            provider: conn.provider,
            provider_user_id: conn.provider_user_id,
            scopes: conn.scopes,
            connected_at: conn.connected_at,
            last_sync_at: conn.last_sync_at,
            is_active: conn.is_active,
        }
    }
}

// ============================================================================
// Query Parameters
// ============================================================================

/// Query parameters for filtering recovery data
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct RecoveryDataQuery {
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,

    #[validate(range(min = 1, max = 1000, message = "Limit must be between 1 and 1000"))]
    pub limit: Option<i64>,

    #[validate(range(min = 1, message = "Page must be positive"))]
    pub page: Option<i64>,
}

impl Default for RecoveryDataQuery {
    fn default() -> Self {
        Self {
            from_date: None,
            to_date: None,
            limit: Some(100),
            page: Some(1),
        }
    }
}

// ============================================================================
// Data Source Enum
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    Oura,
    Whoop,
    Manual,
    AppleHealth,
    Garmin,
    Polar,
    Fitbit,
}

impl DataSource {
    pub fn as_str(&self) -> &str {
        match self {
            DataSource::Oura => "oura",
            DataSource::Whoop => "whoop",
            DataSource::Manual => "manual",
            DataSource::AppleHealth => "apple_health",
            DataSource::Garmin => "garmin",
            DataSource::Polar => "polar",
            DataSource::Fitbit => "fitbit",
        }
    }
}

impl std::fmt::Display for DataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
