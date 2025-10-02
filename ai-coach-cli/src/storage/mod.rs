// Local storage module using sled embedded database
// Avoids rusqlite/sqlx libsqlite3-sys conflict

use anyhow::{Context, Result};
use sled::Db;
use std::path::PathBuf;

use crate::models::{Goal, Workout};

const WORKOUTS_TREE: &str = "workouts";
const GOALS_TREE: &str = "goals";
const SYNC_QUEUE_TREE: &str = "sync_queue";

/// Storage manager for local embedded database
pub struct Storage {
    db: Db,
}

impl Storage {
    /// Get database directory path (~/.ai-coach/)
    pub fn db_path() -> Result<PathBuf> {
        // Check for test environment variable first
        if let Ok(test_path) = std::env::var("AI_COACH_DB_PATH") {
            return Ok(PathBuf::from(test_path));
        }

        let config_dir = crate::config::Config::config_dir()?;
        Ok(config_dir)
    }

    /// Initialize storage with sled database
    pub fn init() -> Result<Self> {
        let db_path = Self::db_path()?;

        tracing::info!("Initializing sled database at {:?}", db_path);

        let db = sled::open(db_path).context("Failed to open sled database")?;

        Ok(Self { db })
    }

    /// Initialize storage with custom path (for testing)
    #[cfg(test)]
    pub fn init_with_path(path: PathBuf) -> Result<Self> {
        tracing::info!("Initializing sled database at {:?}", path);

        let db = sled::open(path).context("Failed to open sled database")?;

        Ok(Self { db })
    }

    /// Check if database is initialized
    pub fn is_initialized() -> Result<bool> {
        let db_path = Self::db_path()?;
        Ok(db_path.exists())
    }

    /// Save a workout
    pub fn save_workout(&self, workout: &Workout) -> Result<()> {
        let tree = self
            .db
            .open_tree(WORKOUTS_TREE)
            .context("Failed to open workouts tree")?;

        let key = workout.id.as_bytes();
        let value = bincode::serialize(workout).context("Failed to serialize workout")?;

        tree.insert(key, value)
            .context("Failed to insert workout")?;

        self.db.flush().context("Failed to flush database")?;

        tracing::debug!("Saved workout {}", workout.id);
        Ok(())
    }

    /// Get a workout by ID
    pub fn get_workout(&self, id: &str) -> Result<Option<Workout>> {
        let tree = self
            .db
            .open_tree(WORKOUTS_TREE)
            .context("Failed to open workouts tree")?;

        let key = id.as_bytes();

        if let Some(value) = tree.get(key).context("Failed to get workout")? {
            let workout: Workout =
                bincode::deserialize(&value).context("Failed to deserialize workout")?;
            Ok(Some(workout))
        } else {
            Ok(None)
        }
    }

    /// List all workouts (unsorted)
    pub fn list_workouts(&self) -> Result<Vec<Workout>> {
        let tree = self
            .db
            .open_tree(WORKOUTS_TREE)
            .context("Failed to open workouts tree")?;

        let mut workouts = Vec::new();

        for item in tree.iter() {
            let (_key, value) = item.context("Failed to iterate workouts")?;
            let workout: Workout =
                bincode::deserialize(&value).context("Failed to deserialize workout")?;
            workouts.push(workout);
        }

        // Sort by date descending (most recent first)
        workouts.sort_by(|a, b| b.date.cmp(&a.date));

        Ok(workouts)
    }

    /// Delete a workout
    pub fn delete_workout(&self, id: &str) -> Result<bool> {
        let tree = self
            .db
            .open_tree(WORKOUTS_TREE)
            .context("Failed to open workouts tree")?;

        let key = id.as_bytes();
        let deleted = tree
            .remove(key)
            .context("Failed to delete workout")?
            .is_some();

        if deleted {
            self.db.flush().context("Failed to flush database")?;
            tracing::debug!("Deleted workout {}", id);
        }

        Ok(deleted)
    }

