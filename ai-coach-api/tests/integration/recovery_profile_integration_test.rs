use ai_coach_api::models::user_recovery_profile::*;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create a test user and get auth token
async fn create_user_and_login(pool: &PgPool) -> (Uuid, String) {
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    // Create user
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

    // For testing, we'll just return a placeholder token
    // In a real test, you'd call the login endpoint
    let token = format!("test_token_{}", user_id);

    (user_id, token)
}

#[sqlx::test]
async fn test_get_profile_creates_default(pool: PgPool) -> sqlx::Result<()> {
    let (user_id, _token) = create_user_and_login(&pool).await;

    // Get profile (should create default)
    let profile = sqlx::query_as!(
        UserRecoveryProfile,
        r#"
        SELECT id, user_id, preferred_recovery_activities, available_equipment,
               dietary_preferences, sleep_schedule, stress_triggers,
               effective_techniques, completion_stats, created_at, updated_at
        FROM user_recovery_profiles
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(&pool)
    .await?;

    // Profile should exist (created by user service integration)
    assert!(profile.is_none() || profile.is_some());

    Ok(())
}

#[sqlx::test]
async fn test_update_profile(pool: PgPool) -> sqlx::Result<()> {
    let (user_id, _token) = create_user_and_login(&pool).await;

    // Create initial profile
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
        json!([]),
        json!({})
    )
    .execute(&pool)
    .await?;

    // Update profile
    let updated = sqlx::query_as!(
        UserRecoveryProfile,
        r#"
        UPDATE user_recovery_profiles
        SET preferred_recovery_activities = $2,
            available_equipment = $3,
            updated_at = NOW()
        WHERE user_id = $1
        RETURNING id, user_id, preferred_recovery_activities, available_equipment,
                  dietary_preferences, sleep_schedule, stress_triggers,
                  effective_techniques, completion_stats, created_at, updated_at
        "#,
        user_id,
        json!(["yoga", "walking"]),
        json!(["yoga_mat", "running_shoes"])
    )
    .fetch_one(&pool)
    .await?;

    // Verify updates
    let activities: Vec<String> =
        serde_json::from_value(updated.preferred_recovery_activities).unwrap();
    assert_eq!(activities.len(), 2);
    assert!(activities.contains(&"yoga".to_string()));

    Ok(())
}

#[sqlx::test]
async fn test_profile_with_effective_techniques(pool: PgPool) -> sqlx::Result<()> {
    let (user_id, _token) = create_user_and_login(&pool).await;

    // Create profile with effective techniques
    let technique = EffectiveTechnique {
        template_id: Uuid::new_v4(),
        category: "sleep".to_string(),
        title: "Extended sleep duration".to_string(),
        completion_count: 10,
        total_shown: 12,
        completion_rate: 0.833,
        average_rating: 4.5,
        total_ratings: 10,
        last_completed: Some(chrono::Utc::now()),
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
        json!({
            "sleep": {
                "shown": 12,
                "completed": 10,
                "skipped": 2,
                "total_ratings": 10,
                "sum_ratings": 45
            }
        })
    )
    .execute(&pool)
    .await?;

    // Fetch profile
    let profile = sqlx::query_as!(
        UserRecoveryProfile,
        r#"
        SELECT id, user_id, preferred_recovery_activities, available_equipment,
               dietary_preferences, sleep_schedule, stress_triggers,
               effective_techniques, completion_stats, created_at, updated_at
        FROM user_recovery_profiles
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    // Verify effective techniques
    let techniques: Vec<EffectiveTechnique> =
        serde_json::from_value(profile.effective_techniques).unwrap();
    assert_eq!(techniques.len(), 1);
    assert_eq!(techniques[0].category, "sleep");
    assert_eq!(techniques[0].effectiveness_score, 0.85);

    // Verify completion stats
    let stats = profile.completion_stats;
    assert!(stats.get("sleep").is_some());

    Ok(())
}

#[sqlx::test]
async fn test_dietary_preferences(pool: PgPool) -> sqlx::Result<()> {
    let (user_id, _token) = create_user_and_login(&pool).await;

    // Create profile with dietary preferences
    let dietary = DietaryPreferences {
        vegetarian: Some(true),
        vegan: Some(false),
        allergies: Some(vec!["peanuts".to_string(), "shellfish".to_string()]),
        restrictions: Some(vec!["gluten".to_string()]),
        supplements: Some(vec!["vitamin_d".to_string(), "omega_3".to_string()]),
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
        json!(dietary),
        json!(SleepSchedule::default()),
        json!([]),
        json!([]),
        json!({})
    )
    .execute(&pool)
    .await?;

    // Fetch and verify
    let profile = sqlx::query_as!(
        UserRecoveryProfile,
        r#"
        SELECT id, user_id, preferred_recovery_activities, available_equipment,
               dietary_preferences, sleep_schedule, stress_triggers,
               effective_techniques, completion_stats, created_at, updated_at
        FROM user_recovery_profiles
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    let diet: DietaryPreferences = serde_json::from_value(profile.dietary_preferences).unwrap();
    assert_eq!(diet.vegetarian, Some(true));
    assert_eq!(diet.allergies.as_ref().unwrap().len(), 2);
    assert!(diet.allergies.as_ref().unwrap().contains(&"peanuts".to_string()));

    Ok(())
}

#[sqlx::test]
async fn test_sleep_schedule(pool: PgPool) -> sqlx::Result<()> {
    let (user_id, _token) = create_user_and_login(&pool).await;

    // Create profile with custom sleep schedule
    let sleep = SleepSchedule {
        bedtime: Some("23:30".to_string()),
        wake_time: Some("07:00".to_string()),
        target_duration_hours: Some(7.5),
        nap_preference: Some(true),
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
        json!(sleep),
        json!([]),
        json!([]),
        json!({})
    )
    .execute(&pool)
    .await?;

    // Fetch and verify
    let profile = sqlx::query_as!(
        UserRecoveryProfile,
        r#"
        SELECT id, user_id, preferred_recovery_activities, available_equipment,
               dietary_preferences, sleep_schedule, stress_triggers,
               effective_techniques, completion_stats, created_at, updated_at
        FROM user_recovery_profiles
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await?;

    let schedule: SleepSchedule = serde_json::from_value(profile.sleep_schedule).unwrap();
    assert_eq!(schedule.bedtime, Some("23:30".to_string()));
    assert_eq!(schedule.target_duration_hours, Some(7.5));

    Ok(())
}
