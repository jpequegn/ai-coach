use axum::{
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post},
    Extension,
    Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    auth::{middleware::auth_middleware, models::Claims, AuthService},
    models::vision_analysis::*,
    services::{VisionAnalysisService, VideoProcessingService, VideoStorageService},
};

/// Shared state for vision API handlers
pub struct VisionState {
    pub analysis_service: Arc<VisionAnalysisService>,
    pub storage_service: Arc<VideoStorageService>,
    pub processing_service: Arc<VideoProcessingService>,
}

/// Upload video for analysis
pub async fn upload_video(
    Extension(claims): Extension<Claims>,
    State(state): State<Arc<VisionState>>,
    mut multipart: Multipart,
) -> Result<Response, VisionError> {
    info!("Video upload request from user: {}", claims.sub);

    let mut video_data: Option<Vec<u8>> = None;
    let mut content_type: Option<String> = None;
    let mut exercise_type: Option<String> = None;

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!("Failed to read multipart field: {}", e);
        VisionError::InvalidRequest("Failed to read upload data".to_string())
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "video" => {
                content_type = field.content_type().map(|s| s.to_string());
                video_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| {
                            error!("Failed to read video bytes: {}", e);
                            VisionError::InvalidRequest("Failed to read video data".to_string())
                        })?
                        .to_vec(),
                );
            }
            "exercise_type" => {
                let bytes = field.bytes().await.map_err(|_| {
                    VisionError::InvalidRequest("Failed to read exercise_type".to_string())
                })?;
                exercise_type = Some(String::from_utf8_lossy(&bytes).to_string());
            }
            _ => {
                // Ignore unknown fields
            }
        }
    }

    // Validate required fields
    let video_data = video_data.ok_or_else(|| {
        VisionError::InvalidRequest("Video file is required".to_string())
    })?;

    let content_type = content_type.unwrap_or_else(|| "video/mp4".to_string());

    // Validate video file size (max 500MB)
    const MAX_SIZE: usize = 500 * 1024 * 1024;
    if video_data.len() > MAX_SIZE {
        return Err(VisionError::InvalidRequest(
            "Video file too large (maximum 500MB)".to_string(),
        ));
    }

    // Validate content type
    if !is_valid_video_type(&content_type) {
        return Err(VisionError::InvalidRequest(format!(
            "Unsupported video format: {}. Supported formats: MP4, MOV, AVI, WebM",
            content_type
        )));
    }

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| VisionError::InvalidRequest("Invalid user ID".to_string()))?;

    // Create analysis record
    let analysis = state
        .analysis_service
        .create_analysis(user_id, String::new(), exercise_type)
        .await
        .map_err(|e| {
            error!("Failed to create analysis: {}", e);
            VisionError::DatabaseError
        })?;

    // Upload video to storage
    let storage_key = state
        .storage_service
        .upload_video(user_id, analysis.id, video_data, &content_type)
        .await
        .map_err(|e| {
            error!("Failed to upload video: {}", e);
            VisionError::StorageError
        })?;

    // Generate presigned URL
    let video_url = state
        .storage_service
        .generate_presigned_url(&storage_key)
        .await
        .map_err(|e| {
            error!("Failed to generate presigned URL: {}", e);
            VisionError::StorageError
        })?;

    // Update analysis with video URL
    sqlx::query("UPDATE vision_analyses SET video_url = $1 WHERE id = $2")
        .bind(&video_url)
        .bind(analysis.id)
        .execute(&state.analysis_service.db)
        .await
        .map_err(|e| {
            error!("Failed to update analysis with video URL: {}", e);
            VisionError::DatabaseError
        })?;

    info!("Video uploaded successfully: analysis_id={}", analysis.id);

    // TODO: Trigger background processing job

    let response = VisionAnalysisUploadResponse {
        id: analysis.id,
        status: analysis.status,
        upload_timestamp: analysis.upload_timestamp,
        video_url,
    };

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// Get analysis result by ID
pub async fn get_analysis(
    Extension(claims): Extension<Claims>,
    State(state): State<Arc<VisionState>>,
    Path(analysis_id): Path<Uuid>,
) -> Result<Response, VisionError> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| VisionError::InvalidRequest("Invalid user ID".to_string()))?;

    // Check authorization
    let analysis = state
        .analysis_service
        .get_user_analysis(analysis_id, user_id)
        .await
        .map_err(|_| VisionError::DatabaseError)?
        .ok_or(VisionError::NotFound)?;

    // Get complete result with scores
    let result = state
        .analysis_service
        .get_complete_result(analysis_id)
        .await
        .map_err(|_| VisionError::DatabaseError)?
        .ok_or(VisionError::NotFound)?;

    Ok(Json(result).into_response())
}

