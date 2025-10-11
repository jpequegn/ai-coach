use ai_coach::models::{AlertDeliveryQueue, DeliveryMethod, DeliveryQueueStatus};
use ai_coach::services::{AlertDeliveryQueueService, NotificationService};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create a test delivery
async fn create_test_delivery(
    pool: &PgPool,
    service: &AlertDeliveryQueueService,
    method: DeliveryMethod,
    recipient: &str,
) -> Uuid {
    service
        .queue_for_delivery(Uuid::new_v4(), method, recipient.to_string())
        .await
        .expect("Failed to create test delivery")
}

/// Helper to mark delivery as failed
async fn mark_as_failed(pool: &PgPool, delivery_id: Uuid, error: &str) {
    sqlx::query!(
        r#"
        UPDATE alert_delivery_queue
        SET status = 'failed',
            attempts = 3,
            error_message = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
        delivery_id,
        error
    )
    .execute(pool)
    .await
    .expect("Failed to mark as failed");
}

/// Helper to mark delivery as delivered
async fn mark_as_delivered(pool: &PgPool, delivery_id: Uuid) {
    sqlx::query!(
        r#"
        UPDATE alert_delivery_queue
        SET status = 'delivered',
            delivered_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        "#,
        delivery_id
    )
    .execute(pool)
    .await
    .expect("Failed to mark as delivered");
}

/// Test getting queue status
#[sqlx::test]
async fn test_get_queue_status(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create test deliveries with various statuses
    let pending_id = create_test_delivery(&pool, &service, DeliveryMethod::Email, "pending@test.com").await;
    let delivered_id = create_test_delivery(&pool, &service, DeliveryMethod::Push, "device-123").await;
    let failed_id = create_test_delivery(&pool, &service, DeliveryMethod::Sms, "+1234567890").await;

    mark_as_delivered(&pool, delivered_id).await;
    mark_as_failed(&pool, failed_id, "Network timeout").await;

    // Query status counts manually (simulating the endpoint logic)
    let status_counts = sqlx::query!(
        r#"
        SELECT
            status as "status: DeliveryQueueStatus",
            COUNT(*) as count
        FROM alert_delivery_queue
        GROUP BY status
        "#
    )
    .fetch_all(&pool)
    .await?;

    // Verify counts
    let mut pending_count = 0;
    let mut delivered_count = 0;
    let mut failed_count = 0;

    for row in status_counts {
        match row.status {
            DeliveryQueueStatus::Pending => pending_count = row.count.unwrap_or(0),
            DeliveryQueueStatus::Delivered => delivered_count = row.count.unwrap_or(0),
            DeliveryQueueStatus::Failed => failed_count = row.count.unwrap_or(0),
            _ => {}
        }
    }

    assert_eq!(pending_count, 1);
    assert_eq!(delivered_count, 1);
    assert_eq!(failed_count, 1);

    Ok(())
}

