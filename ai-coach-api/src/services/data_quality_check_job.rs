use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::{DataQualityMetrics, DataSource, JobExecutionStats, MissingDataReport, UrgencyLevel};

/// Data quality check job
///
/// Runs daily to analyze user data quality metrics including:
/// - Completeness: Percentage of expected data present
/// - Consistency: Data pattern regularity and anomaly detection
/// - Reliability: Source quality and data freshness
pub struct DataQualityCheckJob {
    db: PgPool,
}

impl DataQualityCheckJob {
    /// Create new data quality check job
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Execute data quality check for all users
    ///
    /// Processes users in batches to avoid overwhelming database
    pub async fn execute(&self) -> Result<JobExecutionStats> {
        info!("Starting data quality check job");
        let start_time = Utc::now();

        let mut processed = 0;
        let mut errors = 0;

        // Get all active users
        let users = self.get_active_users().await
            .context("Failed to fetch active users")?;

        info!("Found {} active users to analyze", users.len());

        // Process users in batches
        for user_id in users {
            match self.process_user_data_quality(user_id).await {
                Ok(_) => processed += 1,
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
            status: if errors == 0 { "success".to_string() } else { "completed_with_errors".to_string() },
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
    async fn process_user_data_quality(&self, user_id: Uuid) -> Result<()> {
        const ANALYSIS_DAYS: i32 = 30;

        // Calculate all quality scores
        let completeness = self.calculate_completeness_score(user_id, ANALYSIS_DAYS).await?;
        let consistency = self.calculate_consistency_score(user_id, ANALYSIS_DAYS).await?;
        let reliability = self.calculate_reliability_score(user_id, ANALYSIS_DAYS).await?;

        // Calculate overall score (weighted average)
        let overall = (completeness * 0.4) + (consistency * 0.3) + (reliability * 0.3);

        // Get missing data details
        let missing_data = self.detect_missing_data(user_id, ANALYSIS_DAYS).await?;

        // Check wearable connection status
        let wearable_connected = self.check_wearable_connected(user_id).await?;

        // Upsert metrics record
        sqlx::query!(
            r#"
            INSERT INTO data_quality_metrics (
                user_id,
                completeness_score,
                consistency_score,
                reliability_score,
                overall_score,
                days_analyzed,
                missing_hrv_days,
                missing_sleep_days,
                missing_rhr_days,
                last_hrv_reading,
                last_sleep_reading,
                last_rhr_reading,
                wearable_connected
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (user_id)
            DO UPDATE SET
                completeness_score = EXCLUDED.completeness_score,
                consistency_score = EXCLUDED.consistency_score,
                reliability_score = EXCLUDED.reliability_score,
                overall_score = EXCLUDED.overall_score,
                days_analyzed = EXCLUDED.days_analyzed,
                missing_hrv_days = EXCLUDED.missing_hrv_days,
                missing_sleep_days = EXCLUDED.missing_sleep_days,
                missing_rhr_days = EXCLUDED.missing_rhr_days,
                last_hrv_reading = EXCLUDED.last_hrv_reading,
                last_sleep_reading = EXCLUDED.last_sleep_reading,
                last_rhr_reading = EXCLUDED.last_rhr_reading,
                wearable_connected = EXCLUDED.wearable_connected,
                updated_at = NOW()
            "#,
            user_id,
            completeness,
            consistency,
            reliability,
            overall,
            ANALYSIS_DAYS,
            missing_data.missing_hrv_days,
            missing_data.missing_sleep_days,
            missing_data.missing_rhr_days,
            missing_data.last_hrv_reading,
            missing_data.last_sleep_reading,
            missing_data.last_rhr_reading,
            wearable_connected
        )
        .execute(&self.db)
        .await
        .context("Failed to upsert data quality metrics")?;

        // Log warning if quality is poor
        if overall < 0.5 {
            warn!(
                "User {} has poor data quality: overall={:.2}, completeness={:.2}, consistency={:.2}, reliability={:.2}",
                user_id, overall, completeness, consistency, reliability
            );
        }

        Ok(())
    }

    /// Calculate completeness score (0.0 to 1.0)
    ///
    /// Measures percentage of expected recovery data present in the analysis period
    async fn calculate_completeness_score(&self, user_id: Uuid, days: i32) -> Result<f64> {
        let start_date = (Utc::now() - Duration::days(days as i64)).date_naive();

        // Count days with at least one recovery metric
        let complete_days = sqlx::query_scalar!(
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
        .context("Failed to count complete days")?;

        // Completeness = days with data / expected days
        let score = complete_days as f64 / days as f64;
        Ok(score.min(1.0))
    }

    /// Calculate consistency score (0.0 to 1.0)
    ///
    /// Measures regularity of data submission and absence of long gaps
    async fn calculate_consistency_score(&self, user_id: Uuid, days: i32) -> Result<f64> {
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
            return Ok(0.0);
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
        let avg_gap_score = 1.0 - (avg_gap / 7.0).min(1.0); // 7+ day average gap = 0 score
        let max_gap_score = 1.0 - max_gap_penalty;

        // Weighted: 60% average consistency, 40% no long gaps
        let score = (avg_gap_score * 0.6) + (max_gap_score * 0.4);
        Ok(score.max(0.0))
    }

    /// Calculate reliability score (0.0 to 1.0)
    ///
    /// Measures data source quality and freshness
    async fn calculate_reliability_score(&self, user_id: Uuid, days: i32) -> Result<f64> {
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
            return Ok(0.0);
        }

        // Calculate weighted average reliability based on source quality
        let mut total_records = 0i64;
        let mut weighted_reliability = 0.0;

        for row in source_counts {
            let count = row.count.unwrap_or(0);
            total_records += count;

            let source_reliability = match row.data_source.as_deref() {
                Some("api_integration") => DataSource::ApiIntegration.reliability_score(),
                Some("wearable") => DataSource::Wearable.reliability_score(),
                Some("manual") => DataSource::Manual.reliability_score(),
                _ => 0.5, // Unknown source
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

        Ok(base_reliability * freshness_multiplier)
    }

    /// Detect missing data for a user
    async fn detect_missing_data(&self, user_id: Uuid, days: i32) -> Result<MissingDataReport> {
        let start_date = (Utc::now() - Duration::days(days as i64)).date_naive();

        // Count missing days for each metric
        let missing_hrv = self.count_missing_days(user_id, start_date, "hrv_score").await?;
        let missing_sleep = self.count_missing_days(user_id, start_date, "sleep_score").await?;
        let missing_rhr = self.count_missing_days(user_id, start_date, "resting_hr").await?;

        // Get last reading timestamps
        let (last_hrv, last_sleep, last_rhr) = self.get_last_readings(user_id).await?;

        // Calculate days since last reading
        let days_since = if let Some(latest) = [last_hrv, last_sleep, last_rhr].iter().filter_map(|d| *d).max() {
            (Utc::now() - latest).num_days() as i32
        } else {
            days
        };

        // Determine urgency level based on most missing metric
        let max_missing = missing_hrv.max(missing_sleep).max(missing_rhr);
        let urgency_level = UrgencyLevel::from_days_missing(max_missing);

        // Check wearable connection
        let wearable_connected = self.check_wearable_connected(user_id).await?;

        Ok(MissingDataReport {
            user_id,
            missing_hrv_days: missing_hrv,
            missing_sleep_days: missing_sleep,
            missing_rhr_days: missing_rhr,
            last_hrv_reading: last_hrv,
            last_sleep_reading: last_sleep,
            last_rhr_reading: last_rhr,
            wearable_connected,
            days_since_last_reading: days_since,
            urgency_level,
        })
    }

    /// Count days with missing data for a specific metric
    async fn count_missing_days(&self, user_id: Uuid, start_date: chrono::NaiveDate, metric: &str) -> Result<i32> {
        let days_with_data = sqlx::query_scalar!(
            r#"
            SELECT COUNT(DISTINCT date) as "count!"
            FROM recovery_data
            WHERE user_id = $1
                AND date >= $2
                AND CASE $3
                    WHEN 'hrv_score' THEN hrv_score IS NOT NULL
                    WHEN 'sleep_score' THEN sleep_score IS NOT NULL
                    WHEN 'resting_hr' THEN resting_hr IS NOT NULL
                    ELSE false
                END
            "#,
            user_id,
            start_date,
            metric
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to count days with data")?;

        let expected_days = (Utc::now().date_naive() - start_date).num_days() as i32 + 1;
        let missing = expected_days - days_with_data;
        Ok(missing.max(0))
    }

    /// Get last reading timestamps for each metric
    async fn get_last_readings(&self, user_id: Uuid) -> Result<(Option<DateTime<Utc>>, Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
        let result = sqlx::query!(
            r#"
            SELECT
                MAX(CASE WHEN hrv_score IS NOT NULL THEN created_at END) as last_hrv,
                MAX(CASE WHEN sleep_score IS NOT NULL THEN created_at END) as last_sleep,
                MAX(CASE WHEN resting_hr IS NOT NULL THEN created_at END) as last_rhr
            FROM recovery_data
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch last readings")?;

        Ok((result.last_hrv, result.last_sleep, result.last_rhr))
    }

    /// Check if user has connected wearable device
    async fn check_wearable_connected(&self, user_id: Uuid) -> Result<bool> {
        // Check if user has any Oura token (indicates wearable connection)
        let has_token = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM users
                WHERE id = $1 AND oura_access_token IS NOT NULL
            ) as "connected!"
            "#,
            user_id
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to check wearable connection")?;

        Ok(has_token)
    }

    /// Get job schedule (runs daily at 2 AM)
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
        // Test scoring logic
        let days = 30;
        let complete_days = 25;
        let score = complete_days as f64 / days as f64;
        assert!((score - 0.833).abs() < 0.01);
    }

    #[test]
    fn test_consistency_scoring() {
        // Test gap penalty logic
        let max_gap = 7;
        let days = 30;
        let max_gap_penalty = (max_gap as f64 / days as f64).min(1.0);
        assert!((max_gap_penalty - 0.233).abs() < 0.01);
    }
}
