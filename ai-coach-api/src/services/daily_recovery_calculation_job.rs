use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, Timelike, Utc};
use redis::{AsyncCommands, Client as RedisClient};
use sqlx::PgPool;
use std::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::models::{JobExecutionStats, RecoveryScore};
use crate::services::{Job, RecoveryAlertService, RecoveryAnalysisService};

const BATCH_SIZE: usize = 100;
const BATCH_DELAY_MS: u64 = 100;
const REDIS_CACHE_TTL_SECONDS: usize = 3600; // 1 hour
const HIGH_FAILURE_RATE_THRESHOLD: f64 = 0.05; // 5%

/// User information for batch processing
#[derive(Debug, Clone)]
struct UserForCalculation {
    id: Uuid,
    timezone: String,
}

/// Daily recovery calculation job
///
/// This job runs hourly and calculates recovery scores for all active users
/// whose local time is 6 AM. Results are cached in Redis and alerts are
/// automatically triggered if needed.
pub struct DailyRecoveryCalculationJob {
    db: PgPool,
    analysis_service: RecoveryAnalysisService,
    redis_client: Option<RedisClient>,
}

impl DailyRecoveryCalculationJob {
    pub fn new(
        db: PgPool,
        analysis_service: RecoveryAnalysisService,
        redis_url: Option<String>,
    ) -> Result<Self> {
        let redis_client = redis_url
            .map(|url| RedisClient::open(url))
            .transpose()
            .context("Failed to create Redis client")?;

        Ok(Self {
            db,
            analysis_service,
            redis_client,
        })
    }

    /// Execute the daily recovery calculation job
    pub async fn execute(&self) -> Result<JobExecutionStats> {
        let start_time = Instant::now();
        let mut stats = JobExecutionStats {
            records_processed: 0,
            records_failed: 0,
            execution_time_ms: 0,
            error_message: None,
            metadata: None,
        };

        info!("Starting daily recovery calculation job");

        // Get current UTC hour
        let current_hour = Utc::now().hour();

        // Get users who need calculation (6 AM in their timezone)
        let users = match self.get_users_for_calculation(current_hour).await {
            Ok(users) => users,
            Err(e) => {
                error!("Failed to fetch users for calculation: {}", e);
                stats.error_message = Some(format!("Failed to fetch users: {}", e));
                stats.execution_time_ms = start_time.elapsed().as_millis() as i64;
                return Ok(stats);
            }
        };

        let total_users = users.len();
        info!(
            "Found {} users for calculation at UTC hour {}",
            total_users, current_hour
        );

        if total_users == 0 {
            stats.execution_time_ms = start_time.elapsed().as_millis() as i64;
            return Ok(stats);
        }

        // Process users in batches
        for (batch_num, batch) in users.chunks(BATCH_SIZE).enumerate() {
            info!(
                "Processing batch {}/{} ({} users)",
                batch_num + 1,
                (total_users + BATCH_SIZE - 1) / BATCH_SIZE,
                batch.len()
            );

            self.process_batch(batch, &mut stats).await;

            // Add delay between batches to prevent database overload
            if batch_num < (total_users / BATCH_SIZE) {
                tokio::time::sleep(tokio::time::Duration::from_millis(BATCH_DELAY_MS)).await;
            }
        }

        stats.execution_time_ms = start_time.elapsed().as_millis() as i64;

        // Check for high failure rate
        let failure_rate = stats.records_failed as f64 / total_users as f64;
        if failure_rate > HIGH_FAILURE_RATE_THRESHOLD {
            warn!(
                "High failure rate detected: {:.2}% ({}/{})",
                failure_rate * 100.0,
                stats.records_failed,
                total_users
            );
            stats.metadata = Some(serde_json::json!({
                "high_failure_rate": true,
                "failure_rate": failure_rate,
                "threshold": HIGH_FAILURE_RATE_THRESHOLD
            }));
        }

        info!(
            "Completed daily recovery calculation: {} successful, {} failed, {}ms",
            stats.records_processed, stats.records_failed, stats.execution_time_ms
        );

        Ok(stats)
    }

