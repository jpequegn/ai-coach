use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::{AlertDeliveryQueue, DeliveryMethod, DeliveryQueueStatus};

/// Email batching configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmailBatchConfig {
    /// Enable/disable email batching
    pub enabled: bool,
    /// Batch window in minutes (default: 15)
    pub window_minutes: i64,
    /// Maximum emails per batch (default: 10)
    pub max_batch_size: usize,
}

impl Default for EmailBatchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            window_minutes: 15,
            max_batch_size: 10,
        }
    }
}

/// Batched email delivery information
#[derive(Debug, Clone, Serialize)]
pub struct EmailBatch {
    pub recipient_id: String,
    pub delivery_ids: Vec<Uuid>,
    pub alert_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Email batching service
///
/// Groups multiple email notifications for the same recipient within a time window,
/// reducing the total number of emails sent while maintaining timely delivery.
pub struct EmailBatchingService {
    db: PgPool,
    config: EmailBatchConfig,
}

impl EmailBatchingService {
    /// Create a new email batching service
    pub fn new(db: PgPool, config: EmailBatchConfig) -> Self {
        Self { db, config }
    }

    /// Create with default configuration
    pub fn with_defaults(db: PgPool) -> Self {
        Self::new(db, EmailBatchConfig::default())
    }

    /// Get pending email deliveries eligible for batching
    ///
    /// Returns deliveries where:
    /// - Delivery method is Email
    /// - Status is pending
    /// - Next retry time is due
    /// - Created within the batch window
    pub async fn get_batchable_emails(&self) -> Result<Vec<AlertDeliveryQueue>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let window_start = Utc::now() - Duration::minutes(self.config.window_minutes);

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
            WHERE delivery_method = 'email'
                AND status = 'pending'
                AND (next_retry_at IS NULL OR next_retry_at <= NOW())
                AND attempts < max_attempts
                AND created_at >= $1
            ORDER BY recipient_id, created_at ASC
            "#,
            window_start
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch batchable emails")?;

        Ok(deliveries)
    }

    /// Group deliveries into batches by recipient
    ///
    /// Groups deliveries by recipient_id and applies max_batch_size constraint.
    /// Returns a map of recipient_id -> EmailBatch.
    pub fn group_into_batches(
        &self,
        deliveries: Vec<AlertDeliveryQueue>,
    ) -> HashMap<String, EmailBatch> {
        let mut batches: HashMap<String, Vec<AlertDeliveryQueue>> = HashMap::new();

        // Group by recipient
        for delivery in deliveries {
            batches
                .entry(delivery.recipient_id.clone())
                .or_default()
                .push(delivery);
        }

        // Convert to EmailBatch with max_batch_size limit
        let mut result = HashMap::new();
        for (recipient_id, mut deliveries) in batches {
            // Sort by created_at to process oldest first
            deliveries.sort_by_key(|d| d.created_at);

            // Take only up to max_batch_size
            let batch_deliveries: Vec<_> = deliveries
                .into_iter()
                .take(self.config.max_batch_size)
                .collect();

            if !batch_deliveries.is_empty() {
                let delivery_ids: Vec<Uuid> = batch_deliveries.iter().map(|d| d.id).collect();
                let alert_ids: Vec<Uuid> = batch_deliveries.iter().map(|d| d.alert_id).collect();
                let created_at = batch_deliveries[0].created_at;

                result.insert(
                    recipient_id.clone(),
                    EmailBatch {
                        recipient_id,
                        delivery_ids,
                        alert_ids,
                        created_at,
                    },
                );
            }
        }

        result
    }

