use ai_coach::models::JobExecutionStats;
use ai_coach::services::{
    DailyRecoveryCalculationJob, NotificationService, RecoveryAlertService,
    RecoveryAnalysisService,
};
use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Test dry run execution
#[sqlx::test]
async fn test_dry_run(pool: PgPool) -> sqlx::Result<()> {
    // Create services
    let notification_service = NotificationService::new(pool.clone());
    let alert_service = RecoveryAlertService::new(pool.clone(), notification_service);
    let analysis_service = RecoveryAnalysisService::with_alerts(pool.clone(), alert_service);

    // Create job
    let job = DailyRecoveryCalculationJob::new(pool.clone(), analysis_service, None).unwrap();

    // Execute dry run
    let result = job.dry_run().await.unwrap();

    // Verify result structure
    assert!(result.get("current_utc_hour").is_some());
    assert!(result.get("total_users").is_some());
    assert!(result.get("users_by_timezone").is_some());

    Ok(())
}

/// Test job execution with no users
#[sqlx::test]
async fn test_execute_no_users(pool: PgPool) -> sqlx::Result<()> {
    // Create services
    let notification_service = NotificationService::new(pool.clone());
    let alert_service = RecoveryAlertService::new(pool.clone(), notification_service);
    let analysis_service = RecoveryAnalysisService::with_alerts(pool.clone(), alert_service);

    // Create job
    let job = DailyRecoveryCalculationJob::new(pool.clone(), analysis_service, None).unwrap();

    // Execute job
    let stats = job.execute().await.unwrap();

    // Should have no records processed
    assert_eq!(stats.records_processed, 0);
    assert_eq!(stats.records_failed, 0);
    assert!(stats.execution_time_ms >= 0);

    Ok(())
}

/// Test trigger for specific timezone
#[sqlx::test]
async fn test_trigger_for_timezone(pool: PgPool) -> sqlx::Result<()> {
    // Create a test user with specific timezone
    let user_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO users (id, email, password_hash, timezone, active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, true, NOW(), NOW())
        "#,
        user_id,
        "test@example.com",
        "hashed_password",
        "America/New_York"
    )
    .execute(&pool)
    .await?;

    // Create services
    let notification_service = NotificationService::new(pool.clone());
    let alert_service = RecoveryAlertService::new(pool.clone(), notification_service);
    let analysis_service = RecoveryAnalysisService::with_alerts(pool.clone(), alert_service);

    // Create job
    let job = DailyRecoveryCalculationJob::new(pool.clone(), analysis_service, None).unwrap();

    // Trigger for timezone
    let stats = job.trigger_for_timezone("America/New_York").await.unwrap();

    // Should have attempted to process the user
    // Note: It may fail due to missing recovery data, but should attempt
    let total_attempts = stats.records_processed + stats.records_failed;
    assert_eq!(total_attempts, 1);

    Ok(())
}

/// Test batch processing with multiple users
#[sqlx::test]
async fn test_batch_processing(pool: PgPool) -> sqlx::Result<()> {
    // Create multiple test users
    for i in 0..150 {
        let user_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, password_hash, timezone, active, created_at, updated_at)
            VALUES ($1, $2, $3, 'UTC', true, NOW(), NOW())
            "#,
            user_id,
            format!("test{}@example.com", i),
            "hashed_password"
        )
        .execute(&pool)
        .await?;
    }

    // Create services
    let notification_service = NotificationService::new(pool.clone());
    let alert_service = RecoveryAlertService::new(pool.clone(), notification_service);
    let analysis_service = RecoveryAnalysisService::with_alerts(pool.clone(), alert_service);

    // Create job
    let job = DailyRecoveryCalculationJob::new(pool.clone(), analysis_service, None).unwrap();

    // Execute job (will process all users in batches of 100)
    let stats = job.execute().await.unwrap();

    // Should have attempted to process all 150 users
    // They will likely fail due to missing recovery data, but should be attempted
    let total_attempts = stats.records_processed + stats.records_failed;
    assert_eq!(total_attempts, 150);

    // Execution time should be reasonable
    assert!(stats.execution_time_ms < 60000); // Less than 60 seconds

    Ok(())
}

/// Test clone functionality
#[sqlx::test]
async fn test_job_clone(pool: PgPool) -> sqlx::Result<()> {
    // Create services
    let notification_service = NotificationService::new(pool.clone());
    let alert_service = RecoveryAlertService::new(pool.clone(), notification_service);
    let analysis_service = RecoveryAnalysisService::with_alerts(pool.clone(), alert_service);

    // Create job
    let job = DailyRecoveryCalculationJob::new(pool.clone(), analysis_service, None).unwrap();

    // Clone job
    let job_clone = job.clone();

    // Both should be able to execute
    let _ = job.dry_run().await.unwrap();
    let _ = job_clone.dry_run().await.unwrap();

    Ok(())
}

/// Test high failure rate detection
#[sqlx::test]
async fn test_high_failure_rate_detection(pool: PgPool) -> sqlx::Result<()> {
    // Create 20 test users (none will have recovery data, so all will fail)
    for i in 0..20 {
        let user_id = Uuid::new_v4();
        sqlx::query!(
            r#"
            INSERT INTO users (id, email, password_hash, timezone, active, created_at, updated_at)
            VALUES ($1, $2, $3, 'UTC', true, NOW(), NOW())
            "#,
            user_id,
            format!("failtest{}@example.com", i),
            "hashed_password"
        )
        .execute(&pool)
        .await?;
    }

    // Create services
    let notification_service = NotificationService::new(pool.clone());
    let alert_service = RecoveryAlertService::new(pool.clone(), notification_service);
    let analysis_service = RecoveryAnalysisService::with_alerts(pool.clone(), alert_service);

    // Create job
    let job = DailyRecoveryCalculationJob::new(pool.clone(), analysis_service, None).unwrap();

    // Execute job
    let stats = job.execute().await.unwrap();

    // Should have high failure rate (>5%)
    let failure_rate = stats.records_failed as f64 / 20.0;
    assert!(failure_rate > 0.05);

    // Should have metadata about high failure rate
    assert!(stats.metadata.is_some());
    if let Some(metadata) = stats.metadata {
        assert_eq!(metadata.get("high_failure_rate"), Some(&serde_json::json!(true)));
    }

    Ok(())
}
