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
    RecoveryAlert, RecoveryAlertPreferences, RecoveryHistoryQuery, RecoveryInsightsResponse,
    RecoveryStatusResponse, RecoveryTrendsResponse,
};
use crate::services::{NotificationService, RecoveryAlertService, RecoveryAnalysisService};

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
    pub alert_service: RecoveryAlertService,
}

pub fn recovery_analysis_routes(db: PgPool, auth_service: AuthService) -> Router {
    let analysis_service = RecoveryAnalysisService::new(db.clone());
    let notification_service = NotificationService::new(db.clone());
    let alert_service = RecoveryAlertService::new(db.clone(), notification_service);

    let shared_state = RecoveryAnalysisAppState {
        db,
        auth_service,
        analysis_service,
        alert_service,
    };

    Router::new()
        // Recovery status and analysis
        .route("/status", get(get_recovery_status))
        .route("/trends", get(get_recovery_trends))
        .route("/insights", get(get_recovery_insights))
        // Alert management
        .route("/alerts", get(get_alerts))
        .route("/alerts/:id/acknowledge", axum::routing::post(acknowledge_alert))
        .route("/alerts/settings", get(get_alert_settings))
        .route("/alerts/settings", axum::routing::patch(update_alert_settings))
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

// ============================================================================
// Alert Endpoints
// ============================================================================

#[derive(Debug, serde::Deserialize)]
pub struct AlertsQuery {
    pub limit: Option<i64>,
    pub include_acknowledged: Option<bool>,
}

/// Get user's recovery alerts
pub async fn get_alerts(
    State(state): State<RecoveryAnalysisAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<AlertsQuery>,
) -> Result<Json<Vec<RecoveryAlert>>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let limit = query.limit.unwrap_or(50).min(200);
    let include_acknowledged = query.include_acknowledged.unwrap_or(false);

    let alerts = state
        .alert_service
        .get_alert_history(user_id, limit, include_acknowledged)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get alerts: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve alerts")),
            )
        })?;

    Ok(Json(alerts))
}

/// Acknowledge an alert
pub async fn acknowledge_alert(
    State(state): State<RecoveryAnalysisAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    axum::extract::Path(alert_id): axum::extract::Path<Uuid>,
) -> Result<Json<RecoveryAlert>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let alert = state
        .alert_service
        .acknowledge_alert(alert_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to acknowledge alert: {}", e);
            (
                StatusCode::NOT_FOUND,
                Json(ApiError::new(
                    "ALERT_NOT_FOUND",
                    "Alert not found or already acknowledged",
                )),
            )
        })?;

    Ok(Json(alert))
}

/// Get user alert settings
pub async fn get_alert_settings(
    State(state): State<RecoveryAnalysisAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<RecoveryAlertPreferences>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let settings = state
        .alert_service
        .get_preferences(user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get alert settings: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve alert settings",
                )),
            )
        })?;

    Ok(Json(settings))
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateAlertSettingsRequest {
    pub enabled: Option<bool>,
    pub push_notifications: Option<bool>,
    pub email_notifications: Option<bool>,
    pub poor_recovery_threshold: Option<f64>,
    pub critical_recovery_threshold: Option<f64>,
}

/// Update user alert settings
pub async fn update_alert_settings(
    State(state): State<RecoveryAnalysisAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<UpdateAlertSettingsRequest>,
) -> Result<Json<RecoveryAlertPreferences>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let settings = state
        .alert_service
        .update_preferences(
            user_id,
            request.enabled,
            request.push_notifications,
            request.email_notifications,
            request.poor_recovery_threshold,
            request.critical_recovery_threshold,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to update alert settings: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to update alert settings",
                )),
            )
        })?;

    Ok(Json(settings))
}
