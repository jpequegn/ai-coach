use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{AlertDeliveryQueue, DeliveryMethod, DeliveryQueueStatus};
use crate::services::AlertDeliveryQueueService;

/// Alert delivery admin state
pub struct AlertDeliveryAdminState {
    pub queue_service: AlertDeliveryQueueService,
    pub db: PgPool,
}

/// Query parameters for failed deliveries
#[derive(Debug, Deserialize)]
pub struct FailedDeliveriesQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    pub method: Option<DeliveryMethod>,
}

fn default_limit() -> usize {
    50
}

/// Query parameters for delivery statistics
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    #[serde(default = "default_time_range")]
    pub time_range: String, // "hour", "day", "week"
}

fn default_time_range() -> String {
    "day".to_string()
}

/// Request payload for canceling a delivery
#[derive(Debug, Deserialize)]
pub struct CancelDeliveryRequest {
    pub reason: Option<String>,
}

/// Queue status response
#[derive(Debug, Serialize)]
pub struct QueueStatusResponse {
    pub total_pending: i64,
    pub total_delivered: i64,
    pub total_failed: i64,
    pub total_cancelled: i64,
    pub by_method: Vec<MethodStatusBreakdown>,
    pub recent_failures: Vec<RecentFailure>,
}

/// Method-specific status breakdown
#[derive(Debug, Serialize)]
pub struct MethodStatusBreakdown {
    pub method: String,
    pub pending: i64,
    pub delivered: i64,
    pub failed: i64,
    pub success_rate: f64,
}

/// Recent failure information
#[derive(Debug, Serialize)]
pub struct RecentFailure {
    pub id: Uuid,
    pub alert_id: Uuid,
    pub method: String,
    pub recipient_id: String,
    pub attempts: i32,
    pub error_message: Option<String>,
    pub last_attempt_at: Option<DateTime<Utc>>,
}

/// Aggregated delivery statistics
#[derive(Debug, Serialize)]
pub struct DeliveryStatistics {
    pub time_range: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub total_attempts: i64,
    pub successful_deliveries: i64,
    pub failed_deliveries: i64,
    pub overall_success_rate: f64,
    pub by_method: Vec<MethodStatistics>,
    pub avg_delivery_time_ms: Option<i64>,
}

/// Method-specific statistics
#[derive(Debug, Serialize)]
pub struct MethodStatistics {
    pub method: String,
    pub attempts: i64,
    pub successful: i64,
    pub failed: i64,
    pub success_rate: f64,
}

/// Get alert delivery queue status
///
/// Returns comprehensive queue statistics including:
/// - Total deliveries by status (pending, delivered, failed, cancelled)
/// - Success/failure rates by delivery method
/// - Recent failures with error details
///
/// # Authentication
/// Requires admin role
///
/// # Response
/// Returns queue status with method breakdowns and recent failures
pub async fn get_delivery_status(
    State(state): State<std::sync::Arc<AlertDeliveryAdminState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get total counts by status
    let status_counts = sqlx::query!(
        r#"
        SELECT
            status as "status: DeliveryQueueStatus",
            COUNT(*) as count
        FROM alert_delivery_queue
        GROUP BY status
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch status counts: {}", e),
        )
    })?;

    let mut total_pending = 0i64;
    let mut total_delivered = 0i64;
    let mut total_failed = 0i64;
    let mut total_cancelled = 0i64;

    for row in status_counts {
        match row.status {
            DeliveryQueueStatus::Pending => total_pending = row.count.unwrap_or(0),
            DeliveryQueueStatus::Delivered => total_delivered = row.count.unwrap_or(0),
            DeliveryQueueStatus::Failed => total_failed = row.count.unwrap_or(0),
            DeliveryQueueStatus::Cancelled => total_cancelled = row.count.unwrap_or(0),
        }
    }

    // Get breakdown by method
    let method_breakdown = sqlx::query!(
        r#"
        SELECT
            delivery_method as "method: DeliveryMethod",
            status as "status: DeliveryQueueStatus",
            COUNT(*) as count
        FROM alert_delivery_queue
        GROUP BY delivery_method, status
        ORDER BY delivery_method
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch method breakdown: {}", e),
        )
    })?;

    // Aggregate by method
    let mut method_stats: std::collections::HashMap<String, (i64, i64, i64)> =
        std::collections::HashMap::new();

    for row in method_breakdown {
        let method = format!("{:?}", row.method).to_lowercase();
        let count = row.count.unwrap_or(0);
        let entry = method_stats.entry(method).or_insert((0, 0, 0));

        match row.status {
            DeliveryQueueStatus::Pending => entry.0 += count,
            DeliveryQueueStatus::Delivered => entry.1 += count,
            DeliveryQueueStatus::Failed => entry.2 += count,
            _ => {}
        }
    }

    let by_method: Vec<MethodStatusBreakdown> = method_stats
        .into_iter()
        .map(|(method, (pending, delivered, failed))| {
            let total = delivered + failed;
            let success_rate = if total > 0 {
                (delivered as f64 / total as f64) * 100.0
            } else {
                0.0
            };

            MethodStatusBreakdown {
                method,
                pending,
                delivered,
                failed,
                success_rate,
            }
        })
        .collect();

    // Get recent failures
    let recent_failures = sqlx::query_as!(
        AlertDeliveryQueue,
        r#"
        SELECT
            id, alert_id,
            delivery_method as "delivery_method: DeliveryMethod",
            recipient_id, attempts, max_attempts,
            last_attempt_at, next_retry_at,
            status as "status: DeliveryQueueStatus",
            error_message, delivered_at,
            created_at, updated_at
        FROM alert_delivery_queue
        WHERE status = 'failed'
        ORDER BY updated_at DESC
        LIMIT 10
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch recent failures: {}", e),
        )
    })?;

    let recent_failures: Vec<RecentFailure> = recent_failures
        .into_iter()
        .map(|d| RecentFailure {
            id: d.id,
            alert_id: d.alert_id,
            method: format!("{:?}", d.delivery_method).to_lowercase(),
            recipient_id: d.recipient_id,
            attempts: d.attempts,
            error_message: d.error_message,
            last_attempt_at: d.last_attempt_at,
        })
        .collect();

    let response = QueueStatusResponse {
        total_pending,
        total_delivered,
        total_failed,
        total_cancelled,
        by_method,
        recent_failures,
    };

    Ok(Json(response))
}

