use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use axum_extra::extract::WithRejection;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::auth::{AuthService, Claims};
use crate::models::{
    CreateHrvReadingRequest, CreateRestingHrRequest, CreateSleepDataRequest,
    HrvReadingResponse, HrvReadingsListResponse, RecoveryBaselineResponse,
    RecoveryDataQuery, RestingHrListResponse, RestingHrResponse, SleepDataListResponse,
    SleepDataResponse,
};
use crate::services::RecoveryDataService;

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

    pub fn with_details(code: &str, message: &str, details: serde_json::Value) -> Self {
        Self {
            error_code: code.to_string(),
            message: message.to_string(),
            details: Some(details),
        }
    }
}

#[derive(Clone)]
pub struct RecoveryAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub recovery_service: RecoveryDataService,
}

pub fn recovery_routes(db: PgPool, auth_service: AuthService) -> Router {
    let recovery_service = RecoveryDataService::new(db.clone());
    let shared_state = RecoveryAppState {
        db,
        auth_service,
        recovery_service,
    };

    Router::new()
        // HRV endpoints
        .route("/hrv", post(create_hrv_reading).get(get_hrv_readings))
        // Sleep endpoints
        .route("/sleep", post(create_sleep_data).get(get_sleep_data))
        // Resting HR endpoints
        .route("/resting-hr", post(create_resting_hr).get(get_resting_hr_data))
        // Baseline endpoint
        .route("/baseline", get(get_baseline))
        .with_state(shared_state)
}

// ============================================================================
// HRV Endpoints
// ============================================================================

/// Create HRV reading
pub async fn create_hrv_reading(
    State(state): State<RecoveryAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<CreateHrvReadingRequest>,
) -> Result<Json<HrvReadingResponse>, (StatusCode, Json<ApiError>)> {
    // Validate request
    request.validate().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::with_details(
                "VALIDATION_ERROR",
                "Invalid HRV reading data",
                serde_json::json!({ "errors": e.to_string() }),
            )),
        )
    })?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let reading = state
        .recovery_service
        .create_hrv_reading(user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create HRV reading: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to create HRV reading",
                )),
            )
        })?;

    Ok(Json(reading.into()))
}

/// Get HRV readings
pub async fn get_hrv_readings(
    State(state): State<RecoveryAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<RecoveryDataQuery>,
) -> Result<Json<HrvReadingsListResponse>, (StatusCode, Json<ApiError>)> {
    // Validate query params
    query.validate().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::with_details(
                "VALIDATION_ERROR",
                "Invalid query parameters",
                serde_json::json!({ "errors": e.to_string() }),
            )),
        )
    })?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let readings = state
        .recovery_service
        .get_hrv_readings(user_id, query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get HRV readings: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve HRV readings",
                )),
            )
        })?;

    Ok(Json(readings))
}

// ============================================================================
// Sleep Endpoints
// ============================================================================

/// Create sleep data
pub async fn create_sleep_data(
    State(state): State<RecoveryAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<CreateSleepDataRequest>,
) -> Result<Json<SleepDataResponse>, (StatusCode, Json<ApiError>)> {
    // Validate request
    request.validate().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::with_details(
                "VALIDATION_ERROR",
                "Invalid sleep data",
                serde_json::json!({ "errors": e.to_string() }),
            )),
        )
    })?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let data = state
        .recovery_service
        .create_sleep_data(user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create sleep data: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to create sleep data",
                )),
            )
        })?;

    Ok(Json(data.into()))
}

/// Get sleep data
pub async fn get_sleep_data(
    State(state): State<RecoveryAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<RecoveryDataQuery>,
) -> Result<Json<SleepDataListResponse>, (StatusCode, Json<ApiError>)> {
    // Validate query params
    query.validate().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::with_details(
                "VALIDATION_ERROR",
                "Invalid query parameters",
                serde_json::json!({ "errors": e.to_string() }),
            )),
        )
    })?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let data = state
        .recovery_service
        .get_sleep_data(user_id, query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get sleep data: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve sleep data",
                )),
            )
        })?;

    Ok(Json(data))
}

// ============================================================================
// Resting HR Endpoints
// ============================================================================

/// Create resting HR data
pub async fn create_resting_hr(
    State(state): State<RecoveryAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<CreateRestingHrRequest>,
) -> Result<Json<RestingHrResponse>, (StatusCode, Json<ApiError>)> {
    // Validate request
    request.validate().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::with_details(
                "VALIDATION_ERROR",
                "Invalid resting HR data",
                serde_json::json!({ "errors": e.to_string() }),
            )),
        )
    })?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let data = state
        .recovery_service
        .create_resting_hr(user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create resting HR data: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to create resting HR data",
                )),
            )
        })?;

    Ok(Json(data.into()))
}

/// Get resting HR data
pub async fn get_resting_hr_data(
    State(state): State<RecoveryAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<RecoveryDataQuery>,
) -> Result<Json<RestingHrListResponse>, (StatusCode, Json<ApiError>)> {
    // Validate query params
    query.validate().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::with_details(
                "VALIDATION_ERROR",
                "Invalid query parameters",
                serde_json::json!({ "errors": e.to_string() }),
            )),
        )
    })?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let data = state
        .recovery_service
        .get_resting_hr_data(user_id, query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get resting HR data: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve resting HR data",
                )),
            )
        })?;

    Ok(Json(data))
}

// ============================================================================
// Baseline Endpoint
// ============================================================================

/// Get or calculate baseline
pub async fn get_baseline(
    State(state): State<RecoveryAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<RecoveryBaselineResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let baseline = state
        .recovery_service
        .get_or_calculate_baseline(user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get baseline: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve baseline",
                )),
            )
        })?;

    match baseline {
        Some(b) => Ok(Json(b)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiError::new(
                "INSUFFICIENT_DATA",
                "Not enough data to calculate baseline (need at least 14 days of data)",
            )),
        )),
    }
}