    /// Get workouts that are in the sync queue
    pub fn get_unsynced_workouts(&self) -> Result<Vec<Workout>> {
        let sync_queue = self
            .db
            .open_tree(SYNC_QUEUE_TREE)
            .context("Failed to open sync queue tree")?;
        let workouts_tree = self
            .db
            .open_tree(WORKOUTS_TREE)
            .context("Failed to open workouts tree")?;

        let mut workouts = Vec::new();
        for item in sync_queue.iter() {
            let (key, _value) = item.context("Failed to read sync queue item")?;

            // Get the workout from workouts tree
            if let Some(workout_data) = workouts_tree.get(&key).context("Failed to get workout")? {
                let workout: Workout =
                    bincode::deserialize(&workout_data).context("Failed to deserialize workout")?;
                workouts.push(workout);
            }
        }

        Ok(workouts)
    }

    /// Add workout to sync queue
    pub fn queue_for_sync(&self, workout_id: &str) -> Result<()> {
        let tree = self
            .db
            .open_tree(SYNC_QUEUE_TREE)
            .context("Failed to open sync queue tree")?;

        let key = workout_id.as_bytes();
        let value = chrono::Utc::now().to_rfc3339();

        tree.insert(key, value.as_bytes())
            .context("Failed to insert to sync queue")?;

        self.db.flush().context("Failed to flush database")?;

        tracing::debug!("Queued workout {} for sync", workout_id);
        Ok(())
    }

    /// Remove workout from sync queue
    pub fn remove_from_sync_queue(&self, workout_id: &str) -> Result<()> {
        let tree = self
            .db
            .open_tree(SYNC_QUEUE_TREE)
            .context("Failed to open sync queue tree")?;

        let key = workout_id.as_bytes();
        tree.remove(key)
            .context("Failed to remove from sync queue")?;

        self.db.flush().context("Failed to flush database")?;

        tracing::debug!("Removed workout {} from sync queue", workout_id);
        Ok(())
    }

    // Goal operations

    /// Save a goal
    pub fn save_goal(&self, goal: &Goal) -> Result<()> {
        let tree = self
            .db
            .open_tree(GOALS_TREE)
            .context("Failed to open goals tree")?;

        let key = goal.id.as_bytes();
        let value = bincode::serialize(goal).context("Failed to serialize goal")?;

        tree.insert(key, value)
            .context("Failed to insert goal")?;

        self.db.flush().context("Failed to flush database")?;

        tracing::debug!("Saved goal {}", goal.id);
        Ok(())
    }

    /// Get a goal by ID
    pub fn get_goal(&self, id: &str) -> Result<Option<Goal>> {
        let tree = self
            .db
            .open_tree(GOALS_TREE)
            .context("Failed to open goals tree")?;

        let key = id.as_bytes();

        if let Some(value) = tree.get(key).context("Failed to get goal")? {
            let goal: Goal =
                bincode::deserialize(&value).context("Failed to deserialize goal")?;
            Ok(Some(goal))
        } else {
            Ok(None)
        }
    }

    /// List all goals
    pub fn list_goals(&self, include_completed: bool) -> Result<Vec<Goal>> {
        let tree = self
            .db
            .open_tree(GOALS_TREE)
            .context("Failed to open goals tree")?;

        let mut goals = Vec::new();

        for item in tree.iter() {
            let (_key, value) = item.context("Failed to iterate goals")?;
            let goal: Goal =
                bincode::deserialize(&value).context("Failed to deserialize goal")?;

            if include_completed || !goal.completed {
                goals.push(goal);
            }
        }

        // Sort by target date (closest first)
        goals.sort_by(|a, b| a.target_date.cmp(&b.target_date));

        Ok(goals)
    }

    /// Update a goal
    pub fn update_goal(&self, goal: &Goal) -> Result<()> {
        self.save_goal(goal)
    }

    /// Delete a goal
    pub fn delete_goal(&self, id: &str) -> Result<bool> {
        let tree = self
            .db
            .open_tree(GOALS_TREE)
            .context("Failed to open goals tree")?;

        let key = id.as_bytes();
        let deleted = tree
            .remove(key)
            .context("Failed to delete goal")?
            .is_some();

        if deleted {
            self.db.flush().context("Failed to flush database")?;
            tracing::debug!("Deleted goal {}", id);
        }

        Ok(deleted)
    }

    /// Mark a goal as complete
    pub fn complete_goal(&self, id: &str) -> Result<()> {
        if let Some(mut goal) = self.get_goal(id)? {
            goal.mark_complete();
            self.save_goal(&goal)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Goal {} not found", id))
        }
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
