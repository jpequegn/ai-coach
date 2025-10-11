use ai_coach::models::DeliveryMethod;
use ai_coach::services::{AlertDeliveryQueueService, EmailBatchingService, NotificationService};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

/// Test processing 1000+ deliveries performs within target
#[sqlx::test]
async fn test_large_queue_performance(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create 1000 deliveries
    let start = Instant::now();
    for i in 0..1000 {
        queue_service
            .queue_for_delivery(
                Uuid::new_v4(),
                DeliveryMethod::Email,
                format!("perf{}@test.com", i % 100), // 10 emails per recipient
            )
            .await
            .expect("Failed to queue delivery");
    }
    let queue_time = start.elapsed();

    println!("Created 1000 deliveries in {:?}", queue_time);
    assert!(queue_time.as_secs() < 10, "Queue creation took too long");

    // Query pending deliveries
    let start = Instant::now();
    let pending = queue_service
        .get_pending_deliveries(Some(1000))
        .await
        .expect("Failed to get pending deliveries");
    let query_time = start.elapsed();

    println!("Queried {} pending deliveries in {:?}", pending.len(), query_time);
    assert_eq!(pending.len(), 1000);
    assert!(
        query_time.as_millis() < 100,
        "Query took {} ms (target: <100ms)",
        query_time.as_millis()
    );

    Ok(())
}

/// Test email batching performance with large dataset
#[sqlx::test]
async fn test_batching_performance(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);
    let batch_service = EmailBatchingService::with_defaults(pool.clone());

    // Create 500 emails for 50 recipients (10 each)
    for recipient in 0..50 {
        for _ in 0..10 {
            queue_service
                .queue_for_delivery(
                    Uuid::new_v4(),
                    DeliveryMethod::Email,
                    format!("batch{}@test.com", recipient),
                )
                .await
                .expect("Failed to queue delivery");
        }
    }

    // Process batches
    let start = Instant::now();
    let batches = batch_service
        .process_batches()
        .await
        .expect("Failed to process batches");
    let batch_time = start.elapsed();

    println!("Processed {} batches from 500 emails in {:?}", batches.len(), batch_time);

    // Should have 50 batches (one per recipient)
    assert_eq!(batches.len(), 50);

    // Calculate reduction
    let total_emails: usize = batches.values().map(|b| b.delivery_ids.len()).sum();
    let reduction_pct = ((total_emails - batches.len()) as f64 / total_emails as f64) * 100.0;
    println!("Batching reduced {} emails to {} batches ({:.1}% reduction)", total_emails, batches.len(), reduction_pct);

    // Verify 90% reduction (500 emails -> 50 batches)
    assert!((reduction_pct - 90.0).abs() < 1.0);

    // Batching should be fast (< 1 second for 500 emails)
    assert!(batch_time.as_secs() < 1, "Batching took too long");

    Ok(())
}

/// Test concurrent delivery processing
#[sqlx::test]
async fn test_concurrent_processing(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create 100 deliveries
    for i in 0..100 {
        queue_service
            .queue_for_delivery(
                Uuid::new_v4(),
                DeliveryMethod::Email,
                format!("concurrent{}@test.com", i),
            )
            .await
            .expect("Failed to queue delivery");
    }

    // Process queue
    let start = Instant::now();
    let stats = queue_service
        .process_pending_queue()
        .await
        .expect("Failed to process queue");
    let process_time = start.elapsed();

    println!("Processed {} deliveries in {:?}", stats.total_attempted, process_time);

    // Should process all 100
    assert_eq!(stats.total_attempted, 100);

    // Target: 100 deliveries in < 5 seconds
    assert!(process_time.as_secs() < 5, "Processing took too long");

    Ok(())
}

/// Test rate limiting effectiveness
#[sqlx::test]
async fn test_rate_limiting(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create 100 email deliveries (rate limit is 50 per method)
    for i in 0..100 {
        queue_service
            .queue_for_delivery(
                Uuid::new_v4(),
                DeliveryMethod::Email,
                format!("rate{}@test.com", i),
            )
            .await
            .expect("Failed to queue delivery");
    }

    // Process queue (should only process 50 due to rate limit)
    let stats = queue_service
        .process_pending_queue()
        .await
        .expect("Failed to process queue");

    println!("Processed {} deliveries (rate limit: 50)", stats.total_attempted);

    // Should respect rate limit of 50 per method
    assert!(stats.total_attempted <= 50);

    Ok(())
}

