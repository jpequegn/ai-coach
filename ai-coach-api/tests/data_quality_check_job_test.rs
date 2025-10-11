use ai_coach::models::{DataQualityMetrics, UrgencyLevel};
use ai_coach::services::DataQualityCheckJob;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create test user with recovery data
async fn create_test_user_with_data(
    pool: &PgPool,
    hrv_days: Vec<i32>,
    sleep_days: Vec<i32>,
    rhr_days: Vec<i32>,
    wearable_connected: bool,
) -> Uuid {
    // Create user
    let user_id = Uuid::new_v4();
    let email = format!("quality_test_{}@example.com", Uuid::new_v4());

    sqlx::query!(
        r#"
        INSERT INTO users (id, email, password_hash, oura_access_token)
        VALUES ($1, $2, 'test_hash', $3)
        "#,
        user_id,
        email,
        if wearable_connected { Some("test_token") } else { None }
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    // Create recovery data for specified days
    for day_offset in 0..30 {
        let date = (Utc::now() - Duration::days(day_offset)).date_naive();

        let hrv = if hrv_days.contains(&day_offset) { Some(75.0) } else { None };
        let sleep = if sleep_days.contains(&day_offset) { Some(80.0) } else { None };
        let rhr = if rhr_days.contains(&day_offset) { Some(60) } else { None };

        // Skip days with no data
        if hrv.is_none() && sleep.is_none() && rhr.is_none() {
            continue;
        }

        let data_source = if wearable_connected { "api_integration" } else { "manual" };

        sqlx::query!(
            r#"
            INSERT INTO recovery_data (
                user_id, date, hrv_score, sleep_score, resting_hr, data_source
            ) VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            user_id,
            date,
            hrv,
            sleep,
            rhr,
            data_source
        )
        .execute(pool)
        .await
        .expect("Failed to create recovery data");
    }

    user_id
}

/// Test completeness score calculation with full data
#[sqlx::test]
async fn test_completeness_score_full_data(pool: PgPool) -> sqlx::Result<()> {
    let job = DataQualityCheckJob::new(pool.clone());

    // Create user with data every day for 30 days
    let all_days: Vec<i32> = (0..30).collect();
    let user_id = create_test_user_with_data(&pool, all_days.clone(), all_days.clone(), all_days, true).await;

    // Process user
    job.process_user_data_quality(user_id).await.expect("Failed to process");

    // Check metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id,
            completeness_score, consistency_score, reliability_score, overall_score,
            days_analyzed, missing_hrv_days, missing_sleep_days, missing_rhr_days,
            last_hrv_reading, last_sleep_reading, last_rhr_reading,
            wearable_connected, reminder_sent_at,
            created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    // Full data = 100% completeness
    assert!((metrics.completeness_score - 1.0).abs() < 0.01);
    assert_eq!(metrics.days_analyzed, 30);
    assert_eq!(metrics.missing_hrv_days, 0);
    assert_eq!(metrics.missing_sleep_days, 0);
    assert_eq!(metrics.missing_rhr_days, 0);

    Ok(())
}

/// Test completeness score with partial data
#[sqlx::test]
async fn test_completeness_score_partial_data(pool: PgPool) -> sqlx::Result<()> {
    let job = DataQualityCheckJob::new(pool.clone());

    // Create user with data only 15 out of 30 days (50%)
    let half_days: Vec<i32> = (0..15).collect();
    let user_id = create_test_user_with_data(&pool, half_days.clone(), half_days.clone(), half_days, false).await;

    // Process user
    job.process_user_data_quality(user_id).await.expect("Failed to process");

    // Check metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id,
            completeness_score, consistency_score, reliability_score, overall_score,
            days_analyzed, missing_hrv_days, missing_sleep_days, missing_rhr_days,
            last_hrv_reading, last_sleep_reading, last_rhr_reading,
            wearable_connected, reminder_sent_at,
            created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    // Half data = 50% completeness
    assert!((metrics.completeness_score - 0.5).abs() < 0.01);
    assert_eq!(metrics.missing_hrv_days, 15);
    assert_eq!(metrics.missing_sleep_days, 15);
    assert_eq!(metrics.missing_rhr_days, 15);

    Ok(())
}

/// Test consistency score with regular data
#[sqlx::test]
async fn test_consistency_score_regular_data(pool: PgPool) -> sqlx::Result<()> {
    let job = DataQualityCheckJob::new(pool.clone());

    // Create user with consistent daily data (no gaps)
    let all_days: Vec<i32> = (0..30).collect();
    let user_id = create_test_user_with_data(&pool, all_days.clone(), all_days.clone(), all_days, true).await;

    // Process user
    job.process_user_data_quality(user_id).await.expect("Failed to process");

    // Check metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id,
            completeness_score, consistency_score, reliability_score, overall_score,
            days_analyzed, missing_hrv_days, missing_sleep_days, missing_rhr_days,
            last_hrv_reading, last_sleep_reading, last_rhr_reading,
            wearable_connected, reminder_sent_at,
            created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    // Perfect consistency (no gaps)
    assert!(metrics.consistency_score > 0.95);

    Ok(())
}

/// Test consistency score with gaps
#[sqlx::test]
async fn test_consistency_score_with_gaps(pool: PgPool) -> sqlx::Result<()> {
    let job = DataQualityCheckJob::new(pool.clone());

    // Create user with data, but large gap in middle (days 10-20 missing)
    let days_with_gaps: Vec<i32> = (0..10).chain(20..30).collect();
    let user_id = create_test_user_with_data(&pool, days_with_gaps.clone(), days_with_gaps.clone(), days_with_gaps, true).await;

    // Process user
    job.process_user_data_quality(user_id).await.expect("Failed to process");

    // Check metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id,
            completeness_score, consistency_score, reliability_score, overall_score,
            days_analyzed, missing_hrv_days, missing_sleep_days, missing_rhr_days,
            last_hrv_reading, last_sleep_reading, last_rhr_reading,
            wearable_connected, reminder_sent_at,
            created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    // Poor consistency due to 10-day gap
    assert!(metrics.consistency_score < 0.7);

    Ok(())
}

/// Test reliability score with API integration source
#[sqlx::test]
async fn test_reliability_score_api_integration(pool: PgPool) -> sqlx::Result<()> {
    let job = DataQualityCheckJob::new(pool.clone());

    // Create user with wearable connection (API integration)
    let all_days: Vec<i32> = (0..30).collect();
    let user_id = create_test_user_with_data(&pool, all_days.clone(), all_days.clone(), all_days, true).await;

    // Process user
    job.process_user_data_quality(user_id).await.expect("Failed to process");

    // Check metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id,
            completeness_score, consistency_score, reliability_score, overall_score,
            days_analyzed, missing_hrv_days, missing_sleep_days, missing_rhr_days,
            last_hrv_reading, last_sleep_reading, last_rhr_reading,
            wearable_connected, reminder_sent_at,
            created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    // API integration = high reliability (>0.9)
    assert!(metrics.reliability_score > 0.9);
    assert!(metrics.wearable_connected);

    Ok(())
}

/// Test reliability score with manual source
#[sqlx::test]
async fn test_reliability_score_manual(pool: PgPool) -> sqlx::Result<()> {
    let job = DataQualityCheckJob::new(pool.clone());

    // Create user without wearable (manual entry)
    let all_days: Vec<i32> = (0..30).collect();
    let user_id = create_test_user_with_data(&pool, all_days.clone(), all_days.clone(), all_days, false).await;

    // Process user
    job.process_user_data_quality(user_id).await.expect("Failed to process");

    // Check metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id,
            completeness_score, consistency_score, reliability_score, overall_score,
            days_analyzed, missing_hrv_days, missing_sleep_days, missing_rhr_days,
            last_hrv_reading, last_sleep_reading, last_rhr_reading,
            wearable_connected, reminder_sent_at,
            created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    // Manual entry = lower reliability (~0.7)
    assert!(metrics.reliability_score >= 0.6 && metrics.reliability_score <= 0.8);
    assert!(!metrics.wearable_connected);

    Ok(())
}

/// Test overall score calculation
#[sqlx::test]
async fn test_overall_score_calculation(pool: PgPool) -> sqlx::Result<()> {
    let job = DataQualityCheckJob::new(pool.clone());

    // Create user with good data
    let all_days: Vec<i32> = (0..30).collect();
    let user_id = create_test_user_with_data(&pool, all_days.clone(), all_days.clone(), all_days, true).await;

    // Process user
    job.process_user_data_quality(user_id).await.expect("Failed to process");

    // Check metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id,
            completeness_score, consistency_score, reliability_score, overall_score,
            days_analyzed, missing_hrv_days, missing_sleep_days, missing_rhr_days,
            last_hrv_reading, last_sleep_reading, last_rhr_reading,
            wearable_connected, reminder_sent_at,
            created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    // Overall = weighted average: 40% completeness, 30% consistency, 30% reliability
    let expected_overall = (metrics.completeness_score * 0.4)
        + (metrics.consistency_score * 0.3)
        + (metrics.reliability_score * 0.3);

    assert!((metrics.overall_score - expected_overall).abs() < 0.01);

    Ok(())
}

/// Test missing data detection
#[sqlx::test]
async fn test_missing_data_detection(pool: PgPool) -> sqlx::Result<()> {
    let job = DataQualityCheckJob::new(pool.clone());

    // Create user with only HRV data (sleep and RHR missing)
    let hrv_days: Vec<i32> = (0..30).collect();
    let sleep_days: Vec<i32> = (0..10).collect(); // Only 10 days
    let rhr_days: Vec<i32> = vec![]; // None
    let user_id = create_test_user_with_data(&pool, hrv_days, sleep_days, rhr_days, false).await;

    // Process user
    job.process_user_data_quality(user_id).await.expect("Failed to process");

    // Check metrics
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id,
            completeness_score, consistency_score, reliability_score, overall_score,
            days_analyzed, missing_hrv_days, missing_sleep_days, missing_rhr_days,
            last_hrv_reading, last_sleep_reading, last_rhr_reading,
            wearable_connected, reminder_sent_at,
            created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    // Verify missing day counts
    assert_eq!(metrics.missing_hrv_days, 0);
    assert_eq!(metrics.missing_sleep_days, 20);
    assert_eq!(metrics.missing_rhr_days, 30);

    // Verify last reading timestamps exist
    assert!(metrics.last_hrv_reading.is_some());
    assert!(metrics.last_sleep_reading.is_some());
    assert!(metrics.last_rhr_reading.is_none());

    Ok(())
}

/// Test job execution with multiple users
#[sqlx::test]
async fn test_job_execution_multiple_users(pool: PgPool) -> sqlx::Result<()> {
    let job = DataQualityCheckJob::new(pool.clone());

    // Create 3 users with different data patterns
    let all_days: Vec<i32> = (0..30).collect();
    let half_days: Vec<i32> = (0..15).collect();
    let sparse_days: Vec<i32> = vec![0, 5, 10, 15, 20, 25];

    let _user1 = create_test_user_with_data(&pool, all_days.clone(), all_days.clone(), all_days.clone(), true).await;
    let _user2 = create_test_user_with_data(&pool, half_days.clone(), half_days.clone(), half_days, false).await;
    let _user3 = create_test_user_with_data(&pool, sparse_days.clone(), sparse_days.clone(), sparse_days, false).await;

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    // Verify execution stats
    assert_eq!(stats.job_name, "data_quality_check");
    assert_eq!(stats.records_processed, 3);
    assert_eq!(stats.records_failed, 0);
    assert_eq!(stats.status, "success");

    // Verify all users have metrics
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) as \"count!\" FROM data_quality_metrics"
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, 3);

    Ok(())
}

