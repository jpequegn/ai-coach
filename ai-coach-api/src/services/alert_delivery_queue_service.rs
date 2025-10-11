use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::{
    AlertDeliveryQueue, DeliveryMethod, DeliveryQueueStatus, DeliveryStats, MethodStats,
};
use crate::services::NotificationService;

/// Retry schedule in minutes: 0s, 5min, 15min, 1hr
const RETRY_SCHEDULE_MINUTES: [i64; 4] = [0, 5, 15, 60];
const MAX_ATTEMPTS: i32 = 3;
const DEFAULT_BATCH_SIZE: usize = 100;
const RATE_LIMIT_PER_METHOD: usize = 50; // Max deliveries per method per batch

/// Service for managing alert delivery queue with retry logic
pub struct AlertDeliveryQueueService {
    db: PgPool,
    notification_service: NotificationService,
}

impl AlertDeliveryQueueService {
    /// Create a new alert delivery queue service
    pub fn new(db: PgPool, notification_service: NotificationService) -> Self {
        Self {
            db,
            notification_service,
        }
    }

    /// Queue an alert for delivery
    ///
    /// # Arguments
    /// * `alert_id` - The recovery alert ID
    /// * `delivery_method` - Push, Email, or SMS
    /// * `recipient_id` - Device token, email address, or phone number
    ///
    /// # Returns
    /// The UUID of the created queue entry
    pub async fn queue_for_delivery(
        &self,
        alert_id: Uuid,
        delivery_method: DeliveryMethod,
        recipient_id: String,
    ) -> Result<Uuid> {
        let queue_entry = sqlx::query_as!(
            AlertDeliveryQueue,
            r#"
            INSERT INTO alert_delivery_queue (
                id, alert_id, delivery_method, recipient_id,
                attempts, max_attempts, status,
                next_retry_at, created_at, updated_at
            )
            VALUES ($1, $2, $3::text, $4, 0, $5, $6::text, NOW(), NOW(), NOW())
            RETURNING
                id, alert_id,
                delivery_method as "delivery_method: DeliveryMethod",
                recipient_id, attempts, max_attempts,
                last_attempt_at, next_retry_at,
                status as "status: DeliveryQueueStatus",
                error_message, delivered_at,
                created_at, updated_at
            "#,
            Uuid::new_v4(),
            alert_id,
            delivery_method as DeliveryMethod,
            recipient_id,
            MAX_ATTEMPTS,
            DeliveryQueueStatus::Pending as DeliveryQueueStatus,
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to queue alert for delivery")?;

        info!(
            "Queued alert {} for delivery via {:?} to {}",
            alert_id, delivery_method, recipient_id
        );

        Ok(queue_entry.id)
    }

    /// Get pending deliveries that are due for retry
    ///
    /// Returns deliveries where:
    /// - Status is pending
    /// - next_retry_at is NULL or in the past
    /// - attempts < max_attempts
    pub async fn get_pending_deliveries(&self, limit: Option<usize>) -> Result<Vec<AlertDeliveryQueue>> {
        let limit = limit.unwrap_or(DEFAULT_BATCH_SIZE) as i64;

        let deliveries = sqlx::query_as!(
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
            WHERE status = 'pending'
                AND (next_retry_at IS NULL OR next_retry_at <= NOW())
                AND attempts < max_attempts
            ORDER BY created_at ASC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch pending deliveries")?;

        Ok(deliveries)
    }

    /// Mark a delivery as successfully delivered
    pub async fn mark_delivered(&self, delivery_id: Uuid) -> Result<()> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE alert_delivery_queue
            SET status = 'delivered',
                delivered_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
            delivery_id
        )
        .execute(&self.db)
        .await
        .context("Failed to mark delivery as delivered")?
        .rows_affected();

        if rows_affected == 0 {
            warn!("No delivery found with ID {}", delivery_id);
        } else {
            info!("Marked delivery {} as delivered", delivery_id);
        }

        Ok(())
    }

