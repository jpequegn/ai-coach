use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use axum_extra::extract::WithRejection;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{AuthService, Claims};
use crate::models::validation_framework::*;
use crate::services::ValidationService;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error_code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            error_code: code.to_string(),
            message: message.to_string(),
            details: None,
        }
    }
}

#[derive(Clone)]
pub struct ValidationAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub validation_service: ValidationService,
}

#[derive(Debug, Deserialize)]
pub struct VideoQuery {
    pub exercise_type: Option<String>,
    pub quality_category: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RunQuery {
    pub run_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub fn validation_routes(db: PgPool, auth_service: AuthService) -> Router {
    let validation_service = ValidationService::new(db.clone());
    let shared_state = ValidationAppState {
        db,
        auth_service,
        validation_service,
    };

    Router::new()
        // Test Videos
        .route("/videos", get(list_test_videos).post(create_test_video))
        .route("/videos/:video_id", get(get_test_video))

        // Experts
        .route("/experts", get(list_experts).post(create_expert))
        .route("/experts/:expert_id", get(get_expert))

        // Annotations
        .route("/annotations/keypoints", post(create_keypoint_annotation))
        .route("/annotations/form-ratings", post(create_form_rating))
        .route("/annotations/issues", post(create_issue_annotation))
        .route("/videos/:video_id/annotations/form-ratings", get(get_video_form_ratings))
        .route("/videos/:video_id/annotations/issues", get(get_video_issue_annotations))

        // Validation Runs
        .route("/runs", get(list_validation_runs).post(start_validation_run))
        .route("/runs/:run_id", get(get_validation_run))
        .route("/runs/:run_id/summary", get(get_run_summary))

        .with_state(shared_state)
}

// ============================================================================
// Test Video Endpoints
// ============================================================================

/// Create a new test video
pub async fn create_test_video(
    State(state): State<ValidationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<CreateTestVideoRequest>,
) -> Result<Json<ValidationTestVideo>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let video = state.validation_service
        .create_test_video(user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create test video: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to create test video")))
        })?;

    Ok(Json(video))
}

/// Get test video by ID
pub async fn get_test_video(
    State(state): State<ValidationAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Path(video_id): Path<Uuid>,
) -> Result<Json<ValidationTestVideo>, (StatusCode, Json<ApiError>)> {
    let video = state.validation_service
        .get_test_video(video_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get test video: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to get test video")))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ApiError::new("NOT_FOUND", "Test video not found")))
        })?;

    Ok(Json(video))
}

/// List test videos with filtering
pub async fn list_test_videos(
    State(state): State<ValidationAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<VideoQuery>,
) -> Result<Json<Vec<ValidationTestVideo>>, (StatusCode, Json<ApiError>)> {
    let videos = state.validation_service
        .list_test_videos(
            query.exercise_type,
            query.quality_category,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to list test videos: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to list test videos")))
        })?;

    Ok(Json(videos))
}

// ============================================================================
// Expert Endpoints
// ============================================================================

/// Create a new expert
pub async fn create_expert(
    State(state): State<ValidationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<CreateExpertRequest>,
) -> Result<Json<ValidationExpert>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).ok();

    let expert = state.validation_service
        .create_expert(user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create expert: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to create expert")))
        })?;

    Ok(Json(expert))
}

/// Get expert by ID
pub async fn get_expert(
    State(state): State<ValidationAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Path(expert_id): Path<Uuid>,
) -> Result<Json<ValidationExpert>, (StatusCode, Json<ApiError>)> {
    let expert = state.validation_service
        .get_expert(expert_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get expert: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to get expert")))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ApiError::new("NOT_FOUND", "Expert not found")))
        })?;

    Ok(Json(expert))
}

/// List all experts
pub async fn list_experts(
    State(state): State<ValidationAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<Vec<ValidationExpert>>, (StatusCode, Json<ApiError>)> {
    let experts = state.validation_service
        .list_experts()
        .await
        .map_err(|e| {
            tracing::error!("Failed to list experts: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to list experts")))
        })?;

    Ok(Json(experts))
}

// ============================================================================
// Annotation Endpoints
// ============================================================================

/// Create keypoint annotation
pub async fn create_keypoint_annotation(
    State(state): State<ValidationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<CreateKeypointAnnotationRequest>,
) -> Result<Json<ValidationKeypointAnnotation>, (StatusCode, Json<ApiError>)> {
    let expert_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let annotation = state.validation_service
        .create_keypoint_annotation(expert_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create keypoint annotation: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to create annotation")))
        })?;

    Ok(Json(annotation))
}

/// Create form rating
pub async fn create_form_rating(
    State(state): State<ValidationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<CreateFormRatingRequest>,
) -> Result<Json<ValidationFormRating>, (StatusCode, Json<ApiError>)> {
    let expert_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let rating = state.validation_service
        .create_form_rating(expert_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create form rating: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to create rating")))
        })?;

    Ok(Json(rating))
}

/// Create issue annotation
pub async fn create_issue_annotation(
    State(state): State<ValidationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<CreateIssueAnnotationRequest>,
) -> Result<Json<ValidationIssueAnnotation>, (StatusCode, Json<ApiError>)> {
    let expert_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let annotation = state.validation_service
        .create_issue_annotation(expert_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create issue annotation: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to create annotation")))
        })?;

    Ok(Json(annotation))
}

/// Get form ratings for a video
pub async fn get_video_form_ratings(
    State(state): State<ValidationAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Path(video_id): Path<Uuid>,
) -> Result<Json<Vec<ValidationFormRating>>, (StatusCode, Json<ApiError>)> {
    let ratings = state.validation_service
        .get_video_form_ratings(video_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get form ratings: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to get ratings")))
        })?;

    Ok(Json(ratings))
}

/// Get issue annotations for a video
pub async fn get_video_issue_annotations(
    State(state): State<ValidationAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Path(video_id): Path<Uuid>,
) -> Result<Json<Vec<ValidationIssueAnnotation>>, (StatusCode, Json<ApiError>)> {
    let annotations = state.validation_service
        .get_video_issue_annotations(video_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get issue annotations: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to get annotations")))
        })?;

    Ok(Json(annotations))
}

// ============================================================================
// Validation Run Endpoints
// ============================================================================

/// Start a new validation run
pub async fn start_validation_run(
    State(state): State<ValidationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<StartValidationRunRequest>,
) -> Result<Json<ValidationRun>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let run = state.validation_service
        .start_validation_run(user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to start validation run: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to start validation run")))
        })?;

    Ok(Json(run))
}

/// Get validation run by ID
pub async fn get_validation_run(
    State(state): State<ValidationAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<ValidationRun>, (StatusCode, Json<ApiError>)> {
    let run = state.validation_service
        .get_validation_run(run_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get validation run: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to get validation run")))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(ApiError::new("NOT_FOUND", "Validation run not found")))
        })?;

    Ok(Json(run))
}

/// List validation runs
pub async fn list_validation_runs(
    State(state): State<ValidationAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<RunQuery>,
) -> Result<Json<Vec<ValidationRun>>, (StatusCode, Json<ApiError>)> {
    let runs = state.validation_service
        .list_validation_runs(
            query.run_type,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to list validation runs: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to list validation runs")))
        })?;

    Ok(Json(runs))
}

/// Get validation run summary
pub async fn get_run_summary(
    State(state): State<ValidationAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<ValidationRunSummaryResponse>, (StatusCode, Json<ApiError>)> {
    let summary = state.validation_service
        .get_or_create_run_summary(run_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get run summary: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to get run summary")))
        })?;

    Ok(Json(summary.to_response()))
}