/// Get failed deliveries with filtering
///
/// Returns a paginated list of failed deliveries with error details.
/// Supports filtering by delivery method.
///
/// # Query Parameters
/// - `limit`: Maximum number of results (default: 50, max: 200)
/// - `offset`: Pagination offset (default: 0)
/// - `method`: Filter by delivery method (optional)
///
/// # Authentication
/// Requires admin role
///
/// # Response
/// Returns list of failed deliveries with retry history
pub async fn get_failed_deliveries(
    State(state): State<std::sync::Arc<AlertDeliveryAdminState>>,
    Query(params): Query<FailedDeliveriesQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let limit = params.limit.min(200) as i64;
    let offset = params.offset as i64;

    let deliveries = if let Some(method) = params.method {
        sqlx::query_as!(
            AlertDeliveryQueue,
            r#"
            SELECT
                id, alert_id,
                delivery_method as "delivery_method: DeliveryMethod",
                recipient_id, attempts, max_attempts,
                last_attempt_at, next_retry_at,
                status as "status: DeliveryQueueStatus",
                error_message, delivered_at,
                created_at, updated_at
            FROM alert_delivery_queue
            WHERE status = 'failed'
                AND delivery_method = $1::text
            ORDER BY updated_at DESC
            LIMIT $2 OFFSET $3
            "#,
            method as DeliveryMethod,
            limit,
            offset
        )
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as!(
            AlertDeliveryQueue,
            r#"
            SELECT
                id, alert_id,
                delivery_method as "delivery_method: DeliveryMethod",
                recipient_id, attempts, max_attempts,
                last_attempt_at, next_retry_at,
                status as "status: DeliveryQueueStatus",
                error_message, delivered_at,
                created_at, updated_at
            FROM alert_delivery_queue
            WHERE status = 'failed'
            ORDER BY updated_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&state.db)
        .await
    }
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch failed deliveries: {}", e),
        )
    })?;

    Ok(Json(deliveries))
}

/// Manually retry a failed delivery
///
/// Resets the attempt counter and schedules an immediate retry
/// for a specific delivery. Useful for recovering from transient failures.
///
/// # Path Parameters
/// - `delivery_id`: UUID of the delivery to retry
///
/// # Authentication
/// Requires admin role
///
/// # Response
/// Returns the updated delivery queue entry
pub async fn retry_delivery(
    State(state): State<std::sync::Arc<AlertDeliveryAdminState>>,
    Path(delivery_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Reset attempts and schedule immediate retry
    let updated = sqlx::query_as!(
        AlertDeliveryQueue,
        r#"
        UPDATE alert_delivery_queue
        SET attempts = 0,
            status = 'pending',
            next_retry_at = NOW(),
            error_message = NULL,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id, alert_id,
            delivery_method as "delivery_method: DeliveryMethod",
            recipient_id, attempts, max_attempts,
            last_attempt_at, next_retry_at,
            status as "status: DeliveryQueueStatus",
            error_message, delivered_at,
            created_at, updated_at
        "#,
        delivery_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to retry delivery: {}", e),
        )
    })?;

    match updated {
        Some(delivery) => Ok(Json(delivery)),
        None => Err((
            StatusCode::NOT_FOUND,
            format!("Delivery with ID {} not found", delivery_id),
        )),
    }
}

