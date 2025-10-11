use ai_coach::models::DataQualityMetrics;
use ai_coach::services::{DataQualityCheckJob, NotificationService};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create test user
async fn create_test_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"
        INSERT INTO users (email, password_hash, email_verified)
        VALUES ($1, 'test_hash', true)
        RETURNING id
        "#,
        email
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create test user")
}

/// Helper to create recovery data for a user
async fn create_recovery_data(
    pool: &PgPool,
    user_id: Uuid,
    date: chrono::NaiveDate,
    hrv_score: Option<f64>,
    sleep_score: Option<f64>,
    resting_hr: Option<i32>,
    data_source: &str,
) {
    sqlx::query!(
        r#"
        INSERT INTO recovery_data (user_id, date, hrv_score, sleep_score, resting_hr, data_source)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        user_id,
        date,
        hrv_score,
        sleep_score,
        resting_hr,
        data_source
    )
    .execute(pool)
    .await
    .expect("Failed to create recovery data");
}

/// Test completeness score with full data (100%)
#[sqlx::test]
async fn test_completeness_full_data(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let job = DataQualityCheckJob::new(pool.clone(), notification_service);

    let user_id = create_test_user(&pool, "full@test.com").await;

    // Create recovery data for all 30 days
    let today = Utc::now().date_naive();
    for i in 0..30 {
        let date = today - Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(85.0), Some(80.0), Some(55), "api_integration").await;
    }

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    // Check job stats
    assert_eq!(stats.records_processed, 1);
    assert_eq!(stats.records_failed, 0);

    // Verify metrics stored
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id, metric_date,
            completeness_score, consistency_score, reliability_score,
            last_data_timestamp, days_without_data,
            data_sources, created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1 AND metric_date = $2
        "#,
        user_id,
        today
    )
    .fetch_one(&pool)
    .await?;

    // Completeness should be 100% (30 days with data / 30 days)
    assert!((metrics.completeness_score - 100.0).abs() < 0.1);
    assert_eq!(metrics.days_without_data, 0);

    Ok(())
}

/// Test completeness score with partial data (50%)
#[sqlx::test]
async fn test_completeness_partial_data(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let job = DataQualityCheckJob::new(pool.clone(), notification_service);

    let user_id = create_test_user(&pool, "partial@test.com").await;

    // Create recovery data for only 15 out of 30 days
    let today = Utc::now().date_naive();
    for i in 0..30 {
        if i % 2 == 0 {
            // Only even days
            let date = today - Duration::days(i);
            create_recovery_data(&pool, user_id, date, Some(85.0), None, None, "manual").await;
        }
    }

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    assert_eq!(stats.records_processed, 1);
    assert_eq!(stats.records_failed, 0);

    // Verify metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id, metric_date,
            completeness_score, consistency_score, reliability_score,
            last_data_timestamp, days_without_data,
            data_sources, created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1 AND metric_date = $2
        "#,
        user_id,
        today
    )
    .fetch_one(&pool)
    .await?;

    // Completeness should be 50% (15 days with data / 30 days)
    assert!((metrics.completeness_score - 50.0).abs() < 0.1);

    Ok(())
}

/// Test consistency score with regular data (high score)
#[sqlx::test]
async fn test_consistency_regular_data(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let job = DataQualityCheckJob::new(pool.clone(), notification_service);

    let user_id = create_test_user(&pool, "consistent@test.com").await;

    // Create recovery data every day (no gaps)
    let today = Utc::now().date_naive();
    for i in 0..30 {
        let date = today - Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(85.0), Some(80.0), Some(55), "api_integration").await;
    }

    // Execute job
    job.execute().await.expect("Job execution failed");

    // Verify metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id, metric_date,
            completeness_score, consistency_score, reliability_score,
            last_data_timestamp, days_without_data,
            data_sources, created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1 AND metric_date = $2
        "#,
        user_id,
        today
    )
    .fetch_one(&pool)
    .await?;

    // Consistency should be high (100%) - no gaps
    let consistency = metrics.consistency_score.expect("Consistency score missing");
    assert!(consistency > 99.0);

    Ok(())
}

/// Test consistency score with gaps (low score)
#[sqlx::test]
async fn test_consistency_with_gaps(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let job = DataQualityCheckJob::new(pool.clone(), notification_service);

    let user_id = create_test_user(&pool, "gaps@test.com").await;

    // Create recovery data with large gaps
    let today = Utc::now().date_naive();
    create_recovery_data(&pool, user_id, today, Some(85.0), None, None, "manual").await;
    create_recovery_data(&pool, user_id, today - Duration::days(10), Some(85.0), None, None, "manual").await;
    create_recovery_data(&pool, user_id, today - Duration::days(20), Some(85.0), None, None, "manual").await;

    // Execute job
    job.execute().await.expect("Job execution failed");

    // Verify metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id, metric_date,
            completeness_score, consistency_score, reliability_score,
            last_data_timestamp, days_without_data,
            data_sources, created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1 AND metric_date = $2
        "#,
        user_id,
        today
    )
    .fetch_one(&pool)
    .await?;

    // Consistency should be low due to large gaps
    let consistency = metrics.consistency_score.expect("Consistency score missing");
    assert!(consistency < 50.0);

    Ok(())
}

