use ai_coach_api::models::{
    RecommendationOutcome, UserRecommendation, UserRecommendationStatus,
};
use ai_coach_api::services::RecommendationEffectivenessService;
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create test user
async fn create_user(pool: &PgPool) -> Uuid {
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

/// Helper to create test template
async fn create_template(pool: &PgPool, category: &str) -> Uuid {
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
        category,
        format!("Test {} Template", category),
        "Test Description",
        "Test Action",
        "medium" as _,
        "easy" as _,
        json!({}),
        json!({}),
        0.5
    )
    .execute(pool)
    .await
    .unwrap();

    template_id
}

/// Helper to create recovery score
async fn create_recovery_score(
    pool: &PgPool,
    user_id: Uuid,
    score: f64,
    hours_ago: i64,
) -> Uuid {
    let score_id = Uuid::new_v4();
    let score_date = Utc::now() - Duration::hours(hours_ago);

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
        score
    )
    .execute(pool)
    .await
    .unwrap();

    score_id
}

#[sqlx::test]
async fn test_complete_outcome_tracking_pipeline(pool: PgPool) -> sqlx::Result<()> {
    let service = RecommendationEffectivenessService::new(pool.clone());
    let user_id = create_user(&pool).await;
    let template_id = create_template(&pool, "sleep").await;

    // Create baseline recovery score (25 hours ago)
    let baseline_score_id = create_recovery_score(&pool, user_id, 65.0, 25).await;

    // Create a completed recommendation
    let rec_id = Uuid::new_v4();
    let shown_at = Utc::now() - Duration::hours(25);
    let completed_at = shown_at + Duration::hours(18); // Completed 18 hours later

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
        5i32,
        json!({})
    )
    .execute(&pool)
    .await?;

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

    // Step 1: Track outcome
    let outcome = service
        .track_outcome(&recommendation, Some(baseline_score_id), 65.0)
        .await
        .unwrap();

    assert_eq!(outcome.user_id, user_id);
    assert_eq!(outcome.baseline_recovery_score, 65.0);
    assert!(outcome.effectiveness_score.is_none()); // Not yet calculated

    // Step 2: Create next-day recovery score
    let next_day_score_id = create_recovery_score(&pool, user_id, 78.0, 1).await;

    // Step 3: Update with next-day score
    let updated_outcome = service
        .update_outcome_with_next_day_score(outcome.id, Some(next_day_score_id), 78.0)
        .await
        .unwrap();

    assert_eq!(updated_outcome.next_day_recovery_score, Some(78.0));
    assert_eq!(updated_outcome.recovery_improvement, Some(13.0)); // 78 - 65
    assert!(updated_outcome.effectiveness_score.is_some());

    // Step 4: Verify template effectiveness was updated
    let template = sqlx::query!(
        "SELECT effectiveness_score FROM recommendation_templates WHERE id = $1",
        template_id
    )
    .fetch_one(&pool)
    .await?;

    // Effectiveness should be updated from initial 0.5
    assert_ne!(template.effectiveness_score, 0.5);

    Ok(())
}

#[sqlx::test]
async fn test_analytics_with_multiple_outcomes(pool: PgPool) -> sqlx::Result<()> {
    let service = RecommendationEffectivenessService::new(pool.clone());
    let user_id = create_user(&pool).await;
    let template_id = create_template(&pool, "nutrition").await;

    // Create multiple completed recommendations with outcomes
    for i in 0..5 {
        let rec_id = Uuid::new_v4();
        let shown_at = Utc::now() - Duration::hours(30 + i);
        let completed_at = shown_at + Duration::hours(12);

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
            (3 + i % 3) as i32,
            json!({})
        )
        .execute(&pool)
        .await?;

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

        let baseline_score = 65.0 + (i as f64 * 2.0);
        let baseline_score_id =
            create_recovery_score(&pool, user_id, baseline_score, 30 + i).await;

        let outcome = service
            .track_outcome(&recommendation, Some(baseline_score_id), baseline_score)
            .await
            .unwrap();

        let next_day_score = baseline_score + 8.0;
        let next_day_score_id = create_recovery_score(&pool, user_id, next_day_score, 6 + i).await;

        service
            .update_outcome_with_next_day_score(outcome.id, Some(next_day_score_id), next_day_score)
            .await
            .unwrap();
    }

    // Get analytics for the template
    let filter = ai_coach_api::models::EffectivenessFilter {
        template_id: Some(template_id),
        category: None,
        from_date: None,
        to_date: None,
        min_completions: None,
    };

    let analytics = service.get_effectiveness_analytics(filter).await.unwrap();

    assert_eq!(analytics.len(), 1);
    assert_eq!(analytics[0].total_completions, 5);
    assert!(analytics[0].avg_user_rating.is_some());
    assert!(analytics[0].avg_recovery_improvement.is_some());
    assert!(analytics[0].avg_effectiveness_score.is_some());

    Ok(())
}

