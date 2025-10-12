use axum::{routing::get, Router};
use sqlx::PgPool;
use std::sync::Arc;

use super::data_quality_admin::{
    get_baseline_changes, get_missing_data_users, get_poor_quality_users, get_quality_summary,
    get_quality_trends, get_user_quality_history, DataQualityAdminState,
};
use crate::auth::{admin_only_middleware, jwt_auth_middleware, AuthService};

/// Create data quality administration routes
///
/// All routes require admin authentication and provide access to:
/// - Data quality summary statistics
/// - Poor quality user reports
/// - Missing data user reports
/// - Baseline change history
/// - User quality trends
/// - System-wide quality metrics
pub fn data_quality_admin_routes(db: PgPool, auth_service: AuthService) -> Router {
    let state = Arc::new(DataQualityAdminState { db });

    Router::new()
        // Summary endpoint
        .route("/summary", get(get_quality_summary))
        // User quality endpoints
        .route("/users/poor", get(get_poor_quality_users))
        .route("/users/missing", get(get_missing_data_users))
        .route("/users/:user_id/history", get(get_user_quality_history))
        // Baseline changes endpoint
        .route("/baselines/changes", get(get_baseline_changes))
        // Trends endpoint
        .route("/trends", get(get_quality_trends))
        // Apply JWT authentication first
        .layer(axum::middleware::from_fn_with_state(
            auth_service,
            jwt_auth_middleware,
        ))
        // Then apply admin-only authorization
        .layer(axum::middleware::from_fn(admin_only_middleware))
        .with_state(state)
}
