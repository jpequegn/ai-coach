use ai_coach_api::models::recommendation::{
    RecommendationCategory, RecommendationDifficulty, RecommendationPriority,
    CreateRecommendationTemplateRequest, CurrentRecommendationsQuery,
};
use ai_coach_api::models::RecoveryScore;
use ai_coach_api::services::RecommendationEngine;
use chrono::{NaiveDate, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to insert a test user
async fn insert_test_user(pool: &PgPool) -> sqlx::Result<Uuid> {
    let user_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        "#,
    )
    .bind(user_id)
    .bind(format!("test_{}@example.com", user_id))
    .bind("$argon2id$v=19$m=19456,t=2,p=1$test")
    .execute(pool)
    .await?;

    Ok(user_id)
}

/// Helper to insert a recovery score
async fn insert_recovery_score(
    pool: &PgPool,
    user_id: Uuid,
    readiness: f64,
    hrv_dev: Option<f64>,
    rhr_dev: Option<f64>,
    sleep_quality: Option<f64>,
    hrv_trend: &str,
) -> sqlx::Result<Uuid> {
    let score_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO recovery_scores (
            id, user_id, score_date, readiness_score, hrv_trend,
            hrv_deviation, sleep_quality_score, recovery_adequacy,
            rhr_deviation, training_strain, recovery_status,
            recommended_tss_adjustment, calculated_at, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        "#,
    )
    .bind(score_id)
    .bind(user_id)
    .bind(NaiveDate::from_ymd_opt(2025, 10, 8).unwrap())
    .bind(readiness)
    .bind(hrv_trend)
    .bind(hrv_dev)
    .bind(sleep_quality)
    .bind(Some(75.0))
    .bind(rhr_dev)
    .bind(Some(50.0))
    .bind("fair")
    .bind(Some(0.0))
    .bind(Utc::now())
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(score_id)
}

/// Helper to insert a recommendation template
async fn insert_recommendation_template(
    pool: &PgPool,
    category: RecommendationCategory,
    priority: RecommendationPriority,
    difficulty: RecommendationDifficulty,
    title: &str,
    trigger_conditions: serde_json::Value,
    time_required: Option<i32>,
) -> sqlx::Result<Uuid> {
    let template_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO recommendation_templates (
            id, category, title, description, action, priority_default,
            difficulty, expected_impact, time_required_minutes,
            trigger_conditions, user_constraints, effectiveness_score,
            times_suggested, times_completed, metadata, is_active,
            created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        "#,
    )
    .bind(template_id)
    .bind(category.to_string())
    .bind(title)
    .bind(format!("Description for {}", title))
    .bind(format!("Action for {}", title))
    .bind(priority.to_string())
    .bind(difficulty.to_string())
    .bind(Some("Improve recovery by 10-15%"))
    .bind(time_required)
    .bind(trigger_conditions)
    .bind(json!({}))
    .bind(0.85)
    .bind(10)
    .bind(8)
    .bind(json!({}))
    .bind(true)
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(template_id)
}

#[sqlx::test]
async fn test_full_pipeline_poor_sleep(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool.clone());

    // Setup: Create user, recovery score with poor sleep, and sleep recommendation
    let user_id = insert_test_user(&pool).await?;
    let _score_id = insert_recovery_score(
        &pool,
        user_id,
        75.0,           // decent readiness
        None,
        None,
        Some(65.0),     // poor sleep (< 70)
        "stable",
    ).await?;

    let _template_id = insert_recommendation_template(
        &pool,
        RecommendationCategory::Sleep,
        RecommendationPriority::High,
        RecommendationDifficulty::Easy,
        "Extend sleep duration tonight",
        json!({"poor_sleep": true}),
        Some(30),
    ).await?;

    // Get latest recovery score
    let recovery_score = sqlx::query_as::<_, RecoveryScore>(
        r#"
        SELECT * FROM recovery_scores
        WHERE user_id = $1
        ORDER BY score_date DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    // Generate recommendations
    let context = ai_coach_api::models::recommendation::RecommendationContext::default();
    let response = engine
        .generate_recommendations(user_id, &recovery_score, context, Some(5), None)
        .await
        .expect("Should generate recommendations");

    // Assertions
    assert!(
        !response.recommendations.is_empty(),
        "Should return at least one recommendation"
    );

    let sleep_rec = response
        .recommendations
        .iter()
        .find(|r| matches!(r.category, RecommendationCategory::Sleep));

    assert!(sleep_rec.is_some(), "Should include sleep recommendation");

    let rec = sleep_rec.unwrap();
    assert_eq!(rec.title, "Extend sleep duration tonight");
    assert!(
        rec.why_this_matters.contains("sleep quality has been suboptimal"),
        "Why message should mention sleep quality"
    );

    Ok(())
}

