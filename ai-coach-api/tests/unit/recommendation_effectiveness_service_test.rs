use ai_coach_api::models::{
    calculate_effectiveness_score, determine_effectiveness_trend, update_effectiveness_with_ema,
    RecommendationCategory, RecommendationDifficulty, RecommendationPriority,
    RecommendationTemplate, UserRecommendation, UserRecommendationStatus,
};
use ai_coach_api::services::RecommendationEffectivenessService;
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create a test recommendation template
async fn create_test_template(pool: &PgPool) -> Uuid {
    let template_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO recommendation_templates (
            id, category, title, description, action,
            priority_default, difficulty, trigger_conditions,
            user_constraints, effectiveness_score
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
        template_id,
        "sleep" as _,
        "Test Template",
        "Test Description",
        "Test Action",
        "high" as _,
        "easy" as _,
        json!({}),
        json!({}),
        0.7
    )
    .execute(pool)
    .await
    .unwrap();

    template_id
}

/// Helper to create a test user
async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    sqlx::query!(
        r#"
        INSERT INTO users (id, email, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        "#,
        user_id,
        email,
        "$argon2id$v=19$m=19456,t=2,p=1$test"
    )
    .execute(pool)
    .await
    .unwrap();

    user_id
}

/// Helper to create a test recovery score
async fn create_test_recovery_score(pool: &PgPool, user_id: Uuid, score: f64, date_offset_hours: i64) -> Uuid {
    let score_id = Uuid::new_v4();
    let score_date = Utc::now() - Duration::hours(date_offset_hours);

    sqlx::query!(
        r#"
        INSERT INTO recovery_scores (
            id, user_id, score_date, overall_readiness,
            sleep_quality, hrv_status, resting_heart_rate,
            sleep_duration, created_at
        )
        VALUES ($1, $2, $3, $4, 75, 0, 60, 8.0, NOW())
        "#,
        score_id,
        user_id,
        score_date,
        score,
    )
    .execute(pool)
    .await
    .unwrap();

    score_id
}

/// Helper to create a completed user recommendation
async fn create_completed_recommendation(
    pool: &PgPool,
    user_id: Uuid,
    template_id: Uuid,
) -> Uuid {
    let rec_id = Uuid::new_v4();
    let shown_at = Utc::now() - Duration::hours(25); // 25 hours ago
    let completed_at = shown_at + Duration::hours(12); // Completed 12 hours later

    sqlx::query!(
        r#"
        INSERT INTO user_recommendations (
            id, user_id, recommendation_template_id,
            status, shown_at, completed_at, effectiveness_rating,
            created_at, updated_at, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW(), $8)
        "#,
        rec_id,
        user_id,
        template_id,
        "completed" as _,
        shown_at,
        completed_at,
        4i32,
        json!({})
    )
    .execute(pool)
    .await
    .unwrap();

    rec_id
}

#[sqlx::test]
async fn test_calculate_effectiveness_score_positive_improvement(pool: PgPool) -> sqlx::Result<()> {
    // Test effectiveness calculation with positive recovery improvement
    let score = calculate_effectiveness_score(60.0, 80.0, Some(5), 12.0);

    // Recovery: (80-60)/100 = 0.2, normalized: (0.2+0.5) = 0.7
    // Rating: 5/5 = 1.0
    // Time factor: 1.0 (within 24h)
    // Score: (0.7*0.5 + 1.0*0.5) * 1.0 = 0.85
    assert!((score - 0.85).abs() < 0.01);

    Ok(())
}

#[sqlx::test]
async fn test_calculate_effectiveness_score_negative_improvement(pool: PgPool) -> sqlx::Result<()> {
    // Test effectiveness calculation with negative recovery improvement
    let score = calculate_effectiveness_score(70.0, 60.0, Some(2), 30.0);

    // Recovery: (60-70)/100 = -0.1, normalized: (-0.1+0.5) = 0.4
    // Rating: 2/5 = 0.4
    // Time factor: 0.8 (>24h)
    // Score: (0.4*0.5 + 0.4*0.5) * 0.8 = 0.32
    assert!((score - 0.32).abs() < 0.01);

    Ok(())
}

#[sqlx::test]
async fn test_update_effectiveness_with_ema(pool: PgPool) -> sqlx::Result<()> {
    // Test exponential moving average calculation
    let current = 0.7;
    let new_outcome = 0.9;

    // EMA: 0.3 * 0.9 + 0.7 * 0.7 = 0.27 + 0.49 = 0.76
    let updated = update_effectiveness_with_ema(current, new_outcome);
    assert!((updated - 0.76).abs() < 0.01);

    Ok(())
}

