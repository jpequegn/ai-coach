use ai_coach_api::models::recommendation::{
    RecommendationCategory, RecommendationContext, RecommendationDifficulty, RecommendationPriority,
    RecommendationTemplate, RecoveryIssues, TimeOfDay, Location,
};
use ai_coach_api::models::RecoveryScore;
use ai_coach_api::services::RecommendationEngine;
use chrono::{NaiveDate, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create a recovery score for testing
fn create_test_recovery_score(
    user_id: Uuid,
    readiness: f64,
    hrv_dev: Option<f64>,
    rhr_dev: Option<f64>,
    sleep_quality: Option<f64>,
    hrv_trend: &str,
) -> RecoveryScore {
    RecoveryScore {
        id: Uuid::new_v4(),
        user_id,
        score_date: NaiveDate::from_ymd_opt(2025, 10, 8).unwrap(),
        readiness_score: readiness,
        hrv_trend: hrv_trend.to_string(),
        hrv_deviation: hrv_dev,
        sleep_quality_score: sleep_quality,
        recovery_adequacy: Some(75.0),
        rhr_deviation: rhr_dev,
        training_strain: Some(50.0),
        recovery_status: "fair".to_string(),
        recommended_tss_adjustment: Some(0.0),
        calculated_at: Utc::now(),
        model_version: Some("v1.0".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Helper to create a recommendation template for testing
fn create_test_template(
    category: RecommendationCategory,
    priority: RecommendationPriority,
    difficulty: RecommendationDifficulty,
    trigger_conditions: serde_json::Value,
) -> RecommendationTemplate {
    RecommendationTemplate {
        id: Uuid::new_v4(),
        category,
        title: "Test Recommendation".to_string(),
        description: "Test description".to_string(),
        action: "Test action".to_string(),
        priority_default: priority,
        difficulty,
        expected_impact: Some("Test impact".to_string()),
        time_required_minutes: Some(30),
        trigger_conditions,
        user_constraints: json!({}),
        effectiveness_score: 0.85,
        times_suggested: 10,
        times_completed: 8,
        metadata: json!({}),
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[sqlx::test]
async fn test_identify_issues_poor_sleep(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);
    let user_id = Uuid::new_v4();

    // Create recovery score with poor sleep (< 70)
    let score = create_test_recovery_score(user_id, 75.0, None, None, Some(65.0), "stable");

    let issues = engine.identify_issues(&score).await?;

    assert!(issues.poor_sleep, "Should detect poor sleep");
    assert!(!issues.low_hrv, "Should not detect low HRV");
    assert!(!issues.high_rhr, "Should not detect high RHR");
    assert!(!issues.accumulated_fatigue, "Should not detect fatigue");
    assert!(!issues.high_stress, "Should not detect stress");

    Ok(())
}

#[sqlx::test]
async fn test_identify_issues_low_hrv(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);
    let user_id = Uuid::new_v4();

    // Create recovery score with low HRV (< -10%)
    let score = create_test_recovery_score(user_id, 75.0, Some(-12.0), None, Some(80.0), "stable");

    let issues = engine.identify_issues(&score).await?;

    assert!(!issues.poor_sleep, "Should not detect poor sleep");
    assert!(issues.low_hrv, "Should detect low HRV");
    assert!(!issues.high_rhr, "Should not detect high RHR");

    Ok(())
}

#[sqlx::test]
async fn test_identify_issues_high_rhr(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);
    let user_id = Uuid::new_v4();

    // Create recovery score with high RHR (> 5%)
    let score = create_test_recovery_score(user_id, 75.0, None, Some(7.0), Some(80.0), "stable");

    let issues = engine.identify_issues(&score).await?;

    assert!(!issues.poor_sleep, "Should not detect poor sleep");
    assert!(!issues.low_hrv, "Should not detect low HRV");
    assert!(issues.high_rhr, "Should detect high RHR");

    Ok(())
}

#[sqlx::test]
async fn test_identify_issues_accumulated_fatigue(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);
    let user_id = Uuid::new_v4();

    // Create recovery score with accumulated fatigue (readiness < 60)
    let score = create_test_recovery_score(user_id, 55.0, None, None, Some(80.0), "stable");

    let issues = engine.identify_issues(&score).await?;

    assert!(!issues.poor_sleep, "Should not detect poor sleep");
    assert!(issues.accumulated_fatigue, "Should detect accumulated fatigue");

    Ok(())
}

#[sqlx::test]
async fn test_identify_issues_high_stress(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);
    let user_id = Uuid::new_v4();

    // Create recovery score with high stress (declining HRV trend)
    let score = create_test_recovery_score(user_id, 75.0, Some(-5.0), None, Some(80.0), "declining");

    let issues = engine.identify_issues(&score).await?;

    assert!(issues.high_stress, "Should detect high stress from declining trend");

    Ok(())
}

#[sqlx::test]
async fn test_identify_issues_sharply_declining(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);
    let user_id = Uuid::new_v4();

    // Create recovery score with sharply declining HRV trend
    let score = create_test_recovery_score(user_id, 75.0, Some(-5.0), None, Some(80.0), "sharply_declining");

    let issues = engine.identify_issues(&score).await?;

    assert!(issues.high_stress, "Should detect high stress from sharply declining trend");

    Ok(())
}