#[sqlx::test]
async fn test_system_analytics_multiple_categories(pool: PgPool) -> sqlx::Result<()> {
    let service = RecommendationEffectivenessService::new(pool.clone());
    let user_id = create_user(&pool).await;

    // Create templates for different categories
    let categories = vec!["sleep", "nutrition", "active_recovery"];
    for category in categories {
        let template_id = create_template(&pool, category).await;

        // Create 2 completed recommendations for each category
        for j in 0..2 {
            let rec_id = Uuid::new_v4();
            let shown_at = Utc::now() - Duration::hours(30 + j);
            let completed_at = shown_at + Duration::hours(10);

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
            .execute(&pool)
            .await?;

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

            let baseline_score_id = create_recovery_score(&pool, user_id, 68.0, 30 + j).await;
            let outcome = service
                .track_outcome(&recommendation, Some(baseline_score_id), 68.0)
                .await
                .unwrap();

            let next_day_score_id = create_recovery_score(&pool, user_id, 76.0, 6 + j).await;
            service
                .update_outcome_with_next_day_score(outcome.id, Some(next_day_score_id), 76.0)
                .await
                .unwrap();
        }
    }

    // Get system analytics
    let analytics = service.get_system_analytics().await.unwrap();

    assert!(analytics.total_recommendations_completed >= 6);
    assert!(analytics.category_analytics.len() >= 3);
    assert!(analytics.overall_avg_effectiveness.is_some());

    Ok(())
}

#[sqlx::test]
async fn test_underperforming_template_detection(pool: PgPool) -> sqlx::Result<()> {
    let service = RecommendationEffectivenessService::new(pool.clone());
    let user_id = create_user(&pool).await;
    let template_id = create_template(&pool, "stress_management").await;

    // Create multiple low-performing outcomes
    for i in 0..12 {
        let rec_id = Uuid::new_v4();
        let shown_at = Utc::now() - Duration::days(20 + i);
        let completed_at = shown_at + Duration::hours(15);

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
            1i32, // Low rating
            json!({})
        )
        .execute(&pool)
        .await?;

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

        let baseline_score = 65.0;
        let baseline_score_id =
            create_recovery_score(&pool, user_id, baseline_score, (20 + i) * 24).await;

        let outcome = service
            .track_outcome(&recommendation, Some(baseline_score_id), baseline_score)
            .await
            .unwrap();

        // Minimal or negative improvement
        let next_day_score = baseline_score - 2.0;
        let next_day_score_id =
            create_recovery_score(&pool, user_id, next_day_score, (19 + i) * 24).await;

        service
            .update_outcome_with_next_day_score(outcome.id, Some(next_day_score_id), next_day_score)
            .await
            .unwrap();
    }

    // Get system analytics to check for underperforming templates
    let analytics = service.get_system_analytics().await.unwrap();

    // Should have underperforming templates flagged
    let underperforming = analytics
        .underperforming_templates
        .iter()
        .find(|t| t.template_id == template_id);

    assert!(underperforming.is_some());
    if let Some(template) = underperforming {
        assert!(template.needs_review);
        assert!(template.avg_effectiveness.unwrap_or(1.0) < 0.5);
    }

    Ok(())
}
