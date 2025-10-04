use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, patch, post},
    Router,
};
use axum_extra::extract::WithRejection;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::auth::{AuthService, Claims};
use crate::models::{
    TrainingRecoverySettings, TrainingRecoverySettingsResponse,
    UpdateTrainingRecoverySettingsRequest,
};
use crate::services::TrainingAdjustmentService;

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
pub struct TrainingAdjustmentAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub adjustment_service: TrainingAdjustmentService,
}

pub fn training_adjustment_routes(db: PgPool, auth_service: AuthService) -> Router {
    let adjustment_service = TrainingAdjustmentService::new(db.clone());
    let shared_state = TrainingAdjustmentAppState {
        db,
        auth_service,
        adjustment_service,
    };

    Router::new()
        .route(
            "/recovery-settings",
            get(get_recovery_settings).patch(update_recovery_settings),
        )
        .route("/recommended-adjustment", get(get_recommended_adjustment))
        .with_state(shared_state)
}

// ============================================================================
// Settings Endpoints
// ============================================================================

/// Get training recovery settings
pub async fn get_recovery_settings(
    State(state): State<TrainingAdjustmentAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<TrainingRecoverySettingsResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    // Try to get existing settings
    let settings = sqlx::query_as!(
        TrainingRecoverySettings,
        r#"
        SELECT
            id, user_id, auto_adjust_enabled, adjustment_aggressiveness,
            min_rest_days_per_week, max_consecutive_training_days,
            allow_intensity_reduction, allow_volume_reduction, allow_workout_swap,
            created_at, updated_at
        FROM training_recovery_settings
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch training recovery settings: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(
                "DATABASE_ERROR",
                "Failed to retrieve settings",
            )),
        )
    })?;

    // If settings don't exist, create default settings
    let settings = match settings {
        Some(s) => s,
        None => {
            let default_settings = sqlx::query_as!(
                TrainingRecoverySettings,
                r#"
                INSERT INTO training_recovery_settings (user_id)
                VALUES ($1)
                RETURNING
                    id, user_id, auto_adjust_enabled, adjustment_aggressiveness,
                    min_rest_days_per_week, max_consecutive_training_days,
                    allow_intensity_reduction, allow_volume_reduction, allow_workout_swap,
                    created_at, updated_at
                "#,
                user_id
            )
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create default training recovery settings: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError::new(
                        "DATABASE_ERROR",
                        "Failed to create default settings",
                    )),
                )
            })?;

            default_settings
        }
    };

    Ok(Json(settings.into()))
}

/// Update training recovery settings
pub async fn update_recovery_settings(
    State(state): State<TrainingAdjustmentAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<UpdateTrainingRecoverySettingsRequest>,
) -> Result<Json<TrainingRecoverySettingsResponse>, (StatusCode, Json<ApiError>)> {
    // Validate request
    request.validate().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::with_details(
                "VALIDATION_ERROR",
                "Invalid settings data",
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

    // Build update query dynamically based on provided fields
    let mut updates = Vec::new();
    let mut values: Vec<&(dyn sqlx::Encode<'_, sqlx::Postgres> + Sync)> = vec![&user_id];
    let mut param_count = 1;

    if let Some(auto_adjust) = &request.auto_adjust_enabled {
        param_count += 1;
        updates.push(format!("auto_adjust_enabled = ${}", param_count));
        values.push(auto_adjust);
    }

    if let Some(aggressiveness) = &request.adjustment_aggressiveness {
        param_count += 1;
        updates.push(format!("adjustment_aggressiveness = ${}", param_count));
        values.push(aggressiveness);
    }

    if let Some(min_rest) = &request.min_rest_days_per_week {
        param_count += 1;
        updates.push(format!("min_rest_days_per_week = ${}", param_count));
        values.push(min_rest);
    }

    if let Some(max_consec) = &request.max_consecutive_training_days {
        param_count += 1;
        updates.push(format!("max_consecutive_training_days = ${}", param_count));
        values.push(max_consec);
    }

    if let Some(allow_intensity) = &request.allow_intensity_reduction {
        param_count += 1;
        updates.push(format!("allow_intensity_reduction = ${}", param_count));
        values.push(allow_intensity);
    }

    if let Some(allow_volume) = &request.allow_volume_reduction {
        param_count += 1;
        updates.push(format!("allow_volume_reduction = ${}", param_count));
        values.push(allow_volume);
    }

    if let Some(allow_swap) = &request.allow_workout_swap {
        param_count += 1;
        updates.push(format!("allow_workout_swap = ${}", param_count));
        values.push(allow_swap);
    }

    if updates.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::new(
                "NO_UPDATES",
                "No fields provided for update",
            )),
        ));
    }

    // Construct and execute update query
    let query = format!(
        r#"
        UPDATE training_recovery_settings
        SET {}, updated_at = NOW()
        WHERE user_id = $1
        RETURNING
            id, user_id, auto_adjust_enabled, adjustment_aggressiveness,
            min_rest_days_per_week, max_consecutive_training_days,
            allow_intensity_reduction, allow_volume_reduction, allow_workout_swap,
            created_at, updated_at
        "#,
        updates.join(", ")
    );

    let settings = sqlx::query_as::<_, TrainingRecoverySettings>(&query)
        .bind(user_id)
        .bind(request.auto_adjust_enabled.unwrap_or_default())
        .bind(request.adjustment_aggressiveness.unwrap_or_default())
        .bind(request.min_rest_days_per_week.unwrap_or_default())
        .bind(request.max_consecutive_training_days.unwrap_or_default())
        .bind(request.allow_intensity_reduction.unwrap_or_default())
        .bind(request.allow_volume_reduction.unwrap_or_default())
        .bind(request.allow_workout_swap.unwrap_or_default())
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update training recovery settings: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "DATABASE_ERROR",
                    "Failed to update settings",
                )),
            )
        })?;

    match settings {
        Some(s) => Ok(Json(s.into())),
        None => {
            // Settings don't exist, create them
            get_recovery_settings(State(state), WithRejection(claims, StatusCode::OK)).await
        }
    }
}