#[sqlx::test]
async fn test_identify_issues_multiple(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);
    let user_id = Uuid::new_v4();

    // Create recovery score with multiple issues
    let score = create_test_recovery_score(
        user_id,
        55.0,           // accumulated fatigue
        Some(-12.0),    // low HRV
        Some(8.0),      // high RHR
        Some(65.0),     // poor sleep
        "declining"     // high stress
    );

    let issues = engine.identify_issues(&score).await?;

    assert!(issues.poor_sleep, "Should detect poor sleep");
    assert!(issues.low_hrv, "Should detect low HRV");
    assert!(issues.high_rhr, "Should detect high RHR");
    assert!(issues.accumulated_fatigue, "Should detect accumulated fatigue");
    assert!(issues.high_stress, "Should detect high stress");

    Ok(())
}

#[sqlx::test]
async fn test_identify_issues_no_issues(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);
    let user_id = Uuid::new_v4();

    // Create recovery score with no issues
    let score = create_test_recovery_score(
        user_id,
        85.0,          // good readiness
        Some(5.0),     // good HRV
        Some(0.0),     // normal RHR
        Some(85.0),    // good sleep
        "improving"    // improving trend
    );

    let issues = engine.identify_issues(&score).await?;

    assert!(!issues.poor_sleep, "Should not detect poor sleep");
    assert!(!issues.low_hrv, "Should not detect low HRV");
    assert!(!issues.high_rhr, "Should not detect high RHR");
    assert!(!issues.accumulated_fatigue, "Should not detect accumulated fatigue");
    assert!(!issues.high_stress, "Should not detect high stress");

    Ok(())
}

#[sqlx::test]
async fn test_matches_issues_sleep(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    let template = create_test_template(
        RecommendationCategory::Sleep,
        RecommendationPriority::High,
        RecommendationDifficulty::Easy,
        json!({"poor_sleep": true})
    );

    let mut issues = RecoveryIssues::default();
    issues.poor_sleep = true;

    assert!(engine.matches_issues(&template, &issues), "Should match poor sleep issue");

    issues.poor_sleep = false;
    assert!(!engine.matches_issues(&template, &issues), "Should not match when no poor sleep");

    Ok(())
}

#[sqlx::test]
async fn test_matches_issues_multiple_conditions(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    // Template triggered by either poor sleep OR low HRV
    let template = create_test_template(
        RecommendationCategory::StressManagement,
        RecommendationPriority::High,
        RecommendationDifficulty::Moderate,
        json!({"poor_sleep": true, "low_hrv": true})
    );

    let mut issues = RecoveryIssues::default();

    // Should match with just poor sleep
    issues.poor_sleep = true;
    assert!(engine.matches_issues(&template, &issues), "Should match with poor sleep");

    // Should match with just low HRV
    issues.poor_sleep = false;
    issues.low_hrv = true;
    assert!(engine.matches_issues(&template, &issues), "Should match with low HRV");

    // Should match with both
    issues.poor_sleep = true;
    issues.low_hrv = true;
    assert!(engine.matches_issues(&template, &issues), "Should match with both conditions");

    Ok(())
}

#[sqlx::test]
async fn test_issue_match_score_perfect_match(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    let template = create_test_template(
        RecommendationCategory::Sleep,
        RecommendationPriority::High,
        RecommendationDifficulty::Easy,
        json!({"poor_sleep": true})
    );

    let mut issues = RecoveryIssues::default();
    issues.poor_sleep = true;

    let score = engine.issue_match_score(&template, &issues);

    assert_eq!(score, 1.0, "Perfect match should score 1.0");

    Ok(())
}

