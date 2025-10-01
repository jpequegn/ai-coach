use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Workout entry stored locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workout {
    pub id: String,
    pub date: DateTime<Utc>,
    pub exercise_type: String,
    pub duration_minutes: Option<u32>,
    pub distance_km: Option<f64>,
    pub notes: Option<String>,
    pub synced: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Workout {
    /// Create a new workout with generated ID and timestamps
    pub fn new(
        exercise_type: String,
        duration_minutes: Option<u32>,
        distance_km: Option<f64>,
        notes: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            date: now,
            exercise_type,
            duration_minutes,
            distance_km,
            notes,
            synced: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update workout fields
    pub fn update(
        &mut self,
        exercise_type: Option<String>,
        duration_minutes: Option<u32>,
        distance_km: Option<f64>,
        notes: Option<String>,
    ) {
        if let Some(et) = exercise_type {
            self.exercise_type = et;
        }
        if duration_minutes.is_some() {
            self.duration_minutes = duration_minutes;
        }
        if distance_km.is_some() {
            self.distance_km = distance_km;
        }
        if notes.is_some() {
            self.notes = notes;
        }
        self.updated_at = Utc::now();
        self.synced = false; // Mark as needing sync after update
    }

    /// Mark workout as synced
    pub fn mark_synced(&mut self) {
        self.synced = true;
        self.updated_at = Utc::now();
    }
}

/// Filter criteria for listing workouts
#[derive(Debug, Default)]
pub struct WorkoutFilter {
    pub exercise_type: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub synced: Option<bool>,
}

impl WorkoutFilter {
    pub fn matches(&self, workout: &Workout) -> bool {
        if let Some(ref et) = self.exercise_type {
            if &workout.exercise_type != et {
                return false;
            }
        }

        if let Some(from) = self.from_date {
            if workout.date < from {
                return false;
            }
        }

        if let Some(to) = self.to_date {
            if workout.date > to {
                return false;
            }
        }

        if let Some(synced) = self.synced {
            if workout.synced != synced {
                return false;
            }
        }

        true
    }
}
