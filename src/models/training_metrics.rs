use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Training metrics extracted from workout files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub duration_seconds: Option<i32>,
    pub distance_meters: Option<f64>,
    pub elevation_gain_meters: Option<f64>,
    pub average_power: Option<f64>,
    pub normalized_power: Option<f64>,
    pub average_heart_rate: Option<f64>,
    pub max_heart_rate: Option<f64>,
    pub average_cadence: Option<f64>,
    pub max_cadence: Option<f64>,
    pub average_speed: Option<f64>,
    pub max_speed: Option<f64>,
    pub tss: Option<f64>, // Training Stress Score
    pub intensity_factor: Option<f64>,
    pub work: Option<f64>, // Total work in kJ
    pub calories: Option<f64>,
    pub power_zones: Option<PowerZoneDistribution>,
    pub heart_rate_zones: Option<HeartRateZoneDistribution>,
    pub pace_zones: Option<PaceZoneDistribution>,
}

/// Power zone distribution data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerZoneDistribution {
    pub zone_1: f64, // Active Recovery (<55% FTP)
    pub zone_2: f64, // Endurance (55-75% FTP)
    pub zone_3: f64, // Tempo (75-90% FTP)
    pub zone_4: f64, // Lactate Threshold (90-105% FTP)
    pub zone_5: f64, // VO2 Max (105-120% FTP)
    pub zone_6: f64, // Anaerobic Capacity (120-150% FTP)
    pub zone_7: f64, // Neuromuscular Power (>150% FTP)
}

/// Heart rate zone distribution data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartRateZoneDistribution {
    pub zone_1: f64, // Recovery (<68% LTHR)
    pub zone_2: f64, // Aerobic Base (68-83% LTHR)
    pub zone_3: f64, // Aerobic Build (83-94% LTHR)
    pub zone_4: f64, // Lactate Threshold (94-105% LTHR)
    pub zone_5: f64, // VO2 Max (>105% LTHR)
}

/// Pace zone distribution data (for running)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaceZoneDistribution {
    pub zone_1: f64, // Recovery
    pub zone_2: f64, // Aerobic Base
    pub zone_3: f64, // Aerobic Build
    pub zone_4: f64, // Lactate Threshold
    pub zone_5: f64, // VO2 Max
}

/// Performance Management Chart data point
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PerformanceManagementChart {
    pub id: Uuid,
    pub user_id: Uuid,
    pub date: NaiveDate,
    pub ctl: f64, // Chronic Training Load (Fitness)
    pub atl: f64, // Acute Training Load (Fatigue)
    pub tsb: f64, // Training Stress Balance (Form)
    pub tss_daily: f64, // Daily Training Stress Score
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create PMC entry
#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePMC {
    pub user_id: Uuid,
    pub date: NaiveDate,
    pub ctl: f64,
    pub atl: f64,
    pub tsb: f64,
    pub tss_daily: f64,
}

/// Update PMC entry
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePMC {
    pub ctl: Option<f64>,
    pub atl: Option<f64>,
    pub tsb: Option<f64>,
    pub tss_daily: Option<f64>,
}

/// Zone settings for a user
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ZoneSettings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub ftp: Option<f64>, // Functional Threshold Power (watts)
    pub lthr: Option<f64>, // Lactate Threshold Heart Rate (bpm)
    pub max_heart_rate: Option<f64>, // Maximum Heart Rate (bpm)
    pub resting_heart_rate: Option<f64>, // Resting Heart Rate (bpm)
    pub threshold_pace: Option<f64>, // Threshold pace (seconds per meter)
    pub weight: Option<f64>, // Body weight (kg) for power-to-weight calculations
    pub power_zones: Option<serde_json::Value>, // Custom power zone thresholds
    pub heart_rate_zones: Option<serde_json::Value>, // Custom HR zone thresholds
    pub pace_zones: Option<serde_json::Value>, // Custom pace zone thresholds
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create zone settings
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateZoneSettings {
    pub user_id: Uuid,
    pub ftp: Option<f64>,
    pub lthr: Option<f64>,
    pub max_heart_rate: Option<f64>,
    pub resting_heart_rate: Option<f64>,
    pub threshold_pace: Option<f64>,
    pub weight: Option<f64>,
    pub power_zones: Option<serde_json::Value>,
    pub heart_rate_zones: Option<serde_json::Value>,
    pub pace_zones: Option<serde_json::Value>,
}

/// Update zone settings
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateZoneSettings {
    pub ftp: Option<f64>,
    pub lthr: Option<f64>,
    pub max_heart_rate: Option<f64>,
    pub resting_heart_rate: Option<f64>,
    pub threshold_pace: Option<f64>,
    pub weight: Option<f64>,
    pub power_zones: Option<serde_json::Value>,
    pub heart_rate_zones: Option<serde_json::Value>,
    pub pace_zones: Option<serde_json::Value>,
}

/// Training Stress Score calculation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TSSConfig {
    pub sport: Sport,
    pub use_power: bool,
    pub use_heart_rate: bool,
    pub use_pace: bool,
}

/// Sport type for training analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Sport {
    Cycling,
    Running,
    Swimming,
    Triathlon,
    Other(String),
}