#[sqlx::test]
async fn test_full_pipeline_multiple_issues(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool.clone());

    // Setup: Create user with multiple recovery issues
    let user_id = insert_test_user(&pool).await?;
    let _score_id = insert_recovery_score(
        &pool,
        user_id,
        55.0,           // accumulated fatigue
        Some(-12.0),    // low HRV
        Some(8.0),      // high RHR
        Some(65.0),     // poor sleep
        "declining",    // high stress
    ).await?;

    // Create recommendations for different issues
    let _sleep_template = insert_recommendation_template(
        &pool,
        RecommendationCategory::Sleep,
        RecommendationPriority::High,
        RecommendationDifficulty::Easy,
        "Prioritize sleep tonight",
        json!({"poor_sleep": true}),
        Some(30),
    ).await?;

    let _stress_template = insert_recommendation_template(
        &pool,
        RecommendationCategory::StressManagement,
        RecommendationPriority::High,
        RecommendationDifficulty::Moderate,
        "Practice deep breathing",
        json!({"high_stress": true, "low_hrv": true}),
        Some(15),
    ).await?;

    let _nutrition_template = insert_recommendation_template(
        &pool,
        RecommendationCategory::Nutrition,
        RecommendationPriority::Medium,
        RecommendationDifficulty::Easy,
        "Increase hydration",
        json!({"accumulated_fatigue": true}),
        Some(5),
    ).await?;

    let _recovery_template = insert_recommendation_template(
        &pool,
        RecommendationCategory::ActiveRecovery,
        RecommendationPriority::Medium,
        RecommendationDifficulty::Moderate,
        "Light yoga session",
        json!({"high_rhr": true}),
        Some(30),
    ).await?;

    // Get latest recovery score
    let recovery_score = sqlx::query_as::<_, RecoveryScore>(
        r#"
        SELECT * FROM recovery_scores
        WHERE user_id = $1
        ORDER BY score_date DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    // Generate recommendations
    let context = ai_coach_api::models::recommendation::RecommendationContext::default();
    let response = engine
        .generate_recommendations(user_id, &recovery_score, context, Some(5), None)
        .await
        .expect("Should generate recommendations");

    // Assertions
    assert!(
        response.recommendations.len() >= 3,
        "Should return multiple recommendations for multiple issues"
    );

    // Should have variety of categories (diversity filtering)
    let categories: std::collections::HashSet<String> = response
        .recommendations
        .iter()
        .map(|r| r.category.to_string())
        .collect();

    assert!(
        categories.len() >= 2,
        "Should have recommendations from at least 2 different categories"
    );

    Ok(())
}

#[sqlx::test]
async fn test_full_pipeline_category_filter(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool.clone());

    // Setup
    let user_id = insert_test_user(&pool).await?;
    let _score_id = insert_recovery_score(
        &pool,
        user_id,
        65.0,
        Some(-10.0),
        None,
        Some(65.0),
        "stable",
    ).await?;

    // Create recommendations in different categories
    let _sleep_template = insert_recommendation_template(
        &pool,
        RecommendationCategory::Sleep,
        RecommendationPriority::High,
        RecommendationDifficulty::Easy,
        "Sleep recommendation",
        json!({"poor_sleep": true}),
        Some(30),
    ).await?;

    let _nutrition_template = insert_recommendation_template(
        &pool,
        RecommendationCategory::Nutrition,
        RecommendationPriority::Medium,
        RecommendationDifficulty::Easy,
        "Nutrition recommendation",
        json!({"poor_sleep": true}),
        Some(15),
    ).await?;

    // Get recovery score
    let recovery_score = sqlx::query_as::<_, RecoveryScore>(
        r#"
        SELECT * FROM recovery_scores
        WHERE user_id = $1
        ORDER BY score_date DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    // Generate recommendations with category filter
    let context = ai_coach_api::models::recommendation::RecommendationContext::default();
    let response = engine
        .generate_recommendations(
            user_id,
            &recovery_score,
            context,
            Some(5),
            Some(RecommendationCategory::Sleep),
        )
        .await
        .expect("Should generate recommendations");

    // Assertions
    assert!(
        !response.recommendations.is_empty(),
        "Should return recommendations"
    );

    // All recommendations should be sleep category
    for rec in &response.recommendations {
        assert!(
            matches!(rec.category, RecommendationCategory::Sleep),
            "All recommendations should be in Sleep category"
        );
    }

    Ok(())
}

