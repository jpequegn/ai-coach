use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

/// Recommendation category enum
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum RecommendationCategory {
    #[serde(rename = "sleep")]
    Sleep,
    #[serde(rename = "nutrition")]
    Nutrition,
    #[serde(rename = "active_recovery")]
    ActiveRecovery,
    #[serde(rename = "stress_management")]
    StressManagement,
    #[serde(rename = "training_modification")]
    TrainingModification,
}

impl std::fmt::Display for RecommendationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sleep => write!(f, "sleep"),
            Self::Nutrition => write!(f, "nutrition"),
            Self::ActiveRecovery => write!(f, "active_recovery"),
            Self::StressManagement => write!(f, "stress_management"),
            Self::TrainingModification => write!(f, "training_modification"),
        }
    }
}

/// Priority level for recommendations
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum RecommendationPriority {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "critical")]
    Critical,
}

impl std::fmt::Display for RecommendationPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Difficulty level for recommendations
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum RecommendationDifficulty {
    #[serde(rename = "easy")]
    Easy,
    #[serde(rename = "moderate")]
    Moderate,
    #[serde(rename = "challenging")]
    Challenging,
    #[serde(rename = "advanced")]
    Advanced,
}

impl std::fmt::Display for RecommendationDifficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Easy => write!(f, "easy"),
            Self::Moderate => write!(f, "moderate"),
            Self::Challenging => write!(f, "challenging"),
            Self::Advanced => write!(f, "advanced"),
        }
    }
}

/// Content type for educational materials
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum ContentType {
    #[serde(rename = "article")]
    Article,
    #[serde(rename = "video")]
    Video,
    #[serde(rename = "guide")]
    Guide,
    #[serde(rename = "research")]
    Research,
    #[serde(rename = "tutorial")]
    Tutorial,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Article => write!(f, "article"),
            Self::Video => write!(f, "video"),
            Self::Guide => write!(f, "guide"),
            Self::Research => write!(f, "research"),
            Self::Tutorial => write!(f, "tutorial"),
        }
    }
}

/// Recommendation template model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecommendationTemplate {
    pub id: Uuid,
    pub category: RecommendationCategory,
    pub title: String,
    pub description: String,
    pub action: String,
    pub priority_default: RecommendationPriority,
    pub difficulty: RecommendationDifficulty,
    pub expected_impact: Option<String>,
    pub time_required_minutes: Option<i32>,
    pub trigger_conditions: JsonValue,
    pub user_constraints: JsonValue,
    pub effectiveness_score: f64,
    pub times_suggested: i32,
    pub times_completed: i32,
    pub metadata: JsonValue,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Recommendation content model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecommendationContent {
    pub id: Uuid,
    pub recommendation_template_id: Uuid,
    pub content_type: ContentType,
    pub title: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub duration_minutes: Option<i32>,
    pub author: Option<String>,
    pub source: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request to create a new recommendation template
#[derive(Debug, Deserialize)]
pub struct CreateRecommendationTemplateRequest {
    pub category: RecommendationCategory,
    pub title: String,
    pub description: String,
    pub action: String,
    pub priority_default: Option<RecommendationPriority>,
    pub difficulty: Option<RecommendationDifficulty>,
    pub expected_impact: Option<String>,
    pub time_required_minutes: Option<i32>,
    pub trigger_conditions: Option<JsonValue>,
    pub user_constraints: Option<JsonValue>,
    pub metadata: Option<JsonValue>,
}

/// Request to create recommendation content
#[derive(Debug, Deserialize)]
pub struct CreateRecommendationContentRequest {
    pub recommendation_template_id: Uuid,
    pub content_type: ContentType,
    pub title: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub duration_minutes: Option<i32>,
    pub author: Option<String>,
    pub source: Option<String>,
}

/// Response with template and related content
#[derive(Debug, Serialize)]
pub struct RecommendationTemplateResponse {
    #[serde(flatten)]
    pub template: RecommendationTemplate,
    pub content: Vec<RecommendationContent>,
}

/// Filter options for querying recommendations
#[derive(Debug, Deserialize)]
pub struct RecommendationFilter {
    pub category: Option<RecommendationCategory>,
    pub min_effectiveness: Option<f64>,
    pub difficulty: Option<RecommendationDifficulty>,
    pub is_active: Option<bool>,
}

/// User recommendation status enum
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
pub enum UserRecommendationStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "skipped")]
    Skipped,
    #[serde(rename = "expired")]
    Expired,
}

impl std::fmt::Display for UserRecommendationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Completed => write!(f, "completed"),
            Self::Skipped => write!(f, "skipped"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

/// User recommendation model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRecommendation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub recommendation_template_id: Uuid,
    pub recovery_score_id: Option<Uuid>,
    pub status: UserRecommendationStatus,
    pub effectiveness_rating: Option<i32>,
    pub user_feedback: Option<String>,
    pub skip_reason: Option<String>,
    pub shown_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub skipped_at: Option<DateTime<Utc>>,
    pub expired_at: Option<DateTime<Utc>>,
    pub rated_at: Option<DateTime<Utc>>,
    pub metadata: JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to complete a recommendation
#[derive(Debug, Deserialize)]
pub struct CompleteRecommendationRequest {
    pub completed_at: Option<DateTime<Utc>>,
}

/// Request to skip a recommendation
#[derive(Debug, Deserialize)]
pub struct SkipRecommendationRequest {
    pub reason: Option<String>,
}

/// Request to rate a recommendation
#[derive(Debug, Deserialize)]
pub struct RateRecommendationRequest {
    pub rating: i32,
    pub feedback: Option<String>,
}

/// Filter options for recommendation history
#[derive(Debug, Deserialize)]
pub struct HistoryFilter {
    pub status: Option<UserRecommendationStatus>,
    pub category: Option<RecommendationCategory>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// User recommendation with template details
#[derive(Debug, Serialize)]
pub struct UserRecommendationWithTemplate {
    #[serde(flatten)]
    pub user_recommendation: UserRecommendation,
    pub template: RecommendationTemplate,
}
