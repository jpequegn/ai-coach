use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Training features for machine learning model input
/// Contains all relevant metrics for predicting optimal training load
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingFeatures {
    /// Current Chronic Training Load (CTL) - long-term fitness
    pub current_ctl: f32,

    /// Current Acute Training Load (ATL) - recent fatigue
    pub current_atl: f32,

    /// Current Training Stress Balance (TSB) - readiness (CTL - ATL)
    pub current_tsb: f32,

    /// Days since the athlete's last workout
    pub days_since_last_workout: i32,

    /// Average weekly TSS over the past 4 weeks
    pub avg_weekly_tss_4weeks: f32,

    /// Recent performance trend (-1.0 to 1.0, negative = declining, positive = improving)
    pub recent_performance_trend: f32,

    /// Days until goal event (None if no specific event)
    pub days_until_goal_event: Option<i32>,

    /// Preferred workout types (e.g., "endurance", "threshold", "vo2max")
    pub preferred_workout_types: Vec<String>,

    /// Seasonal factors affecting training (0.0 to 1.0, considering weather, holidays, etc.)
    pub seasonal_factors: f32,
}

/// Training load prediction from ML model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingLoadPrediction {
    /// Recommended TSS for the next workout
    pub recommended_tss: f32,

    /// Confidence interval for the prediction (0.0 to 1.0)
    pub confidence: f32,

    /// Lower bound of confidence interval
    pub confidence_lower: f32,

    /// Upper bound of confidence interval
    pub confidence_upper: f32,

    /// Model version used for prediction
    pub model_version: String,

    /// Recommended workout type
    pub recommended_workout_type: String,

    /// Timestamp when prediction was made
    pub predicted_at: DateTime<Utc>,
}

/// Model evaluation metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    /// Mean Absolute Error for TSS predictions
    pub mae_tss: f32,

    /// Root Mean Square Error for TSS predictions
    pub rmse_tss: f32,

    /// R-squared coefficient for model fit
    pub r_squared: f32,

    /// Number of samples used for evaluation
    pub sample_count: usize,

    /// Model version
    pub model_version: String,

    /// Evaluation timestamp
    pub evaluated_at: DateTime<Utc>,
}

/// Historical training data point for model training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingDataPoint {
    /// Input features for this training example
    pub features: TrainingFeatures,

    /// Actual TSS that was performed
    pub actual_tss: f32,

    /// Actual workout type that was performed
    pub actual_workout_type: String,

    /// Performance outcome (subjective rating 1-10)
    pub performance_outcome: Option<f32>,

    /// Recovery rating after workout (1-10)
    pub recovery_rating: Option<f32>,

    /// Date of the workout
    pub workout_date: DateTime<Utc>,
}

impl TrainingFeatures {
    /// Create a new TrainingFeatures instance with default values
    pub fn new() -> Self {
        Self {
            current_ctl: 0.0,
            current_atl: 0.0,
            current_tsb: 0.0,
            days_since_last_workout: 0,
            avg_weekly_tss_4weeks: 0.0,
            recent_performance_trend: 0.0,
            days_until_goal_event: None,
            preferred_workout_types: Vec::new(),
            seasonal_factors: 1.0,
        }
    }

    /// Convert features to ndarray for ML model input
    pub fn to_ndarray(&self) -> ndarray::Array1<f64> {
        let mut features = vec![
            self.current_ctl as f64,
            self.current_atl as f64,
            self.current_tsb as f64,
            self.days_since_last_workout as f64,
            self.avg_weekly_tss_4weeks as f64,
            self.recent_performance_trend as f64,
            self.days_until_goal_event.unwrap_or(-1) as f64,
            self.seasonal_factors as f64,
        ];

        // Add preferred workout types as one-hot encoding
        let workout_types = ["endurance", "threshold", "vo2max", "recovery", "strength"];
        for workout_type in &workout_types {
            features.push(if self.preferred_workout_types.contains(&workout_type.to_string()) { 1.0 } else { 0.0 });
        }

        ndarray::Array1::from(features)
    }

    /// Get feature names for model interpretation
    pub fn feature_names() -> Vec<String> {
        let mut names = vec![
            "current_ctl".to_string(),
            "current_atl".to_string(),
            "current_tsb".to_string(),
            "days_since_last_workout".to_string(),
            "avg_weekly_tss_4weeks".to_string(),
            "recent_performance_trend".to_string(),
            "days_until_goal_event".to_string(),
            "seasonal_factors".to_string(),
        ];

        // Add preferred workout type feature names
        let workout_types = ["endurance", "threshold", "vo2max", "recovery", "strength"];
        for workout_type in &workout_types {
            names.push(format!("prefers_{}", workout_type));
        }

        names
    }
}

impl Default for TrainingFeatures {
    fn default() -> Self {
        Self::new()
    }
}