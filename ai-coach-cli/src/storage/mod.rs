// Local storage module using sled embedded database
// Avoids rusqlite/sqlx libsqlite3-sys conflict

use anyhow::{Context, Result};
use sled::Db;
use std::path::PathBuf;

use crate::models::Workout;

const WORKOUTS_TREE: &str = "workouts";
const SYNC_QUEUE_TREE: &str = "sync_queue";

/// Storage manager for local embedded database
pub struct Storage {
    db: Db,
}

impl Storage {
    /// Get database directory path (~/.ai-coach/)
    pub fn db_path() -> Result<PathBuf> {
        let config_dir = crate::config::Config::config_dir()?;
        Ok(config_dir)
    }

    /// Initialize storage with sled database
    pub fn init() -> Result<Self> {
        let db_path = Self::db_path()?;

        tracing::info!("Initializing sled database at {:?}", db_path);

        let db = sled::open(db_path)
            .context("Failed to open sled database")?;

        Ok(Self { db })
    }

    /// Check if database is initialized
    pub fn is_initialized() -> Result<bool> {
        let db_path = Self::db_path()?;
        Ok(db_path.exists())
    }

    /// Save a workout
    pub fn save_workout(&self, workout: &Workout) -> Result<()> {
        let tree = self.db.open_tree(WORKOUTS_TREE)
            .context("Failed to open workouts tree")?;

        let key = workout.id.as_bytes();
        let value = bincode::serialize(workout)
            .context("Failed to serialize workout")?;

        tree.insert(key, value)
            .context("Failed to insert workout")?;

        self.db.flush()
            .context("Failed to flush database")?;

        tracing::debug!("Saved workout {}", workout.id);
        Ok(())
    }

    /// Get a workout by ID
    pub fn get_workout(&self, id: &str) -> Result<Option<Workout>> {
        let tree = self.db.open_tree(WORKOUTS_TREE)
            .context("Failed to open workouts tree")?;

        let key = id.as_bytes();

        if let Some(value) = tree.get(key)
            .context("Failed to get workout")? {
            let workout: Workout = bincode::deserialize(&value)
                .context("Failed to deserialize workout")?;
            Ok(Some(workout))
        } else {
            Ok(None)
        }
    }

    /// List all workouts (unsorted)
    pub fn list_workouts(&self) -> Result<Vec<Workout>> {
        let tree = self.db.open_tree(WORKOUTS_TREE)
            .context("Failed to open workouts tree")?;

        let mut workouts = Vec::new();

        for item in tree.iter() {
            let (_key, value) = item.context("Failed to iterate workouts")?;
            let workout: Workout = bincode::deserialize(&value)
                .context("Failed to deserialize workout")?;
            workouts.push(workout);
        }

        // Sort by date descending (most recent first)
        workouts.sort_by(|a, b| b.date.cmp(&a.date));

        Ok(workouts)
    }

    /// Delete a workout
    pub fn delete_workout(&self, id: &str) -> Result<bool> {
        let tree = self.db.open_tree(WORKOUTS_TREE)
            .context("Failed to open workouts tree")?;

        let key = id.as_bytes();
        let deleted = tree.remove(key)
            .context("Failed to delete workout")?
            .is_some();

        if deleted {
            self.db.flush()
                .context("Failed to flush database")?;
            tracing::debug!("Deleted workout {}", id);
        }

        Ok(deleted)
    }

    /// Get workouts that need to be synced
    pub fn get_unsynced_workouts(&self) -> Result<Vec<Workout>> {
        let workouts = self.list_workouts()?;
        Ok(workouts.into_iter().filter(|w| !w.synced).collect())
    }

    /// Add workout to sync queue
    pub fn queue_for_sync(&self, workout_id: &str) -> Result<()> {
        let tree = self.db.open_tree(SYNC_QUEUE_TREE)
            .context("Failed to open sync queue tree")?;

        let key = workout_id.as_bytes();
        let value = chrono::Utc::now().to_rfc3339();

        tree.insert(key, value.as_bytes())
            .context("Failed to insert to sync queue")?;

        self.db.flush()
            .context("Failed to flush database")?;

        tracing::debug!("Queued workout {} for sync", workout_id);
        Ok(())
    }

    /// Remove workout from sync queue
    pub fn remove_from_sync_queue(&self, workout_id: &str) -> Result<()> {
        let tree = self.db.open_tree(SYNC_QUEUE_TREE)
            .context("Failed to open sync queue tree")?;

        let key = workout_id.as_bytes();
        tree.remove(key)
            .context("Failed to remove from sync queue")?;

        self.db.flush()
            .context("Failed to flush database")?;

        tracing::debug!("Removed workout {} from sync queue", workout_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_storage() -> Result<Storage> {
        let dir = tempdir()?;
        let db = sled::open(dir.path())?;
        Ok(Storage { db })
    }

    #[test]
    fn test_storage_init() {
        let storage = create_test_storage();
        assert!(storage.is_ok());
    }

    #[test]
    fn test_save_and_get_workout() -> Result<()> {
        let storage = create_test_storage()?;

        let workout = Workout::new(
            "running".to_string(),
            Some(45),
            Some(8.5),
            Some("Morning run".to_string()),
        );

        storage.save_workout(&workout)?;

        let retrieved = storage.get_workout(&workout.id)?;
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, workout.id);
        assert_eq!(retrieved.exercise_type, "running");
        assert_eq!(retrieved.duration_minutes, Some(45));

        Ok(())
    }

    #[test]
    fn test_list_workouts() -> Result<()> {
        let storage = create_test_storage()?;

        let workout1 = Workout::new("running".to_string(), Some(30), None, None);
        let workout2 = Workout::new("cycling".to_string(), Some(60), None, None);

        storage.save_workout(&workout1)?;
        storage.save_workout(&workout2)?;

        let workouts = storage.list_workouts()?;
        assert_eq!(workouts.len(), 2);

        Ok(())
    }

    #[test]
    fn test_delete_workout() -> Result<()> {
        let storage = create_test_storage()?;

        let workout = Workout::new("running".to_string(), Some(45), None, None);
        storage.save_workout(&workout)?;

        let deleted = storage.delete_workout(&workout.id)?;
        assert!(deleted);

        let retrieved = storage.get_workout(&workout.id)?;
        assert!(retrieved.is_none());

        Ok(())
    }
}
