use ai_coach::models::RecoveryBaseline;
use ai_coach::services::{
    NotificationService, RecoveryDataService, WeeklyBaselineRecalculationJob,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create test user
async fn create_test_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"
        INSERT INTO users (email, password_hash, email_verified, active, timezone)
        VALUES ($1, 'test_hash', true, true, 'UTC')
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
    hrv_rmssd: Option<f64>,
    sleep_score: Option<f64>,
    resting_hr: Option<i32>,
) {
    sqlx::query!(
        r#"
        INSERT INTO recovery_data (user_id, date, hrv_rmssd, sleep_score, resting_hr, data_source)
        VALUES ($1, $2, $3, $4, $5, 'api_integration')
        "#,
        user_id,
        date,
        hrv_rmssd,
        sleep_score,
        resting_hr
    )
    .execute(pool)
    .await
    .expect("Failed to create recovery data");
}

/// Helper to create baseline for a user
async fn create_baseline(
    pool: &PgPool,
    user_id: Uuid,
    hrv_baseline: Option<f64>,
    rhr_baseline: Option<f64>,
    sleep_hours: Option<f64>,
) -> Uuid {
    sqlx::query_scalar!(
        r#"
        INSERT INTO recovery_baselines (
            user_id, hrv_baseline_rmssd, rhr_baseline, typical_sleep_hours,
            calculated_at, data_points_count
        )
        VALUES ($1, $2, $3, $4, NOW(), 30)
        RETURNING id
        "#,
        user_id,
        hrv_baseline,
        rhr_baseline,
        sleep_hours
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create baseline")
}

/// Test job execution with no users
#[sqlx::test]
async fn test_no_users(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let recovery_service = RecoveryDataService::new(pool.clone());
    let job = WeeklyBaselineRecalculationJob::new(
        pool.clone(),
        recovery_service,
        notification_service,
    );

    // Execute job with no active users
    let stats = job.execute().await.expect("Job execution failed");

    assert_eq!(stats.records_processed, 0);
    assert_eq!(stats.records_failed, 0);

    Ok(())
}

/// Test job execution with single user
#[sqlx::test]
async fn test_single_user_recalculation(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let recovery_service = RecoveryDataService::new(pool.clone());
    let job = WeeklyBaselineRecalculationJob::new(
        pool.clone(),
        recovery_service,
        notification_service,
    );

    // Create user
    let user_id = create_test_user(&pool, "user@test.com").await;

    // Create 30 days of recovery data
    let today = Utc::now().date_naive();
    for i in 0..30 {
        let date = today - chrono::Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(50.0), Some(80.0), Some(60)).await;
    }

    // Create initial baseline
    create_baseline(&pool, user_id, Some(50.0), Some(60.0), Some(8.0)).await;

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    // Should process 1 user successfully
    assert_eq!(stats.records_processed, 1);
    assert_eq!(stats.records_failed, 0);

    // Verify new baseline was created
    let baseline_count = sqlx::query_scalar!(
        "SELECT COUNT(*) as \"count!\" FROM recovery_baselines WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert!(baseline_count >= 2); // At least original + new

    Ok(())
}

/// Test detection of significant HRV increase (improvement)
#[sqlx::test]
async fn test_hrv_improvement_detection(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let recovery_service = RecoveryDataService::new(pool.clone());
    let job = WeeklyBaselineRecalculationJob::new(
        pool.clone(),
        recovery_service,
        notification_service,
    );

    let user_id = create_test_user(&pool, "hrv_improve@test.com").await;
    let today = Utc::now().date_naive();

    // Create initial baseline with HRV of 50.0
    create_baseline(&pool, user_id, Some(50.0), Some(60.0), Some(8.0)).await;

    // Create 30 days of data with improved HRV (60.0 = 20% increase)
    for i in 0..30 {
        let date = today - chrono::Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(60.0), Some(80.0), Some(60)).await;
    }

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    assert_eq!(stats.records_processed, 1);
    assert_eq!(stats.records_failed, 0);

    // Verify metadata shows changes detected
    let metadata = stats.metadata.expect("Metadata missing");
    let changes = metadata["significant_changes"].as_i64().unwrap_or(0);
    assert!(changes > 0, "Expected significant changes to be detected");

    Ok(())
}

