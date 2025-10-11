use axum::{routing::{get, post}, Router};
use sqlx::PgPool;
use std::sync::Arc;

use super::alert_delivery_admin::{
    cancel_delivery, get_delivery_statistics, get_delivery_status, get_failed_deliveries,
    retry_delivery, AlertDeliveryAdminState,
};
use crate::auth::{admin_only_middleware, jwt_auth_middleware, AuthService};
use crate::services::AlertDeliveryQueueService;

/// Create alert delivery administration routes
///
/// All routes require admin authentication and provide:
/// - Queue status monitoring
/// - Failed delivery management
/// - Manual retry/cancel operations
/// - Delivery statistics and reporting
pub fn alert_delivery_admin_routes(
    queue_service: AlertDeliveryQueueService,
    db: PgPool,
    auth_service: AuthService,
) -> Router {
    let state = Arc::new(AlertDeliveryAdminState { queue_service, db });

    Router::new()
        // Queue status endpoint
        .route("/status", get(get_delivery_status))
        // Failed deliveries endpoint
        .route("/failed", get(get_failed_deliveries))
        // Manual retry endpoint
        .route("/:delivery_id/retry", post(retry_delivery))
        // Cancel delivery endpoint
        .route("/:delivery_id/cancel", post(cancel_delivery))
        // Statistics endpoint
        .route("/stats", get(get_delivery_statistics))
        // Apply JWT authentication first
        .layer(axum::middleware::from_fn_with_state(
            auth_service,
            jwt_auth_middleware,
        ))
        // Then apply admin-only authorization
        .layer(axum::middleware::from_fn(admin_only_middleware))
        .with_state(state)
}
