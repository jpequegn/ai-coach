use crate::models::{
    AdvancementNotification, CategoryMastery, ExperienceLevel, MasteryLevel,
    ProgressionRequirements, RecommendationCategory, RecommendationProgression,
    UserProgressionResponse,
};
use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

pub struct ProgressionService {
    db: SqlitePool,
}

impl ProgressionService {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Calculate user's experience level based on total completions
    pub async fn calculate_experience_level(&self, user_id: Uuid) -> Result<ExperienceLevel> {
        let row = sqlx::query(
            r#"
            SELECT total_recommendations_completed
            FROM user_recovery_profiles
            WHERE user_id = ?1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some(r) => {
                let count: i32 = r.try_get("total_recommendations_completed").unwrap_or(0);
                Ok(ExperienceLevel::from_completions(count))
            }
            None => Ok(ExperienceLevel::Beginner), // Default for new users
        }
    }

    /// Get mastery level for a specific category
    pub async fn get_category_mastery(
        &self,
        user_id: Uuid,
        category: RecommendationCategory,
    ) -> Result<CategoryMastery> {
        // Get completion count and average rating for this category
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as completions,
                COALESCE(AVG(CAST(ur.effectiveness_rating AS REAL)), 0.0) as avg_rating
            FROM user_recommendations ur
            JOIN recommendation_templates rt ON ur.recommendation_template_id = rt.id
            WHERE ur.user_id = ?1
              AND rt.category = ?2
              AND ur.status = 'completed'
            "#,
        )
        .bind(user_id.to_string())
        .bind(category.to_string())
        .fetch_one(&self.db)
        .await?;

        let completions: i32 = row.try_get("completions").unwrap_or(0);
        let avg_rating: f64 = row.try_get("avg_rating").unwrap_or(0.0);

        let mastery_level = MasteryLevel::from_completions_and_effectiveness(completions, avg_rating);

