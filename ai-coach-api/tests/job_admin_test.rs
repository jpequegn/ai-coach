use ai_coach::models::{JobStatusSummary, JobsHealthStatus, JobExecutionLog};
use ai_coach::services::RecoveryJobScheduler;
use ai_coach::api::job_admin::{JobAdminState, JobHistoryResponse};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Helper to create JobAdminState for testing
async fn create_test_state(pool: PgPool) -> Arc<JobAdminState> {
    let scheduler = RecoveryJobScheduler::new(pool, None)
        .await
        .expect("Failed to create scheduler");

    Arc::new(JobAdminState {
        scheduler: Arc::new(scheduler),
    })
}

/// Test getting all job statuses
#[sqlx::test]
async fn test_get_all_job_statuses(pool: PgPool) -> sqlx::Result<()> {
    let state = create_test_state(pool.clone()).await;

    // Get all job statuses
    let statuses = state
        .scheduler
        .get_all_job_statuses()
        .await
        .expect("Failed to get job statuses");

    // Verify we have registered jobs
    assert!(!statuses.is_empty(), "Should have at least one registered job");

    // Verify structure of each status
    for status in &statuses {
        assert!(!status.job_name.is_empty());
        assert!(status.total_executions >= 0);
        assert!(status.successful_executions >= 0);
        assert!(status.failed_executions >= 0);
    }

    Ok(())
}

/// Test getting specific job status
#[sqlx::test]
async fn test_get_specific_job_status(pool: PgPool) -> sqlx::Result<()> {
    let state = create_test_state(pool.clone()).await;

    // Get all jobs first to find a valid job name
    let all_statuses = state
        .scheduler
        .get_all_job_statuses()
        .await
        .expect("Failed to get all job statuses");

    assert!(!all_statuses.is_empty(), "Need at least one job for this test");

    let job_name = &all_statuses[0].job_name;

    // Get specific job status
    let status = state
        .scheduler
        .get_job_status_summary(job_name)
        .await
        .expect("Failed to get job status");

    // Verify status fields
    assert_eq!(status.job_name, *job_name);
    assert!(status.total_executions >= 0);
    assert!(status.successful_executions >= 0);
    assert!(status.failed_executions >= 0);

    Ok(())
}

/// Test getting job status for non-existent job
#[sqlx::test]
async fn test_get_nonexistent_job_status(pool: PgPool) -> sqlx::Result<()> {
    let state = create_test_state(pool.clone()).await;

    let job_name = "nonexistent_job_12345";

    // Try to get status for non-existent job
    let result = state
        .scheduler
        .get_job_status_summary(job_name)
        .await;

    // Should return an error or empty result
    assert!(result.is_err() || result.unwrap().total_executions == 0);

    Ok(())
}

/// Test getting job execution history
#[sqlx::test]
async fn test_get_job_history(pool: PgPool) -> sqlx::Result<()> {
    let state = create_test_state(pool.clone()).await;

    // Get all jobs first
    let all_statuses = state
        .scheduler
        .get_all_job_statuses()
        .await
        .expect("Failed to get all job statuses");

    assert!(!all_statuses.is_empty(), "Need at least one job for this test");

    let job_name = &all_statuses[0].job_name;

    // Get job history with pagination
    let page_size = 10;
    let offset = 0;

    let history = state
        .scheduler
        .get_job_history(job_name, page_size, offset)
        .await
        .expect("Failed to get job history");

    // Verify history structure (may be empty if job hasn't run yet)
    assert!(history.len() <= page_size as usize);

    // If there are executions, verify their structure
    for execution in &history {
        assert_eq!(execution.job_name, *job_name);
        assert!(execution.execution_time_ms >= 0);
        assert!(execution.records_processed >= 0);
        assert!(execution.records_failed >= 0);
    }

    Ok(())
}

/// Test job history pagination
#[sqlx::test]
async fn test_job_history_pagination(pool: PgPool) -> sqlx::Result<()> {
    let state = create_test_state(pool.clone()).await;

    // Get all jobs
    let all_statuses = state
        .scheduler
        .get_all_job_statuses()
        .await
        .expect("Failed to get all job statuses");

    assert!(!all_statuses.is_empty());

    let job_name = &all_statuses[0].job_name;

    // Get first page
    let page1 = state
        .scheduler
        .get_job_history(job_name, 5, 0)
        .await
        .expect("Failed to get page 1");

    // Get second page
    let page2 = state
        .scheduler
        .get_job_history(job_name, 5, 5)
        .await
        .expect("Failed to get page 2");

    // Verify pagination works
    assert!(page1.len() <= 5);
    assert!(page2.len() <= 5);

    // If both pages have data, they should be different
    if !page1.is_empty() && !page2.is_empty() {
        assert_ne!(page1[0].id, page2[0].id, "Pages should contain different records");
    }

    Ok(())
}

