use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CoachingRecommendation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub recommendation_type: String,
    pub content: String,
    pub confidence: Option<f64>,
    pub metadata: Option<Value>,
    pub applied: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCoachingRecommendation {
    pub user_id: Uuid,
    pub recommendation_type: String,
    pub content: String,
    pub confidence: Option<f64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCoachingRecommendation {
    pub recommendation_type: Option<String>,
    pub content: Option<String>,
    pub confidence: Option<f64>,
    pub metadata: Option<Value>,
    pub applied: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RecommendationType {
    #[serde(rename = "training_adjustment")]
    TrainingAdjustment,
    #[serde(rename = "recovery")]
    Recovery,
    #[serde(rename = "nutrition")]
    Nutrition,
    #[serde(rename = "technique")]
    Technique,
    #[serde(rename = "goal_adjustment")]
    GoalAdjustment,
    #[serde(rename = "equipment")]
    Equipment,
    #[serde(rename = "pacing")]
    Pacing,
    #[serde(rename = "other")]
    Other,
}

impl ToString for RecommendationType {
    fn to_string(&self) -> String {
        match self {
            RecommendationType::TrainingAdjustment => "training_adjustment".to_string(),
            RecommendationType::Recovery => "recovery".to_string(),
            RecommendationType::Nutrition => "nutrition".to_string(),
            RecommendationType::Technique => "technique".to_string(),
            RecommendationType::GoalAdjustment => "goal_adjustment".to_string(),
            RecommendationType::Equipment => "equipment".to_string(),
            RecommendationType::Pacing => "pacing".to_string(),
            RecommendationType::Other => "other".to_string(),
        }
    }
}