use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use uuid::Uuid;

/// User experience level based on total recommendation completions
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum ExperienceLevel {
    #[serde(rename = "beginner")]
    Beginner,      // 0-10 completed recommendations

    #[serde(rename = "intermediate")]
    Intermediate,  // 11-50 completed recommendations

    #[serde(rename = "advanced")]
    Advanced,      // 51-100 completed recommendations

    #[serde(rename = "expert")]
    Expert,        // 100+ completed recommendations
}

impl ExperienceLevel {
    /// Calculate experience level based on total completions
    pub fn from_completions(count: i32) -> Self {
        match count {
            0..=10 => ExperienceLevel::Beginner,
            11..=50 => ExperienceLevel::Intermediate,
            51..=100 => ExperienceLevel::Advanced,
            _ => ExperienceLevel::Expert,
        }
    }

    /// Get description of this experience level
    pub fn description(&self) -> &'static str {
        match self {
            ExperienceLevel::Beginner => "Just getting started with recovery practices",
            ExperienceLevel::Intermediate => "Building consistent recovery habits",
            ExperienceLevel::Advanced => "Experienced with advanced recovery techniques",
            ExperienceLevel::Expert => "Master of recovery optimization",
        }
    }

    /// Get recommended daily recommendation count
    pub fn recommended_daily_recommendations(&self) -> i32 {
        match self {
            ExperienceLevel::Beginner => 1,       // Start slow
            ExperienceLevel::Intermediate => 2,    // Build consistency
            ExperienceLevel::Advanced => 2,        // Maintain habits
            ExperienceLevel::Expert => 3,          // Optimize as needed
        }
    }
}

impl std::fmt::Display for ExperienceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExperienceLevel::Beginner => write!(f, "beginner"),
            ExperienceLevel::Intermediate => write!(f, "intermediate"),
            ExperienceLevel::Advanced => write!(f, "advanced"),
            ExperienceLevel::Expert => write!(f, "expert"),
        }
    }
}

/// Category-specific mastery level
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum MasteryLevel {
    #[serde(rename = "novice")]
    Novice,        // 0-3 completions in category

    #[serde(rename = "developing")]
    Developing,    // 4-10 completions in category

    #[serde(rename = "proficient")]
    Proficient,    // 11-20 completions in category

    #[serde(rename = "mastered")]
    Mastered,      // 20+ completions in category
}

impl MasteryLevel {
    /// Calculate mastery level based on category completions and effectiveness
    pub fn from_completions_and_effectiveness(completions: i32, avg_rating: f64) -> Self {
        match (completions, avg_rating) {
            (0..=3, _) => MasteryLevel::Novice,
            (4..=10, rating) if rating >= 4.0 => MasteryLevel::Developing,
            (11..=20, rating) if rating >= 4.5 => MasteryLevel::Proficient,
            (21.., rating) if rating >= 4.5 => MasteryLevel::Mastered,
            (4..=10, _) => MasteryLevel::Developing,
            (11..=20, _) => MasteryLevel::Proficient,
            _ => MasteryLevel::Proficient,
        }
    }

    /// Check if user is eligible for advancement
    pub fn is_eligible_for_advancement(&self, completions: i32, avg_rating: f64, completion_rate: f64) -> bool {
        match self {
            MasteryLevel::Novice => completions >= 3 && completion_rate >= 0.70,
            MasteryLevel::Developing => completions >= 10 && avg_rating >= 4.0 && completion_rate >= 0.75,
            MasteryLevel::Proficient => completions >= 20 && avg_rating >= 4.5 && completion_rate >= 0.80,
            MasteryLevel::Mastered => false, // Already at max
        }
    }

    /// Get next mastery level
    pub fn next_level(&self) -> Option<Self> {
        match self {
            MasteryLevel::Novice => Some(MasteryLevel::Developing),
            MasteryLevel::Developing => Some(MasteryLevel::Proficient),
            MasteryLevel::Proficient => Some(MasteryLevel::Mastered),
            MasteryLevel::Mastered => None,
        }
    }

    /// Get celebration message for advancement
    pub fn advancement_message(&self, category: &str) -> String {
        match self {
            MasteryLevel::Developing => format!("Great progress! You've advanced to Developing level in {}. Keep building those habits!", category),
            MasteryLevel::Proficient => format!("Excellent work! You're now Proficient in {}. Advanced techniques are available.", category),
            MasteryLevel::Mastered => format!("Outstanding! You've mastered {} recommendations. You're an expert in this area!", category),
            MasteryLevel::Novice => String::from("Welcome! Complete a few more recommendations to advance."),
        }
    }
}

impl std::fmt::Display for MasteryLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MasteryLevel::Novice => write!(f, "novice"),
            MasteryLevel::Developing => write!(f, "developing"),
            MasteryLevel::Proficient => write!(f, "proficient"),
            MasteryLevel::Mastered => write!(f, "mastered"),
        }
    }
}

/// Category mastery tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryMastery {
    pub category: String,  // recommendation_category as string
    pub level: MasteryLevel,
    pub completions: i32,
    pub effectiveness_avg: f64,
    pub last_advancement: Option<DateTime<Utc>>,
}

impl CategoryMastery {
    /// Check if user should advance to next mastery level
    pub fn should_advance(&self, completion_rate: f64) -> bool {
        self.level.is_eligible_for_advancement(
            self.completions,
            self.effectiveness_avg,
            completion_rate,
        )
    }

    /// Advance to next mastery level
    pub fn advance(&mut self) -> Option<String> {
        if let Some(next_level) = self.level.next_level() {
            self.level = next_level.clone();
            self.last_advancement = Some(Utc::now());
            Some(next_level.advancement_message(&self.category))
        } else {
            None
        }
    }
}

/// Progressive recommendation variant model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecommendationProgression {
    pub id: Uuid,
    pub recommendation_template_id: Uuid,
    pub version: ExperienceLevel,
    pub difficulty_modifier: f64,
    pub time_modifier: f64,
    pub complexity_description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to get user progression data
#[derive(Debug, Deserialize)]
pub struct GetProgressionRequest {
    pub user_id: Option<Uuid>,
}

/// Response containing user progression data
#[derive(Debug, Serialize)]
pub struct UserProgressionResponse {
    pub experience_level: ExperienceLevel,
    pub experience_description: String,
    pub total_completions: i32,
    pub weeks_active: i32,
    pub mastery_by_category: HashMap<String, CategoryMastery>,
    pub next_level_requirements: Option<ProgressionRequirements>,
}

/// Requirements for advancing to next level
#[derive(Debug, Serialize)]
pub struct ProgressionRequirements {
    pub next_level: ExperienceLevel,
    pub completions_needed: i32,
    pub current_completion_rate: f64,
    pub target_completion_rate: f64,
}

/// Request to manually adjust user experience level (admin only)
#[derive(Debug, Deserialize)]
pub struct UpdateExperienceLevelRequest {
    pub experience_level: ExperienceLevel,
    pub reason: Option<String>,
}

/// Advancement notification
#[derive(Debug, Serialize)]
pub struct AdvancementNotification {
    pub advancement_type: String,  // "experience_level" or "category_mastery"
    pub category: Option<String>,
    pub old_level: String,
    pub new_level: String,
    pub message: String,
    pub unlocked_features: Vec<String>,
}