        Ok(CategoryMastery {
            category: category.to_string(),
            level: mastery_level,
            completions,
            effectiveness_avg: avg_rating,
            last_advancement: None, // TODO: Track in database
        })
    }

    /// Get all category masteries for a user
    pub async fn get_all_category_masteries(
        &self,
        user_id: Uuid,
    ) -> Result<HashMap<String, CategoryMastery>> {
        let categories = vec![
            RecommendationCategory::Sleep,
            RecommendationCategory::Nutrition,
            RecommendationCategory::ActiveRecovery,
            RecommendationCategory::StressManagement,
            RecommendationCategory::TrainingModification,
        ];

        let mut masteries = HashMap::new();
        for category in categories {
            let mastery = self.get_category_mastery(user_id, category.clone()).await?;
            masteries.insert(category.to_string(), mastery);
        }

        Ok(masteries)
    }

    /// Select appropriate recommendation version based on user experience
    pub async fn select_appropriate_version(
        &self,
        user_id: Uuid,
        template_id: Uuid,
    ) -> Result<Option<RecommendationProgression>> {
        // Get user's experience level
        let experience_level = self.calculate_experience_level(user_id).await?;

        // Get the progression variant for this template and experience level
        let progression = sqlx::query_as::<_, RecommendationProgression>(
            r#"
            SELECT * FROM recommendation_progression
            WHERE recommendation_template_id = ?1
              AND version = ?2
            LIMIT 1
            "#,
        )
        .bind(template_id.to_string())
        .bind(experience_level.to_string())
        .fetch_optional(&self.db)
        .await?;

        Ok(progression)
    }

    /// Check if user is eligible for experience level advancement
    pub async fn check_advancement_eligibility(&self, user_id: Uuid) -> Result<Option<AdvancementNotification>> {
        let current_level = self.calculate_experience_level(user_id).await?;

        // Get total completions and completion rate
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_shown,
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as total_completed
            FROM user_recommendations
            WHERE user_id = ?1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_one(&self.db)
        .await?;

        let total_shown: i32 = row.try_get("total_shown").unwrap_or(0);
        let total_completed: i32 = row.try_get("total_completed").unwrap_or(0);
        let completion_rate = if total_shown > 0 {
            total_completed as f64 / total_shown as f64
        } else {
            0.0
        };

        // Check if user meets criteria for next level
        let should_advance = match current_level {
            ExperienceLevel::Beginner => total_completed >= 10 && completion_rate >= 0.70,
            ExperienceLevel::Intermediate => total_completed >= 50 && completion_rate >= 0.75,
            ExperienceLevel::Advanced => total_completed >= 100 && completion_rate >= 0.80,
            ExperienceLevel::Expert => false, // Already at max
        };

        if should_advance {
            let next_level = match current_level {
                ExperienceLevel::Beginner => ExperienceLevel::Intermediate,
                ExperienceLevel::Intermediate => ExperienceLevel::Advanced,
                ExperienceLevel::Advanced => ExperienceLevel::Expert,
                ExperienceLevel::Expert => return Ok(None),
            };

            Ok(Some(AdvancementNotification {
                advancement_type: "experience_level".to_string(),
                category: None,
                old_level: current_level.to_string(),
                new_level: next_level.to_string(),
                message: format!(
                    "Congratulations! You've advanced to {} level with {} completed recommendations!",
                    next_level, total_completed
                ),
                unlocked_features: self.get_unlocked_features(&next_level),
            }))
        } else {
            Ok(None)
        }
    }

    /// Advance user to next experience level
    pub async fn advance_user_level(&self, user_id: Uuid) -> Result<Option<AdvancementNotification>> {
        if let Some(notification) = self.check_advancement_eligibility(user_id).await? {
            // Parse the new level from the notification
            let new_level_str = notification.new_level.as_str();

            // Update user's experience level
            sqlx::query(
                r#"
                UPDATE user_recovery_profiles
                SET experience_level = ?2
                WHERE user_id = ?1
                "#,
            )
            .bind(user_id.to_string())
            .bind(new_level_str)
            .execute(&self.db)
            .await?;

            info!("User {} advanced to {} level", user_id, new_level_str);

            Ok(Some(notification))
        } else {
            Ok(None)
        }
    }

    /// Check and advance category mastery
    pub async fn check_category_advancement(
        &self,
        user_id: Uuid,
        category: RecommendationCategory,
    ) -> Result<Option<AdvancementNotification>> {
        let mut mastery = self.get_category_mastery(user_id, category.clone()).await?;

        // Get completion rate for this category
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_shown,
                SUM(CASE WHEN ur.status = 'completed' THEN 1 ELSE 0 END) as total_completed
            FROM user_recommendations ur
            JOIN recommendation_templates rt ON ur.recommendation_template_id = rt.id
            WHERE ur.user_id = ?1 AND rt.category = ?2
            "#,
        )
        .bind(user_id.to_string())
        .bind(category.to_string())
        .fetch_one(&self.db)
        .await?;

        let total_shown: i32 = row.try_get("total_shown").unwrap_or(0);
        let total_completed: i32 = row.try_get("total_completed").unwrap_or(0);
        let completion_rate = if total_shown > 0 {
            total_completed as f64 / total_shown as f64
        } else {
            0.0
        };

        if mastery.should_advance(completion_rate) {
            let old_level = mastery.level.to_string();
            if let Some(message) = mastery.advance() {
                // TODO: Store advancement in database (mastery_by_category JSON field)

                Ok(Some(AdvancementNotification {
                    advancement_type: "category_mastery".to_string(),
                    category: Some(category.to_string()),
                    old_level,
                    new_level: mastery.level.to_string(),
                    message,
                    unlocked_features: vec![
                        format!("Advanced {} recommendations", category),
                    ],
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Get user progression data
    pub async fn get_user_progression(&self, user_id: Uuid) -> Result<UserProgressionResponse> {
        // Get user recovery profile
        let row = sqlx::query(
            r#"
            SELECT experience_level, total_recommendations_completed, weeks_active
            FROM user_recovery_profiles
            WHERE user_id = ?1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.db)
        .await?;

        let (experience_level, total_completions, weeks_active) = match row {
            Some(r) => {
                let level_str: String = r.try_get("experience_level").unwrap_or_else(|_| "beginner".to_string());
                let level = match level_str.as_str() {
                    "intermediate" => ExperienceLevel::Intermediate,
                    "advanced" => ExperienceLevel::Advanced,
                    "expert" => ExperienceLevel::Expert,
                    _ => ExperienceLevel::Beginner,
                };
                let completions: i32 = r.try_get("total_recommendations_completed").unwrap_or(0);
                let weeks: i32 = r.try_get("weeks_active").unwrap_or(0);
                (level, completions, weeks)
            }
            None => (ExperienceLevel::Beginner, 0, 0),
        };

        let mastery_by_category = self.get_all_category_masteries(user_id).await?;

        // Calculate requirements for next level
        let next_level_requirements = self.calculate_next_level_requirements(user_id, &experience_level, total_completions).await?;

        Ok(UserProgressionResponse {
            experience_level: experience_level.clone(),
            experience_description: experience_level.description().to_string(),
            total_completions,
            weeks_active,
            mastery_by_category,
            next_level_requirements,
        })
    }

    /// Calculate requirements for next experience level
    async fn calculate_next_level_requirements(
        &self,
        user_id: Uuid,
        current_level: &ExperienceLevel,
        total_completions: i32,
    ) -> Result<Option<ProgressionRequirements>> {
        let (next_level, required_completions, target_rate) = match current_level {
            ExperienceLevel::Beginner => (ExperienceLevel::Intermediate, 10, 0.70),
            ExperienceLevel::Intermediate => (ExperienceLevel::Advanced, 50, 0.75),
            ExperienceLevel::Advanced => (ExperienceLevel::Expert, 100, 0.80),
            ExperienceLevel::Expert => return Ok(None),
        };

        // Get current completion rate
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_shown,
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as total_completed
            FROM user_recommendations
            WHERE user_id = ?1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_one(&self.db)
        .await?;

        let total_shown: i32 = row.try_get("total_shown").unwrap_or(0);
        let total_completed: i32 = row.try_get("total_completed").unwrap_or(0);
        let current_rate = if total_shown > 0 {
            total_completed as f64 / total_shown as f64
        } else {
            0.0
        };

        Ok(Some(ProgressionRequirements {
            next_level,
            completions_needed: required_completions - total_completions,
            current_completion_rate: current_rate,
            target_completion_rate: target_rate,
        }))
    }

    /// Get unlocked features for a level
    fn get_unlocked_features(&self, level: &ExperienceLevel) -> Vec<String> {
        match level {
            ExperienceLevel::Beginner => vec![
                "Basic recovery recommendations".to_string(),
                "Progress tracking".to_string(),
            ],
            ExperienceLevel::Intermediate => vec![
                "Intermediate difficulty recommendations".to_string(),
                "Category mastery tracking".to_string(),
                "Advanced insights".to_string(),
            ],
            ExperienceLevel::Advanced => vec![
                "Advanced recovery techniques".to_string(),
                "Personalized optimization".to_string(),
                "Expert-level protocols".to_string(),
            ],
            ExperienceLevel::Expert => vec![
                "Expert recommendations".to_string(),
                "Cutting-edge recovery science".to_string(),
                "Custom protocol design".to_string(),
            ],
        }
    }

    /// Update total recommendations completed count
    pub async fn update_completion_count(&self, user_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE user_recovery_profiles
            SET total_recommendations_completed = (
                SELECT COUNT(*)
                FROM user_recommendations
                WHERE user_id = ?1 AND status = 'completed'
            )
            WHERE user_id = ?1
            "#,
        )
        .bind(user_id.to_string())
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Update weeks active count (should be called weekly via background job)
    pub async fn update_weeks_active(&self, user_id: Uuid) -> Result<()> {
        // Calculate weeks since first recommendation
        let row = sqlx::query(
            r#"
            SELECT MIN(shown_at) as first_rec_date
            FROM user_recommendations
            WHERE user_id = ?1
            "#,
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.db)
        .await?;

        if let Some(r) = row {
            if let Ok(first_date_str) = r.try_get::<String, _>("first_rec_date") {
                if let Ok(first_date) = chrono::DateTime::parse_from_rfc3339(&first_date_str) {
                    let first_date_utc = first_date.with_timezone(&Utc);
                    let weeks = (Utc::now() - first_date_utc).num_weeks() as i32;

                    sqlx::query(
                        r#"
                        UPDATE user_recovery_profiles
                        SET weeks_active = ?2
                        WHERE user_id = ?1
                        "#,
                    )
                    .bind(user_id.to_string())
                    .bind(weeks)
                    .execute(&self.db)
                    .await?;
                }
            }
        }

        Ok(())
    }
}