    /// Get users who need calculation for the current hour
    async fn get_users_for_calculation(&self, current_utc_hour: u32) -> Result<Vec<UserForCalculation>> {
        // Calculate which timezones correspond to 6 AM local time
        // For example, if current UTC hour is 10, we want users in UTC+4 timezone (10 - 6 = 4)
        let target_hour = 6u32;
        let target_utc_offset = (current_utc_hour as i32 - target_hour as i32 + 24) % 24;

        // This is a simplified approach - in production, you'd want to use proper timezone library
        // For now, we'll fetch all active users and filter based on their timezone
        let users = sqlx::query_as!(
            UserForCalculation,
            r#"
            SELECT id, timezone
            FROM users
            WHERE active = true
            "#
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch active users")?;

        // TODO: Implement proper timezone filtering using chrono_tz or similar
        // For now, return all active users
        Ok(users)
    }

    /// Process a batch of users
    async fn process_batch(&self, batch: &[UserForCalculation], stats: &mut JobExecutionStats) {
        // Use tokio tasks for parallel processing within the batch
        let mut handles = Vec::new();

        for user in batch {
            let user_id = user.id;
            let analysis_service = self.analysis_service.clone();
            let redis_client = self.redis_client.clone();

            let handle = tokio::spawn(async move {
                Self::calculate_user_recovery_static(
                    user_id,
                    analysis_service,
                    redis_client,
                )
                .await
            });

            handles.push(handle);
        }

        // Collect results
        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => {
                    stats.records_processed += 1;
                }
                Ok(Err(e)) => {
                    stats.records_failed += 1;
                    error!("User recovery calculation failed: {}", e);
                }
                Err(e) => {
                    stats.records_failed += 1;
                    error!("Task join error: {}", e);
                }
            }
        }
    }

    /// Calculate recovery for a single user (static method for use in spawned tasks)
    async fn calculate_user_recovery_static(
        user_id: Uuid,
        analysis_service: RecoveryAnalysisService,
        redis_client: Option<RedisClient>,
    ) -> Result<()> {
        let target_date = Utc::now().date_naive();

        // 1. Calculate recovery score
        let score = analysis_service
            .calculate_daily_recovery(user_id, target_date)
            .await
            .context("Failed to calculate recovery score")?;

        // 2. Cache in Redis if available
        if let Some(client) = redis_client {
            if let Err(e) = Self::cache_recovery_score_static(client, user_id, &score).await {
                // Log error but don't fail the calculation
                warn!("Failed to cache recovery score for user {}: {}", user_id, e);
            }
        }

        // 3. Alert service is integrated in RecoveryAnalysisService
        // Alerts are automatically evaluated and sent

        Ok(())
    }

    /// Cache recovery score in Redis (static method)
    async fn cache_recovery_score_static(
        client: RedisClient,
        user_id: Uuid,
        score: &RecoveryScore,
    ) -> Result<()> {
        let mut conn = client
            .get_async_connection()
            .await
            .context("Failed to get Redis connection")?;

        let cache_key = format!("recovery_score:{}:{}", user_id, score.date);
        let score_json = serde_json::to_string(score)?;

        conn.set_ex(&cache_key, score_json, REDIS_CACHE_TTL_SECONDS)
            .await
            .context("Failed to set Redis cache")?;

        Ok(())
    }

    /// Manual trigger for specific user (for testing and error recovery)
    pub async fn trigger_for_user(&self, user_id: Uuid) -> Result<()> {
        info!("Manual trigger for user {}", user_id);

        let target_date = Utc::now().date_naive();

        Self::calculate_user_recovery_static(
            user_id,
            self.analysis_service.clone(),
            self.redis_client.clone(),
        )
        .await
    }

    /// Manual trigger for specific timezone (for testing)
    pub async fn trigger_for_timezone(&self, timezone: &str) -> Result<JobExecutionStats> {
        info!("Manual trigger for timezone {}", timezone);

        let start_time = Instant::now();
        let mut stats = JobExecutionStats {
            records_processed: 0,
            records_failed: 0,
            execution_time_ms: 0,
            error_message: None,
            metadata: Some(serde_json::json!({"timezone": timezone})),
        };

        let users = sqlx::query_as!(
            UserForCalculation,
            r#"
            SELECT id, timezone
            FROM users
            WHERE active = true AND timezone = $1
            "#,
            timezone
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch users for timezone")?;

        for batch in users.chunks(BATCH_SIZE) {
            self.process_batch(batch, &mut stats).await;
        }

        stats.execution_time_ms = start_time.elapsed().as_millis() as i64;
        Ok(stats)
    }

    /// Dry run mode - checks what would be processed without actually calculating
    pub async fn dry_run(&self) -> Result<serde_json::Value> {
        let current_hour = Utc::now().hour();
        let users = self.get_users_for_calculation(current_hour).await?;

        Ok(serde_json::json!({
            "current_utc_hour": current_hour,
            "total_users": users.len(),
            "users_by_timezone": self.group_users_by_timezone(&users)
        }))
    }

    /// Group users by timezone for reporting
    fn group_users_by_timezone(&self, users: &[UserForCalculation]) -> serde_json::Value {
        let mut timezone_counts = std::collections::HashMap::new();

        for user in users {
            *timezone_counts.entry(user.timezone.clone()).or_insert(0) += 1;
        }

        serde_json::to_value(timezone_counts).unwrap_or(serde_json::json!({}))
    }
}

impl Clone for DailyRecoveryCalculationJob {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            analysis_service: self.analysis_service.clone(),
            redis_client: self.redis_client.clone(),
        }
    }
}

#[async_trait]
impl Job for DailyRecoveryCalculationJob {
    fn name(&self) -> &'static str {
        "daily_recovery_calculation"
    }

    fn schedule(&self) -> &'static str {
        "0 0 * * * *" // Hourly at top of hour
    }

    async fn execute(&self) -> Result<()> {
        self.execute().await?;
        Ok(())
    }
}
