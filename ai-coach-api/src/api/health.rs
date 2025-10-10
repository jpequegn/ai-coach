use axum::{extract::State, http::StatusCode, response::Json};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::models::JobHealthStatus;
use crate::services::RecoveryJobScheduler;

/// Application health state
pub struct HealthState {
    pub scheduler: Option<Arc<RecoveryJobScheduler>>,
}

/// Basic health check endpoint
///
/// Returns service health status without detailed checks
pub async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "status": "healthy",
        "service": "ai-coach",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Detailed health check with job monitoring
///
/// Returns comprehensive health including background job status
pub async fn health_check_detailed(
    State(state): State<Arc<HealthState>>,
) -> Result<Json<Value>, StatusCode> {
    let mut response = json!({
        "status": "healthy",
        "service": "ai-coach",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    // Add job health if scheduler is available
    if let Some(scheduler) = &state.scheduler {
        match scheduler.check_all_jobs_health().await {
            Ok(job_health) => {
                let overall_status = match job_health.overall_status {
                    JobHealthStatus::Healthy => "healthy",
                    JobHealthStatus::Degraded => "degraded",
                    JobHealthStatus::Unhealthy => "unhealthy",
                };

                response["jobs"] = json!({
                    "status": overall_status,
                    "total": job_health.total_jobs,
                    "healthy": job_health.healthy_jobs,
                    "degraded": job_health.degraded_jobs,
                    "unhealthy": job_health.unhealthy_jobs,
                });

                // Update overall status if jobs are unhealthy
                if job_health.overall_status == JobHealthStatus::Unhealthy {
                    response["status"] = json!("degraded");
                }
            }
            Err(e) => {
                tracing::error!("Failed to check job health: {}", e);
                response["jobs"] = json!({
                    "status": "unknown",
                    "error": "Failed to check job health"
                });
            }
        }
    }

    Ok(Json(response))
}