use axum::{routing::get, Router};
use std::sync::Arc;

use super::job_admin::{
    check_job_health, check_jobs_health, get_all_job_statuses, get_job_history, get_job_status,
    get_latest_execution, JobAdminState,
};
use crate::auth::{AuthService, jwt_auth_middleware, admin_only_middleware};
use crate::services::RecoveryJobScheduler;

/// Create job administration routes
///
/// All routes require admin authentication and provide access to:
/// - Job execution history
/// - Job status monitoring
/// - Job health checks
pub fn job_admin_routes(
    scheduler: Arc<RecoveryJobScheduler>,
    auth_service: AuthService,
) -> Router {
    let state = Arc::new(JobAdminState { scheduler });

    Router::new()
        // Job status endpoints
        .route("/status", get(get_all_job_statuses))
        .route("/status/:job_name", get(get_job_status))
        // Job history endpoints
        .route("/history/:job_name", get(get_job_history))
        .route("/latest/:job_name", get(get_latest_execution))
        // Job health endpoints
        .route("/health", get(check_jobs_health))
        .route("/health/:job_name", get(check_job_health))
        // Apply JWT authentication first
        .layer(axum::middleware::from_fn_with_state(
            auth_service,
            jwt_auth_middleware,
        ))
        // Then apply admin-only authorization
        .layer(axum::middleware::from_fn(admin_only_middleware))
        .with_state(state)
}
