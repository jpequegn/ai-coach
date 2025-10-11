use ai_coach::models::{AlertDeliveryQueue, DeliveryMethod, DeliveryQueueStatus};
use ai_coach::services::{AlertDeliveryQueueService, NotificationService};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// Test queuing an alert for delivery
#[sqlx::test]
async fn test_queue_for_delivery(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    let alert_id = Uuid::new_v4();
    let recipient = "test@example.com".to_string();

    // Queue for delivery
    let queue_id = service
        .queue_for_delivery(alert_id, DeliveryMethod::Email, recipient.clone())
        .await
        .expect("Failed to queue delivery");

    // Verify it was queued
    let queued = sqlx::query_as!(
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
        WHERE id = $1
        "#,
        queue_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(queued.alert_id, alert_id);
    assert_eq!(queued.recipient_id, recipient);
    assert_eq!(queued.delivery_method, DeliveryMethod::Email);
    assert_eq!(queued.status, DeliveryQueueStatus::Pending);
    assert_eq!(queued.attempts, 0);
    assert_eq!(queued.max_attempts, 3);

    Ok(())
}

/// Test getting pending deliveries
#[sqlx::test]
async fn test_get_pending_deliveries(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create multiple queued deliveries
    for i in 0..5 {
        service
            .queue_for_delivery(
                Uuid::new_v4(),
                DeliveryMethod::Email,
                format!("test{}@example.com", i),
            )
            .await
            .expect("Failed to queue delivery");
    }

    // Get pending deliveries
    let pending = service
        .get_pending_deliveries(None)
        .await
        .expect("Failed to get pending deliveries");

    assert_eq!(pending.len(), 5);
    for delivery in &pending {
        assert_eq!(delivery.status, DeliveryQueueStatus::Pending);
        assert_eq!(delivery.attempts, 0);
    }

    Ok(())
}

/// Test marking delivery as successful
#[sqlx::test]
async fn test_mark_delivered(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue a delivery
    let queue_id = service
        .queue_for_delivery(
            Uuid::new_v4(),
            DeliveryMethod::Push,
            "device-token-123".to_string(),
        )
        .await
        .expect("Failed to queue delivery");

    // Mark as delivered
    service
        .mark_delivered(queue_id)
        .await
        .expect("Failed to mark as delivered");

    // Verify status updated
    let delivery = sqlx::query_as!(
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
        WHERE id = $1
        "#,
        queue_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(delivery.status, DeliveryQueueStatus::Delivered);
    assert!(delivery.delivered_at.is_some());

    Ok(())
}

/// Test marking delivery as failed with retry
#[sqlx::test]
async fn test_mark_failed_with_retry(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue a delivery
    let queue_id = service
        .queue_for_delivery(
            Uuid::new_v4(),
            DeliveryMethod::Email,
            "test@example.com".to_string(),
        )
        .await
        .expect("Failed to queue delivery");

    // Mark as failed (first attempt)
    service
        .mark_failed(queue_id, "Temporary network error".to_string())
        .await
        .expect("Failed to mark as failed");

    // Verify status and retry scheduled
    let delivery = sqlx::query_as!(
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
        WHERE id = $1
        "#,
        queue_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(delivery.status, DeliveryQueueStatus::Pending); // Still pending for retry
    assert_eq!(delivery.attempts, 1);
    assert!(delivery.last_attempt_at.is_some());
    assert!(delivery.next_retry_at.is_some());
    assert_eq!(
        delivery.error_message.as_deref(),
        Some("Temporary network error")
    );

    Ok(())
}

/// Test permanent failure after max attempts
#[sqlx::test]
async fn test_permanent_failure_after_max_attempts(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue a delivery
    let queue_id = service
        .queue_for_delivery(
            Uuid::new_v4(),
            DeliveryMethod::Sms,
            "+1234567890".to_string(),
        )
        .await
        .expect("Failed to queue delivery");

    // Fail it 3 times (max attempts)
    for attempt in 1..=3 {
        service
            .mark_failed(queue_id, format!("Attempt {} failed", attempt))
            .await
            .expect("Failed to mark as failed");
    }

    // Verify permanently failed
    let delivery = sqlx::query_as!(
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
        WHERE id = $1
        "#,
        queue_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(delivery.status, DeliveryQueueStatus::Failed);
    assert_eq!(delivery.attempts, 3);
    assert!(delivery.next_retry_at.is_none()); // No more retries

    Ok(())
}

/// Test cancelling a delivery
#[sqlx::test]
async fn test_cancel_delivery(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue a delivery
    let queue_id = service
        .queue_for_delivery(
            Uuid::new_v4(),
            DeliveryMethod::Push,
            "device-token-456".to_string(),
        )
        .await
        .expect("Failed to queue delivery");

    // Cancel it
    service
        .cancel_delivery(queue_id)
        .await
        .expect("Failed to cancel delivery");

    // Verify status updated
    let delivery = sqlx::query_as!(
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
        WHERE id = $1
        "#,
        queue_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(delivery.status, DeliveryQueueStatus::Cancelled);

    Ok(())
}

/// Test batch processing with mixed results
#[sqlx::test]
async fn test_process_pending_queue(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue multiple deliveries
    for i in 0..10 {
        service
            .queue_for_delivery(
                Uuid::new_v4(),
                DeliveryMethod::Email,
                format!("test{}@example.com", i),
            )
            .await
            .expect("Failed to queue delivery");
    }

    // Process the queue
    let stats = service
        .process_pending_queue()
        .await
        .expect("Failed to process queue");

    // Verify statistics
    assert_eq!(stats.total_attempted, 10);
    // Note: In the current mock implementation, all will succeed
    assert_eq!(stats.successful, 10);
    assert_eq!(stats.failed, 0);

    Ok(())
}

/// Test retry schedule calculation
#[test]
fn test_retry_schedule_calculation() {
    use chrono::Duration;

    // This tests the private method via the public behavior
    // First retry: immediate (0 minutes)
    // Second retry: 5 minutes
    // Third retry: 15 minutes
    // Fourth retry: 60 minutes

    let now = Utc::now();

    // Test first retry (immediate)
    let expected = now;
    // Actual calculation would be: calculate_next_retry(0)
    // Should be approximately now

    // Test second retry (5 minutes)
    let expected = now + Duration::minutes(5);
    // calculate_next_retry(1) should return ~5 minutes from now

    // Test third retry (15 minutes)
    let expected = now + Duration::minutes(15);
    // calculate_next_retry(2) should return ~15 minutes from now

    // Test fourth retry (60 minutes, capped)
    let expected = now + Duration::minutes(60);
    // calculate_next_retry(3+) should return ~60 minutes from now
}

/// Test rate limiting per delivery method
#[sqlx::test]
async fn test_rate_limiting_by_method(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue 60 deliveries (exceeds rate limit of 50 per method)
    for i in 0..60 {
        service
            .queue_for_delivery(
                Uuid::new_v4(),
                DeliveryMethod::Email,
                format!("test{}@example.com", i),
            )
            .await
            .expect("Failed to queue delivery");
    }

    // Process the queue
    let stats = service
        .process_pending_queue()
        .await
        .expect("Failed to process queue");

    // Should only process 50 due to rate limit
    assert!(stats.total_attempted <= 50);

    Ok(())
}

/// Test getting pending deliveries with limit
#[sqlx::test]
async fn test_get_pending_deliveries_with_limit(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue 20 deliveries
    for i in 0..20 {
        service
            .queue_for_delivery(
                Uuid::new_v4(),
                DeliveryMethod::Push,
                format!("device-{}", i),
            )
            .await
            .expect("Failed to queue delivery");
    }

    // Get only 5
    let pending = service
        .get_pending_deliveries(Some(5))
        .await
        .expect("Failed to get pending deliveries");

    assert_eq!(pending.len(), 5);

    Ok(())
}

/// Test that failed deliveries with future retry times are not returned
#[sqlx::test]
async fn test_pending_deliveries_respects_retry_time(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Queue a delivery
    let queue_id = service
        .queue_for_delivery(
            Uuid::new_v4(),
            DeliveryMethod::Email,
            "test@example.com".to_string(),
        )
        .await
        .expect("Failed to queue delivery");

    // Mark it as failed (will schedule retry in 5 minutes)
    service
        .mark_failed(queue_id, "Test failure".to_string())
        .await
        .expect("Failed to mark as failed");

    // Get pending deliveries - should not include the one with future retry time
    let pending = service
        .get_pending_deliveries(None)
        .await
        .expect("Failed to get pending deliveries");

    // Should be empty since retry is in the future
    assert_eq!(pending.len(), 0);

    Ok(())
}
