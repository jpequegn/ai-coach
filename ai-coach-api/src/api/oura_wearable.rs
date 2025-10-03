use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Redirect},
    routing::{delete, get, post},
    Router,
};
use axum_extra::extract::WithRejection;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{AuthService, Claims};
use crate::services::OuraIntegrationService;

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
pub struct OuraAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub oura_service: OuraIntegrationService,
}

pub fn oura_wearable_routes(
    db: PgPool,
    auth_service: AuthService,
    oura_client_id: String,
    oura_client_secret: String,
    oura_redirect_uri: String,
) -> Router {
    let oura_service = OuraIntegrationService::new(
        db.clone(),
        oura_client_id,
        oura_client_secret,
        oura_redirect_uri,
    )
    .expect("Failed to create Oura integration service");

    let shared_state = OuraAppState {
        db,
        auth_service,
        oura_service,
    };

    Router::new()
        .route("/authorize", get(authorize_oura))
        .route("/callback", get(oura_callback))
        .route("/sync", post(sync_oura_data))
        .route("/disconnect", delete(disconnect_oura))
        .with_state(shared_state)
}

// ============================================================================
// OAuth Flow Endpoints
// ============================================================================

/// Start Oura OAuth authorization
pub async fn authorize_oura(
    State(state): State<OuraAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Redirect, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let auth_url = state.oura_service.get_authorization_url(user_id);

    Ok(Redirect::to(&auth_url))
}

#[derive(Debug, Deserialize)]
pub struct OuraCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OuraConnectionResponse {
    pub success: bool,
    pub message: String,
    pub provider: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// Handle Oura OAuth callback
pub async fn oura_callback(
    State(state): State<OuraAppState>,
    Query(params): Query<OuraCallbackQuery>,
) -> Result<Json<OuraConnectionResponse>, (StatusCode, Json<ApiError>)> {
    // Check for OAuth errors
    if let Some(error) = params.error {
        let description = params.error_description.unwrap_or_default();
        tracing::error!("Oura OAuth error: {} - {}", error, description);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::new(
                "OAUTH_ERROR",
                &format!("Oura authorization failed: {}", error),
            )),
        ));
    }

    let code = params.code.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new(
                "MISSING_CODE",
                "Authorization code is missing",
            )),
        )
    })?;

    let state_value = params.state.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("MISSING_STATE", "State parameter is missing")),
        )
    })?;

    // Extract user_id from state (format: "user_id:oura")
    let user_id = state_value
        .split(':')
        .next()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiError::new("INVALID_STATE", "Invalid state parameter")),
            )
        })?;

    // Handle OAuth callback
    let connection = state
        .oura_service
        .handle_oauth_callback(user_id, &code)
        .await
        .map_err(|e| {
            tracing::error!("Failed to handle Oura OAuth callback: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "OAUTH_CALLBACK_ERROR",
                    "Failed to complete Oura authorization",
                )),
            )
        })?;

    Ok(Json(OuraConnectionResponse {
        success: true,
        message: "Oura Ring successfully connected".to_string(),
        provider: connection.provider,
        connected_at: connection.connected_at,
    }))
}

// ============================================================================
// Data Sync Endpoints
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SyncQuery {
    pub days_back: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub success: bool,
    pub sleep_records: usize,
    pub hrv_readings: usize,
    pub rhr_readings: usize,
    pub errors: Vec<String>,
}

/// Manually trigger Oura data sync
pub async fn sync_oura_data(
    State(state): State<OuraAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<SyncQuery>,
) -> Result<Json<SyncResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let days_back = query.days_back.unwrap_or(30).min(90); // Max 90 days

    let result = state
        .oura_service
        .sync_user_data(user_id, days_back)
        .await
        .map_err(|e| {
            tracing::error!("Failed to sync Oura data: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "SYNC_ERROR",
                    &format!("Failed to sync Oura data: {}", e),
                )),
            )
        })?;

    Ok(Json(SyncResponse {
        success: result.errors.is_empty(),
        sleep_records: result.sleep_records,
        hrv_readings: result.hrv_readings,
        rhr_readings: result.rhr_readings,
        errors: result.errors,
    }))
}

// ============================================================================
// Disconnect Endpoint
// ============================================================================

#[derive(Debug, Serialize)]
pub struct DisconnectResponse {
    pub success: bool,
    pub message: String,
}

/// Disconnect Oura Ring
pub async fn disconnect_oura(
    State(state): State<OuraAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<DisconnectResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    state
        .oura_service
        .disconnect(user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to disconnect Oura: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DISCONNECT_ERROR",
                    "Failed to disconnect Oura Ring",
                )),
            )
        })?;

    Ok(Json(DisconnectResponse {
        success: true,
        message: "Oura Ring successfully disconnected".to_string(),
    }))
}
