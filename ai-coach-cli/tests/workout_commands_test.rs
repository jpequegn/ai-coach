use ai_coach_cli::models::Workout;
use ai_coach_cli::storage::Storage;
use anyhow::Result;
use chrono::Utc;
use tempfile::TempDir;

/// Helper to create a temporary storage for testing
fn setup_test_storage() -> Result<(Storage, TempDir)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_ai_coach.db");

    // Set environment variable for test database
    std::env::set_var("AI_COACH_DB_PATH", db_path.to_str().unwrap());

    let storage = Storage::init()?;
    Ok((storage, temp_dir))
}

/// Serial test marker to avoid concurrent database access
/// Use with: cargo test -- --test-threads=1
mod serial_tests {
    use super::*;

#[test]
fn test_workout_creation() -> Result<()> {
    let workout = Workout::new(
        "running".to_string(),
        Some(30),
        Some(5.0),
        Some("Morning run".to_string()),
    );

    assert_eq!(workout.exercise_type, "running");
    assert_eq!(workout.duration_minutes, Some(30));
    assert_eq!(workout.distance_km, Some(5.0));
    assert_eq!(workout.notes, Some("Morning run".to_string()));
    assert_eq!(workout.synced, false);
    assert!(!workout.id.is_empty());

    Ok(())
}

#[test]
fn test_workout_update() -> Result<()> {
    let mut workout = Workout::new(
        "running".to_string(),
        Some(30),
        Some(5.0),
        None,
    );

    let initial_updated_at = workout.updated_at;

    // Sleep briefly to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    workout.update(
        Some("cycling".to_string()),
        Some(60),
        Some(25.0),
        Some("Bike ride".to_string()),
    );

    assert_eq!(workout.exercise_type, "cycling");
    assert_eq!(workout.duration_minutes, Some(60));
    assert_eq!(workout.distance_km, Some(25.0));
    assert_eq!(workout.notes, Some("Bike ride".to_string()));
    assert_eq!(workout.synced, false);
    assert!(workout.updated_at > initial_updated_at);

    Ok(())
}

#[test]
fn test_storage_save_and_get_workout() -> Result<()> {
    let (storage, _temp_dir) = setup_test_storage()?;

    let workout = Workout::new(
        "swimming".to_string(),
        Some(45),
        Some(2.0),
        Some("Pool session".to_string()),
    );

    let workout_id = workout.id.clone();

    // Save workout
    storage.save_workout(&workout)?;

    // Retrieve workout
    let retrieved = storage.get_workout(&workout_id)?;
    assert!(retrieved.is_some());

    let retrieved_workout = retrieved.unwrap();
    assert_eq!(retrieved_workout.id, workout_id);
    assert_eq!(retrieved_workout.exercise_type, "swimming");
    assert_eq!(retrieved_workout.duration_minutes, Some(45));
    assert_eq!(retrieved_workout.distance_km, Some(2.0));

    Ok(())
}

#[test]
fn test_storage_list_workouts() -> Result<()> {
    let (storage, _temp_dir) = setup_test_storage()?;

    // Create multiple workouts
    let workout1 = Workout::new("running".to_string(), Some(30), Some(5.0), None);
    let workout2 = Workout::new("cycling".to_string(), Some(60), Some(25.0), None);
    let workout3 = Workout::new("swimming".to_string(), Some(45), Some(2.0), None);

    storage.save_workout(&workout1)?;
    std::thread::sleep(std::time::Duration::from_millis(10));
    storage.save_workout(&workout2)?;
    std::thread::sleep(std::time::Duration::from_millis(10));
    storage.save_workout(&workout3)?;

    // List all workouts
    let workouts = storage.list_workouts()?;
    assert_eq!(workouts.len(), 3);

    // Workouts should be sorted by date (most recent first)
    assert_eq!(workouts[0].exercise_type, "swimming");
    assert_eq!(workouts[1].exercise_type, "cycling");
    assert_eq!(workouts[2].exercise_type, "running");

    Ok(())
}

#[test]
fn test_storage_delete_workout() -> Result<()> {
    let (storage, _temp_dir) = setup_test_storage()?;

    let workout = Workout::new("running".to_string(), Some(30), Some(5.0), None);
    let workout_id = workout.id.clone();

    storage.save_workout(&workout)?;

    // Verify workout exists
    assert!(storage.get_workout(&workout_id)?.is_some());

    // Delete workout
    storage.delete_workout(&workout_id)?;

    // Verify workout is deleted
    assert!(storage.get_workout(&workout_id)?.is_none());

    Ok(())
}

