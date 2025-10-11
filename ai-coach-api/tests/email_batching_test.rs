use ai_coach::models::{AlertDeliveryQueue, DeliveryMethod};
use ai_coach::services::{
    AlertDeliveryQueueService, EmailBatchConfig, EmailBatchingService, NotificationService,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create test email delivery
async fn create_email_delivery(
    pool: &PgPool,
    service: &AlertDeliveryQueueService,
    recipient: &str,
) -> Uuid {
    service
        .queue_for_delivery(Uuid::new_v4(), DeliveryMethod::Email, recipient.to_string())
        .await
        .expect("Failed to create email delivery")
}

/// Test email batching groups by recipient
#[sqlx::test]
async fn test_email_batching_groups_by_recipient(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);
    let batch_service = EmailBatchingService::with_defaults(pool.clone());

    // Create multiple emails for same recipient
    for _ in 0..5 {
        create_email_delivery(&pool, &queue_service, "batch@test.com").await;
    }

    // Create emails for different recipient
    for _ in 0..3 {
        create_email_delivery(&pool, &queue_service, "other@test.com").await;
    }

    // Process batches
    let batches = batch_service
        .process_batches()
        .await
        .expect("Failed to process batches");

    // Should have 2 batches (one per recipient)
    assert_eq!(batches.len(), 2);

    // Check first batch
    let batch1 = batches.get("batch@test.com").expect("Missing batch1");
    assert_eq!(batch1.delivery_ids.len(), 5);
    assert_eq!(batch1.alert_ids.len(), 5);

    // Check second batch
    let batch2 = batches.get("other@test.com").expect("Missing batch2");
    assert_eq!(batch2.delivery_ids.len(), 3);
    assert_eq!(batch2.alert_ids.len(), 3);

    Ok(())
}

/// Test max batch size limit
#[sqlx::test]
async fn test_max_batch_size_limit(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    let config = EmailBatchConfig {
        enabled: true,
        window_minutes: 15,
        max_batch_size: 5, // Limit to 5
    };
    let batch_service = EmailBatchingService::new(pool.clone(), config);

    // Create 10 emails for same recipient
    for _ in 0..10 {
        create_email_delivery(&pool, &queue_service, "limit@test.com").await;
    }

    // Process batches
    let batches = batch_service
        .process_batches()
        .await
        .expect("Failed to process batches");

    // Should have 1 batch with max 5 emails
    assert_eq!(batches.len(), 1);
    let batch = batches.get("limit@test.com").expect("Missing batch");
    assert_eq!(batch.delivery_ids.len(), 5); // Only 5, not 10

    Ok(())
}

/// Test batching disabled
#[sqlx::test]
async fn test_batching_disabled(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    let config = EmailBatchConfig {
        enabled: false,
        window_minutes: 15,
        max_batch_size: 10,
    };
    let batch_service = EmailBatchingService::new(pool.clone(), config);

    // Create emails
    for _ in 0..5 {
        create_email_delivery(&pool, &queue_service, "disabled@test.com").await;
    }

    // Process batches
    let batches = batch_service
        .process_batches()
        .await
        .expect("Failed to process batches");

    // Should have no batches (batching disabled)
    assert_eq!(batches.len(), 0);

    Ok(())
}

/// Test marking batch as delivered
#[sqlx::test]
async fn test_mark_batch_delivered(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);
    let batch_service = EmailBatchingService::with_defaults(pool.clone());

    // Create emails
    for _ in 0..3 {
        create_email_delivery(&pool, &queue_service, "mark@test.com").await;
    }

    // Process batches
    let batches = batch_service
        .process_batches()
        .await
        .expect("Failed to process batches");

    let batch = batches.get("mark@test.com").expect("Missing batch");

    // Mark as delivered
    batch_service
        .mark_batch_delivered(batch)
        .await
        .expect("Failed to mark batch delivered");

    // Verify all deliveries marked as delivered
    for delivery_id in &batch.delivery_ids {
        let delivery = sqlx::query_as!(
            AlertDeliveryQueue,
            r#"
            SELECT
                id, alert_id,
                delivery_method as "delivery_method: DeliveryMethod",
                recipient_id, attempts, max_attempts,
                last_attempt_at, next_retry_at,
                status as "status: _",
                error_message, delivered_at,
                created_at, updated_at
            FROM alert_delivery_queue
            WHERE id = $1
            "#,
            delivery_id
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(delivery.status, "delivered");
        assert!(delivery.delivered_at.is_some());
    }

    Ok(())
}

