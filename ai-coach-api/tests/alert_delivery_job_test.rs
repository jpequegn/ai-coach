use ai_coach::models::{AlertDeliveryQueue, DeliveryMethod, DeliveryQueueStatus};
use ai_coach::services::{AlertDeliveryJob, AlertDeliveryQueueService, NotificationService};
use sqlx::PgPool;
use uuid::Uuid;

/// Test alert delivery job execution with empty queue
#[sqlx::test]
async fn test_job_execution_empty_queue(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);
    let job = AlertDeliveryJob::new(queue_service);

    // Execute job with empty queue
    let stats = job.execute().await.expect("Job execution failed");

    // Should complete successfully with zero records
    assert_eq!(stats.records_processed, 0);
    assert_eq!(stats.records_failed, 0);
    assert!(stats.error_message.is_none());
    assert!(stats.metadata.is_some());

    Ok(())
}

/// Test alert delivery job execution with pending deliveries
#[sqlx::test]
async fn test_job_execution_with_pending_deliveries(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue some deliveries
    for i in 0..5 {
        queue_service
            .queue_for_delivery(
                Uuid::new_v4(),
                DeliveryMethod::Email,
                format!("test{}@example.com", i),
            )
            .await
            .expect("Failed to queue delivery");
    }

    let job = AlertDeliveryJob::new(queue_service);

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    // Should have processed all 5 deliveries successfully
    assert_eq!(stats.records_processed, 5);
    assert_eq!(stats.records_failed, 0);
    assert!(stats.error_message.is_none());

    // Check metadata
    let metadata = stats.metadata.expect("Missing metadata");
    assert_eq!(metadata["total_attempted"], 5);
    assert_eq!(metadata["successful"], 5);
    assert_eq!(metadata["failed"], 0);

    Ok(())
}

/// Test job execution with high failure rate detection
#[sqlx::test]
async fn test_high_failure_rate_detection(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue deliveries (in real scenario, some would fail)
    // For this test, we're just verifying the metadata structure
    for i in 0..10 {
        queue_service
            .queue_for_delivery(
                Uuid::new_v4(),
                DeliveryMethod::Push,
                format!("device-token-{}", i),
            )
            .await
            .expect("Failed to queue delivery");
    }

    let job = AlertDeliveryJob::new(queue_service);
    let stats = job.execute().await.expect("Job execution failed");

    // Verify metadata includes failure rate tracking
    let metadata = stats.metadata.expect("Missing metadata");
    assert!(metadata.get("failure_rate").is_some());
    assert!(metadata.get("high_failure_rate").is_some());

    Ok(())
}

/// Test job execution time tracking
#[sqlx::test]
async fn test_execution_time_tracking(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue a few deliveries
    for i in 0..3 {
        queue_service
            .queue_for_delivery(
                Uuid::new_v4(),
                DeliveryMethod::Sms,
                format!("+123456789{}", i),
            )
            .await
            .expect("Failed to queue delivery");
    }

    let job = AlertDeliveryJob::new(queue_service);
    let stats = job.execute().await.expect("Job execution failed");

    // Execution time should be tracked and reasonable
    assert!(stats.execution_time_ms > 0);
    assert!(stats.execution_time_ms < 10000); // Should complete in under 10 seconds

    Ok(())
}

/// Test job schedule configuration
#[test]
fn test_job_schedule() {
    let schedule = AlertDeliveryJob::get_schedule();
    assert_eq!(schedule, "*/5 * * * *"); // Every 5 minutes
}

/// Test job name configuration
#[test]
fn test_job_name() {
    let name = AlertDeliveryJob::get_job_name();
    assert_eq!(name, "alert_delivery");
}

/// Test job metadata structure
#[sqlx::test]
async fn test_metadata_structure(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue deliveries with different methods
    queue_service
        .queue_for_delivery(
            Uuid::new_v4(),
            DeliveryMethod::Email,
            "email@example.com".to_string(),
        )
        .await
        .expect("Failed to queue email");

    queue_service
        .queue_for_delivery(
            Uuid::new_v4(),
            DeliveryMethod::Push,
            "device-token".to_string(),
        )
        .await
        .expect("Failed to queue push");

    let job = AlertDeliveryJob::new(queue_service);
    let stats = job.execute().await.expect("Job execution failed");

    // Verify metadata contains all expected fields
    let metadata = stats.metadata.expect("Missing metadata");
    assert!(metadata.get("total_attempted").is_some());
    assert!(metadata.get("successful").is_some());
    assert!(metadata.get("failed").is_some());
    assert!(metadata.get("retrying").is_some());
    assert!(metadata.get("failure_rate").is_some());
    assert!(metadata.get("avg_delivery_time_ms").is_some());
    assert!(metadata.get("by_method").is_some());
    assert!(metadata.get("high_failure_rate").is_some());

    Ok(())
}

/// Test job with multiple delivery methods
#[sqlx::test]
async fn test_multiple_delivery_methods(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue deliveries for all three methods
    queue_service
        .queue_for_delivery(
            Uuid::new_v4(),
            DeliveryMethod::Email,
            "email@example.com".to_string(),
        )
        .await?;

    queue_service
        .queue_for_delivery(
            Uuid::new_v4(),
            DeliveryMethod::Push,
            "device-token".to_string(),
        )
        .await?;

    queue_service
        .queue_for_delivery(
            Uuid::new_v4(),
            DeliveryMethod::Sms,
            "+1234567890".to_string(),
        )
        .await?;

    let job = AlertDeliveryJob::new(queue_service);
    let stats = job.execute().await.expect("Job execution failed");

    // Should process all three methods
    assert_eq!(stats.records_processed, 3);
    assert_eq!(stats.records_failed, 0);

    // Check by_method statistics exist
    let metadata = stats.metadata.expect("Missing metadata");
    let by_method = metadata.get("by_method").expect("Missing by_method");
    assert!(by_method.is_object());

    Ok(())
}