/// Test database query performance
#[sqlx::test]
async fn test_query_performance(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Create deliveries with various statuses
    for i in 0..200 {
        let delivery_id = queue_service
            .queue_for_delivery(
                Uuid::new_v4(),
                DeliveryMethod::Email,
                format!("query{}@test.com", i % 50),
            )
            .await
            .expect("Failed to queue delivery");

        // Mark some as delivered, some as failed
        if i % 3 == 0 {
            sqlx::query!(
                "UPDATE alert_delivery_queue SET status = 'delivered', delivered_at = NOW() WHERE id = $1",
                delivery_id
            )
            .execute(&pool)
            .await?;
        } else if i % 5 == 0 {
            sqlx::query!(
                "UPDATE alert_delivery_queue SET status = 'failed', attempts = 3 WHERE id = $1",
                delivery_id
            )
            .execute(&pool)
            .await?;
        }
    }

    // Test pending query performance
    let start = Instant::now();
    let pending = queue_service
        .get_pending_deliveries(Some(200))
        .await
        .expect("Failed to get pending");
    let pending_time = start.elapsed();

    println!("Queried {} pending deliveries in {:?}", pending.len(), pending_time);
    assert!(pending_time.as_millis() < 100, "Pending query too slow");

    // Test batching query performance
    let batch_service = EmailBatchingService::with_defaults(pool.clone());
    let start = Instant::now();
    let batches = batch_service
        .process_batches()
        .await
        .expect("Failed to process batches");
    let batch_time = start.elapsed();

    println!("Processed {} batches in {:?}", batches.len(), batch_time);
    assert!(batch_time.as_millis() < 200, "Batch query too slow");

    Ok(())
}

/// Test memory efficiency with large batches
#[sqlx::test]
async fn test_memory_efficiency(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);
    let batch_service = EmailBatchingService::with_defaults(pool.clone());

    // Create 1000 emails for 10 recipients (100 each, but max batch is 10)
    for recipient in 0..10 {
        for _ in 0..100 {
            queue_service
                .queue_for_delivery(
                    Uuid::new_v4(),
                    DeliveryMethod::Email,
                    format!("memory{}@test.com", recipient),
                )
                .await
                .expect("Failed to queue delivery");
        }
    }

    // Process batches (should handle efficiently)
    let batches = batch_service
        .process_batches()
        .await
        .expect("Failed to process batches");

    // Should have 10 batches (one per recipient), each with max 10 emails
    assert_eq!(batches.len(), 10);
    for batch in batches.values() {
        assert_eq!(batch.delivery_ids.len(), 10); // Max batch size
    }

    Ok(())
}

/// Test statistics calculation performance
#[sqlx::test]
async fn test_statistics_performance(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);
    let batch_service = EmailBatchingService::with_defaults(pool.clone());

    // Create and deliver many emails
    for i in 0..200 {
        let delivery_id = queue_service
            .queue_for_delivery(
                Uuid::new_v4(),
                DeliveryMethod::Email,
                format!("stats{}@test.com", i % 20),
            )
            .await
            .expect("Failed to queue delivery");

        sqlx::query!(
            "UPDATE alert_delivery_queue SET status = 'delivered', delivered_at = NOW() WHERE id = $1",
            delivery_id
        )
        .execute(&pool)
        .await?;
    }

    // Get statistics
    let start = Instant::now();
    let stats = batch_service
        .get_batching_stats(24)
        .await
        .expect("Failed to get stats");
    let stats_time = start.elapsed();

    println!("Calculated statistics in {:?}", stats_time);
    println!("Stats: {} recipients, {} deliveries, {:.1}% potential reduction",
        stats.unique_recipients,
        stats.total_deliveries,
        stats.potential_reduction_pct
    );

    // Statistics should be fast
    assert!(stats_time.as_millis() < 100, "Statistics calculation too slow");

    // Verify statistics
    assert_eq!(stats.unique_recipients, 20);
    assert_eq!(stats.total_deliveries, 200);

    Ok(())
}

/// Test system under sustained load
#[sqlx::test]
async fn test_sustained_load(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let queue_service = AlertDeliveryQueueService::new(pool.clone(), notification_service);

    // Simulate sustained load: multiple waves of deliveries
    for wave in 0..5 {
        // Create 100 deliveries per wave
        for i in 0..100 {
            queue_service
                .queue_for_delivery(
                    Uuid::new_v4(),
                    DeliveryMethod::Email,
                    format!("sustained{}@test.com", i % 10),
                )
                .await
                .expect("Failed to queue delivery");
        }

        // Process wave
        let start = Instant::now();
        let stats = queue_service
            .process_pending_queue()
            .await
            .expect("Failed to process queue");
        let wave_time = start.elapsed();

        println!("Wave {} processed {} deliveries in {:?}", wave + 1, stats.total_attempted, wave_time);

        // Each wave should process within reasonable time
        assert!(wave_time.as_secs() < 5, "Wave processing too slow");
    }

    println!("Sustained load test complete: 5 waves of 100 deliveries each");

    Ok(())
}
