use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ModelPrediction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub prediction_type: String,
    pub data: Value,
    pub confidence: Option<f64>,
    pub model_version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateModelPrediction {
    pub user_id: Uuid,
    pub prediction_type: String,
    pub data: Value,
    pub confidence: Option<f64>,
    pub model_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateModelPrediction {
    pub prediction_type: Option<String>,
    pub data: Option<Value>,
    pub confidence: Option<f64>,
    pub model_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PredictionType {
    #[serde(rename = "performance")]
    Performance,
    #[serde(rename = "injury_risk")]
    InjuryRisk,
    #[serde(rename = "fatigue")]
    Fatigue,
    #[serde(rename = "optimal_training")]
    OptimalTraining,
    #[serde(rename = "recovery_time")]
    RecoveryTime,
    #[serde(rename = "race_prediction")]
    RacePrediction,
    #[serde(rename = "adaptation")]
    Adaptation,
    #[serde(rename = "other")]
    Other,
}

impl ToString for PredictionType {
    fn to_string(&self) -> String {
        match self {
            PredictionType::Performance => "performance".to_string(),
            PredictionType::InjuryRisk => "injury_risk".to_string(),
            PredictionType::Fatigue => "fatigue".to_string(),
            PredictionType::OptimalTraining => "optimal_training".to_string(),
            PredictionType::RecoveryTime => "recovery_time".to_string(),
            PredictionType::RacePrediction => "race_prediction".to_string(),
            PredictionType::Adaptation => "adaptation".to_string(),
            PredictionType::Other => "other".to_string(),
        }
    }
}