#[sqlx::test]
async fn test_issue_match_score_partial_match(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    // Template addresses sleep issues
    let template = create_test_template(
        RecommendationCategory::Sleep,
        RecommendationPriority::High,
        RecommendationDifficulty::Easy,
        json!({"poor_sleep": true})
    );

    // User has sleep issues AND stress issues
    let mut issues = RecoveryIssues::default();
    issues.poor_sleep = true;
    issues.high_stress = true;

    let score = engine.issue_match_score(&template, &issues);

    // Should be 1 match out of 2 issues = 0.5
    assert_eq!(score, 0.5, "Partial match should score 0.5");

    Ok(())
}

#[sqlx::test]
async fn test_issue_match_score_no_issues(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    let template = create_test_template(
        RecommendationCategory::Sleep,
        RecommendationPriority::Low,
        RecommendationDifficulty::Easy,
        json!({"poor_sleep": true})
    );

    let issues = RecoveryIssues::default();

    let score = engine.issue_match_score(&template, &issues);

    assert_eq!(score, 0.5, "No issues should score 0.5 (medium relevance)");

    Ok(())
}

#[sqlx::test]
async fn test_novelty_score_never_shown(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    let template = create_test_template(
        RecommendationCategory::Sleep,
        RecommendationPriority::High,
        RecommendationDifficulty::Easy,
        json!({"poor_sleep": true})
    );

    let history = ai_coach_api::models::recommendation::UserRecommendationHistory {
        completed_templates: vec![],
        skipped_templates: vec![],
        recent_categories: vec![],
        last_shown_dates: std::collections::HashMap::new(),
        completion_rate_by_template: std::collections::HashMap::new(),
    };

    let score = engine.novelty_score(&template, &history);

    assert_eq!(score, 1.0, "Never shown should score 1.0");

    Ok(())
}

#[sqlx::test]
async fn test_novelty_score_recently_shown(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    let template = create_test_template(
        RecommendationCategory::Sleep,
        RecommendationPriority::High,
        RecommendationDifficulty::Easy,
        json!({"poor_sleep": true})
    );

    let mut last_shown = std::collections::HashMap::new();
    last_shown.insert(template.id, Utc::now()); // Shown today

    let history = ai_coach_api::models::recommendation::UserRecommendationHistory {
        completed_templates: vec![],
        skipped_templates: vec![],
        recent_categories: vec![],
        last_shown_dates: last_shown,
        completion_rate_by_template: std::collections::HashMap::new(),
    };

    let score = engine.novelty_score(&template, &history);

    assert_eq!(score, 0.2, "Recently shown (0-2 days) should score 0.2");

    Ok(())
}

#[sqlx::test]
async fn test_determine_priority_critical_fatigue(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    let template = create_test_template(
        RecommendationCategory::TrainingModification,
        RecommendationPriority::Medium,
        RecommendationDifficulty::Easy,
        json!({"accumulated_fatigue": true})
    );

    let mut issues = RecoveryIssues::default();
    issues.accumulated_fatigue = true;

    let priority = engine.determine_priority(0.8, &issues, &template);

    assert_eq!(
        priority,
        RecommendationPriority::Critical,
        "High score with accumulated fatigue should be critical"
    );

    Ok(())
}

#[sqlx::test]
async fn test_determine_priority_upgrade_on_high_score(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    let template = create_test_template(
        RecommendationCategory::Sleep,
        RecommendationPriority::Low,
        RecommendationDifficulty::Easy,
        json!({"poor_sleep": true})
    );

    let issues = RecoveryIssues::default();

    let priority = engine.determine_priority(0.85, &issues, &template);

    assert_eq!(
        priority,
        RecommendationPriority::Medium,
        "High score should upgrade Low to Medium"
    );

    Ok(())
}

#[sqlx::test]
async fn test_generate_why_message_single_issue(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    let template = create_test_template(
        RecommendationCategory::Sleep,
        RecommendationPriority::High,
        RecommendationDifficulty::Easy,
        json!({"poor_sleep": true})
    );

    let mut issues = RecoveryIssues::default();
    issues.poor_sleep = true;

    let message = engine.generate_why_message(&template, &issues);

    assert!(
        message.contains("your sleep quality has been suboptimal"),
        "Message should mention sleep quality"
    );
    assert!(
        message.contains("Test impact"),
        "Message should include expected impact"
    );

    Ok(())
}

