use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use std::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::{BaselineChange, ChangeSignificance, JobExecutionStats, RecoveryBaseline};
use crate::services::{Job, NotificationService, RecoveryDataService};

const BATCH_SIZE: usize = 50;
const BATCH_DELAY_MS: u64 = 100;

/// Weekly baseline recalculation job
///
/// Runs weekly to recalculate recovery baselines for all active users,
/// detect significant changes, and notify users of important shifts.
pub struct WeeklyBaselineRecalculationJob {
    db: PgPool,
    recovery_data_service: RecoveryDataService,
    notification_service: NotificationService,
}

impl WeeklyBaselineRecalculationJob {
    /// Create new weekly baseline recalculation job
    pub fn new(
        db: PgPool,
        recovery_data_service: RecoveryDataService,
        notification_service: NotificationService,
    ) -> Self {
        Self {
            db,
            recovery_data_service,
            notification_service,
        }
    }

    /// Execute the weekly baseline recalculation job
    pub async fn execute(&self) -> Result<JobExecutionStats> {
        let start_time = Instant::now();
        let mut stats = JobExecutionStats {
            records_processed: 0,
            records_failed: 0,
            execution_time_ms: 0,
            error_message: None,
            metadata: None,
        };

        info!("Starting weekly baseline recalculation job");

        // Get all active users with recovery data
        let users = match self.get_active_users().await {
            Ok(users) => users,
            Err(e) => {
                error!("Failed to fetch active users: {}", e);
                stats.error_message = Some(format!("Failed to fetch users: {}", e));
                stats.execution_time_ms = start_time.elapsed().as_millis() as i64;
                return Ok(stats);
            }
        };

        let total_users = users.len();
        info!("Found {} active users for baseline recalculation", total_users);

        if total_users == 0 {
            stats.execution_time_ms = start_time.elapsed().as_millis() as i64;
            return Ok(stats);
        }

        // Track baseline changes
        let mut total_changes = 0;
        let mut significant_changes = 0;

        // Process users in batches
        for (batch_num, batch) in users.chunks(BATCH_SIZE).enumerate() {
            info!(
                "Processing batch {}/{} ({} users)",
                batch_num + 1,
                (total_users + BATCH_SIZE - 1) / BATCH_SIZE,
                batch.len()
            );

            let batch_result = self.process_batch(batch, &mut stats).await;
            total_changes += batch_result.0;
            significant_changes += batch_result.1;

            // Add delay between batches to prevent database overload
            if batch_num < (total_users / BATCH_SIZE) {
                tokio::time::sleep(tokio::time::Duration::from_millis(BATCH_DELAY_MS)).await;
            }
        }

        stats.execution_time_ms = start_time.elapsed().as_millis() as i64;

        // Add metadata about changes detected
        stats.metadata = Some(serde_json::json!({
            "total_changes_detected": total_changes,
            "significant_changes": significant_changes,
            "notification_threshold": "moderate"
        }));

        info!(
            "Completed weekly baseline recalculation: {} users processed, {} failed, {} changes detected ({} significant), {}ms",
            stats.records_processed,
            stats.records_failed,
            total_changes,
            significant_changes,
            stats.execution_time_ms
        );

        Ok(stats)
    }