// ============================================================================
// Adjustment Recommendation Endpoints
// ============================================================================

use chrono::Utc;
use crate::services::training_adjustment_service::TssAdjustment;

#[derive(Debug, Serialize)]
pub struct RecommendedAdjustmentResponse {
    pub date: String,
    pub has_recovery_data: bool,
    pub tss_adjustment: Option<TssAdjustment>,
    pub rest_recommendation: Option<RestDayInfo>,
}

#[derive(Debug, Serialize)]
pub struct RestDayInfo {
    pub should_rest: bool,
    pub confidence: f64,
    pub reasoning: String,
    pub alternative_action: Option<String>,
}

/// Get today's recommended TSS adjustment
pub async fn get_recommended_adjustment(
    State(state): State<TrainingAdjustmentAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<RecommendedAdjustmentResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")),
        )
    })?;

    let today = Utc::now().date_naive();

    // Check if recovery data exists for today
    let has_recovery = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM recovery_scores
            WHERE user_id = $1 AND score_date = $2
        )
        "#,
        user_id,
        today
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to check recovery data: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(
                "DATABASE_ERROR",
                "Failed to check recovery data",
            )),
        )
    })?
    .unwrap_or(false);

    if !has_recovery {
        return Ok(Json(RecommendedAdjustmentResponse {
            date: today.to_string(),
            has_recovery_data: false,
            tss_adjustment: None,
            rest_recommendation: None,
        }));
    }

    // Get rest day recommendation
    let rest_rec = state
        .adjustment_service
        .should_schedule_rest_day(user_id, today)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get rest day recommendation: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "SERVICE_ERROR",
                    "Failed to calculate rest recommendation",
                )),
            )
        })?;

    // If rest is strongly recommended, include that in response
    let rest_info = if rest_rec.should_rest {
        Some(RestDayInfo {
            should_rest: rest_rec.should_rest,
            confidence: rest_rec.confidence,
            reasoning: rest_rec.reasoning,
            alternative_action: rest_rec.alternative_action,
        })
    } else {
        None
    };

    // Get TSS adjustment for a hypothetical 100 TSS workout
    let tss_adj = state
        .adjustment_service
        .calculate_daily_tss_adjustment(user_id, today, 100.0)
        .await
        .map_err(|e| {
            tracing::error!("Failed to calculate TSS adjustment: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new(
                    "SERVICE_ERROR",
                    "Failed to calculate TSS adjustment",
                )),
            )
        })?;

    Ok(Json(RecommendedAdjustmentResponse {
        date: today.to_string(),
        has_recovery_data: true,
        tss_adjustment: Some(tss_adj),
        rest_recommendation: rest_info,
    }))
}
