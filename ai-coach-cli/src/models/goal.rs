use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Goal data model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub goal_type: GoalType,
    pub target_date: DateTime<Utc>,
    pub target_value: Option<f64>, // distance in km or duration in minutes
    pub current_value: f64,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalType {
    Distance,  // Total distance goal (km)
    Duration,  // Total duration goal (minutes)
    Event,     // Specific event preparation
    Frequency, // Number of workouts
}

impl Goal {
    /// Create a new goal
    pub fn new(
        title: String,
        goal_type: GoalType,
        target_date: DateTime<Utc>,
        target_value: Option<f64>,
        notes: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            goal_type,
            target_date,
            target_value,
            current_value: 0.0,
            completed: false,
            completed_at: None,
            notes,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update goal details
    pub fn update(
        &mut self,
        title: Option<String>,
        target_date: Option<DateTime<Utc>>,
        target_value: Option<f64>,
        notes: Option<String>,
    ) {
        if let Some(t) = title {
            self.title = t;
        }
        if let Some(td) = target_date {
            self.target_date = td;
        }
        if let Some(tv) = target_value {
            self.target_value = Some(tv);
        }
        if let Some(n) = notes {
            self.notes = Some(n);
        }
        self.updated_at = Utc::now();
    }

    /// Mark goal as complete
    pub fn mark_complete(&mut self) {
        self.completed = true;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Calculate progress percentage
    pub fn progress_percentage(&self) -> f64 {
        if let Some(target) = self.target_value {
            if target > 0.0 {
                return (self.current_value / target * 100.0).min(100.0);
            }
        }
        0.0
    }

    /// Days remaining until target date
    pub fn days_remaining(&self) -> i64 {
        let now = Utc::now();
        (self.target_date - now).num_days()
    }

    /// Update current progress value
    pub fn update_progress(&mut self, value: f64) {
        self.current_value = value;
        self.updated_at = Utc::now();
    }
}

impl std::fmt::Display for GoalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoalType::Distance => write!(f, "Distance"),
            GoalType::Duration => write!(f, "Duration"),
            GoalType::Event => write!(f, "Event"),
            GoalType::Frequency => write!(f, "Frequency"),
        }
    }
}

impl std::str::FromStr for GoalType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "distance" => Ok(GoalType::Distance),
            "duration" => Ok(GoalType::Duration),
            "event" => Ok(GoalType::Event),
            "frequency" => Ok(GoalType::Frequency),
            _ => Err(anyhow::anyhow!("Invalid goal type: {}", s)),
        }
    }
}