/// Test job execution with no users
#[sqlx::test]
async fn test_job_execution_no_users(pool: PgPool) -> sqlx::Result<()> {
    let job = DataQualityCheckJob::new(pool.clone());

    // Execute job with no active users
    let stats = job.execute().await.expect("Job execution failed");

    // Verify execution stats
    assert_eq!(stats.records_processed, 0);
    assert_eq!(stats.records_failed, 0);
    assert_eq!(stats.status, "success");

    Ok(())
}

/// Test metrics upsert (update existing record)
#[sqlx::test]
async fn test_metrics_upsert(pool: PgPool) -> sqlx::Result<()> {
    let job = DataQualityCheckJob::new(pool.clone());

    // Create user
    let all_days: Vec<i32> = (0..30).collect();
    let user_id = create_test_user_with_data(&pool, all_days.clone(), all_days.clone(), all_days, true).await;

    // First execution
    job.process_user_data_quality(user_id).await.expect("Failed to process");

    let first_metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id,
            completeness_score, consistency_score, reliability_score, overall_score,
            days_analyzed, missing_hrv_days, missing_sleep_days, missing_rhr_days,
            last_hrv_reading, last_sleep_reading, last_rhr_reading,
            wearable_connected, reminder_sent_at,
            created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    // Wait a moment
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Second execution (should update, not insert)
    job.process_user_data_quality(user_id).await.expect("Failed to process");

    let second_metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id, user_id,
            completeness_score, consistency_score, reliability_score, overall_score,
            days_analyzed, missing_hrv_days, missing_sleep_days, missing_rhr_days,
            last_hrv_reading, last_sleep_reading, last_rhr_reading,
            wearable_connected, reminder_sent_at,
            created_at, updated_at
        FROM data_quality_metrics
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    // Verify same record (updated, not new)
    assert_eq!(first_metrics.id, second_metrics.id);
    assert!(second_metrics.updated_at > first_metrics.updated_at);

    // Verify only one record exists
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) as \"count!\" FROM data_quality_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, 1);

    Ok(())
}

/// Test job metadata
#[test]
fn test_job_metadata() {
    assert_eq!(DataQualityCheckJob::get_job_name(), "data_quality_check");
    assert_eq!(DataQualityCheckJob::get_schedule(), "0 2 * * *");
}