/// Cancel a pending delivery
///
/// Marks a delivery as cancelled and prevents any further retry attempts.
/// Optionally accepts a cancellation reason for audit purposes.
///
/// # Path Parameters
/// - `delivery_id`: UUID of the delivery to cancel
///
/// # Request Body
/// - `reason`: Optional cancellation reason
///
/// # Authentication
/// Requires admin role
///
/// # Response
/// Returns 204 No Content on success
pub async fn cancel_delivery(
    State(state): State<std::sync::Arc<AlertDeliveryAdminState>>,
    Path(delivery_id): Path<Uuid>,
    Json(payload): Json<CancelDeliveryRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let error_message = payload
        .reason
        .unwrap_or_else(|| "Cancelled by admin".to_string());

    let result = sqlx::query!(
        r#"
        UPDATE alert_delivery_queue
        SET status = 'cancelled',
            error_message = $2,
            updated_at = NOW()
        WHERE id = $1 AND status = 'pending'
        "#,
        delivery_id,
        error_message
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to cancel delivery: {}", e),
        )
    })?;

    if result.rows_affected() == 0 {
        Err((
            StatusCode::NOT_FOUND,
            format!(
                "Delivery with ID {} not found or not in pending status",
                delivery_id
            ),
        ))
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

/// Get delivery statistics for a time range
///
/// Returns aggregated statistics for alert deliveries over a specified
/// time period. Includes overall success rates and per-method breakdowns.
///
/// # Query Parameters
/// - `time_range`: Time range for statistics ("hour", "day", "week") (default: "day")
///
/// # Authentication
/// Requires admin role
///
/// # Response
/// Returns aggregated delivery statistics with time range information
pub async fn get_delivery_statistics(
    State(state): State<std::sync::Arc<AlertDeliveryAdminState>>,
    Query(params): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let end_time = Utc::now();
    let start_time = match params.time_range.as_str() {
        "hour" => end_time - chrono::Duration::hours(1),
        "day" => end_time - chrono::Duration::days(1),
        "week" => end_time - chrono::Duration::weeks(1),
        _ => end_time - chrono::Duration::days(1), // Default to day
    };

    // Get overall statistics
    let overall = sqlx::query!(
        r#"
        SELECT
            COUNT(*) as total_attempts,
            COUNT(*) FILTER (WHERE status = 'delivered') as successful,
            COUNT(*) FILTER (WHERE status = 'failed') as failed
        FROM alert_delivery_queue
        WHERE created_at BETWEEN $1 AND $2
        "#,
        start_time,
        end_time
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch overall statistics: {}", e),
        )
    })?;

    let total_attempts = overall.total_attempts.unwrap_or(0);
    let successful = overall.successful.unwrap_or(0);
    let failed = overall.failed.unwrap_or(0);

    let overall_success_rate = if total_attempts > 0 {
        (successful as f64 / total_attempts as f64) * 100.0
    } else {
        0.0
    };

    // Get per-method statistics
    let method_stats = sqlx::query!(
        r#"
        SELECT
            delivery_method as "method: DeliveryMethod",
            COUNT(*) as attempts,
            COUNT(*) FILTER (WHERE status = 'delivered') as successful,
            COUNT(*) FILTER (WHERE status = 'failed') as failed
        FROM alert_delivery_queue
        WHERE created_at BETWEEN $1 AND $2
        GROUP BY delivery_method
        "#,
        start_time,
        end_time
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch method statistics: {}", e),
        )
    })?;

    let by_method: Vec<MethodStatistics> = method_stats
        .into_iter()
        .map(|row| {
            let attempts = row.attempts.unwrap_or(0);
            let successful = row.successful.unwrap_or(0);
            let failed = row.failed.unwrap_or(0);
            let success_rate = if attempts > 0 {
                (successful as f64 / attempts as f64) * 100.0
            } else {
                0.0
            };

            MethodStatistics {
                method: format!("{:?}", row.method).to_lowercase(),
                attempts,
                successful,
                failed,
                success_rate,
            }
        })
        .collect();

    let statistics = DeliveryStatistics {
        time_range: params.time_range,
        start_time,
        end_time,
        total_attempts,
        successful_deliveries: successful,
        failed_deliveries: failed,
        overall_success_rate,
        by_method,
        avg_delivery_time_ms: None, // Could be calculated if we track delivery times
    };

    Ok(Json(statistics))
}
