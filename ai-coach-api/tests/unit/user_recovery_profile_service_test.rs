use ai_coach_api::models::recommendation::{UserRecommendation, UserRecommendationStatus};
use ai_coach_api::models::user_recovery_profile::*;
use ai_coach_api::services::UserRecoveryProfileService;
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create a test user
async fn create_test_user(pool: &PgPool) -> sqlx::Result<Uuid> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO users (id, email, password_hash, created_at, updated_at)
        VALUES ($1, $2, $3, NOW(), NOW())
        "#,
        user_id,
        format!("test_{}@example.com", user_id),
        "$argon2id$v=19$m=19456,t=2,p=1$test"
    )
    .execute(pool)
    .await?;

    Ok(user_id)
}

/// Helper to create a recommendation template
async fn create_test_template(pool: &PgPool, category: &str) -> sqlx::Result<Uuid> {
    let template_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO recommendation_templates (
            id, category, title, description, action, priority_default,
            difficulty, trigger_conditions, user_constraints,
            effectiveness_score, times_suggested, times_completed,
            metadata, is_active, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW(), NOW())
        "#,
        template_id,
        category,
        "Test Recommendation",
        "Test description",
        "Test action",
        "medium",
        "easy",
        json!({}),
        json!({}),
        0.85,
        10,
        8,
        json!({}),
        true
    )
    .execute(pool)
    .await?;

    Ok(template_id)
}

#[sqlx::test]
async fn test_create_default_profile(pool: PgPool) -> sqlx::Result<()> {
    let service = UserRecoveryProfileService::new(pool.clone());
    let user_id = create_test_user(&pool).await?;

    let profile = service.create_default_profile(user_id).await.unwrap();

    assert_eq!(profile.user_id, user_id);
    assert_eq!(profile.preferred_recovery_activities, json!([]));
    assert_eq!(profile.available_equipment, json!([]));
    assert!(profile.dietary_preferences.is_object());
    assert!(profile.sleep_schedule.is_object());
    assert_eq!(profile.stress_triggers, json!([]));
    assert_eq!(profile.effective_techniques, json!([]));
    assert_eq!(profile.completion_stats, json!({}));

    Ok(())
}

#[sqlx::test]
async fn test_get_profile_creates_if_not_exists(pool: PgPool) -> sqlx::Result<()> {
    let service = UserRecoveryProfileService::new(pool.clone());
    let user_id = create_test_user(&pool).await?;

    // Should create profile automatically if it doesn't exist
    let profile = service.get_profile(user_id).await.unwrap();

    assert_eq!(profile.user_id, user_id);

    Ok(())
}

#[sqlx::test]
async fn test_update_profile_preferences(pool: PgPool) -> sqlx::Result<()> {
    let service = UserRecoveryProfileService::new(pool.clone());
    let user_id = create_test_user(&pool).await?;

    // Create initial profile
    let _initial = service.get_profile(user_id).await.unwrap();

    // Update profile
    let updates = UpdateProfileRequest {
        preferred_recovery_activities: Some(vec!["yoga".to_string(), "foam_rolling".to_string()]),
        available_equipment: Some(vec!["yoga_mat".to_string(), "foam_roller".to_string()]),
        dietary_preferences: Some(DietaryPreferences {
            vegetarian: Some(true),
            vegan: Some(false),
            allergies: Some(vec!["peanuts".to_string()]),
            restrictions: Some(vec![]),
            supplements: Some(vec!["vitamin_d".to_string()]),
        }),
        sleep_schedule: Some(SleepSchedule {
            bedtime: Some("23:00".to_string()),
            wake_time: Some("07:00".to_string()),
            target_duration_hours: Some(8.0),
            nap_preference: Some(true),
        }),
        stress_triggers: Some(vec!["work_deadlines".to_string()]),
    };

    let updated = service.update_profile(user_id, updates).await.unwrap();

    // Verify updates
    let activities: Vec<String> =
        serde_json::from_value(updated.preferred_recovery_activities).unwrap();
    assert_eq!(activities.len(), 2);
    assert!(activities.contains(&"yoga".to_string()));

    let equipment: Vec<String> = serde_json::from_value(updated.available_equipment).unwrap();
    assert_eq!(equipment.len(), 2);

    let diet: DietaryPreferences = serde_json::from_value(updated.dietary_preferences).unwrap();
    assert_eq!(diet.vegetarian, Some(true));

    let sleep: SleepSchedule = serde_json::from_value(updated.sleep_schedule).unwrap();
    assert_eq!(sleep.bedtime, Some("23:00".to_string()));

    Ok(())
}