#[test]
fn test_storage_sync_queue() -> Result<()> {
    let (storage, _temp_dir) = setup_test_storage()?;

    let workout1 = Workout::new("running".to_string(), Some(30), Some(5.0), None);
    let workout2 = Workout::new("cycling".to_string(), Some(60), Some(25.0), None);

    let workout1_id = workout1.id.clone();
    let workout2_id = workout2.id.clone();

    storage.save_workout(&workout1)?;
    storage.save_workout(&workout2)?;

    // Queue both workouts for sync
    storage.queue_for_sync(&workout1_id)?;
    storage.queue_for_sync(&workout2_id)?;

    // Get unsynced workouts
    let unsynced = storage.get_unsynced_workouts()?;
    assert_eq!(unsynced.len(), 2);

    // Remove one from sync queue
    storage.remove_from_sync_queue(&workout1_id)?;

    let unsynced_after = storage.get_unsynced_workouts()?;
    assert_eq!(unsynced_after.len(), 1);
    assert_eq!(unsynced_after[0].id, workout2_id);

    Ok(())
}

#[test]
fn test_workout_partial_data() -> Result<()> {
    // Test workout with only exercise type (no duration or distance)
    let workout = Workout::new("strength".to_string(), None, None, None);

    assert_eq!(workout.exercise_type, "strength");
    assert_eq!(workout.duration_minutes, None);
    assert_eq!(workout.distance_km, None);
    assert_eq!(workout.notes, None);

    Ok(())
}

#[test]
fn test_storage_get_nonexistent_workout() -> Result<()> {
    let (storage, _temp_dir) = setup_test_storage()?;

    let result = storage.get_workout("nonexistent-id")?;
    assert!(result.is_none());

    Ok(())
}

#[test]
fn test_storage_delete_nonexistent_workout() -> Result<()> {
    let (storage, _temp_dir) = setup_test_storage()?;

    // Deleting a nonexistent workout should succeed (no-op)
    let result = storage.delete_workout("nonexistent-id");
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_workout_update_partial_fields() -> Result<()> {
    let mut workout = Workout::new(
        "running".to_string(),
        Some(30),
        Some(5.0),
        Some("Initial notes".to_string()),
    );

    // Update only exercise type and notes, keep duration and distance
    workout.update(
        Some("cycling".to_string()),
        Some(30), // Keep same duration
        Some(5.0), // Keep same distance
        Some("Updated notes".to_string()),
    );

    assert_eq!(workout.exercise_type, "cycling");
    assert_eq!(workout.duration_minutes, Some(30));
    assert_eq!(workout.distance_km, Some(5.0));
    assert_eq!(workout.notes, Some("Updated notes".to_string()));
    assert_eq!(workout.synced, false); // Should be marked as unsynced after update

    Ok(())
}

#[test]
fn test_storage_persistence_across_instances() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_ai_coach.db");
    std::env::set_var("AI_COACH_DB_PATH", db_path.to_str().unwrap());

    let workout_id: String;

    // Create storage instance 1 and save workout
    {
        let storage1 = Storage::init()?;
        let workout = Workout::new("running".to_string(), Some(30), Some(5.0), None);
        workout_id = workout.id.clone();
        storage1.save_workout(&workout)?;
    }

    // Create storage instance 2 and verify workout persisted
    {
        let storage2 = Storage::init()?;
        let retrieved = storage2.get_workout(&workout_id)?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().exercise_type, "running");
    }

    Ok(())
}

#[test]
fn test_workout_timestamps() -> Result<()> {
    let before = Utc::now();
    std::thread::sleep(std::time::Duration::from_millis(10));

    let workout = Workout::new("running".to_string(), Some(30), None, None);

    std::thread::sleep(std::time::Duration::from_millis(10));
    let after = Utc::now();

    // Timestamps should be between before and after
    assert!(workout.created_at > before);
    assert!(workout.created_at < after);
    assert_eq!(workout.created_at, workout.updated_at);
    assert_eq!(workout.created_at, workout.date);

    Ok(())
}

} // End of serial_tests module