    /// Get all active users with recovery data in the last 60 days
    async fn get_active_users(&self) -> Result<Vec<Uuid>> {
        let cutoff_date = (Utc::now() - chrono::Duration::days(60)).date_naive();

        let users = sqlx::query_scalar!(
            r#"
            SELECT DISTINCT user_id
            FROM recovery_data
            WHERE date >= $1
            ORDER BY user_id
            "#,
            cutoff_date
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch active users")?;

        Ok(users)
    }

    /// Process a batch of users
    /// Returns (total_changes, significant_changes)
    async fn process_batch(
        &self,
        batch: &[Uuid],
        stats: &mut JobExecutionStats,
    ) -> (i32, i32) {
        let mut total_changes = 0;
        let mut significant_changes = 0;

        for &user_id in batch {
            match self.process_user(user_id).await {
                Ok(change_count) => {
                    stats.records_processed += 1;
                    if change_count > 0 {
                        total_changes += change_count;
                        significant_changes += 1;
                    }
                }
                Err(e) => {
                    stats.records_failed += 1;
                    error!("Failed to process user {}: {}", user_id, e);
                }
            }
        }

        (total_changes, significant_changes)
    }

    /// Process a single user - recalculate baseline and detect changes
    /// Returns count of significant changes detected
    async fn process_user(&self, user_id: Uuid) -> Result<i32> {
        // 1. Get current baseline
        let old_baseline = self.get_current_baseline(user_id).await?;

        // 2. Recalculate baseline
        self.recovery_data_service
            .calculate_baseline(user_id)
            .await
            .context("Failed to recalculate baseline")?;

        // 3. Get new baseline
        let new_baseline = self
            .get_current_baseline(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No baseline after recalculation"))?;

        // 4. Detect and handle changes
        let changes = if let Some(old) = old_baseline {
            self.detect_baseline_changes(&old, &new_baseline, user_id)
                .await?
        } else {
            // First baseline calculation - no changes to detect
            vec![]
        };

        // 5. Send notifications for significant changes
        let mut change_count = 0;
        for change in changes {
            if self.should_notify(&change) {
                if let Err(e) = self.notify_baseline_change(user_id, &change).await {
                    warn!(
                        "Failed to send notification for user {}: {}",
                        user_id, e
                    );
                }
                change_count += 1;
            }
        }

        Ok(change_count)
    }

    /// Get current baseline for a user
    async fn get_current_baseline(&self, user_id: Uuid) -> Result<Option<RecoveryBaseline>> {
        let baseline = sqlx::query_as!(
            RecoveryBaseline,
            r#"
            SELECT
                id, user_id,
                hrv_baseline_rmssd, rhr_baseline, typical_sleep_hours,
                calculated_at, data_points_count,
                created_at as "created_at!", updated_at as "updated_at!"
            FROM recovery_baselines
            WHERE user_id = $1
            ORDER BY calculated_at DESC
            LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch baseline")?;

        Ok(baseline)
    }

    /// Detect significant baseline changes between old and new baselines
    async fn detect_baseline_changes(
        &self,
        old: &RecoveryBaseline,
        new: &RecoveryBaseline,
        user_id: Uuid,
    ) -> Result<Vec<BaselineChange>> {
        let mut changes = Vec::new();

        // Check HRV baseline change
        if let (Some(old_hrv), Some(new_hrv)) = (old.hrv_baseline_rmssd, new.hrv_baseline_rmssd) {
            if old_hrv > 0.0 {
                let change = BaselineChange::new(
                    user_id,
                    "HRV".to_string(),
                    old_hrv,
                    new_hrv,
                );

                if change.significance != ChangeSignificance::Minor {
                    changes.push(change);
                }
            }
        }

        // Check RHR baseline change
        if let (Some(old_rhr), Some(new_rhr)) = (old.rhr_baseline, new.rhr_baseline) {
            if old_rhr > 0.0 {
                let change = BaselineChange::new(
                    user_id,
                    "RHR".to_string(),
                    old_rhr,
                    new_rhr,
                );

                if change.significance != ChangeSignificance::Minor {
                    changes.push(change);
                }
            }
        }

        // Check sleep duration baseline change
        if let (Some(old_sleep), Some(new_sleep)) = (old.typical_sleep_hours, new.typical_sleep_hours) {
            if old_sleep > 0.0 {
                let change = BaselineChange::new(
                    user_id,
                    "Sleep Duration".to_string(),
                    old_sleep,
                    new_sleep,
                );

                if change.significance != ChangeSignificance::Minor {
                    changes.push(change);
                }
            }
        }

        Ok(changes)
    }

    /// Determine if a change should trigger a notification
    fn should_notify(&self, change: &BaselineChange) -> bool {
        // Only notify for moderate and major changes
        matches!(
            change.significance,
            ChangeSignificance::Moderate | ChangeSignificance::Major
        )
    }

    /// Send notification about baseline change
    async fn notify_baseline_change(&self, user_id: Uuid, change: &BaselineChange) -> Result<()> {
        let (title, message) = self.generate_notification_content(change);

        // Get user's notification preferences
        let user = sqlx::query!(
            r#"
            SELECT email, push_notifications_enabled, email_notifications_enabled
            FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch user")?;

        // Send push notification if enabled and change is significant enough
        if user.push_notifications_enabled.unwrap_or(false) {
            if change.significance == ChangeSignificance::Major {
                self.notification_service
                    .send_push_notification(user_id, title.clone(), message.clone())
                    .await
                    .ok(); // Don't fail job if notification fails
            }
        }

        // Send email notification for major changes if enabled
        if user.email_notifications_enabled.unwrap_or(false) {
            if change.significance == ChangeSignificance::Major {
                self.notification_service
                    .send_email_notification(user.email, title, message)
                    .await
                    .ok(); // Don't fail job if notification fails
            }
        }

        Ok(())
    }

    /// Generate user-friendly notification content
    fn generate_notification_content(&self, change: &BaselineChange) -> (String, String) {
        let direction = if change.is_improvement() {
            "improved"
        } else {
            "changed"
        };

        let emoji = if change.is_improvement() {
            "📈"
        } else {
            "📊"
        };

        let title = format!(
            "{} Your {} baseline has {}",
            emoji, change.metric, direction
        );

        let percent_str = if change.percent_change > 0.0 {
            format!("+{:.1}%", change.percent_change)
        } else {
            format!("{:.1}%", change.percent_change)
        };

        let message = match change.metric.as_str() {
            "HRV" => {
                if change.is_improvement() {
                    format!(
                        "Great news! Your HRV baseline has increased by {} (from {:.1} to {:.1}ms). This suggests improved recovery and adaptation to training.",
                        percent_str, change.old_value, change.new_value
                    )
                } else {
                    format!(
                        "Your HRV baseline has decreased by {} (from {:.1} to {:.1}ms). Consider reviewing your training load and recovery practices.",
                        percent_str.replace('+', ""), change.old_value, change.new_value
                    )
                }
            }
            "RHR" => {
                if change.is_improvement() {
                    format!(
                        "Excellent! Your resting heart rate has decreased by {} (from {:.1} to {:.1} bpm). This indicates improved cardiovascular fitness.",
                        percent_str.replace('+', ""), change.old_value, change.new_value
                    )
                } else {
                    format!(
                        "Your resting heart rate has increased by {} (from {:.1} to {:.1} bpm). This could indicate fatigue or stress. Monitor your recovery carefully.",
                        percent_str, change.old_value, change.new_value
                    )
                }
            }
            "Sleep Duration" => {
                format!(
                    "Your typical sleep duration has changed by {} (from {:.1} to {:.1} hours). Consistent sleep patterns are important for recovery.",
                    percent_str, change.old_value, change.new_value
                )
            }
            _ => {
                format!(
                    "Your {} baseline has changed by {} (from {:.1} to {:.1}).",
                    change.metric, percent_str, change.old_value, change.new_value
                )
            }
        };

        (title, message)
    }

    /// Get job schedule (runs weekly on Sunday at 3 AM UTC)
    pub fn get_schedule() -> &'static str {
        "0 3 * * 0" // Sunday at 3 AM
    }

    /// Get job name
    pub fn get_job_name() -> &'static str {
        "weekly_baseline_recalculation"
    }
}

impl Clone for WeeklyBaselineRecalculationJob {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            recovery_data_service: self.recovery_data_service.clone(),
            notification_service: self.notification_service.clone(),
        }
    }
}

