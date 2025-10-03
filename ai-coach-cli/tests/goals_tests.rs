use ai_coach_cli::models::{Goal, GoalType};
use ai_coach_cli::storage::Storage;
use anyhow::Result;
use chrono::{Duration, Utc};
use std::env;
use tempfile::tempdir;

fn create_test_storage() -> Result<Storage> {
    let dir = tempdir()?;
    // Set environment variable for test database path
    env::set_var("AI_COACH_DB_PATH", dir.path());
    Storage::init()
}

#[test]
fn test_goal_creation() -> Result<()> {
    let target_date = Utc::now() + Duration::days(30);
    let goal = Goal::new(
        "Run 100km".to_string(),
        GoalType::Distance,
        target_date,
        Some(100.0),
        Some("Marathon training".to_string()),
    );

    assert_eq!(goal.title, "Run 100km");
    assert_eq!(goal.goal_type, GoalType::Distance);
    assert_eq!(goal.target_value, Some(100.0));
    assert_eq!(goal.current_value, 0.0);
    assert_eq!(goal.completed, false);
    assert_eq!(goal.notes, Some("Marathon training".to_string()));

    Ok(())
}

#[test]
fn test_goal_progress_calculation() -> Result<()> {
    let target_date = Utc::now() + Duration::days(30);
    let mut goal = Goal::new(
        "Run 100km".to_string(),
        GoalType::Distance,
        target_date,
        Some(100.0),
        None,
    );

    // Test 0% progress
    assert_eq!(goal.progress_percentage(), 0.0);

    // Test 50% progress
    goal.update_progress(50.0);
    assert_eq!(goal.progress_percentage(), 50.0);

    // Test 100% progress
    goal.update_progress(100.0);
    assert_eq!(goal.progress_percentage(), 100.0);

    // Test over 100% (should cap at 100)
    goal.update_progress(150.0);
    assert_eq!(goal.progress_percentage(), 100.0);

    Ok(())
}

#[test]
fn test_goal_days_remaining() -> Result<()> {
    let target_date = Utc::now() + Duration::days(7);
    let goal = Goal::new(
        "Run marathon".to_string(),
        GoalType::Event,
        target_date,
        None,
        None,
    );

    let days = goal.days_remaining();
    assert!(days >= 6 && days <= 7); // Account for timing variations

    Ok(())
}

#[test]
fn test_goal_completion() -> Result<()> {
    let target_date = Utc::now() + Duration::days(30);
    let mut goal = Goal::new(
        "Run 100km".to_string(),
        GoalType::Distance,
        target_date,
        Some(100.0),
        None,
    );

    assert_eq!(goal.completed, false);
    assert!(goal.completed_at.is_none());

    goal.mark_complete();

    assert_eq!(goal.completed, true);
    assert!(goal.completed_at.is_some());

    Ok(())
}

#[test]
fn test_save_and_get_goal() -> Result<()> {
    let storage = create_test_storage()?;
    let target_date = Utc::now() + Duration::days(30);

    let goal = Goal::new(
        "Run 100km".to_string(),
        GoalType::Distance,
        target_date,
        Some(100.0),
        None,
    );

    storage.save_goal(&goal)?;

    let retrieved = storage.get_goal(&goal.id)?;
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, goal.id);
    assert_eq!(retrieved.title, goal.title);
    assert_eq!(retrieved.goal_type, goal.goal_type);

    Ok(())
}

#[test]
fn test_list_goals() -> Result<()> {
    let storage = create_test_storage()?;
    let target_date = Utc::now() + Duration::days(30);

    let goal1 = Goal::new(
        "Run 100km".to_string(),
        GoalType::Distance,
        target_date,
        Some(100.0),
        None,
    );

    let goal2 = Goal::new(
        "Cycle 500km".to_string(),
        GoalType::Distance,
        target_date + Duration::days(10),
        Some(500.0),
        None,
    );

    storage.save_goal(&goal1)?;
    storage.save_goal(&goal2)?;

    let goals = storage.list_goals(false)?;
    assert_eq!(goals.len(), 2);

    // Goals should be sorted by target date
    assert_eq!(goals[0].id, goal1.id);
    assert_eq!(goals[1].id, goal2.id);

    Ok(())
}

#[test]
fn test_list_goals_filter_completed() -> Result<()> {
    let storage = create_test_storage()?;
    let target_date = Utc::now() + Duration::days(30);

    let mut goal1 = Goal::new(
        "Run 100km".to_string(),
        GoalType::Distance,
        target_date,
        Some(100.0),
        None,
    );
    goal1.mark_complete();

    let goal2 = Goal::new(
        "Cycle 500km".to_string(),
        GoalType::Distance,
        target_date + Duration::days(10),
        Some(500.0),
        None,
    );

    storage.save_goal(&goal1)?;
    storage.save_goal(&goal2)?;

    // List active goals only
    let active_goals = storage.list_goals(false)?;
    assert_eq!(active_goals.len(), 1);
    assert_eq!(active_goals[0].id, goal2.id);

    // List all goals including completed
    let all_goals = storage.list_goals(true)?;
    assert_eq!(all_goals.len(), 2);

    Ok(())
}

#[test]
fn test_update_goal() -> Result<()> {
    let storage = create_test_storage()?;
    let target_date = Utc::now() + Duration::days(30);

    let goal = Goal::new(
        "Run 100km".to_string(),
        GoalType::Distance,
        target_date,
        Some(100.0),
        None,
    );

    storage.save_goal(&goal)?;

    let mut updated_goal = storage.get_goal(&goal.id)?.unwrap();
    updated_goal.update(
        Some("Run 150km".to_string()),
        None,
        Some(150.0),
        Some("Updated goal".to_string()),
    );

    storage.save_goal(&updated_goal)?;

    let retrieved = storage.get_goal(&goal.id)?.unwrap();
    assert_eq!(retrieved.title, "Run 150km");
    assert_eq!(retrieved.target_value, Some(150.0));
    assert_eq!(retrieved.notes, Some("Updated goal".to_string()));

    Ok(())
}

#[test]
fn test_delete_goal() -> Result<()> {
    let storage = create_test_storage()?;
    let target_date = Utc::now() + Duration::days(30);

    let goal = Goal::new(
        "Run 100km".to_string(),
        GoalType::Distance,
        target_date,
        Some(100.0),
        None,
    );

    storage.save_goal(&goal)?;

    let deleted = storage.delete_goal(&goal.id)?;
    assert!(deleted);

    let retrieved = storage.get_goal(&goal.id)?;
    assert!(retrieved.is_none());

    Ok(())
}

#[test]
fn test_goal_type_display() {
    assert_eq!(format!("{}", GoalType::Distance), "Distance");
    assert_eq!(format!("{}", GoalType::Duration), "Duration");
    assert_eq!(format!("{}", GoalType::Event), "Event");
    assert_eq!(format!("{}", GoalType::Frequency), "Frequency");
}

#[test]
fn test_goal_type_from_str() -> Result<()> {
    use std::str::FromStr;

    assert_eq!(GoalType::from_str("distance")?, GoalType::Distance);
    assert_eq!(GoalType::from_str("Duration")?, GoalType::Duration);
    assert_eq!(GoalType::from_str("EVENT")?, GoalType::Event);
    assert_eq!(GoalType::from_str("frequency")?, GoalType::Frequency);

    assert!(GoalType::from_str("invalid").is_err());

    Ok(())
}
