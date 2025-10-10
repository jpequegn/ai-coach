use axum::{routing::{get, post}, Router};
use std::sync::Arc;

use super::daily_recovery_job_admin::{
    dry_run, trigger_all, trigger_timezone, trigger_user, DailyRecoveryJobAdminState,
};
use crate::auth::{jwt_auth_middleware, admin_only_middleware, AuthService};
use crate::services::DailyRecoveryCalculationJob;

/// Create daily recovery job administration routes
///
/// All routes require admin authentication and provide:
/// - Manual job triggering (all users, specific user, specific timezone)
/// - Dry run mode for testing
pub fn daily_recovery_job_admin_routes(
    job: Arc<DailyRecoveryCalculationJob>,
    auth_service: AuthService,
) -> Router {
    let state = Arc::new(DailyRecoveryJobAdminState { job });

    Router::new()
        // Manual trigger endpoints
        .route("/trigger", post(trigger_all))
        .route("/trigger/user/:user_id", post(trigger_user))
        .route("/trigger/timezone", post(trigger_timezone))
        // Dry run endpoint
        .route("/dry-run", get(dry_run))
        // Apply JWT authentication first
        .layer(axum::middleware::from_fn_with_state(
            auth_service,
            jwt_auth_middleware,
        ))
        // Then apply admin-only authorization
        .layer(axum::middleware::from_fn(admin_only_middleware))
        .with_state(state)
}
