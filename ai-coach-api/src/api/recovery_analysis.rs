use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use axum_extra::extract::WithRejection;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{AuthService, Claims};
use crate::models::{
    RecoveryHistoryQuery, RecoveryInsightsResponse, RecoveryStatusResponse, RecoveryTrendsResponse,
};
use crate::services::RecoveryAnalysisService;

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
pub struct RecoveryAnalysisAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub analysis_service: RecoveryAnalysisService,
}

pub fn recovery_analysis_routes(db: PgPool, auth_service: AuthService) -> Router {
    let analysis_service = RecoveryAnalysisService::new(db.clone());
    let shared_state = RecoveryAnalysisAppState {
        db,
        auth_service,
        analysis_service,
    };

    Router::new()
        .route("/status", get(get_recovery_status))
        .route("/trends", get(get_recovery_trends))
        .route("/insights", get(get_recovery_insights))
        .with_state(shared_state)
}

/// Get current recovery status
pub async fn get_recovery_status(
    State(state): State<RecoveryAnalysisAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<RecoveryStatusResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let status = state
        .analysis_service
        .get_recovery_status(user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get recovery status: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve recovery status",
                )),
            )
        })?;

    match status {
        Some(s) => Ok(Json(s)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiError::new(
                "INSUFFICIENT_DATA",
                "Not enough recovery data to calculate status. Please log HRV, sleep, or resting HR data.",
            )),
        )),
    }
}

/// Get recovery trends over a period
pub async fn get_recovery_trends(
    State(state): State<RecoveryAnalysisAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<TrendsQuery>,
) -> Result<Json<RecoveryTrendsResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let period_days = query.period_days.unwrap_or(30).min(365).max(1);

    let trends = state
        .analysis_service
        .get_recovery_trends(user_id, period_days)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get recovery trends: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve recovery trends",
                )),
            )
        })?;

    Ok(Json(trends))
}

/// Get recovery insights
pub async fn get_recovery_insights(
    State(state): State<RecoveryAnalysisAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<RecoveryInsightsResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let insights = state
        .analysis_service
        .get_recovery_insights(user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get recovery insights: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve recovery insights",
                )),
            )
        })?;

    Ok(Json(insights))
}

#[derive(Debug, serde::Deserialize)]
pub struct TrendsQuery {
    pub period_days: Option<i32>,
}