/// Test detection of significant RHR decrease (improvement)
#[sqlx::test]
async fn test_rhr_improvement_detection(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let recovery_service = RecoveryDataService::new(pool.clone());
    let job = WeeklyBaselineRecalculationJob::new(
        pool.clone(),
        recovery_service,
        notification_service,
    );

    let user_id = create_test_user(&pool, "rhr_improve@test.com").await;
    let today = Utc::now().date_naive();

    // Create initial baseline with RHR of 65.0
    create_baseline(&pool, user_id, Some(50.0), Some(65.0), Some(8.0)).await;

    // Create 30 days of data with improved RHR (55.0 = -15.4% decrease)
    for i in 0..30 {
        let date = today - chrono::Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(50.0), Some(80.0), Some(55)).await;
    }

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    assert_eq!(stats.records_processed, 1);
    assert_eq!(stats.records_failed, 0);

    // Verify metadata shows changes detected
    let metadata = stats.metadata.expect("Metadata missing");
    let changes = metadata["significant_changes"].as_i64().unwrap_or(0);
    assert!(changes > 0, "Expected significant changes to be detected");

    Ok(())
}

/// Test no detection for minor changes (<10%)
#[sqlx::test]
async fn test_minor_changes_not_detected(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let recovery_service = RecoveryDataService::new(pool.clone());
    let job = WeeklyBaselineRecalculationJob::new(
        pool.clone(),
        recovery_service,
        notification_service,
    );

    let user_id = create_test_user(&pool, "minor@test.com").await;
    let today = Utc::now().date_naive();

    // Create initial baseline with HRV of 50.0
    create_baseline(&pool, user_id, Some(50.0), Some(60.0), Some(8.0)).await;

    // Create 30 days of data with minor HRV change (52.0 = 4% increase)
    for i in 0..30 {
        let date = today - chrono::Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(52.0), Some(80.0), Some(60)).await;
    }

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    assert_eq!(stats.records_processed, 1);
    assert_eq!(stats.records_failed, 0);

    // Verify metadata shows no significant changes
    let metadata = stats.metadata.expect("Metadata missing");
    let changes = metadata["significant_changes"].as_i64().unwrap_or(0);
    assert_eq!(changes, 0, "Minor changes should not be significant");

    Ok(())
}

/// Test batch processing with multiple users
#[sqlx::test]
async fn test_multiple_users_batch_processing(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let recovery_service = RecoveryDataService::new(pool.clone());
    let job = WeeklyBaselineRecalculationJob::new(
        pool.clone(),
        recovery_service,
        notification_service,
    );

    let today = Utc::now().date_naive();

    // Create 5 test users with data
    for i in 0..5 {
        let email = format!("user{}@test.com", i);
        let user_id = create_test_user(&pool, &email).await;

        // Create baseline
        create_baseline(&pool, user_id, Some(50.0), Some(60.0), Some(8.0)).await;

        // Create 30 days of data
        for j in 0..30 {
            let date = today - chrono::Duration::days(j);
            create_recovery_data(&pool, user_id, date, Some(50.0), Some(80.0), Some(60)).await;
        }
    }

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    // Should process all 5 users successfully
    assert_eq!(stats.records_processed, 5);
    assert_eq!(stats.records_failed, 0);

    Ok(())
}

/// Test job handles user with no previous baseline (first calculation)
#[sqlx::test]
async fn test_first_baseline_calculation(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let recovery_service = RecoveryDataService::new(pool.clone());
    let job = WeeklyBaselineRecalculationJob::new(
        pool.clone(),
        recovery_service,
        notification_service,
    );

    let user_id = create_test_user(&pool, "firsttime@test.com").await;
    let today = Utc::now().date_naive();

    // Create 30 days of data but NO existing baseline
    for i in 0..30 {
        let date = today - chrono::Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(50.0), Some(80.0), Some(60)).await;
    }

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    assert_eq!(stats.records_processed, 1);
    assert_eq!(stats.records_failed, 0);

    // Verify baseline was created
    let baseline_count = sqlx::query_scalar!(
        "SELECT COUNT(*) as \"count!\" FROM recovery_baselines WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(baseline_count, 1); // First baseline created

    // Verify no changes detected (no previous baseline to compare)
    let metadata = stats.metadata.expect("Metadata missing");
    let total_changes = metadata["total_changes_detected"].as_i64().unwrap_or(0);
    assert_eq!(total_changes, 0, "No changes should be detected for first baseline");

    Ok(())
}