/// Test batching statistics
#[sqlx::test]
async fn test_batching_statistics(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);
    let batch_service = EmailBatchingService::with_defaults(pool.clone());

    // Create and deliver emails for multiple recipients
    for i in 0..3 {
        let delivery_id =
            create_email_delivery(&pool, &queue_service, &format!("stats{}@test.com", i)).await;
        // Mark as delivered
        sqlx::query!(
            "UPDATE alert_delivery_queue SET status = 'delivered', delivered_at = NOW() WHERE id = $1",
            delivery_id
        )
        .execute(&pool)
        .await?;
    }

    // Get statistics
    let stats = batch_service
        .get_batching_stats(24)
        .await
        .expect("Failed to get stats");

    assert_eq!(stats.unique_recipients, 3);
    assert_eq!(stats.total_deliveries, 3);

    Ok(())
}

/// Test batching with time window
#[sqlx::test]
async fn test_batching_time_window(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    let config = EmailBatchConfig {
        enabled: true,
        window_minutes: 15, // 15 minute window
        max_batch_size: 10,
    };
    let batch_service = EmailBatchingService::new(pool.clone(), config);

    // Create recent emails
    for _ in 0..3 {
        create_email_delivery(&pool, &queue_service, "recent@test.com").await;
    }

    // Create old emails (outside window)
    for _ in 0..2 {
        let delivery_id =
            create_email_delivery(&pool, &queue_service, "old@test.com").await;
        // Backdate created_at to 30 minutes ago (outside 15 min window)
        sqlx::query!(
            "UPDATE alert_delivery_queue SET created_at = NOW() - INTERVAL '30 minutes' WHERE id = $1",
            delivery_id
        )
        .execute(&pool)
        .await?;
    }

    // Process batches
    let batches = batch_service
        .process_batches()
        .await
        .expect("Failed to process batches");

    // Should only have recent batch (old emails outside window)
    assert_eq!(batches.len(), 1);
    assert!(batches.contains_key("recent@test.com"));
    assert!(!batches.contains_key("old@test.com"));

    Ok(())
}

/// Test batching reduction calculation
#[sqlx::test]
async fn test_batching_reduction(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);
    let batch_service = EmailBatchingService::with_defaults(pool.clone());

    // Create 10 emails for same recipient
    for _ in 0..10 {
        create_email_delivery(&pool, &queue_service, "reduce@test.com").await;
    }

    // Process batches
    let batches = batch_service
        .process_batches()
        .await
        .expect("Failed to process batches");

    // Should have 1 batch with 10 emails (90% reduction: 10 emails -> 1 batch)
    assert_eq!(batches.len(), 1);
    let batch = batches.get("reduce@test.com").expect("Missing batch");
    assert_eq!(batch.delivery_ids.len(), 10);

    // Reduction: (10 - 1) / 10 = 90%
    let reduction_pct = ((10.0 - 1.0) / 10.0) * 100.0;
    assert!((reduction_pct - 90.0).abs() < 0.01);

    Ok(())
}

/// Test empty batch processing
#[sqlx::test]
async fn test_empty_batch_processing(pool: PgPool) -> sqlx::Result<()> {
    let batch_service = EmailBatchingService::with_defaults(pool.clone());

    // Process batches with no pending emails
    let batches = batch_service
        .process_batches()
        .await
        .expect("Failed to process batches");

    // Should have no batches
    assert_eq!(batches.len(), 0);

    Ok(())
}

/// Test batching only processes pending emails
#[sqlx::test]
async fn test_batching_only_pending(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);
    let batch_service = EmailBatchingService::with_defaults(pool.clone());

    // Create pending email
    create_email_delivery(&pool, &queue_service, "pending@test.com").await;

    // Create and mark as delivered
    let delivered_id = create_email_delivery(&pool, &queue_service, "pending@test.com").await;
    sqlx::query!(
        "UPDATE alert_delivery_queue SET status = 'delivered', delivered_at = NOW() WHERE id = $1",
        delivered_id
    )
    .execute(&pool)
    .await?;

    // Create and mark as failed
    let failed_id = create_email_delivery(&pool, &queue_service, "pending@test.com").await;
    sqlx::query!(
        "UPDATE alert_delivery_queue SET status = 'failed', attempts = 3 WHERE id = $1",
        failed_id
    )
    .execute(&pool)
    .await?;

    // Process batches
    let batches = batch_service
        .process_batches()
        .await
        .expect("Failed to process batches");

    // Should only have 1 email in batch (pending only)
    assert_eq!(batches.len(), 1);
    let batch = batches.get("pending@test.com").expect("Missing batch");
    assert_eq!(batch.delivery_ids.len(), 1);

    Ok(())
}