#[sqlx::test]
async fn test_generate_why_message_multiple_issues(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    let template = create_test_template(
        RecommendationCategory::ActiveRecovery,
        RecommendationPriority::High,
        RecommendationDifficulty::Moderate,
        json!({"low_hrv": true, "high_rhr": true})
    );

    let mut issues = RecoveryIssues::default();
    issues.low_hrv = true;
    issues.high_rhr = true;

    let message = engine.generate_why_message(&template, &issues);

    assert!(
        message.contains("your HRV is below baseline"),
        "Message should mention HRV"
    );
    assert!(
        message.contains("your resting heart rate is elevated"),
        "Message should mention RHR"
    );
    assert!(
        message.contains(", and "),
        "Multiple issues should be joined with 'and'"
    );

    Ok(())
}

#[sqlx::test]
async fn test_diversity_filtering_max_per_category(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    // Create 3 sleep recommendations
    let sleep_recs: Vec<ai_coach_api::models::recommendation::ScoredRecommendation> = (0..3)
        .map(|i| {
            let template = create_test_template(
                RecommendationCategory::Sleep,
                RecommendationPriority::High,
                RecommendationDifficulty::Easy,
                json!({"poor_sleep": true})
            );
            ai_coach_api::models::recommendation::ScoredRecommendation {
                template,
                score: 0.9 - (i as f64 * 0.01),
                priority: RecommendationPriority::High,
                why_this_matters: "Test".to_string(),
                recovery_score_id: None,
            }
        })
        .collect();

    let filtered = engine.apply_diversity_filtering(sleep_recs, 5);

    assert_eq!(
        filtered.len(),
        2,
        "Should limit to 2 recommendations per category"
    );

    Ok(())
}

#[sqlx::test]
async fn test_diversity_filtering_variety(pool: PgPool) -> sqlx::Result<()> {
    let engine = RecommendationEngine::new(pool);

    // Create recommendations from different categories
    let mut recs = vec![];

    // 2 sleep
    for i in 0..2 {
        let template = create_test_template(
            RecommendationCategory::Sleep,
            RecommendationPriority::High,
            RecommendationDifficulty::Easy,
            json!({"poor_sleep": true})
        );
        recs.push(ai_coach_api::models::recommendation::ScoredRecommendation {
            template,
            score: 0.9 - (i as f64 * 0.01),
            priority: RecommendationPriority::High,
            why_this_matters: "Test".to_string(),
            recovery_score_id: None,
        });
    }

    // 2 nutrition
    for i in 0..2 {
        let template = create_test_template(
            RecommendationCategory::Nutrition,
            RecommendationPriority::Medium,
            RecommendationDifficulty::Easy,
            json!({"accumulated_fatigue": true})
        );
        recs.push(ai_coach_api::models::recommendation::ScoredRecommendation {
            template,
            score: 0.85 - (i as f64 * 0.01),
            priority: RecommendationPriority::Medium,
            why_this_matters: "Test".to_string(),
            recovery_score_id: None,
        });
    }

    // 1 active recovery
    let template = create_test_template(
        RecommendationCategory::ActiveRecovery,
        RecommendationPriority::Low,
        RecommendationDifficulty::Moderate,
        json!({"high_stress": true})
    );
    recs.push(ai_coach_api::models::recommendation::ScoredRecommendation {
        template,
        score: 0.80,
        priority: RecommendationPriority::Low,
        why_this_matters: "Test".to_string(),
        recovery_score_id: None,
    });

    let filtered = engine.apply_diversity_filtering(recs, 5);

    assert_eq!(filtered.len(), 5, "Should return 5 recommendations");

    // Count categories
    let sleep_count = filtered.iter().filter(|r| matches!(r.template.category, RecommendationCategory::Sleep)).count();
    let nutrition_count = filtered.iter().filter(|r| matches!(r.template.category, RecommendationCategory::Nutrition)).count();
    let recovery_count = filtered.iter().filter(|r| matches!(r.template.category, RecommendationCategory::ActiveRecovery)).count();

    assert_eq!(sleep_count, 2, "Should have 2 sleep recommendations");
    assert_eq!(nutrition_count, 2, "Should have 2 nutrition recommendations");
    assert_eq!(recovery_count, 1, "Should have 1 active recovery recommendation");

    Ok(())
}