/// Test getting failed deliveries
#[sqlx::test]
async fn test_get_failed_deliveries(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create multiple failed deliveries
    for i in 0..5 {
        let delivery_id = create_test_delivery(
            &pool,
            &service,
            DeliveryMethod::Email,
            &format!("failed{}@test.com", i),
        )
        .await;
        mark_as_failed(&pool, delivery_id, &format!("Error {}", i)).await;
    }

    // Query failed deliveries
    let failed = sqlx::query_as!(
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
        WHERE status = 'failed'
        ORDER BY updated_at DESC
        LIMIT 10
        "#
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(failed.len(), 5);
    for delivery in &failed {
        assert_eq!(delivery.status, DeliveryQueueStatus::Failed);
        assert!(delivery.error_message.is_some());
    }

    Ok(())
}

/// Test getting failed deliveries with method filter
#[sqlx::test]
async fn test_get_failed_deliveries_with_filter(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create failed deliveries with different methods
    let email_id = create_test_delivery(&pool, &service, DeliveryMethod::Email, "email@test.com").await;
    let push_id = create_test_delivery(&pool, &service, DeliveryMethod::Push, "device-456").await;
    let sms_id = create_test_delivery(&pool, &service, DeliveryMethod::Sms, "+1234567890").await;

    mark_as_failed(&pool, email_id, "Email error").await;
    mark_as_failed(&pool, push_id, "Push error").await;
    mark_as_failed(&pool, sms_id, "SMS error").await;

    // Query failed deliveries filtered by Email
    let email_failed = sqlx::query_as!(
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
        WHERE status = 'failed' AND delivery_method = $1::text
        ORDER BY updated_at DESC
        "#,
        DeliveryMethod::Email as DeliveryMethod
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(email_failed.len(), 1);
    assert_eq!(email_failed[0].delivery_method, DeliveryMethod::Email);

    Ok(())
}

/// Test manual retry
#[sqlx::test]
async fn test_manual_retry(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create and fail a delivery
    let delivery_id = create_test_delivery(&pool, &service, DeliveryMethod::Email, "retry@test.com").await;
    mark_as_failed(&pool, delivery_id, "Temporary failure").await;

    // Verify initial state
    let before = sqlx::query_as!(
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
        delivery_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(before.status, DeliveryQueueStatus::Failed);
    assert_eq!(before.attempts, 3);

    // Perform manual retry
    let updated = sqlx::query_as!(
        AlertDeliveryQueue,
        r#"
        UPDATE alert_delivery_queue
        SET attempts = 0,
            status = 'pending',
            next_retry_at = NOW(),
            error_message = NULL,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id, alert_id,
            delivery_method as "delivery_method: DeliveryMethod",
            recipient_id, attempts, max_attempts,
            last_attempt_at, next_retry_at,
            status as "status: DeliveryQueueStatus",
            error_message, delivered_at,
            created_at, updated_at
        "#,
        delivery_id
    )
    .fetch_one(&pool)
    .await?;

    // Verify retry was successful
    assert_eq!(updated.status, DeliveryQueueStatus::Pending);
    assert_eq!(updated.attempts, 0);
    assert!(updated.next_retry_at.is_some());
    assert!(updated.error_message.is_none());

    Ok(())
}

/// Test canceling a delivery
#[sqlx::test]
async fn test_cancel_delivery(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create a pending delivery
    let delivery_id = create_test_delivery(&pool, &service, DeliveryMethod::Push, "device-789").await;

    // Verify initial state
    let before = sqlx::query_as!(
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
        delivery_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(before.status, DeliveryQueueStatus::Pending);

    // Cancel the delivery
    let reason = "Cancelled by admin for testing";
    let result = sqlx::query!(
        r#"
        UPDATE alert_delivery_queue
        SET status = 'cancelled',
            error_message = $2,
            updated_at = NOW()
        WHERE id = $1 AND status = 'pending'
        "#,
        delivery_id,
        reason
    )
    .execute(&pool)
    .await?;

    assert_eq!(result.rows_affected(), 1);

    // Verify cancellation
    let after = sqlx::query_as!(
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
        delivery_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(after.status, DeliveryQueueStatus::Cancelled);
    assert_eq!(after.error_message, Some(reason.to_string()));

    Ok(())
}

/// Test canceling non-pending delivery fails
#[sqlx::test]
async fn test_cancel_non_pending_delivery(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create and deliver a delivery
    let delivery_id = create_test_delivery(&pool, &service, DeliveryMethod::Email, "delivered@test.com").await;
    mark_as_delivered(&pool, delivery_id).await;

    // Try to cancel delivered delivery (should affect 0 rows)
    let result = sqlx::query!(
        r#"
        UPDATE alert_delivery_queue
        SET status = 'cancelled',
            error_message = $2,
            updated_at = NOW()
        WHERE id = $1 AND status = 'pending'
        "#,
        delivery_id,
        "Should not work"
    )
    .execute(&pool)
    .await?;

    assert_eq!(result.rows_affected(), 0);

    Ok(())
}

/// Test delivery statistics calculation
#[sqlx::test]
async fn test_delivery_statistics(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create deliveries with various outcomes
    let delivered1 = create_test_delivery(&pool, &service, DeliveryMethod::Email, "stat1@test.com").await;
    let delivered2 = create_test_delivery(&pool, &service, DeliveryMethod::Email, "stat2@test.com").await;
    let failed1 = create_test_delivery(&pool, &service, DeliveryMethod::Push, "device-stat-1").await;

    mark_as_delivered(&pool, delivered1).await;
    mark_as_delivered(&pool, delivered2).await;
    mark_as_failed(&pool, failed1, "Test failure").await;

    // Calculate statistics
    let start_time = chrono::Utc::now() - chrono::Duration::days(1);
    let end_time = chrono::Utc::now();

    let overall = sqlx::query!(
        r#"
        SELECT
            COUNT(*) as total_attempts,
            COUNT(*) FILTER (WHERE status = 'delivered') as successful,
            COUNT(*) FILTER (WHERE status = 'failed') as failed
        FROM alert_delivery_queue
        WHERE created_at BETWEEN $1 AND $2
        "#,
        start_time,
        end_time
    )
    .fetch_one(&pool)
    .await?;

    let total = overall.total_attempts.unwrap_or(0);
    let successful = overall.successful.unwrap_or(0);
    let failed = overall.failed.unwrap_or(0);

    assert_eq!(total, 3);
    assert_eq!(successful, 2);
    assert_eq!(failed, 1);

    let success_rate = (successful as f64 / total as f64) * 100.0;
    assert!((success_rate - 66.67).abs() < 0.1);

    Ok(())
}

/// Test method-specific statistics
#[sqlx::test]
async fn test_method_statistics(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create deliveries for different methods
    let email1 = create_test_delivery(&pool, &service, DeliveryMethod::Email, "method1@test.com").await;
    let email2 = create_test_delivery(&pool, &service, DeliveryMethod::Email, "method2@test.com").await;
    let push1 = create_test_delivery(&pool, &service, DeliveryMethod::Push, "device-method-1").await;

    mark_as_delivered(&pool, email1).await;
    mark_as_failed(&pool, email2, "Email failure").await;
    mark_as_delivered(&pool, push1).await;

    // Calculate method statistics
    let start_time = chrono::Utc::now() - chrono::Duration::days(1);
    let end_time = chrono::Utc::now();

    let method_stats = sqlx::query!(
        r#"
        SELECT
            delivery_method as "method: DeliveryMethod",
            COUNT(*) as attempts,
            COUNT(*) FILTER (WHERE status = 'delivered') as successful,
            COUNT(*) FILTER (WHERE status = 'failed') as failed
        FROM alert_delivery_queue
        WHERE created_at BETWEEN $1 AND $2
        GROUP BY delivery_method
        "#,
        start_time,
        end_time
    )
    .fetch_all(&pool)
    .await?;

    // Verify method statistics
    for row in method_stats {
        let attempts = row.attempts.unwrap_or(0);
        let successful = row.successful.unwrap_or(0);
        let failed = row.failed.unwrap_or(0);

        match row.method {
            DeliveryMethod::Email => {
                assert_eq!(attempts, 2);
                assert_eq!(successful, 1);
                assert_eq!(failed, 1);
            }
            DeliveryMethod::Push => {
                assert_eq!(attempts, 1);
                assert_eq!(successful, 1);
                assert_eq!(failed, 0);
            }
            _ => {}
        }
    }

    Ok(())
}

/// Test pagination of failed deliveries
#[sqlx::test]
async fn test_failed_deliveries_pagination(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create many failed deliveries
    for i in 0..15 {
        let delivery_id = create_test_delivery(
            &pool,
            &service,
            DeliveryMethod::Email,
            &format!("page{}@test.com", i),
        )
        .await;
        mark_as_failed(&pool, delivery_id, &format!("Error {}", i)).await;
    }

    // Test first page
    let page1 = sqlx::query_as!(
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
        WHERE status = 'failed'
        ORDER BY updated_at DESC
        LIMIT 10 OFFSET 0
        "#
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(page1.len(), 10);

    // Test second page
    let page2 = sqlx::query_as!(
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
        WHERE status = 'failed'
        ORDER BY updated_at DESC
        LIMIT 10 OFFSET 10
        "#
    )
    .fetch_all(&pool)
    .await?;

    assert_eq!(page2.len(), 5);

    Ok(())
}