    /// Mark a delivery as failed and schedule retry if attempts remain
    ///
    /// # Arguments
    /// * `delivery_id` - The delivery queue entry ID
    /// * `error_message` - Error message describing the failure
    pub async fn mark_failed(&self, delivery_id: Uuid, error_message: String) -> Result<()> {
        // First, get the current delivery to check attempts
        let delivery = sqlx::query_as!(
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
            WHERE id = $1
            "#,
            delivery_id
        )
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch delivery")?;

        let Some(delivery) = delivery else {
            warn!("No delivery found with ID {}", delivery_id);
            return Ok(());
        };

        let new_attempts = delivery.attempts + 1;
        let next_retry_at = if new_attempts < delivery.max_attempts {
            Some(Self::calculate_next_retry(new_attempts))
        } else {
            None
        };

        let new_status = if new_attempts >= delivery.max_attempts {
            DeliveryQueueStatus::Failed
        } else {
            DeliveryQueueStatus::Pending
        };

        sqlx::query!(
            r#"
            UPDATE alert_delivery_queue
            SET attempts = $1,
                last_attempt_at = NOW(),
                next_retry_at = $2,
                status = $3::text,
                error_message = $4,
                updated_at = NOW()
            WHERE id = $5
            "#,
            new_attempts,
            next_retry_at,
            new_status as DeliveryQueueStatus,
            error_message,
            delivery_id
        )
        .execute(&self.db)
        .await
        .context("Failed to mark delivery as failed")?;

        if new_status == DeliveryQueueStatus::Failed {
            error!(
                "Delivery {} permanently failed after {} attempts: {}",
                delivery_id, new_attempts, error_message
            );
        } else {
            warn!(
                "Delivery {} failed (attempt {}/{}), will retry at {:?}: {}",
                delivery_id, new_attempts, delivery.max_attempts, next_retry_at, error_message
            );
        }

        Ok(())
    }

    /// Cancel a pending delivery
    pub async fn cancel_delivery(&self, delivery_id: Uuid) -> Result<()> {
        let rows_affected = sqlx::query!(
            r#"
            UPDATE alert_delivery_queue
            SET status = 'cancelled',
                updated_at = NOW()
            WHERE id = $1 AND status = 'pending'
            "#,
            delivery_id
        )
        .execute(&self.db)
        .await
        .context("Failed to cancel delivery")?
        .rows_affected();

        if rows_affected == 0 {
            warn!("No pending delivery found with ID {}", delivery_id);
        } else {
            info!("Cancelled delivery {}", delivery_id);
        }

        Ok(())
    }