#[sqlx::test]
async fn test_determine_effectiveness_trend_improving(pool: PgPool) -> sqlx::Result<()> {
    let trend = determine_effectiveness_trend(Some(0.8), Some(0.7));
    assert_eq!(trend, "improving");

    Ok(())
}

#[sqlx::test]
async fn test_determine_effectiveness_trend_declining(pool: PgPool) -> sqlx::Result<()> {
    let trend = determine_effectiveness_trend(Some(0.6), Some(0.75));
    assert_eq!(trend, "declining");

    Ok(())
}

#[sqlx::test]
async fn test_determine_effectiveness_trend_stable(pool: PgPool) -> sqlx::Result<()> {
    let trend = determine_effectiveness_trend(Some(0.72), Some(0.70));
    assert_eq!(trend, "stable");

    Ok(())
}

#[sqlx::test]
async fn test_track_outcome_creates_record(pool: PgPool) -> sqlx::Result<()> {
    let service = RecommendationEffectivenessService::new(pool.clone());
    let user_id = create_test_user(&pool).await;
    let template_id = create_test_template(&pool).await;
    let rec_id = create_completed_recommendation(&pool, user_id, template_id).await;

    // Get the recommendation
    let recommendation = sqlx::query_as!(
        UserRecommendation,
        r#"
        SELECT
            id, user_id, recommendation_template_id, recovery_score_id,
            status as "status: UserRecommendationStatus",
            effectiveness_rating, user_feedback, skip_reason,
            shown_at, completed_at, skipped_at, expired_at, rated_at,
            metadata, created_at, updated_at
        FROM user_recommendations
        WHERE id = $1
        "#,
        rec_id
    )
    .fetch_one(&pool)
    .await?;

    // Create baseline recovery score
    let baseline_score_id = create_test_recovery_score(&pool, user_id, 65.0, 25).await;

    // Track outcome
    let outcome = service
        .track_outcome(&recommendation, Some(baseline_score_id), 65.0)
        .await
        .unwrap();

    assert_eq!(outcome.user_id, user_id);
    assert_eq!(outcome.recommendation_template_id, template_id);
    assert_eq!(outcome.baseline_recovery_score, 65.0);
    assert!(outcome.completion_time_hours > 0.0);

    Ok(())
}

#[sqlx::test]
async fn test_update_outcome_with_next_day_score(pool: PgPool) -> sqlx::Result<()> {
    let service = RecommendationEffectivenessService::new(pool.clone());
    let user_id = create_test_user(&pool).await;
    let template_id = create_test_template(&pool).await;
    let rec_id = create_completed_recommendation(&pool, user_id, template_id).await;

    // Get the recommendation
    let recommendation = sqlx::query_as!(
        UserRecommendation,
        r#"
        SELECT
            id, user_id, recommendation_template_id, recovery_score_id,
            status as "status: UserRecommendationStatus",
            effectiveness_rating, user_feedback, skip_reason,
            shown_at, completed_at, skipped_at, expired_at, rated_at,
            metadata, created_at, updated_at
        FROM user_recommendations
        WHERE id = $1
        "#,
        rec_id
    )
    .fetch_one(&pool)
    .await?;

    // Create baseline recovery score
    let baseline_score_id = create_test_recovery_score(&pool, user_id, 65.0, 25).await;

    // Track outcome
    let outcome = service
        .track_outcome(&recommendation, Some(baseline_score_id), 65.0)
        .await
        .unwrap();

    // Create next-day recovery score
    let next_day_score_id = create_test_recovery_score(&pool, user_id, 78.0, 1).await;

    // Update with next-day score
    let updated_outcome = service
        .update_outcome_with_next_day_score(outcome.id, Some(next_day_score_id), 78.0)
        .await
        .unwrap();

    assert_eq!(updated_outcome.next_day_recovery_score, Some(78.0));
    assert_eq!(updated_outcome.recovery_improvement, Some(13.0)); // 78 - 65
    assert!(updated_outcome.effectiveness_score.is_some());
    assert!(updated_outcome.effectiveness_score.unwrap() > 0.5); // Good improvement

    Ok(())
}

