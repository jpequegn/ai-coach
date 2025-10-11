use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::{DataQualityMetrics, DataSourceStatus, JobExecutionStats, MissingDataReport};
use crate::services::NotificationService;

/// Data quality check job
///
/// Runs daily to analyze user data quality metrics including:
/// - Completeness: Percentage of days with recovery data
/// - Consistency: Day-to-day data variation patterns
/// - Reliability: Data source quality assessment
/// - Missing data detection and reminders
pub struct DataQualityCheckJob {
    db: PgPool,
    notification_service: NotificationService,
}

impl DataQualityCheckJob {
    /// Create new data quality check job
    pub fn new(db: PgPool, notification_service: NotificationService) -> Self {
        Self {
            db,
            notification_service,
        }
    }

    /// Execute data quality check for all active users
    ///
    /// Processes users and stores daily metrics in data_quality_metrics table
    pub async fn execute(&self) -> Result<JobExecutionStats> {
        info!("Starting data quality check job");
        let start_time = Utc::now();

        let mut processed = 0;
        let mut errors = 0;

        // Get all active users (with recovery data in last 60 days)
        let users = self.get_active_users().await
            .context("Failed to fetch active users")?;

        info!("Found {} active users to analyze", users.len());

        // Process users
        for user_id in users {
            match self.process_user_data_quality(user_id).await {
                Ok(needs_reminder) => {
                    processed += 1;

                    // Send reminder if needed
                    if needs_reminder {
                        if let Err(e) = self.send_data_quality_reminder(user_id).await {
                            warn!("Failed to send reminder to user {}: {}", user_id, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to process data quality for user {}: {}", user_id, e);
                    errors += 1;
                }
            }
        }

        let duration = (Utc::now() - start_time).num_seconds();

        info!(
            "Data quality check completed: {} users processed, {} errors in {} seconds",
            processed, errors, duration
        );

        Ok(JobExecutionStats {
            job_name: Self::get_job_name().to_string(),
            execution_time: start_time,
            duration_seconds: duration,
            records_processed: processed,
            records_failed: errors,
            status: if errors == 0 {
                "success".to_string()
            } else {
                "completed_with_errors".to_string()
            },
            error_message: if errors > 0 {
                Some(format!("{} users failed data quality check", errors))
            } else {
                None
            },
        })
    }

    /// Get all active users (with recovery data in last 60 days)
    async fn get_active_users(&self) -> Result<Vec<Uuid>> {
        let cutoff_date = Utc::now() - Duration::days(60);

        let users = sqlx::query_scalar!(
            r#"
            SELECT DISTINCT user_id
            FROM recovery_data
            WHERE date >= $1
            ORDER BY user_id
            "#,
            cutoff_date.date_naive()
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch active users")?;

        Ok(users)
    }

    /// Process data quality metrics for a single user
    ///
    /// Returns true if user needs a reminder
    async fn process_user_data_quality(&self, user_id: Uuid) -> Result<bool> {
        const ANALYSIS_DAYS: i32 = 30;
        let today = Utc::now().date_naive();

        // Calculate all quality scores
        let completeness = self
            .calculate_completeness_score(user_id, ANALYSIS_DAYS)
            .await?;
        let consistency = self
            .calculate_consistency_score(user_id, ANALYSIS_DAYS)
            .await?;
        let reliability = self
            .calculate_reliability_score(user_id, ANALYSIS_DAYS)
            .await?;

        // Get last data timestamp and days without data
        let (last_data_timestamp, days_without_data) =
            self.get_last_data_info(user_id).await?;

        // Get data sources status
        let data_sources = self.get_data_sources_status(user_id).await?;

        // Upsert daily metrics record
        sqlx::query!(
            r#"
            INSERT INTO data_quality_metrics (
                user_id,
                metric_date,
                completeness_score,
                consistency_score,
                reliability_score,
                last_data_timestamp,
                days_without_data,
                data_sources
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (user_id, metric_date)
            DO UPDATE SET
                completeness_score = EXCLUDED.completeness_score,
                consistency_score = EXCLUDED.consistency_score,
                reliability_score = EXCLUDED.reliability_score,
                last_data_timestamp = EXCLUDED.last_data_timestamp,
                days_without_data = EXCLUDED.days_without_data,
                data_sources = EXCLUDED.data_sources,
                updated_at = NOW()
            "#,
            user_id,
            today,
            completeness,
            consistency,
            reliability,
            last_data_timestamp,
            days_without_data,
            data_sources
        )
        .execute(&self.db)
        .await
        .context("Failed to upsert data quality metrics")?;

        // Determine if reminder needed
        let needs_reminder = days_without_data >= 3 || completeness < 50.0;

        // Log warning if quality is poor
        if completeness < 50.0 {
            warn!(
                "User {} has poor data completeness: {:.1}% (last data: {} days ago)",
                user_id, completeness, days_without_data
            );
        }

        Ok(needs_reminder)
    }

    /// Calculate completeness score (0-100%)
    ///
    /// Percentage of days with at least one recovery metric in the analysis period
    async fn calculate_completeness_score(&self, user_id: Uuid, days: i32) -> Result<f64> {
        let start_date = (Utc::now() - Duration::days(days as i64)).date_naive();

        // Count days with at least one recovery metric
        let days_with_data = sqlx::query_scalar!(
            r#"
            SELECT COUNT(DISTINCT date) as "count!"
            FROM recovery_data
            WHERE user_id = $1
                AND date >= $2
                AND (hrv_score IS NOT NULL OR sleep_score IS NOT NULL OR resting_hr IS NOT NULL)
            "#,
            user_id,
            start_date
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to count days with data")?;

        // Completeness = (days with data / expected days) * 100
        let score = (days_with_data as f64 / days as f64) * 100.0;
        Ok(score.min(100.0))
    }

    /// Calculate consistency score (0-100%)
    ///
    /// Measures regularity of data submission - penalizes gaps
    async fn calculate_consistency_score(&self, user_id: Uuid, days: i32) -> Result<Option<f64>> {
        let start_date = (Utc::now() - Duration::days(days as i64)).date_naive();

        // Get all dates with data
        let dates = sqlx::query_scalar!(
            r#"
            SELECT DISTINCT date
            FROM recovery_data
            WHERE user_id = $1
                AND date >= $2
            ORDER BY date ASC
            "#,
            user_id,
            start_date
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch recovery dates")?;

        if dates.is_empty() {
            return Ok(None);
        }

        // Calculate gaps between consecutive data points
        let mut max_gap = 0;
        let mut total_gap = 0;
        let mut gap_count = 0;

        for window in dates.windows(2) {
            let gap = (window[1] - window[0]).num_days() as i32 - 1;
            if gap > 0 {
                max_gap = max_gap.max(gap);
                total_gap += gap;
                gap_count += 1;
            }
        }

        // Penalize large gaps heavily
        let max_gap_penalty = (max_gap as f64 / days as f64).min(1.0);

        // Calculate average gap
        let avg_gap = if gap_count > 0 {
            total_gap as f64 / gap_count as f64
        } else {
            0.0
        };

        // Good consistency = small gaps
        let avg_gap_score = 100.0 - ((avg_gap / 7.0).min(1.0) * 100.0); // 7+ day avg gap = 0
        let max_gap_score = 100.0 - (max_gap_penalty * 100.0);

        // Weighted: 60% average consistency, 40% no long gaps
        let score = (avg_gap_score * 0.6) + (max_gap_score * 0.4);
        Ok(Some(score.max(0.0)))
    }

    /// Calculate reliability score (0-100%)
    ///
    /// Based on data source quality and freshness
    async fn calculate_reliability_score(&self, user_id: Uuid, days: i32) -> Result<Option<f64>> {
        let start_date = (Utc::now() - Duration::days(days as i64)).date_naive();

        // Get data source breakdown
        let source_counts = sqlx::query!(
            r#"
            SELECT
                data_source,
                COUNT(*) as count
            FROM recovery_data
            WHERE user_id = $1
                AND date >= $2
            GROUP BY data_source
            "#,
            user_id,
            start_date
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch data sources")?;

        if source_counts.is_empty() {
            return Ok(None);
        }

        // Calculate weighted average reliability based on source quality
        let mut total_records = 0i64;
        let mut weighted_reliability = 0.0;

        for row in source_counts {
            let count = row.count.unwrap_or(0);
            total_records += count;

            // Source reliability weights
            let source_reliability = match row.data_source.as_deref() {
                Some("api_integration") => 95.0,
                Some("wearable") => 90.0,
                Some("manual") => 70.0,
                _ => 50.0, // Unknown source
            };

            weighted_reliability += (count as f64) * source_reliability;
        }

        let base_reliability = weighted_reliability / total_records as f64;

        // Check data freshness (penalize if no data in last 3 days)
        let latest_data = sqlx::query_scalar!(
            r#"
            SELECT MAX(date) as "latest"
            FROM recovery_data
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch latest data date")?;

        let freshness_multiplier = if let Some(latest) = latest_data {
            let days_since = (Utc::now().date_naive() - latest).num_days();
            if days_since <= 1 {
                1.0
            } else if days_since <= 3 {
                0.9
            } else if days_since <= 7 {
                0.7
            } else {
                0.5
            }
        } else {
            0.0
        };

        Ok(Some(base_reliability * freshness_multiplier))
    }

    /// Get last data timestamp and consecutive days without data
    async fn get_last_data_info(&self, user_id: Uuid) -> Result<(Option<DateTime<Utc>>, i32)> {
        let last_data = sqlx::query!(
            r#"
            SELECT MAX(created_at) as last_timestamp, MAX(date) as last_date
            FROM recovery_data
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch last data info")?;

        let days_without_data = if let Some(last_date) = last_data.last_date {
            (Utc::now().date_naive() - last_date).num_days() as i32
        } else {
            365 // No data at all
        };

        Ok((last_data.last_timestamp, days_without_data.max(0)))
    }

    /// Get data sources status as JSON
    async fn get_data_sources_status(&self, user_id: Uuid) -> Result<Option<serde_json::Value>> {
        // Check which data sources have recent data (last 7 days)
        let recent_cutoff = (Utc::now() - Duration::days(7)).date_naive();

        let sources = sqlx::query!(
            r#"
            SELECT
                BOOL_OR(hrv_score IS NOT NULL) as has_hrv,
                BOOL_OR(sleep_score IS NOT NULL) as has_sleep,
                BOOL_OR(resting_hr IS NOT NULL) as has_rhr,
                MAX(created_at) as last_sync
            FROM recovery_data
            WHERE user_id = $1
                AND date >= $2
            "#,
            user_id,
            recent_cutoff
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch data sources")?;

        // Check wearable connection
        let wearable_connected = sqlx::query_scalar!(
            r#"
            SELECT oura_access_token IS NOT NULL as "connected!"
            FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to check wearable connection")?;

        let status = DataSourceStatus {
            hrv: sources.has_hrv.unwrap_or(false),
            sleep: sources.has_sleep.unwrap_or(false),
            rhr: sources.has_rhr.unwrap_or(false),
            training: false, // TODO: Check training sessions table
            wearable_connected,
            last_sync: sources.last_sync,
        };

        Ok(Some(status.to_json()))
    }

    /// Send data quality reminder to user
    async fn send_data_quality_reminder(&self, user_id: Uuid) -> Result<()> {
        // Get user email for notification
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

        // Send push notification if enabled
        if user.push_notifications_enabled.unwrap_or(false) {
            self.notification_service
                .send_push_notification(
                    user_id,
                    "Update your recovery data".to_string(),
                    "Log your recovery data to get personalized insights".to_string(),
                )
                .await
                .ok(); // Don't fail job if notification fails
        }

        // Send email if enabled
        if user.email_notifications_enabled.unwrap_or(false) {
            self.notification_service
                .send_email_notification(
                    user.email,
                    "Recovery Data Reminder".to_string(),
                    "We noticed you haven't logged recovery data recently. Update your data to continue receiving personalized coaching insights.".to_string(),
                )
                .await
                .ok(); // Don't fail job if notification fails
        }

        Ok(())
    }

    /// Get job schedule (runs daily at 2 AM UTC)
    pub fn get_schedule() -> &'static str {
        "0 2 * * *"
    }

    /// Get job name
    pub fn get_job_name() -> &'static str {
        "data_quality_check"
    }
}

impl Clone for DataQualityCheckJob {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            notification_service: self.notification_service.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_metadata() {
        assert_eq!(DataQualityCheckJob::get_job_name(), "data_quality_check");
        assert_eq!(DataQualityCheckJob::get_schedule(), "0 2 * * *");
    }

    #[test]
    fn test_completeness_calculation() {
        // Test scoring logic (0-100% scale)
        let days = 30;
        let days_with_data = 25;
        let score = (days_with_data as f64 / days as f64) * 100.0;
        assert!((score - 83.33).abs() < 0.01);
    }

    #[test]
    fn test_consistency_scoring() {
        // Test gap penalty logic (0-100% scale)
        let max_gap = 7;
        let days = 30;
        let max_gap_penalty = (max_gap as f64 / days as f64).min(1.0);
        let max_gap_score = 100.0 - (max_gap_penalty * 100.0);
        assert!((max_gap_score - 76.67).abs() < 0.01);
    }
}