#[sqlx::test]
async fn test_update_from_recommendation_completed(pool: PgPool) -> sqlx::Result<()> {
    let service = UserRecoveryProfileService::new(pool.clone());
    let user_id = create_test_user(&pool).await?;
    let template_id = create_test_template(&pool, "sleep").await?;

    // Create initial profile
    let _profile = service.get_profile(user_id).await.unwrap();

    // Create a completed recommendation
    let rec_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO user_recommendations (
            id, user_id, recommendation_template_id, status,
            shown_at, completed_at, created_at, updated_at, metadata
        ) VALUES ($1, $2, $3, $4, NOW(), NOW(), NOW(), NOW(), $5)
        "#,
        rec_id,
        user_id,
        template_id,
        "completed",
        json!({})
    )
    .execute(&pool)
    .await?;

    let recommendation = sqlx::query_as!(
        UserRecommendation,
        r#"
        SELECT id, user_id, recommendation_template_id, recovery_score_id,
               status as "status: _", effectiveness_rating, user_feedback,
               skip_reason, shown_at, completed_at, skipped_at, expired_at,
               rated_at, metadata, created_at, updated_at
        FROM user_recommendations WHERE id = $1
        "#,
        rec_id
    )
    .fetch_one(&pool)
    .await?;

    // Update profile from recommendation with good rating
    service
        .update_from_recommendation(user_id, &recommendation, Some(5))
        .await
        .unwrap();

    // Verify completion stats were updated
    let updated_profile = service.get_profile(user_id).await.unwrap();
    let stats: serde_json::Value = updated_profile.completion_stats;
    assert!(stats.get("sleep").is_some());

    // Verify effective techniques were updated
    let techniques: Vec<EffectiveTechnique> =
        serde_json::from_value(updated_profile.effective_techniques).unwrap();
    assert_eq!(techniques.len(), 1);
    assert_eq!(techniques[0].template_id, template_id);
    assert_eq!(techniques[0].category, "sleep");
    assert_eq!(techniques[0].average_rating, 5.0);

    Ok(())
}

#[sqlx::test]
async fn test_update_from_recommendation_low_rating(pool: PgPool) -> sqlx::Result<()> {
    let service = UserRecoveryProfileService::new(pool.clone());
    let user_id = create_test_user(&pool).await?;
    let template_id = create_test_template(&pool, "nutrition").await?;

    // Create initial profile
    let _profile = service.get_profile(user_id).await.unwrap();

    // Create a completed recommendation
    let rec_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO user_recommendations (
            id, user_id, recommendation_template_id, status,
            shown_at, completed_at, created_at, updated_at, metadata
        ) VALUES ($1, $2, $3, $4, NOW(), NOW(), NOW(), NOW(), $5)
        "#,
        rec_id,
        user_id,
        template_id,
        "completed",
        json!({})
    )
    .execute(&pool)
    .await?;

    let recommendation = sqlx::query_as!(
        UserRecommendation,
        r#"
        SELECT id, user_id, recommendation_template_id, recovery_score_id,
               status as "status: _", effectiveness_rating, user_feedback,
               skip_reason, shown_at, completed_at, skipped_at, expired_at,
               rated_at, metadata, created_at, updated_at
        FROM user_recommendations WHERE id = $1
        "#,
        rec_id
    )
    .fetch_one(&pool)
    .await?;

    // Update profile from recommendation with low rating (< 4)
    service
        .update_from_recommendation(user_id, &recommendation, Some(2))
        .await
        .unwrap();

    // Verify completion stats were updated but not effective techniques
    let updated_profile = service.get_profile(user_id).await.unwrap();
    let stats: serde_json::Value = updated_profile.completion_stats;
    assert!(stats.get("nutrition").is_some());

    // Effective techniques should be empty (rating < 4)
    let techniques: Vec<EffectiveTechnique> =
        serde_json::from_value(updated_profile.effective_techniques).unwrap();
    assert_eq!(techniques.len(), 0);

    Ok(())
}

