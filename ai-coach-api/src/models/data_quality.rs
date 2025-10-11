use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Data quality metrics for a user on a specific date
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DataQualityMetrics {
    pub id: Uuid,
    pub user_id: Uuid,
    pub metric_date: NaiveDate,

    // Quality scores (0-100%)
    pub completeness_score: f64,
    pub consistency_score: Option<f64>,
    pub reliability_score: Option<f64>,

    // Data tracking
    pub last_data_timestamp: Option<DateTime<Utc>>,
    pub days_without_data: i32,
    pub data_sources: Option<serde_json::Value>,

    // Metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Report of missing recovery data for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingDataReport {
    pub user_id: Uuid,
    pub hrv_days_missing: i32,
    pub sleep_days_missing: i32,
    pub rhr_days_missing: i32,
    pub disconnected_wearables: Vec<String>,
    pub last_manual_entry: Option<DateTime<Utc>>,
    pub needs_reminder: bool,
}

impl MissingDataReport {
    /// Check if user needs a reminder based on missing data
    pub fn needs_reminder(&self) -> bool {
        self.needs_reminder
    }

    /// Get the longest gap in days
    pub fn max_gap_days(&self) -> i32 {
        self.hrv_days_missing
            .max(self.sleep_days_missing)
            .max(self.rhr_days_missing)
    }
}

/// Detected change in user's recovery baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineChange {
    pub user_id: Uuid,
    pub metric: String,
    pub old_value: f64,
    pub new_value: f64,
    pub percent_change: f64,
    pub significance: ChangeSignificance,
    pub detected_at: DateTime<Utc>,
    pub explanation: Option<String>,
}

impl BaselineChange {
    /// Create a new baseline change record
    pub fn new(
        user_id: Uuid,
        metric: String,
        old_value: f64,
        new_value: f64,
    ) -> Self {
        let percent_change = ((new_value - old_value) / old_value) * 100.0;
        let significance = Self::classify_change(percent_change);

        Self {
            user_id,
            metric,
            old_value,
            new_value,
            percent_change,
            significance,
            detected_at: Utc::now(),
            explanation: None,
        }
    }

    /// Classify the significance of a percentage change
    fn classify_change(percent_change: f64) -> ChangeSignificance {
        let abs_change = percent_change.abs();

        if abs_change >= 20.0 {
            ChangeSignificance::Major
        } else if abs_change >= 10.0 {
            ChangeSignificance::Moderate
        } else {
            ChangeSignificance::Minor
        }
    }

    /// Check if this is an improvement or decline
    pub fn is_improvement(&self) -> bool {
        // For HRV/Sleep, increase is improvement
        // For RHR, decrease is improvement
        match self.metric.as_str() {
            "rhr" | "RHR" | "resting_heart_rate" => self.percent_change < 0.0,
            _ => self.percent_change > 0.0,
        }
    }
}

/// Significance level of a baseline change
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeSignificance {
    #[serde(rename = "minor")]
    Minor,      // <10% change
    #[serde(rename = "moderate")]
    Moderate,   // 10-20% change
    #[serde(rename = "major")]
    Major,      // >=20% change
}

/// Data source status for tracking active integrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceStatus {
    pub hrv: bool,
    pub sleep: bool,
    pub rhr: bool,
    pub training: bool,
    pub wearable_connected: bool,
    pub last_sync: Option<DateTime<Utc>>,
}

impl DataSourceStatus {
    /// Create status from JSON value
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(value.clone()).ok()
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({}))
    }

    /// Count active data sources
    pub fn active_count(&self) -> usize {
        let mut count = 0;
        if self.hrv { count += 1; }
        if self.sleep { count += 1; }
        if self.rhr { count += 1; }
        if self.training { count += 1; }
        count
    }
}

impl Default for DataSourceStatus {
    fn default() -> Self {
        Self {
            hrv: false,
            sleep: false,
            rhr: false,
            training: false,
            wearable_connected: false,
            last_sync: None,
        }
    }
}

/// Summary statistics for data quality across users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualitySummary {
    pub total_users: usize,
    pub users_with_good_quality: usize,     // >=80% completeness
    pub users_with_poor_quality: usize,     // <50% completeness
    pub users_with_missing_data: usize,     // >=3 days gap
    pub avg_completeness_score: f64,
    pub avg_days_without_data: f64,
}

impl Default for DataQualitySummary {
    fn default() -> Self {
        Self {
            total_users: 0,
            users_with_good_quality: 0,
            users_with_poor_quality: 0,
            users_with_missing_data: 0,
            avg_completeness_score: 0.0,
            avg_days_without_data: 0.0,
        }
    }
}

impl DataQualityMetrics {
    /// Check if user needs a reminder based on metrics
    pub fn needs_reminder(&self) -> bool {
        self.days_without_data >= 3 || self.completeness_score < 50.0
    }

    /// Get reminder urgency level
    pub fn reminder_urgency(&self) -> ReminderUrgency {
        if self.days_without_data >= 7 {
            ReminderUrgency::High
        } else if self.days_without_data >= 3 || self.completeness_score < 30.0 {
            ReminderUrgency::Medium
        } else if self.completeness_score < 50.0 {
            ReminderUrgency::Low
        } else {
            ReminderUrgency::None
        }
    }
}

/// Urgency level for data quality reminders
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReminderUrgency {
    None,
    Low,
    Medium,
    High,
}