#[sqlx:test]
async fn test_update_template_effectiveness_uses_ema(pool: PgPool) -> sqlx::Result<()> {
    let service = RecommendationEffectivenessService::new(pool.clone());
    let template_id = create_test_template(&pool).await;

    // Initial effectiveness is 0.7 (set in create_test_template)

    // Update with new effectiveness of 0.9
    service
        .update_template_effectiveness(template_id, 0.9)
        .await
        .unwrap();

    // Get updated template
    let template = sqlx::query!(
        "SELECT effectiveness_score FROM recommendation_templates WHERE id = $1",
        template_id
    )
    .fetch_one(&pool)
    .await?;

    // Expected: (0.3 * 0.9) + (0.7 * 0.7) = 0.27 + 0.49 = 0.76
    assert!((template.effectiveness_score - 0.76).abs() < 0.01);

    Ok(())
}

#[sqlx::test]
async fn test_get_effectiveness_analytics_filters_by_template(pool: PgPool) -> sqlx::Result<()> {
    let service = RecommendationEffectivenessService::new(pool.clone());
    let user_id = create_test_user(&pool).await;
    let template_id = create_test_template(&pool).await;
    let rec_id = create_completed_recommendation(&pool, user_id, template_id).await;

    // Create outcome with effectiveness score
    let recommendation = sqlx::query_as!(
        UserRecommendation,
        r#"
        SELECT
            id, user_id, recommendation_template_id, recovery_score_id,
            status as "status: UserRecommendationStatus",
            effectiveness_rating, user_feedback, skip_reason,
            shown_at, completed_at, skipped_at, expired_at, rated_at,
            metadata, created_at, updated_at
        FROM user_recommendations
        WHERE id = $1
        "#,
        rec_id
    )
    .fetch_one(&pool)
    .await?;

    let baseline_score_id = create_test_recovery_score(&pool, user_id, 65.0, 25).await;
    let outcome = service
        .track_outcome(&recommendation, Some(baseline_score_id), 65.0)
        .await
        .unwrap();

    let next_day_score_id = create_test_recovery_score(&pool, user_id, 75.0, 1).await;
    service
        .update_outcome_with_next_day_score(outcome.id, Some(next_day_score_id), 75.0)
        .await
        .unwrap();

    // Get analytics filtered by template
    let filter = ai_coach_api::models::EffectivenessFilter {
        template_id: Some(template_id),
        category: None,
        from_date: None,
        to_date: None,
        min_completions: None,
    };

    let analytics = service.get_effectiveness_analytics(filter).await.unwrap();

    assert_eq!(analytics.len(), 1);
    assert_eq!(analytics[0].template_id, Some(template_id));
    assert_eq!(analytics[0].total_completions, 1);
    assert!(analytics[0].avg_effectiveness_score.is_some());

    Ok(())
}

#[sqlx::test]
async fn test_get_system_analytics(pool: PgPool) -> sqlx::Result<()> {
    let service = RecommendationEffectivenessService::new(pool.clone());
    let user_id = create_test_user(&pool).await;
    let template_id = create_test_template(&pool).await;

    // Create multiple completed recommendations
    for _ in 0..3 {
        let rec_id = create_completed_recommendation(&pool, user_id, template_id).await;
        let recommendation = sqlx::query_as!(
            UserRecommendation,
            r#"
            SELECT
                id, user_id, recommendation_template_id, recovery_score_id,
                status as "status: UserRecommendationStatus",
                effectiveness_rating, user_feedback, skip_reason,
                shown_at, completed_at, skipped_at, expired_at, rated_at,
                metadata, created_at, updated_at
            FROM user_recommendations
            WHERE id = $1
            "#,
            rec_id
        )
        .fetch_one(&pool)
        .await?;

        let baseline_score_id = create_test_recovery_score(&pool, user_id, 65.0, 25).await;
        let outcome = service
            .track_outcome(&recommendation, Some(baseline_score_id), 65.0)
            .await
            .unwrap();

        let next_day_score_id = create_test_recovery_score(&pool, user_id, 75.0, 1).await;
        service
            .update_outcome_with_next_day_score(outcome.id, Some(next_day_score_id), 75.0)
            .await
            .unwrap();
    }

    // Get system analytics
    let analytics = service.get_system_analytics().await.unwrap();

    assert!(analytics.total_recommendations_completed >= 3);
    assert!(analytics.overall_completion_rate > 0.0);
    assert!(!analytics.category_analytics.is_empty());

    Ok(())
}