/// Get analysis status
pub async fn get_analysis_status(
    Extension(claims): Extension<Claims>,
    State(state): State<Arc<VisionState>>,
    Path(analysis_id): Path<Uuid>,
) -> Result<Response, VisionError> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| VisionError::InvalidRequest("Invalid user ID".to_string()))?;

    let analysis = state
        .analysis_service
        .get_user_analysis(analysis_id, user_id)
        .await
        .map_err(|_| VisionError::DatabaseError)?
        .ok_or(VisionError::NotFound)?;

    let status_response = serde_json::json!({
        "id": analysis.id,
        "status": analysis.status,
        "upload_timestamp": analysis.upload_timestamp,
        "processing_started_at": analysis.processing_started_at,
        "processing_completed_at": analysis.processing_completed_at,
        "error_message": analysis.error_message,
    });

    Ok(Json(status_response).into_response())
}

/// List user's analyses
#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    20
}

pub async fn list_analyses(
    Extension(claims): Extension<Claims>,
    State(state): State<Arc<VisionState>>,
    Query(query): Query<ListQuery>,
) -> Result<Response, VisionError> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| VisionError::InvalidRequest("Invalid user ID".to_string()))?;

    let analyses = state
        .analysis_service
        .list_user_analyses(user_id, query.limit, query.offset)
        .await
        .map_err(|_| VisionError::DatabaseError)?;

    // Get overall scores for each analysis
    let mut list_items = Vec::new();
    for analysis in analyses {
        let score = state
            .analysis_service
            .get_movement_score(analysis.id)
            .await
            .ok()
            .flatten()
            .map(|s| s.overall_score);

        list_items.push(analysis.to_list_item(score));
    }

    Ok(Json(list_items).into_response())
}

/// Delete analysis
pub async fn delete_analysis(
    Extension(claims): Extension<Claims>,
    State(state): State<Arc<VisionState>>,
    Path(analysis_id): Path<Uuid>,
) -> Result<Response, VisionError> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| VisionError::InvalidRequest("Invalid user ID".to_string()))?;

    // Check authorization
    let analysis = state
        .analysis_service
        .get_user_analysis(analysis_id, user_id)
        .await
        .map_err(|_| VisionError::DatabaseError)?
        .ok_or(VisionError::NotFound)?;

    // Delete from storage
    // Extract storage key from video URL if possible
    // TODO: Implement proper storage key extraction

    // Delete from database (cascades to pose_detections and movement_scores)
    state
        .analysis_service
        .delete_analysis(analysis_id)
        .await
        .map_err(|_| VisionError::DatabaseError)?;

    info!("Deleted analysis: {}", analysis_id);

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Validate video content type
fn is_valid_video_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "video/mp4"
            | "video/quicktime"
            | "video/x-msvideo"
            | "video/webm"
            | "video/x-matroska"
    )
}

/// Vision API errors
#[derive(Debug)]
pub enum VisionError {
    InvalidRequest(String),
    NotFound,
    DatabaseError,
    StorageError,
    ProcessingError,
}

impl IntoResponse for VisionError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            VisionError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            VisionError::NotFound => (StatusCode::NOT_FOUND, "Analysis not found".to_string()),
            VisionError::DatabaseError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            ),
            VisionError::StorageError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Storage error".to_string(),
            ),
            VisionError::ProcessingError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Processing error".to_string(),
            ),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

/// Create vision API routes
///
/// Note: This is a Phase 1 implementation stub. The vision routes are defined
/// but the full S3 storage integration requires additional configuration:
/// - AWS credentials and S3 bucket setup
/// - FFmpeg installation for video processing
/// - Background job queue for async processing
///
/// TODO for Phase 2:
/// - Initialize S3 client with proper AWS configuration
/// - Set up video storage service with bucket details
/// - Implement background processing pipeline
/// - Add video validation and thumbnail generation
pub fn vision_routes(_db: PgPool, auth_service: AuthService) -> Router {
    // Placeholder router until S3 and FFmpeg infrastructure is set up
    // The routes are commented out to prevent runtime errors
    // Uncomment and configure when infrastructure is ready

    Router::new()
        // .route("/upload", post(upload_video))
        // .route("/history", get(list_analyses))
        // .route("/:id", get(get_analysis))
        // .route("/:id/status", get(get_analysis_status))
        // .route("/:id", delete(delete_analysis))
        .layer(middleware::from_fn_with_state(
            auth_service.clone(),
            auth_middleware,
        ))
}
