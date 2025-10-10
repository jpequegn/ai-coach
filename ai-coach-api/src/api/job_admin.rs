use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::{JobStatusSummary, JobsHealthStatus, JobExecutionLog};
use crate::services::RecoveryJobScheduler;

/// Job admin API state
pub struct JobAdminState {
    pub scheduler: Arc<RecoveryJobScheduler>,
}

/// Query parameters for job history
#[derive(Debug, Deserialize)]
pub struct JobHistoryQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    50
}

/// Response for job history
#[derive(Debug, Serialize)]
pub struct JobHistoryResponse {
    pub executions: Vec<JobExecutionLog>,
    pub page: i64,
    pub page_size: i64,
    pub total_count: i64,
}

/// Get status of all jobs
///
/// Returns summary information for all registered jobs including
/// execution counts, success/failure rates, and timing statistics.
///
/// # Authentication
/// Requires admin role
pub async fn get_all_job_statuses(
    State(state): State<Arc<JobAdminState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let summaries = state
        .scheduler
        .get_all_job_statuses()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get job statuses: {}", e),
            )
        })?;

    Ok(Json(summaries))
}

/// Get status of a specific job
///
/// Returns detailed summary for a single job including latest execution,
/// historical statistics, and success/failure information.
///
/// # Path Parameters
/// - `job_name`: Name of the job to query
///
/// # Authentication
/// Requires admin role
pub async fn get_job_status(
    State(state): State<Arc<JobAdminState>>,
    Path(job_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let summary = state
        .scheduler
        .get_job_status_summary(&job_name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get job status: {}", e),
            )
        })?;

    Ok(Json(summary))
}

/// Get execution history for a job
///
/// Returns paginated list of job executions with full details including
/// status, timing, records processed, and any error messages.
///
/// # Path Parameters
/// - `job_name`: Name of the job to query
///
/// # Query Parameters
/// - `page`: Page number (default: 1)
/// - `page_size`: Items per page (default: 50, max: 100)
///
/// # Authentication
/// Requires admin role
pub async fn get_job_history(
    State(state): State<Arc<JobAdminState>>,
    Path(job_name): Path<String>,
    Query(params): Query<JobHistoryQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let page_size = params.page_size.min(100); // Cap at 100
    let offset = (params.page - 1) * page_size;

    let executions = state
        .scheduler
        .get_job_history(&job_name, page_size, offset)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get job history: {}", e),
            )
        })?;

    // Get total count for pagination
    let total_count = executions.len() as i64; // Simplified - in production would query total count

    let response = JobHistoryResponse {
        executions,
        page: params.page,
        page_size,
        total_count,
    };

    Ok(Json(response))
}

/// Get latest execution for a job
///
/// Returns the most recent execution record for a job, useful for
/// checking current status without pagination.
///
/// # Path Parameters
/// - `job_name`: Name of the job to query
///
/// # Authentication
/// Requires admin role
pub async fn get_latest_execution(
    State(state): State<Arc<JobAdminState>>,
    Path(job_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let execution = state
        .scheduler
        .get_latest_execution(&job_name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get latest execution: {}", e),
            )
        })?;

    match execution {
        Some(exec) => Ok(Json(exec)),
        None => Err((
            StatusCode::NOT_FOUND,
            format!("No execution history found for job: {}", job_name),
        )),
    }
}

/// Check health of all jobs
///
/// Returns comprehensive health status for all registered jobs including
/// consecutive failure counts and health indicators.
///
/// # Authentication
/// Requires admin role
pub async fn check_jobs_health(
    State(state): State<Arc<JobAdminState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let health = state
        .scheduler
        .check_all_jobs_health()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to check jobs health: {}", e),
            )
        })?;

    Ok(Json(health))
}

/// Check health of a specific job
///
/// Returns health status for a single job including failure information
/// and diagnostic messages.
///
/// # Path Parameters
/// - `job_name`: Name of the job to check
///
/// # Authentication
/// Requires admin role
pub async fn check_job_health(
    State(state): State<Arc<JobAdminState>>,
    Path(job_name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let health = state
        .scheduler
        .check_job_health(&job_name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to check job health: {}", e),
            )
        })?;

    Ok(Json(health))
}