#[sqlx::test]
async fn test_get_effective_techniques_sorted(pool: PgPool) -> sqlx::Result<()> {
    let service = UserRecoveryProfileService::new(pool.clone());
    let user_id = create_test_user(&pool).await?;

    // Create profile with multiple effective techniques
    let technique1 = EffectiveTechnique {
        template_id: Uuid::new_v4(),
        category: "sleep".to_string(),
        title: "Sleep technique 1".to_string(),
        completion_count: 5,
        total_shown: 5,
        completion_rate: 1.0,
        average_rating: 4.5,
        total_ratings: 5,
        last_completed: Some(Utc::now()),
        effectiveness_score: 0.85,
    };

    let technique2 = EffectiveTechnique {
        template_id: Uuid::new_v4(),
        category: "nutrition".to_string(),
        title: "Nutrition technique 1".to_string(),
        completion_count: 3,
        total_shown: 5,
        completion_rate: 0.6,
        average_rating: 4.0,
        total_ratings: 3,
        last_completed: Some(Utc::now()),
        effectiveness_score: 0.65,
    };

    let technique3 = EffectiveTechnique {
        template_id: Uuid::new_v4(),
        category: "stress_management".to_string(),
        title: "Stress technique 1".to_string(),
        completion_count: 8,
        total_shown: 10,
        completion_rate: 0.8,
        average_rating: 5.0,
        total_ratings: 8,
        last_completed: Some(Utc::now()),
        effectiveness_score: 0.92,
    };

    // Create profile with these techniques
    sqlx::query!(
        r#"
        INSERT INTO user_recovery_profiles (
            user_id, preferred_recovery_activities, available_equipment,
            dietary_preferences, sleep_schedule, stress_triggers,
            effective_techniques, completion_stats
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        user_id,
        json!([]),
        json!([]),
        json!(DietaryPreferences::default()),
        json!(SleepSchedule::default()),
        json!([]),
        json!([technique1, technique2, technique3]),
        json!({})
    )
    .execute(&pool)
    .await?;

    // Get effective techniques (should be sorted by effectiveness score)
    let techniques = service.get_effective_techniques(user_id, None).await.unwrap();

    assert_eq!(techniques.len(), 3);
    // Should be sorted by effectiveness score (highest first)
    assert_eq!(techniques[0].effectiveness_score, 0.92); // technique3
    assert_eq!(techniques[1].effectiveness_score, 0.85); // technique1
    assert_eq!(techniques[2].effectiveness_score, 0.65); // technique2

    Ok(())
}

#[sqlx::test]
async fn test_get_effective_techniques_with_limit(pool: PgPool) -> sqlx::Result<()> {
    let service = UserRecoveryProfileService::new(pool.clone());
    let user_id = create_test_user(&pool).await?;

    // Create profile with multiple techniques
    let techniques: Vec<EffectiveTechnique> = (0..5)
        .map(|i| EffectiveTechnique {
            template_id: Uuid::new_v4(),
            category: "sleep".to_string(),
            title: format!("Technique {}", i),
            completion_count: 5,
            total_shown: 5,
            completion_rate: 1.0,
            average_rating: 4.5,
            total_ratings: 5,
            last_completed: Some(Utc::now()),
            effectiveness_score: 0.9 - (i as f64 * 0.05),
        })
        .collect();

    sqlx::query!(
        r#"
        INSERT INTO user_recovery_profiles (
            user_id, preferred_recovery_activities, available_equipment,
            dietary_preferences, sleep_schedule, stress_triggers,
            effective_techniques, completion_stats
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        user_id,
        json!([]),
        json!([]),
        json!(DietaryPreferences::default()),
        json!(SleepSchedule::default()),
        json!([]),
        json!(techniques),
        json!({})
    )
    .execute(&pool)
    .await?;

    // Get with limit
    let limited_techniques = service
        .get_effective_techniques(user_id, Some(3))
        .await
        .unwrap();

    assert_eq!(limited_techniques.len(), 3);

    Ok(())
}

#[sqlx::test]
async fn test_calculate_effectiveness_score(pool: PgPool) -> sqlx::Result<()> {
    // Test perfect score
    let perfect_score = calculate_effectiveness_score(1.0, 5.0, 20);
    assert!(perfect_score > 0.9 && perfect_score <= 1.0);

    // Test good score
    let good_score = calculate_effectiveness_score(0.8, 4.5, 15);
    assert!(good_score > 0.7 && good_score < 0.9);

    // Test average score
    let avg_score = calculate_effectiveness_score(0.6, 3.5, 10);
    assert!(avg_score > 0.4 && avg_score < 0.7);

    // Test low score
    let low_score = calculate_effectiveness_score(0.3, 2.5, 5);
    assert!(low_score < 0.4);

    // Test frequency impact (more interactions = higher confidence)
    let score_few = calculate_effectiveness_score(0.8, 4.0, 5);
    let score_many = calculate_effectiveness_score(0.8, 4.0, 20);
    assert!(score_many > score_few);

    Ok(())
}

#[sqlx::test]
async fn test_get_insights(pool: PgPool) -> sqlx::Result<()> {
    let service = UserRecoveryProfileService::new(pool.clone());
    let user_id = create_test_user(&pool).await?;
    let template_id = create_test_template(&pool, "sleep").await?;

    // Create profile with effective technique
    let technique = EffectiveTechnique {
        template_id,
        category: "sleep".to_string(),
        title: "Sleep optimization".to_string(),
        completion_count: 8,
        total_shown: 10,
        completion_rate: 0.8,
        average_rating: 4.5,
        total_ratings: 8,
        last_completed: Some(Utc::now()),
        effectiveness_score: 0.85,
    };

    sqlx::query!(
        r#"
        INSERT INTO user_recovery_profiles (
            user_id, preferred_recovery_activities, available_equipment,
            dietary_preferences, sleep_schedule, stress_triggers,
            effective_techniques, completion_stats
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        user_id,
        json!([]),
        json!([]),
        json!(DietaryPreferences::default()),
        json!(SleepSchedule::default()),
        json!([]),
        json!([technique]),
        json!({})
    )
    .execute(&pool)
    .await?;

    // Create some recommendations
    for i in 0..5 {
        let status = if i < 4 { "completed" } else { "skipped" };
        sqlx::query!(
            r#"
            INSERT INTO user_recommendations (
                id, user_id, recommendation_template_id, status,
                shown_at, created_at, updated_at, metadata
            ) VALUES ($1, $2, $3, $4, NOW(), NOW(), NOW(), $5)
            "#,
            Uuid::new_v4(),
            user_id,
            template_id,
            status,
            json!({})
        )
        .execute(&pool)
        .await?;
    }

    // Get insights
    let insights = service.get_insights(user_id).await.unwrap();

    // Should have at least one insight about effective category
    assert!(!insights.insights.is_empty());

    // Should have recommendations summary
    assert_eq!(insights.recommendations_summary.total_shown, 5);
    assert_eq!(insights.recommendations_summary.total_completed, 4);
    assert_eq!(insights.recommendations_summary.total_skipped, 1);
    assert_eq!(insights.recommendations_summary.overall_completion_rate, 0.8);

    Ok(())
}
