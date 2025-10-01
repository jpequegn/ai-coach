use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Status of a vision analysis
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AnalysisStatus {
    Uploaded,
    Processing,
    Completed,
    Failed,
}

impl std::fmt::Display for AnalysisStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisStatus::Uploaded => write!(f, "uploaded"),
            AnalysisStatus::Processing => write!(f, "processing"),
            AnalysisStatus::Completed => write!(f, "completed"),
            AnalysisStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Exercise types supported by the vision analysis system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExerciseType {
    Squat,
    Deadlift,
    #[serde(rename = "push-up")]
    PushUp,
    Running,
    Plank,
    Lunge,
    #[serde(rename = "bench-press")]
    BenchPress,
    #[serde(rename = "overhead-press")]
    OverheadPress,
    #[serde(rename = "pull-up")]
    PullUp,
    Other(String),
}

impl std::fmt::Display for ExerciseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExerciseType::Squat => write!(f, "squat"),
            ExerciseType::Deadlift => write!(f, "deadlift"),
            ExerciseType::PushUp => write!(f, "push-up"),
            ExerciseType::Running => write!(f, "running"),
            ExerciseType::Plank => write!(f, "plank"),
            ExerciseType::Lunge => write!(f, "lunge"),
            ExerciseType::BenchPress => write!(f, "bench-press"),
            ExerciseType::OverheadPress => write!(f, "overhead-press"),
            ExerciseType::PullUp => write!(f, "pull-up"),
            ExerciseType::Other(s) => write!(f, "{}", s),
        }
    }
}

/// Main vision analysis record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VisionAnalysis {
    pub id: Uuid,
    pub user_id: Uuid,
    pub video_url: String,
    pub video_duration_seconds: Option<f64>,
    pub video_resolution: Option<String>,
    pub video_format: Option<String>,
    pub video_size_bytes: Option<i64>,
    pub status: AnalysisStatus,
    pub exercise_type: Option<String>,
    pub upload_timestamp: DateTime<Utc>,
    pub processing_started_at: Option<DateTime<Utc>>,
    pub processing_completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new vision analysis
#[derive(Debug, Deserialize)]
pub struct CreateVisionAnalysisRequest {
    pub exercise_type: Option<ExerciseType>,
}

/// Response after uploading a video for analysis
#[derive(Debug, Serialize)]
pub struct VisionAnalysisUploadResponse {
    pub id: Uuid,
    pub status: AnalysisStatus,
    pub upload_timestamp: DateTime<Utc>,
    pub video_url: String,
}

/// Pose keypoint detected in a frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keypoint {
    pub joint_name: String,
    pub x: f32,
    pub y: f32,
    pub z: Option<f32>, // For 3D pose estimation
    pub confidence: f32,
}

/// Pose detection for a single frame
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PoseDetection {
    pub id: i64,
    pub analysis_id: Uuid,
    pub frame_number: i32,
    pub timestamp_ms: i32,
    pub keypoints: serde_json::Value, // JSON array of Keypoint
    pub confidence_score: f64,
    pub created_at: DateTime<Utc>,
}

/// Issue detected in movement analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementIssue {
    pub severity: IssueSeverity,
    #[serde(rename = "type")]
    pub issue_type: String,
    pub description: String,
    pub frames: Vec<i32>, // Frame numbers where issue occurs
    pub confidence: f32,
}

/// Severity level of detected issues
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Critical,
    Warning,
    Minor,
}

/// Recommendation for improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementRecommendation {
    pub priority: RecommendationPriority,
    pub issue: String,
    pub suggestion: String,
    pub exercises: Vec<String>,
    pub cue: String,
}

/// Priority level of recommendations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RecommendationPriority {
    High,
    Medium,
    Low,
}

/// Movement quality scores and feedback
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MovementScore {
    pub id: Uuid,
    pub analysis_id: Uuid,
    pub overall_score: f64,
    pub form_quality: Option<f64>,
    pub injury_risk: Option<f64>,
    pub range_of_motion: Option<f64>,
    pub tempo_consistency: Option<f64>,
    pub rep_count: Option<i32>,
    pub issues: serde_json::Value, // JSON array of MovementIssue
    pub recommendations: serde_json::Value, // JSON array of MovementRecommendation
    pub biomechanics_data: serde_json::Value, // Additional biomechanics metrics
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Complete analysis result response
#[derive(Debug, Serialize)]
pub struct VisionAnalysisResult {
    pub id: Uuid,
    pub user_id: Uuid,
    pub video_url: String,
    pub status: AnalysisStatus,
    pub exercise_type: Option<String>,
    pub duration_seconds: Option<f64>,
    pub processing_time_seconds: Option<f64>,
    pub scores: Option<ScoresSummary>,
    pub rep_count: Option<i32>,
    pub issues: Vec<MovementIssue>,
    pub recommendations: Vec<MovementRecommendation>,
    pub overlay_url: Option<String>,
    pub keypoints_data_url: Option<String>,
    pub upload_timestamp: DateTime<Utc>,
    pub processing_completed_at: Option<DateTime<Utc>>,
}

/// Summary of quality scores
#[derive(Debug, Serialize)]
pub struct ScoresSummary {
    pub overall: f64,
    pub form_quality: Option<f64>,
    pub injury_risk: Option<f64>,
    pub range_of_motion: Option<f64>,
    pub tempo_consistency: Option<f64>,
}

/// Analysis history list item
#[derive(Debug, Serialize)]
pub struct VisionAnalysisListItem {
    pub id: Uuid,
    pub exercise_type: Option<String>,
    pub status: AnalysisStatus,
    pub overall_score: Option<f64>,
    pub upload_timestamp: DateTime<Utc>,
    pub processing_completed_at: Option<DateTime<Utc>>,
}

impl VisionAnalysis {
    /// Calculate processing time if completed
    pub fn processing_time_seconds(&self) -> Option<f64> {
        if let (Some(started), Some(completed)) = (self.processing_started_at, self.processing_completed_at) {
            Some((completed - started).num_milliseconds() as f64 / 1000.0)
        } else {
            None
        }
    }

    /// Convert to list item representation
    pub fn to_list_item(&self, overall_score: Option<f64>) -> VisionAnalysisListItem {
        VisionAnalysisListItem {
            id: self.id,
            exercise_type: self.exercise_type.clone(),
            status: self.status.clone(),
            overall_score,
            upload_timestamp: self.upload_timestamp,
            processing_completed_at: self.processing_completed_at,
        }
    }
}