/// Test getting latest execution
#[sqlx::test]
async fn test_get_latest_execution(pool: PgPool) -> sqlx::Result<()> {
    let state = create_test_state(pool.clone()).await;

    // Get all jobs
    let all_statuses = state
        .scheduler
        .get_all_job_statuses()
        .await
        .expect("Failed to get all job statuses");

    assert!(!all_statuses.is_empty());

    let job_name = &all_statuses[0].job_name;

    // Get latest execution
    let latest = state
        .scheduler
        .get_latest_execution(job_name)
        .await
        .expect("Failed to get latest execution");

    // Latest execution might be None if job hasn't run yet
    if let Some(execution) = latest {
        assert_eq!(execution.job_name, *job_name);
        assert!(execution.execution_time_ms >= 0);
    }

    Ok(())
}

/// Test checking all jobs health
#[sqlx::test]
async fn test_check_all_jobs_health(pool: PgPool) -> sqlx::Result<()> {
    let state = create_test_state(pool.clone()).await;

    // Check health of all jobs
    let health = state
        .scheduler
        .check_all_jobs_health()
        .await
        .expect("Failed to check jobs health");

    // Verify health structure
    assert!(!health.jobs.is_empty(), "Should have at least one job health status");
    assert!(health.total_jobs > 0);
    assert!(health.healthy_jobs >= 0);
    assert!(health.unhealthy_jobs >= 0);

    // Verify individual job health
    for job_health in &health.jobs {
        assert!(!job_health.job_name.is_empty());
        assert!(job_health.consecutive_failures >= 0);
    }

    Ok(())
}

/// Test checking specific job health
#[sqlx::test]
async fn test_check_specific_job_health(pool: PgPool) -> sqlx::Result<()> {
    let state = create_test_state(pool.clone()).await;

    // Get all jobs
    let all_statuses = state
        .scheduler
        .get_all_job_statuses()
        .await
        .expect("Failed to get all job statuses");

    assert!(!all_statuses.is_empty());

    let job_name = &all_statuses[0].job_name;

    // Check specific job health
    let health = state
        .scheduler
        .check_job_health(job_name)
        .await
        .expect("Failed to check job health");

    // Verify health fields
    assert_eq!(health.job_name, *job_name);
    assert!(health.consecutive_failures >= 0);

    Ok(())
}

/// Test checking health of non-existent job
#[sqlx::test]
async fn test_check_nonexistent_job_health(pool: PgPool) -> sqlx::Result<()> {
    let state = create_test_state(pool.clone()).await;

    let job_name = "nonexistent_job_health_12345";

    // Try to check health of non-existent job
    let result = state
        .scheduler
        .check_job_health(job_name)
        .await;

    // Should return an error
    assert!(result.is_err(), "Should fail for non-existent job");

    Ok(())
}

/// Test job health status changes
#[sqlx::test]
async fn test_job_health_status_transitions(pool: PgPool) -> sqlx::Result<()> {
    let state = create_test_state(pool.clone()).await;

    // Get initial health
    let initial_health = state
        .scheduler
        .check_all_jobs_health()
        .await
        .expect("Failed to get initial health");

    assert!(initial_health.total_jobs > 0);

    // Verify health counts are consistent
    let sum_health = initial_health.healthy_jobs + initial_health.unhealthy_jobs;
    assert_eq!(
        sum_health, initial_health.total_jobs,
        "Health counts should add up to total jobs"
    );

    Ok(())
}

/// Test job execution statistics accuracy
#[sqlx::test]
async fn test_job_execution_statistics(pool: PgPool) -> sqlx::Result<()> {
    let state = create_test_state(pool.clone()).await;

    // Get all job statuses
    let statuses = state
        .scheduler
        .get_all_job_statuses()
        .await
        .expect("Failed to get job statuses");

    for status in statuses {
        // Verify execution counts are consistent
        assert!(
            status.successful_executions + status.failed_executions <= status.total_executions,
            "Success + failures should not exceed total executions for job: {}",
            status.job_name
        );

        // Verify success rate calculation
        if status.total_executions > 0 {
            let expected_rate = (status.successful_executions as f64
                / status.total_executions as f64) * 100.0;
            assert!(
                (status.success_rate - expected_rate).abs() < 0.01,
                "Success rate mismatch for job: {}",
                status.job_name
            );
        }

        // Verify timing fields
        if status.last_execution_time.is_some() {
            assert!(status.avg_execution_time_ms.is_some());
        }
    }

    Ok(())
}

/// Test concurrent job status requests
#[sqlx::test]
async fn test_concurrent_job_status_requests(pool: PgPool) -> sqlx::Result<()> {
    let state = create_test_state(pool.clone()).await;

    // Get all jobs
    let all_statuses = state
        .scheduler
        .get_all_job_statuses()
        .await
        .expect("Failed to get all job statuses");

    assert!(!all_statuses.is_empty());

    // Make multiple concurrent requests
    let mut handles = vec![];
    for _ in 0..5 {
        let state_clone = Arc::clone(&state);
        let job_name = all_statuses[0].job_name.clone();

        let handle = tokio::spawn(async move {
            state_clone
                .scheduler
                .get_job_status_summary(&job_name)
                .await
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let result = handle.await.expect("Task panicked");
        assert!(result.is_ok(), "Concurrent request failed");
    }

    Ok(())
}