#[async_trait]
impl Job for WeeklyBaselineRecalculationJob {
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
    fn test_job_metadata() {
        assert_eq!(
            WeeklyBaselineRecalculationJob::get_job_name(),
            "weekly_baseline_recalculation"
        );
        assert_eq!(WeeklyBaselineRecalculationJob::get_schedule(), "0 3 * * 0");
    }

    #[test]
    fn test_should_notify() {
        let job = WeeklyBaselineRecalculationJob {
            db: PgPool::connect("").await.unwrap(), // Mock
            recovery_data_service: RecoveryDataService::new(PgPool::connect("").await.unwrap()),
            notification_service: NotificationService::new(PgPool::connect("").await.unwrap()),
        };

        // Minor changes should not notify
        let minor_change = BaselineChange {
            user_id: Uuid::new_v4(),
            metric: "HRV".to_string(),
            old_value: 50.0,
            new_value: 54.0, // 8% change
            percent_change: 8.0,
            significance: ChangeSignificance::Minor,
            detected_at: Utc::now(),
            explanation: None,
        };
        assert!(!job.should_notify(&minor_change));

        // Moderate changes should notify
        let moderate_change = BaselineChange {
            user_id: Uuid::new_v4(),
            metric: "HRV".to_string(),
            old_value: 50.0,
            new_value: 56.0, // 12% change
            percent_change: 12.0,
            significance: ChangeSignificance::Moderate,
            detected_at: Utc::now(),
            explanation: None,
        };
        assert!(job.should_notify(&moderate_change));

        // Major changes should notify
        let major_change = BaselineChange {
            user_id: Uuid::new_v4(),
            metric: "HRV".to_string(),
            old_value: 50.0,
            new_value: 61.0, // 22% change
            percent_change: 22.0,
            significance: ChangeSignificance::Major,
            detected_at: Utc::now(),
            explanation: None,
        };
        assert!(job.should_notify(&major_change));
    }
}