impl Sport {
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cycling" | "bike" | "bicycle" => Sport::Cycling,
            "running" | "run" => Sport::Running,
            "swimming" | "swim" => Sport::Swimming,
            "triathlon" | "tri" => Sport::Triathlon,
            other => Sport::Other(other.to_string()),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Sport::Cycling => "cycling".to_string(),
            Sport::Running => "running".to_string(),
            Sport::Swimming => "swimming".to_string(),
            Sport::Triathlon => "triathlon".to_string(),
            Sport::Other(s) => s.clone(),
        }
    }
}

/// Zone calculation methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZoneMethod {
    /// Coggan power zones (7 zones)
    CogganPower,
    /// Traditional heart rate zones (5 zones)
    TraditionalHeartRate,
    /// Jack Daniels running zones
    DanielsRunning,
    /// Custom zones defined by user
    Custom,
}

/// Training load metrics for periodization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingLoad {
    pub acute_load: f64,      // 7-day rolling average
    pub chronic_load: f64,    // 42-day rolling average
    pub training_balance: f64, // chronic - acute
    pub ramp_rate: f64,       // Rate of fitness gain/loss
}

/// Weekly training summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyTrainingSummary {
    pub week_start: NaiveDate,
    pub total_tss: f64,
    pub total_time: i32, // seconds
    pub total_distance: f64, // meters
    pub session_count: i32,
    pub intensity_distribution: IntensityDistribution,
    pub training_load: TrainingLoad,
}

/// Intensity distribution (polarized model)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntensityDistribution {
    pub zone_1_percentage: f64, // Low intensity
    pub zone_2_percentage: f64, // Moderate intensity
    pub zone_3_percentage: f64, // High intensity
}

/// Power curve data for analyzing peak power efforts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerCurve {
    pub user_id: Uuid,
    pub date: NaiveDate,
    pub duration_seconds: i32,
    pub max_power: f64,
    pub session_id: Option<Uuid>,
}

/// Critical power model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalPowerModel {
    pub user_id: Uuid,
    pub critical_power: f64, // CP in watts
    pub work_capacity: f64,  // W' in kJ
    pub model_r_squared: f64, // Goodness of fit
    pub calculated_date: DateTime<Utc>,
    pub test_duration_range: String, // e.g., "3-20 minutes"
}

impl TrainingMetrics {
    /// Calculate Training Stress Score based on available data
    pub fn calculate_tss(&self, zone_settings: &ZoneSettings) -> Option<f64> {
        if let (Some(duration), Some(normalized_power), Some(ftp)) =
            (self.duration_seconds, self.normalized_power, zone_settings.ftp) {
            // Power-based TSS calculation
            let intensity_factor = normalized_power / ftp;
            let tss = (duration as f64 / 3600.0) * intensity_factor.powi(2) * 100.0;
            Some(tss)
        } else if let (Some(duration), Some(avg_hr), Some(lthr)) =
            (self.duration_seconds, self.average_heart_rate, zone_settings.lthr) {
            // Heart rate-based TSS estimation (HRSS)
            let hr_ratio = avg_hr / lthr;
            let hrss = (duration as f64 / 3600.0) * hr_ratio.powi(2) * 100.0;
            Some(hrss)
        } else {
            None
        }
    }

    /// Calculate Intensity Factor
    pub fn calculate_intensity_factor(&self, zone_settings: &ZoneSettings) -> Option<f64> {
        if let (Some(normalized_power), Some(ftp)) = (self.normalized_power, zone_settings.ftp) {
            Some(normalized_power / ftp)
        } else {
            None
        }
    }

    /// Estimate power zones distribution from power data
    pub fn calculate_power_zones(&self, power_data: &[f64], ftp: f64) -> PowerZoneDistribution {
        let total_points = power_data.len() as f64;
        if total_points == 0.0 {
            return PowerZoneDistribution {
                zone_1: 0.0, zone_2: 0.0, zone_3: 0.0, zone_4: 0.0,
                zone_5: 0.0, zone_6: 0.0, zone_7: 0.0,
            };
        }

        let mut zone_counts = [0; 7];

        for &power in power_data {
            let zone_index = if power < ftp * 0.55 { 0 }
            else if power < ftp * 0.75 { 1 }
            else if power < ftp * 0.90 { 2 }
            else if power < ftp * 1.05 { 3 }
            else if power < ftp * 1.20 { 4 }
            else if power < ftp * 1.50 { 5 }
            else { 6 };

            zone_counts[zone_index] += 1;
        }

        PowerZoneDistribution {
            zone_1: (zone_counts[0] as f64 / total_points) * 100.0,
            zone_2: (zone_counts[1] as f64 / total_points) * 100.0,
            zone_3: (zone_counts[2] as f64 / total_points) * 100.0,
            zone_4: (zone_counts[3] as f64 / total_points) * 100.0,
            zone_5: (zone_counts[4] as f64 / total_points) * 100.0,
            zone_6: (zone_counts[5] as f64 / total_points) * 100.0,
            zone_7: (zone_counts[6] as f64 / total_points) * 100.0,
        }
    }
}