    /// Process email batches
    ///
    /// Groups pending emails by recipient and returns batches ready for delivery.
    /// This allows the caller to send a single digest email per recipient instead
    /// of individual emails for each alert.
    ///
    /// # Returns
    /// A map of recipient_id -> EmailBatch for processing
    pub async fn process_batches(&self) -> Result<HashMap<String, EmailBatch>> {
        if !self.config.enabled {
            info!("Email batching is disabled");
            return Ok(HashMap::new());
        }

        let deliveries = self.get_batchable_emails().await?;

        if deliveries.is_empty() {
            return Ok(HashMap::new());
        }

        info!(
            "Found {} email deliveries eligible for batching",
            deliveries.len()
        );

        let batches = self.group_into_batches(deliveries);

        info!(
            "Created {} batches from {} deliveries (avg {:.1} emails per batch)",
            batches.len(),
            batches.values().map(|b| b.delivery_ids.len()).sum::<usize>(),
            batches.values().map(|b| b.delivery_ids.len()).sum::<usize>() as f64
                / batches.len().max(1) as f64
        );

        // Calculate reduction percentage
        let total_deliveries: usize = batches.values().map(|b| b.delivery_ids.len()).sum();
        let total_batches = batches.len();
        if total_deliveries > total_batches {
            let reduction_pct =
                ((total_deliveries - total_batches) as f64 / total_deliveries as f64) * 100.0;
            info!(
                "Email batching reduced {} deliveries to {} batches ({:.1}% reduction)",
                total_deliveries, total_batches, reduction_pct
            );
        }

        Ok(batches)
    }

    /// Mark batch deliveries as delivered
    ///
    /// Updates all deliveries in a batch to delivered status.
    /// Should be called after successfully sending the batch email.
    pub async fn mark_batch_delivered(&self, batch: &EmailBatch) -> Result<()> {
        let delivery_ids: Vec<Uuid> = batch.delivery_ids.clone();

        let result = sqlx::query!(
            r#"
            UPDATE alert_delivery_queue
            SET status = 'delivered',
                delivered_at = NOW(),
                updated_at = NOW()
            WHERE id = ANY($1)
            "#,
            &delivery_ids
        )
        .execute(&self.db)
        .await
        .context("Failed to mark batch as delivered")?;

        info!(
            "Marked {} deliveries as delivered for recipient {}",
            result.rows_affected(),
            batch.recipient_id
        );

        Ok(())
    }

    /// Get batching statistics
    ///
    /// Returns statistics about email batching effectiveness over a time period.
    pub async fn get_batching_stats(&self, hours: i64) -> Result<BatchingStats> {
        let start_time = Utc::now() - Duration::hours(hours);

        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT recipient_id) as unique_recipients,
                COUNT(*) as total_deliveries
            FROM alert_delivery_queue
            WHERE delivery_method = 'email'
                AND status = 'delivered'
                AND created_at >= $1
            "#,
            start_time
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch batching statistics")?;

        let unique_recipients = stats.unique_recipients.unwrap_or(0);
        let total_deliveries = stats.total_deliveries.unwrap_or(0);

        let potential_reduction = if unique_recipients > 0 && total_deliveries > unique_recipients
        {
            ((total_deliveries - unique_recipients) as f64 / total_deliveries as f64) * 100.0
        } else {
            0.0
        };

        Ok(BatchingStats {
            time_period_hours: hours,
            unique_recipients,
            total_deliveries,
            potential_reduction_pct: potential_reduction,
        })
    }
}

/// Email batching statistics
#[derive(Debug, Clone, Serialize)]
pub struct BatchingStats {
    pub time_period_hours: i64,
    pub unique_recipients: i64,
    pub total_deliveries: i64,
    pub potential_reduction_pct: f64,
}

impl Clone for EmailBatchingService {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EmailBatchConfig::default();
        assert!(config.enabled);
        assert_eq!(config.window_minutes, 15);
        assert_eq!(config.max_batch_size, 10);
    }

    #[test]
    fn test_batch_grouping() {
        // This would be tested with integration tests using actual database
        // Unit test here just verifies the structure compiles
        let config = EmailBatchConfig::default();
        assert!(config.enabled);
    }
}
