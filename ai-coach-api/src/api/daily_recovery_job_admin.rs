use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::services::DailyRecoveryCalculationJob;

/// Daily recovery job admin state
pub struct DailyRecoveryJobAdminState {
    pub job: Arc<DailyRecoveryCalculationJob>,
}

/// Query parameters for timezone trigger
#[derive(Debug, Deserialize)]
pub struct TriggerTimezoneQuery {
    pub timezone: String,
}

/// Manually trigger daily recovery calculation for all users
///
/// Triggers the full daily recovery calculation job immediately,
/// processing all active users across all timezones.
///
/// # Authentication
/// Requires admin role
///
/// # Response
/// Returns job execution statistics including successful/failed counts
pub async fn trigger_all(
    State(state): State<Arc<DailyRecoveryJobAdminState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let stats = state.job.execute().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to execute job: {}", e),
        )
    })?;

    Ok(Json(stats))
}

/// Manually trigger daily recovery calculation for a specific user
///
/// Triggers recovery calculation for a single user immediately.
/// Useful for testing or recovering from errors.
///
/// # Path Parameters
/// - `user_id`: UUID of the user to calculate recovery for
///
/// # Authentication
/// Requires admin role
///
/// # Response
/// Returns 200 OK on success
pub async fn trigger_user(
    State(state): State<Arc<DailyRecoveryJobAdminState>>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state.job.trigger_for_user(user_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to trigger calculation for user: {}", e),
        )
    })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": user_id,
        "message": "Recovery calculation triggered successfully"
    })))
}

/// Manually trigger daily recovery calculation for a specific timezone
///
/// Triggers recovery calculation for all active users in a specific timezone.
/// Useful for testing timezone-based scheduling.
///
/// # Query Parameters
/// - `timezone`: IANA timezone string (e.g., "America/New_York", "UTC")
///
/// # Authentication
/// Requires admin role
///
/// # Response
/// Returns job execution statistics for the specified timezone
pub async fn trigger_timezone(
    State(state): State<Arc<DailyRecoveryJobAdminState>>,
    Query(params): Query<TriggerTimezoneQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let stats = state
        .job
        .trigger_for_timezone(&params.timezone)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to trigger calculation for timezone: {}", e),
            )
        })?;

    Ok(Json(stats))
}

/// Dry run - check what would be processed without executing
///
/// Returns information about which users would be processed
/// if the job were to run now, without actually calculating anything.
/// Useful for testing and validation.
///
/// # Authentication
/// Requires admin role
///
/// # Response
/// Returns:
/// - current_utc_hour: Current UTC hour
/// - total_users: Total number of users that would be processed
/// - users_by_timezone: Count of users grouped by timezone
pub async fn dry_run(
    State(state): State<Arc<DailyRecoveryJobAdminState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let result = state.job.dry_run().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to execute dry run: {}", e),
        )
    })?;

    Ok(Json(result))
}
