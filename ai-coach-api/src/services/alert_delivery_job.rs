use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use std::time::Instant;
use tracing::{error, info, warn};

use crate::models::JobExecutionStats;
use crate::services::{AlertDeliveryQueueService, Job};

const HIGH_FAILURE_RATE_THRESHOLD: f64 = 0.20; // 20%

/// Background job that processes pending alert deliveries every 5 minutes
///
/// This job:
/// - Fetches pending deliveries from the queue
/// - Processes them in batches with rate limiting
/// - Tracks delivery success/failure rates
/// - Alerts on high failure rates (>20%)
/// - Returns execution statistics for monitoring
pub struct AlertDeliveryJob {
    queue_service: AlertDeliveryQueueService,
}

impl AlertDeliveryJob {
    /// Create a new alert delivery job
    ///
    /// # Arguments
    /// * `queue_service` - The alert delivery queue service for processing deliveries
    pub fn new(queue_service: AlertDeliveryQueueService) -> Self {
        Self { queue_service }
    }

    /// Execute the alert delivery job
    ///
    /// Processes all pending deliveries in the queue and returns execution statistics.
    /// Logs warnings if failure rate exceeds 20%.
    ///
    /// # Returns
    /// JobExecutionStats with delivery metrics
    pub async fn execute(&self) -> Result<JobExecutionStats> {
        let start_time = Instant::now();

        info!("Starting alert delivery job execution");

        // Process the pending queue
        let delivery_stats = self.queue_service.process_pending_queue().await?;

        let execution_time_ms = start_time.elapsed().as_millis() as i64;

        // Check for high failure rate
        let failure_rate = if delivery_stats.total_attempted > 0 {
            delivery_stats.failed as f64 / delivery_stats.total_attempted as f64
        } else {
            0.0
        };

        if failure_rate > HIGH_FAILURE_RATE_THRESHOLD && delivery_stats.total_attempted >= 10 {
            warn!(
                "High alert delivery failure rate: {:.1}% ({}/{} deliveries failed)",
                failure_rate * 100.0,
                delivery_stats.failed,
                delivery_stats.total_attempted
            );
        }

        // Convert delivery stats to job execution stats
        let job_stats = JobExecutionStats {
            records_processed: delivery_stats.successful as i32,
            records_failed: delivery_stats.failed as i32,
            execution_time_ms,
            error_message: None,
            metadata: Some(serde_json::json!({
                "total_attempted": delivery_stats.total_attempted,
                "successful": delivery_stats.successful,
                "failed": delivery_stats.failed,
                "retrying": delivery_stats.retrying,
                "failure_rate": format!("{:.2}%", failure_rate * 100.0),
                "avg_delivery_time_ms": delivery_stats.delivery_time_avg_ms,
                "by_method": delivery_stats.by_method,
                "high_failure_rate": failure_rate > HIGH_FAILURE_RATE_THRESHOLD
            })),
        };

        info!(
            "Alert delivery job completed: {} successful, {} failed, {} retrying ({}ms)",
            delivery_stats.successful,
            delivery_stats.failed,
            delivery_stats.retrying,
            execution_time_ms
        );

        Ok(job_stats)
    }

    /// Get the recommended cron schedule for this job
    ///
    /// # Returns
    /// Cron expression for every 5 minutes: "*/5 * * * *"
    pub fn get_schedule() -> &'static str {
        "*/5 * * * *" // Every 5 minutes
    }

    /// Get the job name for registration
    ///
    /// # Returns
    /// Job name identifier: "alert_delivery"
    pub fn get_job_name() -> &'static str {
        "alert_delivery"
    }
}

impl Clone for AlertDeliveryJob {
    fn clone(&self) -> Self {
        Self {
            queue_service: self.queue_service.clone(),
        }
    }
}

#[async_trait]
impl Job for AlertDeliveryJob {
    fn name(&self) -> &'static str {
        Self::get_job_name()
    }

    fn schedule(&self) -> &'static str {
        Self::get_schedule()
    }

    async fn execute(&self) -> Result<()> {
        self.execute().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_format() {
        let schedule = AlertDeliveryJob::get_schedule();
        assert_eq!(schedule, "*/5 * * * *");
    }

    #[test]
    fn test_job_name() {
        let name = AlertDeliveryJob::get_job_name();
        assert_eq!(name, "alert_delivery");
    }

    #[test]
    fn test_high_failure_rate_threshold() {
        assert_eq!(HIGH_FAILURE_RATE_THRESHOLD, 0.20);
    }
}
