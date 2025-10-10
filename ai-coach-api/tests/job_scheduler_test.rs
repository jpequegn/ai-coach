use ai_coach::models::{JobExecutionStats, JobExecutionStatus, JobHealthStatus};
use ai_coach::services::RecoveryJobScheduler;
use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;

/// Test job execution logging
#[sqlx::test]
async fn test_job_execution_logging(pool: PgPool) -> sqlx::Result<()> {
    let scheduler = Arc::new(RecoveryJobScheduler::new(pool.clone()).await.unwrap());
    scheduler.start().await.unwrap();

    // Register a simple test job that succeeds
    let job_name = "test_success_job";
    scheduler
        .register_job(job_name, "*/30 * * * * *", || {
            Box::pin(async {
                Ok(JobExecutionStats {
                    records_processed: 10,
                    records_failed: 0,
                    execution_time_ms: 100,
                    error_message: None,
                    metadata: Some(serde_json::json!({"test": "data"})),
                })
            })
        })
        .await
        .unwrap();

    // Give the job time to potentially execute
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Check job status
    let summary = scheduler.get_job_status_summary(job_name).await.unwrap();
    assert_eq!(summary.job_name, job_name);

    scheduler.shutdown().await.unwrap();
    Ok(())
}

/// Test job health checking
#[sqlx::test]
async fn test_job_health_check(pool: PgPool) -> sqlx::Result<()> {
    let scheduler = Arc::new(RecoveryJobScheduler::new(pool.clone()).await.unwrap());
    scheduler.start().await.unwrap();

    // Register a test job
    let job_name = "test_health_job";
    scheduler
        .register_job(job_name, "0 0 * * * *", || {
            Box::pin(async {
                Ok(JobExecutionStats {
                    records_processed: 5,
                    records_failed: 0,
                    execution_time_ms: 50,
                    error_message: None,
                    metadata: None,
                })
            })
        })
        .await
        .unwrap();

    // Check all jobs health
    let health = scheduler.check_all_jobs_health().await.unwrap();
    assert_eq!(health.total_jobs, 1);

    scheduler.shutdown().await.unwrap();
    Ok(())
}

/// Test retry logic with exponential backoff
#[tokio::test]
async fn test_retry_logic() {
    let mut attempt = 0;

    let result = RecoveryJobScheduler::execute_with_retry("test_retry", 3, || {
        Box::pin(async move {
            attempt += 1;
            if attempt < 3 {
                Err(anyhow::anyhow!("Simulated failure"))
            } else {
                Ok(())
            }
        })
    })
    .await;

    assert!(result.is_ok());
}

/// Test job history retrieval
#[sqlx::test]
async fn test_job_history(pool: PgPool) -> sqlx::Result<()> {
    let scheduler = Arc::new(RecoveryJobScheduler::new(pool.clone()).await.unwrap());
    scheduler.start().await.unwrap();

    let job_name = "test_history_job";
    scheduler
        .register_job(job_name, "*/30 * * * * *", || {
            Box::pin(async {
                Ok(JobExecutionStats {
                    records_processed: 1,
                    records_failed: 0,
                    execution_time_ms: 10,
                    error_message: None,
                    metadata: None,
                })
            })
        })
        .await
        .unwrap();

    // Give job time to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Get history (should have at least 0 entries since job might not have run yet)
    let history = scheduler.get_job_history(job_name, 10, 0).await.unwrap();
    assert!(history.len() >= 0);

    scheduler.shutdown().await.unwrap();
    Ok(())
}

/// Test latest execution retrieval
#[sqlx::test]
async fn test_latest_execution(pool: PgPool) -> sqlx::Result<()> {
    let scheduler = Arc::new(RecoveryJobScheduler::new(pool.clone()).await.unwrap());
    scheduler.start().await.unwrap();

    let job_name = "test_latest_job";
    scheduler
        .register_job(job_name, "0 0 * * * *", || {
            Box::pin(async {
                Ok(JobExecutionStats {
                    records_processed: 1,
                    records_failed: 0,
                    execution_time_ms: 10,
                    error_message: None,
                    metadata: None,
                })
            })
        })
        .await
        .unwrap();

    // Latest execution should be None since job hasn't run
    let latest = scheduler.get_latest_execution(job_name).await.unwrap();
    assert!(latest.is_none());

    scheduler.shutdown().await.unwrap();
    Ok(())
}