    /// Execute a single delivery attempt
    ///
    /// # Arguments
    /// * `delivery` - The delivery queue entry
    ///
    /// # Returns
    /// Ok(()) if delivery succeeded, Err if failed
    pub async fn execute_delivery(&self, delivery: &AlertDeliveryQueue) -> Result<()> {
        let start = Instant::now();

        info!(
            "Executing delivery {} (attempt {}/{}) via {:?}",
            delivery.id,
            delivery.attempts + 1,
            delivery.max_attempts,
            delivery.delivery_method
        );

        // Execute the delivery through NotificationService
        let result = match delivery.delivery_method {
            DeliveryMethod::Push => {
                self.send_push_notification(delivery).await
            }
            DeliveryMethod::Email => {
                self.send_email_notification(delivery).await
            }
            DeliveryMethod::Sms => {
                self.send_sms_notification(delivery).await
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(_) => {
                self.mark_delivered(delivery.id).await?;
                info!(
                    "Successfully delivered {} in {}ms",
                    delivery.id, duration_ms
                );
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.mark_failed(delivery.id, error_msg.clone()).await?;
                Err(e).context("Delivery execution failed")
            }
        }
    }

    /// Process all pending deliveries in the queue
    ///
    /// # Returns
    /// Statistics about the batch processing
    pub async fn process_pending_queue(&self) -> Result<DeliveryStats> {
        let start = Instant::now();
        let mut stats = DeliveryStats::default();
        let mut method_counters: HashMap<String, MethodCounter> = HashMap::new();

        // Get pending deliveries
        let deliveries = self.get_pending_deliveries(Some(DEFAULT_BATCH_SIZE)).await?;

        if deliveries.is_empty() {
            info!("No pending deliveries to process");
            return Ok(stats);
        }

        info!("Processing {} pending deliveries", deliveries.len());

        // Group deliveries by method to respect rate limits
        let mut by_method: HashMap<DeliveryMethod, Vec<AlertDeliveryQueue>> = HashMap::new();
        for delivery in deliveries {
            by_method.entry(delivery.delivery_method.clone())
                .or_default()
                .push(delivery);
        }

        // Process each method group with rate limiting
        for (method, deliveries) in by_method {
            let method_str = format!("{:?}", method).to_lowercase();
            let counter = method_counters.entry(method_str.clone()).or_default();

            // Respect rate limit per method
            let to_process = deliveries.into_iter()
                .take(RATE_LIMIT_PER_METHOD)
                .collect::<Vec<_>>();

            info!(
                "Processing {} {:?} deliveries (rate limit: {})",
                to_process.len(),
                method,
                RATE_LIMIT_PER_METHOD
            );

            for delivery in to_process {
                stats.total_attempted += 1;
                counter.attempted += 1;

                match self.execute_delivery(&delivery).await {
                    Ok(_) => {
                        stats.successful += 1;
                        counter.successful += 1;
                    }
                    Err(e) => {
                        error!("Failed to execute delivery {}: {}", delivery.id, e);
                        stats.failed += 1;
                        counter.failed += 1;

                        // Check if it will be retried
                        if delivery.attempts + 1 < delivery.max_attempts {
                            stats.retrying += 1;
                        }
                    }
                }
            }
        }

        // Calculate average delivery time and per-method stats
        let duration_ms = start.elapsed().as_millis() as u64;
        stats.delivery_time_avg_ms = if stats.total_attempted > 0 {
            duration_ms / stats.total_attempted as u64
        } else {
            0
        };

        // Convert counters to method stats
        for (method, counter) in method_counters {
            stats.by_method.insert(method, counter.to_stats());
        }

        info!(
            "Processed {} deliveries: {} successful, {} failed, {} retrying ({}ms avg)",
            stats.total_attempted,
            stats.successful,
            stats.failed,
            stats.retrying,
            stats.delivery_time_avg_ms
        );

        Ok(stats)
    }

    // Private helper methods

    /// Calculate the next retry time based on exponential backoff
    ///
    /// Retry schedule: 0s (immediate), 5min, 15min, 1hr
    fn calculate_next_retry(attempt: i32) -> DateTime<Utc> {
        let index = attempt.min(RETRY_SCHEDULE_MINUTES.len() as i32 - 1) as usize;
        let delay_minutes = RETRY_SCHEDULE_MINUTES[index];
        Utc::now() + Duration::minutes(delay_minutes)
    }

    /// Send push notification (delegates to NotificationService)
    async fn send_push_notification(&self, delivery: &AlertDeliveryQueue) -> Result<()> {
        // In a real implementation, this would integrate with the NotificationService
        // For now, we'll simulate the delivery
        info!(
            "Sending push notification to {} for alert {}",
            delivery.recipient_id, delivery.alert_id
        );

        // TODO: Integrate with actual notification service
        // self.notification_service.send_push(...).await?;

        Ok(())
    }

    /// Send email notification (delegates to NotificationService)
    async fn send_email_notification(&self, delivery: &AlertDeliveryQueue) -> Result<()> {
        info!(
            "Sending email notification to {} for alert {}",
            delivery.recipient_id, delivery.alert_id
        );

        // TODO: Integrate with actual notification service
        // self.notification_service.send_email(...).await?;

        Ok(())
    }

    /// Send SMS notification (delegates to NotificationService)
    async fn send_sms_notification(&self, delivery: &AlertDeliveryQueue) -> Result<()> {
        info!(
            "Sending SMS notification to {} for alert {}",
            delivery.recipient_id, delivery.alert_id
        );

        // TODO: Integrate with actual notification service
        // self.notification_service.send_sms(...).await?;

        Ok(())
    }
}

/// Internal counter for method statistics
#[derive(Default)]
struct MethodCounter {
    attempted: usize,
    successful: usize,
    failed: usize,
}

impl MethodCounter {
    fn to_stats(&self) -> MethodStats {
        let success_rate = if self.attempted > 0 {
            (self.successful as f64 / self.attempted as f64) * 100.0
        } else {
            0.0
        };

        MethodStats {
            attempted: self.attempted,
            successful: self.successful,
            failed: self.failed,
            success_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_schedule() {
        // First attempt (0) - immediate
        let next = AlertDeliveryQueueService::calculate_next_retry(0);
        let now = Utc::now();
        assert!((next - now).num_seconds() < 5);

        // Second attempt (1) - 5 minutes
        let next = AlertDeliveryQueueService::calculate_next_retry(1);
        let expected = now + Duration::minutes(5);
        assert!((next - expected).num_seconds().abs() < 5);

        // Third attempt (2) - 15 minutes
        let next = AlertDeliveryQueueService::calculate_next_retry(2);
        let expected = now + Duration::minutes(15);
        assert!((next - expected).num_seconds().abs() < 5);

        // Fourth attempt (3) - 60 minutes (capped)
        let next = AlertDeliveryQueueService::calculate_next_retry(3);
        let expected = now + Duration::minutes(60);
        assert!((next - expected).num_seconds().abs() < 5);

        // Beyond max attempts - still 60 minutes
        let next = AlertDeliveryQueueService::calculate_next_retry(10);
        let expected = now + Duration::minutes(60);
        assert!((next - expected).num_seconds().abs() < 5);
    }
}