/// Test reliability score with API integration source (95%)
#[sqlx::test]
async fn test_reliability_api_integration(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let job = DataQualityCheckJob::new(pool.clone(), notification_service);

    let user_id = create_test_user(&pool, "api@test.com").await;

    // Create recovery data from API integration (highest reliability)
    let today = Utc::now().date_naive();
    for i in 0..10 {
        let date = today - Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(85.0), Some(80.0), Some(55), "api_integration").await;
    }

    // Execute job
    job.execute().await.expect("Job execution failed");

    // Verify metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id, metric_date,
            completeness_score, consistency_score, reliability_score,
            last_data_timestamp, days_without_data,
            data_sources, created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1 AND metric_date = $2
        "#,
        user_id,
        today
    )
    .fetch_one(&pool)
    .await?;

    // Reliability should be high (95% for api_integration with fresh data)
    let reliability = metrics.reliability_score.expect("Reliability score missing");
    assert!(reliability > 90.0);

    Ok(())
}

/// Test reliability score with manual entry (70%)
#[sqlx::test]
async fn test_reliability_manual_entry(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let job = DataQualityCheckJob::new(pool.clone(), notification_service);

    let user_id = create_test_user(&pool, "manual@test.com").await;

    // Create recovery data from manual entry (lower reliability)
    let today = Utc::now().date_naive();
    for i in 0..10 {
        let date = today - Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(85.0), None, None, "manual").await;
    }

    // Execute job
    job.execute().await.expect("Job execution failed");

    // Verify metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id, metric_date,
            completeness_score, consistency_score, reliability_score,
            last_data_timestamp, days_without_data,
            data_sources, created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1 AND metric_date = $2
        "#,
        user_id,
        today
    )
    .fetch_one(&pool)
    .await?;

    // Reliability should be moderate (70% for manual entry with fresh data)
    let reliability = metrics.reliability_score.expect("Reliability score missing");
    assert!(reliability > 60.0 && reliability < 80.0);

    Ok(())
}

/// Test job execution with multiple users
#[sqlx::test]
async fn test_multiple_users(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let job = DataQualityCheckJob::new(pool.clone(), notification_service);

    // Create 3 test users with different data patterns
    let user1 = create_test_user(&pool, "user1@test.com").await;
    let user2 = create_test_user(&pool, "user2@test.com").await;
    let user3 = create_test_user(&pool, "user3@test.com").await;

    let today = Utc::now().date_naive();

    // User 1: Full data
    for i in 0..30 {
        let date = today - Duration::days(i);
        create_recovery_data(&pool, user1, date, Some(85.0), Some(80.0), Some(55), "api_integration").await;
    }

    // User 2: Partial data
    for i in 0..15 {
        let date = today - Duration::days(i * 2);
        create_recovery_data(&pool, user2, date, Some(85.0), None, None, "manual").await;
    }

    // User 3: Minimal data
    for i in 0..5 {
        let date = today - Duration::days(i);
        create_recovery_data(&pool, user3, date, Some(85.0), None, None, "wearable").await;
    }

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    // Should process all 3 users
    assert_eq!(stats.records_processed, 3);
    assert_eq!(stats.records_failed, 0);

    // Verify all users have metrics
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) as \"count!\" FROM data_quality_metrics WHERE metric_date = $1",
        today
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, 3);

    Ok(())
}

/// Test job execution with no users
#[sqlx::test]
async fn test_no_users(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let job = DataQualityCheckJob::new(pool.clone(), notification_service);

    // Execute job with no active users
    let stats = job.execute().await.expect("Job execution failed");

    assert_eq!(stats.records_processed, 0);
    assert_eq!(stats.records_failed, 0);
    assert_eq!(stats.status, "success");

    Ok(())
}