/// Test job metadata (name and schedule)
#[test]
fn test_job_metadata() {
    assert_eq!(
        WeeklyBaselineRecalculationJob::get_job_name(),
        "weekly_baseline_recalculation"
    );
    assert_eq!(WeeklyBaselineRecalculationJob::get_schedule(), "0 3 * * 0");
}

/// Test detection of sleep duration changes
#[sqlx::test]
async fn test_sleep_duration_change_detection(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let recovery_service = RecoveryDataService::new(pool.clone());
    let job = WeeklyBaselineRecalculationJob::new(
        pool.clone(),
        recovery_service,
        notification_service,
    );

    let user_id = create_test_user(&pool, "sleep@test.com").await;
    let today = Utc::now().date_naive();

    // Create initial baseline with 8 hours sleep
    create_baseline(&pool, user_id, Some(50.0), Some(60.0), Some(8.0)).await;

    // Create 30 days of data with 9.2 hours sleep (15% increase)
    for i in 0..30 {
        let date = today - chrono::Duration::days(i);
        // Sleep score of 92 should result in ~9.2 hours
        create_recovery_data(&pool, user_id, date, Some(50.0), Some(92.0), Some(60)).await;
    }

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    assert_eq!(stats.records_processed, 1);
    assert_eq!(stats.records_failed, 0);

    Ok(())
}

/// Test job handles missing data gracefully
#[sqlx::test]
async fn test_user_with_no_recent_data(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let recovery_service = RecoveryDataService::new(pool.clone());
    let job = WeeklyBaselineRecalculationJob::new(
        pool.clone(),
        recovery_service,
        notification_service,
    );

    let user_id = create_test_user(&pool, "olddata@test.com").await;

    // Create baseline
    create_baseline(&pool, user_id, Some(50.0), Some(60.0), Some(8.0)).await;

    // Create old data (>60 days ago) - user should not be processed
    let old_date = Utc::now().date_naive() - chrono::Duration::days(65);
    create_recovery_data(&pool, user_id, old_date, Some(50.0), Some(80.0), Some(60)).await;

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    // User should not be processed (no recent data)
    assert_eq!(stats.records_processed, 0);
    assert_eq!(stats.records_failed, 0);

    Ok(())
}

/// Test major change detection (>=20%)
#[sqlx::test]
async fn test_major_change_detection(pool: PgPool) -> sqlx::Result<()> {
    let notification_service = NotificationService::new(pool.clone());
    let recovery_service = RecoveryDataService::new(pool.clone());
    let job = WeeklyBaselineRecalculationJob::new(
        pool.clone(),
        recovery_service,
        notification_service,
    );

    let user_id = create_test_user(&pool, "major@test.com").await;
    let today = Utc::now().date_naive();

    // Enable notifications for user
    sqlx::query!(
        "UPDATE users SET push_notifications_enabled = true, email_notifications_enabled = true WHERE id = $1",
        user_id
    )
    .execute(&pool)
    .await?;

    // Create initial baseline with HRV of 50.0
    create_baseline(&pool, user_id, Some(50.0), Some(60.0), Some(8.0)).await;

    // Create 30 days of data with major HRV change (63.0 = 26% increase)
    for i in 0..30 {
        let date = today - chrono::Duration::days(i);
        create_recovery_data(&pool, user_id, date, Some(63.0), Some(80.0), Some(60)).await;
    }

    // Execute job
    let stats = job.execute().await.expect("Job execution failed");

    assert_eq!(stats.records_processed, 1);
    assert_eq!(stats.records_failed, 0);

    // Verify significant changes detected
    let metadata = stats.metadata.expect("Metadata missing");
    let changes = metadata["significant_changes"].as_i64().unwrap_or(0);
    assert!(changes > 0, "Major changes should be detected");

    Ok(())
}
