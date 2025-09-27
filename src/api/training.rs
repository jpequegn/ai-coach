use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use axum_extra::extract::WithRejection;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{AuthService, Claims};
use crate::services::{TrainingAnalysisService, TrainingSessionService, BackgroundJobService};
use crate::models::{TrainingSession, CreateTrainingSession};

#[derive(Debug, Deserialize)]
pub struct UploadQuery {
    /// Whether to process the file immediately after upload
    pub process_immediately: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    /// Maximum number of items to return (default: 50, max: 100)
    pub limit: Option<i64>,
    /// Number of items to skip (default: 0)
    pub offset: Option<i64>,
}

impl PaginationQuery {
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Some(limit) = self.limit {
            if limit < 1 || limit > 100 {
                return Err("Limit must be between 1 and 100");
            }
        }
        if let Some(offset) = self.offset {
            if offset < 0 {
                return Err("Offset must be non-negative");
            }
        }
        Ok(())
    }

    pub fn get_limit(&self) -> i64 {
        self.limit.unwrap_or(50).min(100).max(1)
    }

    pub fn get_offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

#[derive(Debug, Serialize)]
pub struct FileUploadResponse {
    /// Unique identifier for the uploaded file/training session
    pub file_id: String,
    /// Original filename of the uploaded file
    pub filename: String,
    /// Server path where the file is stored
    pub file_path: String,
    /// Current processing status: uploaded, processing, processed, failed
    pub processing_status: String,
    /// Extracted training metrics (if processed)
    pub metrics: Option<serde_json::Value>,
    /// Background job ID (if processing asynchronously)
    pub job_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TrainingMetricsResponse {
    /// Training session identifier
    pub session_id: Uuid,
    /// Extracted training metrics and analysis data
    pub metrics: serde_json::Value,
    /// Current processing status: uploaded, processing, processed, failed
    pub processing_status: String,
    /// When the metrics were last updated
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct PMCResponse {
    /// User identifier
    pub user_id: Uuid,
    /// Performance Management Chart data points
    pub pmc_data: Vec<PMCDataPoint>,
    /// Description of the date range analyzed
    pub date_range: String,
    /// When the data was calculated
    pub calculated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct PMCDataPoint {
    /// Date of the data point
    pub date: chrono::NaiveDate,
    /// Chronic Training Load (Fitness)
    pub ctl: f64,
    /// Acute Training Load (Fatigue)
    pub atl: f64,
    /// Training Stress Balance (Form)
    pub tsb: f64,
    /// Daily Training Stress Score
    pub tss_daily: f64,
}

#[derive(Debug, Deserialize)]
pub struct PMCQuery {
    /// Number of days to analyze (default: 90, min: 7, max: 365)
    pub days: Option<i32>,
}

impl PMCQuery {
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Some(days) = self.days {
            if days < 7 || days > 365 {
                return Err("Days must be between 7 and 365");
            }
        }
        Ok(())
    }

    pub fn get_days(&self) -> i32 {
        self.days.unwrap_or(90).min(365).max(7)
    }
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    /// Error code for programmatic handling
    pub error_code: String,
    /// Human-readable error message
    pub message: String,
    /// Additional error details (optional)
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

    pub fn with_details(code: &str, message: &str, details: serde_json::Value) -> Self {
        Self {
            error_code: code.to_string(),
            message: message.to_string(),
            details: Some(details),
        }
    }
}

pub fn training_routes(db: PgPool, auth_service: AuthService) -> Router {
    let training_analysis_service = TrainingAnalysisService::new(
        db.clone(),
        std::env::var("REDIS_URL").ok(),
    ).expect("Failed to create TrainingAnalysisService");

    let training_session_service = TrainingSessionService::new(db.clone());

    let background_job_service = Arc::new(
        BackgroundJobService::new(
            db.clone(),
            std::env::var("REDIS_URL").ok(),
        ).expect("Failed to create BackgroundJobService")
    );

    let shared_state = AppState {
        db,
        auth_service,
        training_analysis_service,
        training_session_service,
        background_job_service,
    };

    Router::new()
        .route("/upload", post(upload_training_file))
        .route("/sessions/:session_id/metrics", get(get_training_metrics))
        .route("/sessions", get(get_training_sessions))
        .route("/pmc", get(get_performance_management_chart))
        .route("/process/:session_id", post(process_training_session))
        .route("/jobs/:job_id", get(get_job_status))
        .route("/jobs", get(get_user_jobs))
        .with_state(shared_state)
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub training_analysis_service: TrainingAnalysisService,
    pub training_session_service: TrainingSessionService,
    pub background_job_service: Arc<BackgroundJobService>,
}

/// Upload a training file (TCX, GPX, CSV)
pub async fn upload_training_file(
    State(state): State<AppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<UploadQuery>,
    mut multipart: Multipart,
) -> Result<Json<FileUploadResponse>, StatusCode> {
    let user_id = claims.sub;

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let filename = field
                .file_name()
                .ok_or(StatusCode::BAD_REQUEST)?
                .to_string();

            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();

            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

            // Validate file
            if let Err(_) = state.training_analysis_service.validate_training_file(&data, &filename) {
                return Err(StatusCode::BAD_REQUEST);
            }

            // Save file to storage
            let file_path = state
                .training_analysis_service
                .save_training_file(data.clone(), &filename, user_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Create training session record
            let session_data = CreateTrainingSession {
                user_id,
                date: chrono::Utc::now().date_naive(),
                uploaded_file_path: Some(file_path.clone()),
                session_type: Some("uploaded".to_string()),
                duration_seconds: None,
                distance_meters: None,
            };

            let session = state
                .training_session_service
                .create_session(session_data)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let mut response = FileUploadResponse {
                file_id: session.id.to_string(),
                filename: filename.clone(),
                file_path,
                processing_status: "uploaded".to_string(),
                metrics: None,
            };

            // Process immediately if requested
            if query.process_immediately.unwrap_or(false) {
                match state
                    .training_analysis_service
                    .process_training_file(data, &filename, user_id, None)
                    .await
                {
                    Ok(metrics) => {
                        let metrics_json = serde_json::to_value(&metrics)
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                        // Update session with metrics
                        let update_data = crate::models::UpdateTrainingSession {
                            date: None,
                            trainrs_data: Some(metrics_json.clone()),
                            uploaded_file_path: None,
                            session_type: None,
                            duration_seconds: metrics.duration_seconds,
                            distance_meters: metrics.distance_meters,
                        };

                        let _ = state
                            .training_session_service
                            .update_session(session.id, update_data)
                            .await;

                        response.processing_status = "processed".to_string();
                        response.metrics = Some(metrics_json);
                    }
                    Err(_) => {
                        response.processing_status = "processing_failed".to_string();
                    }
                }
            }

            return Ok(Json(response));
        }
    }

    Err(StatusCode::BAD_REQUEST)
}

/// Get training metrics for a specific session
pub async fn get_training_metrics(
    State(state): State<AppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<TrainingMetricsResponse>, StatusCode> {
    let user_id = claims.sub;

    // Get the training session
    let session = state
        .training_session_service
        .get_session_by_id(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify user owns this session
    if session.user_id != user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    let processing_status = if session.trainrs_data.is_some() {
        "processed"
    } else if session.uploaded_file_path.is_some() {
        "uploaded"
    } else {
        "pending"
    };

    let response = TrainingMetricsResponse {
        session_id: session.id,
        metrics: session.trainrs_data.unwrap_or_else(|| serde_json::json!({})),
        processing_status: processing_status.to_string(),
    };

    Ok(Json(response))
}

/// Get training sessions for a user
pub async fn get_training_sessions(
    State(state): State<AppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Vec<TrainingSession>>, StatusCode> {
    let user_id = claims.sub;

    // Validate pagination parameters
    if let Err(_) = pagination.validate() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let limit = Some(pagination.get_limit());
    let offset = Some(pagination.get_offset());

    let sessions = state
        .training_session_service
        .get_sessions_by_user_id(user_id, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(sessions))
}

/// Get Performance Management Chart data
pub async fn get_performance_management_chart(
    State(state): State<AppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<PMCQuery>,
) -> Result<Json<PMCResponse>, StatusCode> {
    let user_id = claims.sub;

    // Validate query parameters
    if let Err(_) = query.validate() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let days = query.get_days();

    let pmc_data = state
        .training_analysis_service
        .calculate_pmc(user_id, days)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let pmc_points: Vec<PMCDataPoint> = pmc_data
        .into_iter()
        .map(|pmc| PMCDataPoint {
            date: pmc.date,
            ctl: pmc.ctl,
            atl: pmc.atl,
            tsb: pmc.tsb,
            tss_daily: 0.0, // This would come from the actual PMC calculation
        })
        .collect();

    let response = PMCResponse {
        user_id,
        pmc_data: pmc_points,
        date_range: format!("{} days", days),
        calculated_at: chrono::Utc::now(),
    };

    Ok(Json(response))
}

/// Process a training session file that was previously uploaded
pub async fn process_training_session(
    State(state): State<AppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<TrainingMetricsResponse>, StatusCode> {
    let user_id = claims.sub;

    // Get the training session
    let session = state
        .training_session_service
        .get_session_by_id(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify user owns this session
    if session.user_id != user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if file exists
    let file_path = session
        .uploaded_file_path
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Read file and process
    let file_data = tokio::fs::read(&file_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");

    let metrics = state
        .training_analysis_service
        .process_training_file(Bytes::from(file_data), filename, user_id, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let metrics_json = serde_json::to_value(&metrics)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update session with metrics
    let update_data = crate::models::UpdateTrainingSession {
        date: None,
        trainrs_data: Some(metrics_json.clone()),
        uploaded_file_path: None,
        session_type: None,
        duration_seconds: metrics.duration_seconds,
        distance_meters: metrics.distance_meters,
    };

    let _ = state
        .training_session_service
        .update_session(session.id, update_data)
        .await;

    let response = TrainingMetricsResponse {
        session_id: session.id,
        metrics: metrics_json,
        processing_status: "processed".to_string(),
    };

    Ok(Json(response))
}

/// Get background job status
pub async fn get_job_status(
    State(state): State<AppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _user_id = claims.sub; // Could be used for additional authorization

    let job = state
        .background_job_service
        .get_job_status(job_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let response = serde_json::json!({
        "job_id": job.id,
        "job_type": format!("{:?}", job.job_type),
        "status": format!("{:?}", job.status),
        "created_at": job.created_at,
        "started_at": job.started_at,
        "completed_at": job.completed_at,
        "error_message": job.error_message,
        "retries": job.retries
    });

    Ok(Json(response))
}

/// Get all background jobs for the authenticated user
pub async fn get_user_jobs(
    State(state): State<AppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let user_id = claims.sub;

    let jobs = state
        .background_job_service
        .get_user_jobs(user_id)
        .await;

    let response: Vec<serde_json::Value> = jobs
        .into_iter()
        .map(|job| serde_json::json!({
            "job_id": job.id,
            "job_type": format!("{:?}", job.job_type),
            "status": format!("{:?}", job.status),
            "created_at": job.created_at,
            "started_at": job.started_at,
            "completed_at": job.completed_at,
            "error_message": job.error_message,
            "retries": job.retries
        }))
        .collect();

    Ok(Json(response))
}