/// Test metrics upsert (verify daily records created)
#[sqlx::test]
async fn test_metrics_upsert(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let job = DataQualityCheckJob::new(pool.clone(), notification_service);

    let user_id = create_test_user(&pool, "upsert@test.com").await;
    let today = Utc::now().date_naive();

    // Create initial data
    for i in 0..10 {
        let date = today - Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(85.0), Some(80.0), Some(55), "api_integration").await;
    }

    // Execute job first time
    job.execute().await.expect("Job execution failed");

    let metrics1 = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id, metric_date,
            completeness_score, consistency_score, reliability_score,
            last_data_timestamp, days_without_data,
            data_sources, created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1 AND metric_date = $2
        "#,
        user_id,
        today
    )
    .fetch_one(&pool)
    .await?;

    let first_updated_at = metrics1.updated_at;

    // Add more data
    for i in 10..20 {
        let date = today - Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(85.0), Some(80.0), Some(55), "api_integration").await;
    }

    // Wait a moment to ensure updated_at changes
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Execute job second time (should upsert)
    job.execute().await.expect("Job execution failed");

    let metrics2 = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id, metric_date,
            completeness_score, consistency_score, reliability_score,
            last_data_timestamp, days_without_data,
            data_sources, created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1 AND metric_date = $2
        "#,
        user_id,
        today
    )
    .fetch_one(&pool)
    .await?;

    // Should be same record (same id) but updated
    assert_eq!(metrics1.id, metrics2.id);
    assert!(metrics2.updated_at > first_updated_at);

    // Completeness should improve (now 20 days / 30 days)
    assert!(metrics2.completeness_score > metrics1.completeness_score);

    Ok(())
}

/// Test reminder sending logic
#[sqlx::test]
async fn test_reminder_logic(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let job = DataQualityCheckJob::new(pool.clone(), notification_service);

    // User with poor completeness (<50%)
    let user1 = create_test_user(&pool, "poor@test.com").await;
    let today = Utc::now().date_naive();

    // Only 10 days of data (33% completeness)
    for i in 0..10 {
        let date = today - Duration::days(i);
        create_recovery_data(&pool, user1, date, Some(85.0), None, None, "manual").await;
    }

    // User with gap >= 3 days
    let user2 = create_test_user(&pool, "gap@test.com").await;

    // Last data 4 days ago
    create_recovery_data(&pool, user2, today - Duration::days(4), Some(85.0), None, None, "manual").await;

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    // Both users processed
    assert_eq!(stats.records_processed, 2);

    // Verify metrics stored with correct days_without_data
    let metrics1 = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id, metric_date,
            completeness_score, consistency_score, reliability_score,
            last_data_timestamp, days_without_data,
            data_sources, created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1 AND metric_date = $2
        "#,
        user1,
        today
    )
    .fetch_one(&pool)
    .await?;

    let metrics2 = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id, metric_date,
            completeness_score, consistency_score, reliability_score,
            last_data_timestamp, days_without_data,
            data_sources, created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1 AND metric_date = $2
        "#,
        user2,
        today
    )
    .fetch_one(&pool)
    .await?;

    // User1: Low completeness triggers reminder
    assert!(metrics1.completeness_score < 50.0);
    assert_eq!(metrics1.days_without_data, 0);

    // User2: Gap >= 3 days triggers reminder
    assert_eq!(metrics2.days_without_data, 4);

    Ok(())
}

/// Test job metadata (name, schedule)
#[test]
fn test_job_metadata() {
    assert_eq!(DataQualityCheckJob::get_job_name(), "data_quality_check");
    assert_eq!(DataQualityCheckJob::get_schedule(), "0 2 * * *");
}

/// Test score calculation edge cases
#[sqlx::test]
async fn test_score_edge_cases(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let job = DataQualityCheckJob::new(pool.clone(), notification_service);

    // User with no data at all
    let user1 = create_test_user(&pool, "nodata@test.com").await;

    // User with only very old data (>60 days ago)
    let user2 = create_test_user(&pool, "olddata@test.com").await;
    let old_date = Utc::now().date_naive() - Duration::days(65);
    create_recovery_data(&pool, user2, old_date, Some(85.0), None, None, "manual").await;

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    // No users processed (both outside 60-day window)
    assert_eq!(stats.records_processed, 0);
    assert_eq!(stats.status, "success");

    Ok(())
}

/// Test data sources status tracking
#[sqlx::test]
async fn test_data_sources_status(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let job = DataQualityCheckJob::new(pool.clone(), notification_service);

    let user_id = create_test_user(&pool, "sources@test.com").await;
    let today = Utc::now().date_naive();

    // Create data with different metrics in last 7 days
    for i in 0..7 {
        let date = today - Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(85.0), Some(80.0), Some(55), "api_integration").await;
    }

    // Execute job
    job.execute().await.expect("Job execution failed");

    // Verify data_sources field is populated
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id, metric_date,
            completeness_score, consistency_score, reliability_score,
            last_data_timestamp, days_without_data,
            data_sources, created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1 AND metric_date = $2
        "#,
        user_id,
        today
    )
    .fetch_one(&pool)
    .await?;

    // Verify data_sources JSON contains expected fields
    assert!(metrics.data_sources.is_some());
    let sources = metrics.data_sources.unwrap();
    assert!(sources.get("hrv").is_some());
    assert!(sources.get("sleep").is_some());
    assert!(sources.get("rhr").is_some());

    Ok(())
}
