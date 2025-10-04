use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// Test Dataset Models
// ============================================================================

/// Test video with ground truth labels
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ValidationTestVideo {
    pub id: Uuid,
    pub video_url: String,
    pub video_name: String,
    pub exercise_type: String,
    pub quality_category: String, // 'good_form', 'poor_form', 'mixed'
    pub body_type: Option<String>,
    pub lighting_condition: Option<String>,
    pub camera_angle: Option<String>,
    pub video_duration_seconds: Option<f64>,
    pub video_resolution: Option<String>,
    pub fps: Option<i32>,
    pub notes: Option<String>,
    pub uploaded_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Expert coach providing ground truth annotations
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ValidationExpert {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub expert_name: String,
    pub credentials: Option<String>,
    pub specialization: Option<Vec<String>>,
    pub years_experience: Option<i32>,
    pub certification_level: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Manual keypoint annotation (ground truth)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ValidationKeypointAnnotation {
    pub id: Uuid,
    pub video_id: Uuid,
    pub expert_id: Uuid,
    pub frame_number: i32,
    pub timestamp_ms: i32,
    pub keypoints: serde_json::Value, // JSONB array of keypoints
    pub annotation_method: Option<String>,
    pub annotation_quality: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Expert form rating (ground truth)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ValidationFormRating {
    pub id: Uuid,
    pub video_id: Uuid,
    pub expert_id: Uuid,
    pub overall_score: f64,
    pub form_quality: Option<f64>,
    pub injury_risk: Option<f64>,
    pub range_of_motion: Option<f64>,
    pub tempo_consistency: Option<f64>,
    pub rep_count: Option<i32>,
    pub rating_confidence: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Expert issue annotation (ground truth)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ValidationIssueAnnotation {
    pub id: Uuid,
    pub video_id: Uuid,
    pub expert_id: Uuid,
    pub issue_type: String,
    pub severity: String,
    pub description: String,
    pub affected_frames: Option<Vec<i32>>,
    pub timestamp_ranges: Option<serde_json::Value>,
    pub confidence: Option<f64>,
    pub corrective_action: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Validation Run Models
// ============================================================================

/// Validation run metadata
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ValidationRun {
    pub id: Uuid,
    pub run_name: String,
    pub model_version: Option<String>,
    pub threshold_config: Option<serde_json::Value>,
    pub dataset_size: Option<i32>,
    pub run_type: String, // 'pose_accuracy', 'form_scoring', 'issue_detection', 'full'
    pub initiated_by: Option<Uuid>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: String, // 'running', 'completed', 'failed'
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Pose estimation accuracy metrics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ValidationPoseAccuracy {
    pub id: Uuid,
    pub run_id: Uuid,
    pub video_id: Uuid,
    pub frame_count: i32,
    pub mean_per_joint_position_error: Option<f64>,
    pub percentage_correct_keypoints: Option<f64>,
    pub mean_average_precision: Option<f64>,
    pub keypoint_accuracy_by_joint: Option<serde_json::Value>,
    pub detection_rate: Option<f64>,
    pub false_positive_rate: Option<f64>,
    pub processing_time_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// Form scoring validation metrics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ValidationFormScoring {
    pub id: Uuid,
    pub run_id: Uuid,
    pub video_id: Uuid,
    pub predicted_overall_score: Option<f64>,
    pub expert_overall_score: Option<f64>,
    pub score_difference: Option<f64>,
    pub pearson_correlation: Option<f64>,
    pub score_category_accuracy: Option<f64>,
    pub component_scores: Option<serde_json::Value>,
    pub expert_scores: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Issue detection validation metrics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ValidationIssueDetection {
    pub id: Uuid,
    pub run_id: Uuid,
    pub video_id: Uuid,
    pub detected_issues: Option<serde_json::Value>,
    pub expert_issues: Option<serde_json::Value>,
    pub true_positives: Option<i32>,
    pub false_positives: Option<i32>,
    pub false_negatives: Option<i32>,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1_score: Option<f64>,
    pub issue_type_accuracy: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Threshold optimization experiment
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ValidationThresholdExperiment {
    pub id: Uuid,
    pub run_id: Uuid,
    pub threshold_name: String,
    pub threshold_value: f64,
    pub metric_name: String,
    pub metric_value: f64,
    pub dataset_subset: Option<String>,
    pub is_optimal: Option<bool>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Validation run summary
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ValidationRunSummary {
    pub id: Uuid,
    pub run_id: Uuid,

    // Pose Accuracy Summary
    pub avg_pose_map: Option<f64>,
    pub avg_pck: Option<f64>,
    pub avg_mpjpe: Option<f64>,
    pub pose_target_met: Option<bool>,

    // Form Scoring Summary
    pub avg_pearson_correlation: Option<f64>,
    pub avg_score_difference: Option<f64>,
    pub form_target_met: Option<bool>,

    // Issue Detection Summary
    pub avg_precision: Option<f64>,
    pub avg_recall: Option<f64>,
    pub avg_f1_score: Option<f64>,
    pub issue_target_met: Option<bool>,

    // Overall
    pub all_targets_met: Option<bool>,
    pub recommendations: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Request DTOs
// ============================================================================

/// Create test video request
#[derive(Debug, Deserialize)]
pub struct CreateTestVideoRequest {
    pub video_url: String,
    pub video_name: String,
    pub exercise_type: String,
    pub quality_category: String,
    pub body_type: Option<String>,
    pub lighting_condition: Option<String>,
    pub camera_angle: Option<String>,
    pub notes: Option<String>,
}

/// Create expert request
#[derive(Debug, Deserialize)]
pub struct CreateExpertRequest {
    pub expert_name: String,
    pub credentials: Option<String>,
    pub specialization: Option<Vec<String>>,
    pub years_experience: Option<i32>,
    pub certification_level: Option<String>,
}

/// Create keypoint annotation request
#[derive(Debug, Deserialize)]
pub struct CreateKeypointAnnotationRequest {
    pub video_id: Uuid,
    pub frame_number: i32,
    pub timestamp_ms: i32,
    pub keypoints: serde_json::Value,
    pub annotation_method: Option<String>,
    pub annotation_quality: Option<String>,
    pub notes: Option<String>,
}

/// Create form rating request
#[derive(Debug, Deserialize)]
pub struct CreateFormRatingRequest {
    pub video_id: Uuid,
    pub overall_score: f64,
    pub form_quality: Option<f64>,
    pub injury_risk: Option<f64>,
    pub range_of_motion: Option<f64>,
    pub tempo_consistency: Option<f64>,
    pub rep_count: Option<i32>,
    pub rating_confidence: Option<String>,
    pub notes: Option<String>,
}

/// Create issue annotation request
#[derive(Debug, Deserialize)]
pub struct CreateIssueAnnotationRequest {
    pub video_id: Uuid,
    pub issue_type: String,
    pub severity: String,
    pub description: String,
    pub affected_frames: Option<Vec<i32>>,
    pub timestamp_ranges: Option<serde_json::Value>,
    pub confidence: Option<f64>,
    pub corrective_action: Option<String>,
}

/// Start validation run request
#[derive(Debug, Deserialize)]
pub struct StartValidationRunRequest {
    pub run_name: String,
    pub model_version: Option<String>,
    pub threshold_config: Option<serde_json::Value>,
    pub run_type: String, // 'pose_accuracy', 'form_scoring', 'issue_detection', 'full'
    pub video_ids: Vec<Uuid>, // Which test videos to validate
    pub notes: Option<String>,
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Validation run response
#[derive(Debug, Serialize)]
pub struct ValidationRunResponse {
    pub id: Uuid,
    pub run_name: String,
    pub model_version: Option<String>,
    pub run_type: String,
    pub status: String,
    pub dataset_size: Option<i32>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub summary: Option<ValidationRunSummaryResponse>,
}

/// Validation run summary response
#[derive(Debug, Serialize)]
pub struct ValidationRunSummaryResponse {
    pub pose_accuracy: PoseAccuracyMetrics,
    pub form_scoring: FormScoringMetrics,
    pub issue_detection: IssueDetectionMetrics,
    pub all_targets_met: bool,
    pub recommendations: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PoseAccuracyMetrics {
    pub avg_map: f64,
    pub avg_pck: f64,
    pub avg_mpjpe: f64,
    pub target_met: bool, // >90% accuracy
}

#[derive(Debug, Serialize)]
pub struct FormScoringMetrics {
    pub avg_correlation: f64,
    pub avg_difference: f64,
    pub target_met: bool, // >0.8 correlation
}

#[derive(Debug, Serialize)]
pub struct IssueDetectionMetrics {
    pub avg_precision: f64,
    pub avg_recall: f64,
    pub avg_f1_score: f64,
    pub target_met: bool, // >85% precision
}

/// Test video response
#[derive(Debug, Serialize)]
pub struct TestVideoResponse {
    pub id: Uuid,
    pub video_name: String,
    pub exercise_type: String,
    pub quality_category: String,
    pub video_url: String,
    pub duration_seconds: Option<f64>,
    pub has_keypoint_annotations: bool,
    pub has_form_ratings: bool,
    pub has_issue_annotations: bool,
    pub created_at: DateTime<Utc>,
}

/// Expert response
#[derive(Debug, Serialize)]
pub struct ExpertResponse {
    pub id: Uuid,
    pub expert_name: String,
    pub credentials: Option<String>,
    pub specialization: Option<Vec<String>>,
    pub years_experience: Option<i32>,
    pub annotation_count: i64,
}

/// Threshold optimization result
#[derive(Debug, Serialize)]
pub struct ThresholdOptimizationResult {
    pub threshold_name: String,
    pub optimal_value: f64,
    pub metric_achieved: f64,
    pub improvement_percentage: f64,
}

// ============================================================================
// Helper Implementations
// ============================================================================

impl ValidationRun {
    /// Calculate duration of validation run
    pub fn duration_seconds(&self) -> Option<i64> {
        self.completed_at.map(|completed| {
            (completed - self.started_at).num_seconds()
        })
    }

    /// Check if validation run is complete
    pub fn is_complete(&self) -> bool {
        self.status == "completed"
    }
}

impl ValidationRunSummary {
    /// Convert to response DTO
    pub fn to_response(&self) -> ValidationRunSummaryResponse {
        ValidationRunSummaryResponse {
            pose_accuracy: PoseAccuracyMetrics {
                avg_map: self.avg_pose_map.unwrap_or(0.0),
                avg_pck: self.avg_pck.unwrap_or(0.0),
                avg_mpjpe: self.avg_mpjpe.unwrap_or(0.0),
                target_met: self.pose_target_met.unwrap_or(false),
            },
            form_scoring: FormScoringMetrics {
                avg_correlation: self.avg_pearson_correlation.unwrap_or(0.0),
                avg_difference: self.avg_score_difference.unwrap_or(0.0),
                target_met: self.form_target_met.unwrap_or(false),
            },
            issue_detection: IssueDetectionMetrics {
                avg_precision: self.avg_precision.unwrap_or(0.0),
                avg_recall: self.avg_recall.unwrap_or(0.0),
                avg_f1_score: self.avg_f1_score.unwrap_or(0.0),
                target_met: self.issue_target_met.unwrap_or(false),
            },
            all_targets_met: self.all_targets_met.unwrap_or(false),
            recommendations: self.recommendations.clone(),
        }
    }
}
