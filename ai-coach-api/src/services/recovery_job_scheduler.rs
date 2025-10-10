use anyhow::{Result, Context as AnyhowContext};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, error, warn};
use uuid::Uuid;

use crate::models::{
    JobExecutionLog, JobExecutionStats, JobExecutionStatus, JobHealthCheck, JobHealthStatus,
    JobStatusSummary, JobsHealthStatus,
};

/// Recovery job scheduler for managing background jobs related to recovery monitoring
pub struct RecoveryJobScheduler {
    db: PgPool,
    scheduler: Arc<RwLock<JobScheduler>>,
    registered_jobs: Arc<RwLock<Vec<String>>>,
}

impl RecoveryJobScheduler {
    /// Create a new recovery job scheduler
    pub async fn new(db: PgPool) -> Result<Self> {
        let scheduler = JobScheduler::new()
            .await
            .context("Failed to create job scheduler")?;

        Ok(Self {
            db,
            scheduler: Arc::new(RwLock::new(scheduler)),
            registered_jobs: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Start the job scheduler
    pub async fn start(&self) -> Result<()> {
        let scheduler = self.scheduler.read().await;
        scheduler
            .start()
            .await
            .context("Failed to start job scheduler")?;

        info!("Recovery job scheduler started");
        Ok(())
    }

    /// Stop the job scheduler
    pub async fn shutdown(&self) -> Result<()> {
        let scheduler = self.scheduler.read().await;
        scheduler
            .shutdown()
            .await
            .context("Failed to shutdown job scheduler")?;

        info!("Recovery job scheduler stopped");
        Ok(())
    }

    /// Register a job with the scheduler
    pub async fn register_job<F>(&self, job_name: &str, schedule: &str, job_fn: F) -> Result<Uuid>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<JobExecutionStats>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let job_name = job_name.to_string();
        let db = self.db.clone();
        let job_name_clone = job_name.clone();

        // Wrap the job function with execution logging
        let job = Job::new_async(schedule, move |_uuid, _l| {
            let db = db.clone();
            let job_name = job_name_clone.clone();
            let job_fn_ref = &job_fn;

            Box::pin(async move {
                // Log job start
                let execution_id = match Self::log_job_start(&db, &job_name).await {
                    Ok(id) => id,
                    Err(e) => {
                        error!("Failed to log job start for {}: {}", job_name, e);
                        return;
                    }
                };

                let start_time = Instant::now();

                // Execute the job
                let result = job_fn_ref().await;

                let execution_time_ms = start_time.elapsed().as_millis() as i64;

                // Log job completion
                match result {
                    Ok(stats) => {
                        let mut final_stats = stats;
                        final_stats.execution_time_ms = execution_time_ms;

                        if let Err(e) = Self::log_job_completion(&db, execution_id, final_stats).await {
                            error!("Failed to log job completion for {}: {}", job_name, e);
                        }
                    }
                    Err(e) => {
                        let error_stats = JobExecutionStats {
                            records_processed: 0,
                            records_failed: 0,
                            execution_time_ms,
                            error_message: Some(e.to_string()),
                            metadata: None,
                        };

                        if let Err(log_err) = Self::log_job_failure(&db, execution_id, error_stats).await {
                            error!("Failed to log job failure for {}: {}", job_name, log_err);
                        }
                    }
                }
            })
        })
        .context("Failed to create job")?;

        // Add job to scheduler
        let mut scheduler = self.scheduler.write().await;
        let job_id = scheduler
            .add(job)
            .await
            .context("Failed to add job to scheduler")?;

        // Track registered job
        let mut registered = self.registered_jobs.write().await;
        registered.push(job_name.clone());

        info!("Registered job: {} with schedule: {}", job_name, schedule);
        Ok(job_id)
    }

    /// Execute a job with retry logic using exponential backoff
    pub async fn execute_with_retry<F, T>(
        job_name: &str,
        max_retries: u32,
        operation: F,
    ) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send,
    {
        let mut retries = 0;
        let mut last_error = None;

        while retries <= max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    retries += 1;

                    if retries <= max_retries {
                        let backoff_ms = 2u64.pow(retries) * 1000; // Exponential backoff: 2s, 4s, 8s, 16s
                        warn!(
                            "Job {} failed (attempt {}/{}), retrying in {}ms: {}",
                            job_name,
                            retries,
                            max_retries,
                            backoff_ms,
                            last_error.as_ref().unwrap()
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Log the start of a job execution
    async fn log_job_start(db: &PgPool, job_name: &str) -> Result<Uuid> {
        let execution_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO job_execution_log (
                id, job_name, started_at, status, records_processed, records_failed
            )
            VALUES ($1, $2, NOW(), 'running', 0, 0)
            "#,
            execution_id,
            job_name
        )
        .execute(db)
        .await
        .context("Failed to log job start")?;

        Ok(execution_id)
    }

    /// Log successful job completion
    async fn log_job_completion(
        db: &PgPool,
        execution_id: Uuid,
        stats: JobExecutionStats,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE job_execution_log
            SET
                completed_at = NOW(),
                status = 'success',
                records_processed = $2,
                records_failed = $3,
                execution_time_ms = $4,
                metadata = $5
            WHERE id = $1
            "#,
            execution_id,
            stats.records_processed,
            stats.records_failed,
            stats.execution_time_ms,
            stats.metadata.unwrap_or_else(|| serde_json::json!({}))
        )
        .execute(db)
        .await
        .context("Failed to log job completion")?;

        Ok(())
    }

    /// Log job failure
    async fn log_job_failure(
        db: &PgPool,
        execution_id: Uuid,
        stats: JobExecutionStats,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE job_execution_log
            SET
                completed_at = NOW(),
                status = 'failed',
                records_processed = $2,
                records_failed = $3,
                execution_time_ms = $4,
                error_message = $5
            WHERE id = $1
            "#,
            execution_id,
            stats.records_processed,
            stats.records_failed,
            stats.execution_time_ms,
            stats.error_message
        )
        .execute(db)
        .await
        .context("Failed to log job failure")?;

        Ok(())
    }

    /// Get execution history for a job
    pub async fn get_job_history(
        &self,
        job_name: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<JobExecutionLog>> {
        let logs = sqlx::query_as!(
            JobExecutionLog,
            r#"
            SELECT
                id,
                job_name,
                started_at,
                completed_at,
                status as "status: JobExecutionStatus",
                records_processed,
                records_failed,
                error_message,
                execution_time_ms,
                metadata,
                created_at,
                updated_at
            FROM job_execution_log
            WHERE job_name = $1
            ORDER BY started_at DESC
            LIMIT $2 OFFSET $3
            "#,
            job_name,
            limit,
            offset
        )
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch job history")?;

        Ok(logs)
    }

    /// Get latest execution for a job
    pub async fn get_latest_execution(&self, job_name: &str) -> Result<Option<JobExecutionLog>> {
        let log = sqlx::query_as!(
            JobExecutionLog,
            r#"
            SELECT
                id,
                job_name,
                started_at,
                completed_at,
                status as "status: JobExecutionStatus",
                records_processed,
                records_failed,
                error_message,
                execution_time_ms,
                metadata,
                created_at,
                updated_at
            FROM job_execution_log
            WHERE job_name = $1
            ORDER BY started_at DESC
            LIMIT 1
            "#,
            job_name
        )
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch latest execution")?;

        Ok(log)
    }

    /// Get status summary for a job
    pub async fn get_job_status_summary(&self, job_name: &str) -> Result<JobStatusSummary> {
        let latest_execution = self.get_latest_execution(job_name).await?;

        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as "total_executions!",
                COUNT(*) FILTER (WHERE status = 'success') as "success_count!",
                COUNT(*) FILTER (WHERE status = 'failed') as "failed_count!",
                AVG(execution_time_ms) FILTER (WHERE status = 'success') as "avg_execution_time",
                MAX(completed_at) FILTER (WHERE status = 'success') as "last_success_at",
                MAX(completed_at) FILTER (WHERE status = 'failed') as "last_failure_at"
            FROM job_execution_log
            WHERE job_name = $1
            "#,
            job_name
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to fetch job statistics")?;

        Ok(JobStatusSummary {
            job_name: job_name.to_string(),
            latest_execution,
            total_executions: stats.total_executions,
            success_count: stats.success_count,
            failed_count: stats.failed_count,
            average_execution_time_ms: stats.avg_execution_time,
            last_success_at: stats.last_success_at,
            last_failure_at: stats.last_failure_at,
        })
    }

    /// Get status summaries for all registered jobs
    pub async fn get_all_job_statuses(&self) -> Result<Vec<JobStatusSummary>> {
        let jobs = self.registered_jobs.read().await;
        let mut summaries = Vec::new();

        for job_name in jobs.iter() {
            match self.get_job_status_summary(job_name).await {
                Ok(summary) => summaries.push(summary),
                Err(e) => {
                    error!("Failed to get status for job {}: {}", job_name, e);
                }
            }
        }

        Ok(summaries)
    }

    /// Check health of a specific job
    pub async fn check_job_health(&self, job_name: &str) -> Result<JobHealthCheck> {
        let latest = self.get_latest_execution(job_name).await?;

        let (status, message, consecutive_failures) = match latest {
            Some(ref log) => {
                // Count consecutive failures
                let failures = sqlx::query_scalar!(
                    r#"
                    WITH ranked AS (
                        SELECT
                            status,
                            ROW_NUMBER() OVER (ORDER BY started_at DESC) as rn
                        FROM job_execution_log
                        WHERE job_name = $1
                    )
                    SELECT COUNT(*) as "count!"
                    FROM ranked
                    WHERE rn <= (
                        SELECT MIN(rn)
                        FROM ranked
                        WHERE status != 'failed'
                    ) AND status = 'failed'
                    "#,
                    job_name
                )
                .fetch_one(&self.db)
                .await
                .unwrap_or(0);

                // Determine health based on recent executions
                if log.status == JobExecutionStatus::Failed {
                    if failures >= 3 {
                        (
                            JobHealthStatus::Unhealthy,
                            format!("Job has failed {} consecutive times", failures),
                            failures,
                        )
                    } else {
                        (
                            JobHealthStatus::Degraded,
                            format!("Job failed on last execution (attempt {}/3)", failures),
                            failures,
                        )
                    }
                } else if log.status == JobExecutionStatus::Running {
                    // Check if job is stuck (running for more than 1 hour)
                    let elapsed = Utc::now() - log.started_at;
                    if elapsed.num_hours() > 1 {
                        (
                            JobHealthStatus::Degraded,
                            "Job may be stuck (running for >1 hour)".to_string(),
                            0,
                        )
                    } else {
                        (JobHealthStatus::Healthy, "Job is running".to_string(), 0)
                    }
                } else {
                    (JobHealthStatus::Healthy, "Job is healthy".to_string(), 0)
                }
            }
            None => (
                JobHealthStatus::Degraded,
                "No execution history found".to_string(),
                0,
            ),
        };

        Ok(JobHealthCheck {
            job_name: job_name.to_string(),
            status,
            message,
            last_execution_at: latest.as_ref().map(|l| l.started_at),
            consecutive_failures,
            checked_at: Utc::now(),
        })
    }

    /// Check health of all jobs
    pub async fn check_all_jobs_health(&self) -> Result<JobsHealthStatus> {
        let jobs = self.registered_jobs.read().await;
        let mut job_checks = Vec::new();
        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;

        for job_name in jobs.iter() {
            match self.check_job_health(job_name).await {
                Ok(check) => {
                    match check.status {
                        JobHealthStatus::Healthy => healthy += 1,
                        JobHealthStatus::Degraded => degraded += 1,
                        JobHealthStatus::Unhealthy => unhealthy += 1,
                    }
                    job_checks.push(check);
                }
                Err(e) => {
                    error!("Failed to check health for job {}: {}", job_name, e);
                    unhealthy += 1;
                    job_checks.push(JobHealthCheck {
                        job_name: job_name.clone(),
                        status: JobHealthStatus::Unhealthy,
                        message: format!("Health check failed: {}", e),
                        last_execution_at: None,
                        consecutive_failures: 0,
                        checked_at: Utc::now(),
                    });
                }
            }
        }

        let overall_status = if unhealthy > 0 {
            JobHealthStatus::Unhealthy
        } else if degraded > 0 {
            JobHealthStatus::Degraded
        } else {
            JobHealthStatus::Healthy
        };

        Ok(JobsHealthStatus {
            overall_status,
            total_jobs: jobs.len(),
            healthy_jobs: healthy,
            degraded_jobs: degraded,
            unhealthy_jobs: unhealthy,
            job_checks,
            checked_at: Utc::now(),
        })
    }
}