#[sqlx::test]
async fn test_full_pipeline_time_constraints(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool.clone());

    // Setup
    let user_id = insert_test_user(&pool).await?;
    let _score_id = insert_recovery_score(
        &pool,
        user_id,
        65.0,
        None,
        None,
        Some(65.0),
        "stable",
    ).await?;

    // Create recommendations with different time requirements
    let _quick_template = insert_recommendation_template(
        &pool,
        RecommendationCategory::Sleep,
        RecommendationPriority::High,
        RecommendationDifficulty::Easy,
        "Quick sleep tip",
        json!({"poor_sleep": true}),
        Some(5),
    ).await?;

    let _long_template = insert_recommendation_template(
        &pool,
        RecommendationCategory::Sleep,
        RecommendationPriority::Medium,
        RecommendationDifficulty::Moderate,
        "Extended sleep routine",
        json!({"poor_sleep": true}),
        Some(60),
    ).await?;

    // Get recovery score
    let recovery_score = sqlx::query_as::<_, RecoveryScore>(
        r#"
        SELECT * FROM recovery_scores
        WHERE user_id = $1
        ORDER BY score_date DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    // Generate recommendations with time constraint
    let mut context = ai_coach_api::models::recommendation::RecommendationContext::default();
    context.time_constraints = Some(15); // Only 15 minutes available

    let response = engine
        .generate_recommendations(user_id, &recovery_score, context, Some(5), None)
        .await
        .expect("Should generate recommendations");

    // Assertions
    assert!(
        !response.recommendations.is_empty(),
        "Should return recommendations"
    );

    // Should only include quick recommendation
    let has_quick = response
        .recommendations
        .iter()
        .any(|r| r.title == "Quick sleep tip");

    let has_long = response
        .recommendations
        .iter()
        .any(|r| r.title == "Extended sleep routine");

    assert!(has_quick, "Should include quick recommendation");
    assert!(!has_long, "Should not include long recommendation");

    Ok(())
}

#[sqlx::test]
async fn test_full_pipeline_no_recovery_data(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool.clone());

    // Create user but no recovery data
    let user_id = insert_test_user(&pool).await?;

    // Try to get recovery score - should return None
    let recovery_score = sqlx::query_as::<_, RecoveryScore>(
        r#"
        SELECT * FROM recovery_scores
        WHERE user_id = $1
        ORDER BY score_date DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&pool)
    .await?;

    assert!(
        recovery_score.is_none(),
        "Should have no recovery data for new user"
    );

    Ok(())
}

#[sqlx::test]
async fn test_full_pipeline_limit_parameter(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool.clone());

    // Setup
    let user_id = insert_test_user(&pool).await?;
    let _score_id = insert_recovery_score(
        &pool,
        user_id,
        60.0,
        Some(-12.0),
        Some(8.0),
        Some(65.0),
        "declining",
    ).await?;

    // Create multiple recommendations
    for i in 0..10 {
        let _template = insert_recommendation_template(
            &pool,
            RecommendationCategory::Sleep,
            RecommendationPriority::High,
            RecommendationDifficulty::Easy,
            &format!("Sleep recommendation {}", i),
            json!({"poor_sleep": true}),
            Some(30),
        ).await?;
    }

    // Get recovery score
    let recovery_score = sqlx::query_as::<_, RecoveryScore>(
        r#"
        SELECT * FROM recovery_scores
        WHERE user_id = $1
        ORDER BY score_date DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    // Generate recommendations with limit
    let context = ai_coach_api::models::recommendation::RecommendationContext::default();
    let response = engine
        .generate_recommendations(user_id, &recovery_score, context, Some(3), None)
        .await
        .expect("Should generate recommendations");

    // Assertions
    assert_eq!(
        response.recommendations.len(),
        2,
        "Should respect limit and diversity (max 2 per category)"
    );

    Ok(())
}

#[sqlx::test]
async fn test_scoring_with_user_history(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool.clone());

    // Setup
    let user_id = insert_test_user(&pool).await?;
    let _score_id = insert_recovery_score(
        &pool,
        user_id,
        65.0,
        None,
        None,
        Some(65.0),
        "stable",
    ).await?;

    // Create template
    let template_id = insert_recommendation_template(
        &pool,
        RecommendationCategory::Sleep,
        RecommendationPriority::High,
        RecommendationDifficulty::Easy,
        "Sleep recommendation",
        json!({"poor_sleep": true}),
        Some(30),
    ).await?;

    let score_id = Uuid::new_v4();

    // Create user recommendation history (completed)
    sqlx::query(
        r#"
        INSERT INTO user_recommendations (
            id, user_id, recommendation_template_id, recovery_score_id,
            status, shown_at, completed_at, created_at, updated_at, metadata
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(template_id)
    .bind(score_id)
    .bind("completed")
    .bind(Utc::now() - chrono::Duration::days(5))
    .bind(Some(Utc::now() - chrono::Duration::days(5)))
    .bind(Utc::now())
    .bind(Utc::now())
    .bind(json!({}))
    .execute(&pool)
    .await?;

    // Get recovery score
    let recovery_score = sqlx::query_as::<_, RecoveryScore>(
        r#"
        SELECT * FROM recovery_scores
        WHERE user_id = $1
        ORDER BY score_date DESC
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

    // Generate recommendations
    let context = ai_coach_api::models::recommendation::RecommendationContext::default();
    let response = engine
        .generate_recommendations(user_id, &recovery_score, context, Some(5), None)
        .await
        .expect("Should generate recommendations");

    // Should still return the recommendation (was completed in the past)
    assert!(
        !response.recommendations.is_empty(),
        "Should return recommendations even with history"
    );

    Ok(())
}
