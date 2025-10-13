use crate::models::recommendation::{RecommendationCategory, UserRecommendation, UserRecommendationStatus};
use crate::models::user_recovery_profile::*;
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct UserRecoveryProfileService {
    db: PgPool,
}

impl UserRecoveryProfileService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Get user recovery profile, creating one if it doesn't exist
    pub async fn get_profile(&self, user_id: Uuid) -> Result<UserRecoveryProfile> {
        // Try to fetch existing profile
        let profile = sqlx::query_as::<_, UserRecoveryProfile>(
            r#"
            SELECT * FROM user_recovery_profiles
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch user recovery profile")?;

        // If profile doesn't exist, create default one
        match profile {
            Some(p) => Ok(p),
            None => {
                info!("Creating default recovery profile for user {}", user_id);
                self.create_default_profile(user_id).await
            }
        }
    }

    /// Create a default recovery profile for a new user
    pub async fn create_default_profile(&self, user_id: Uuid) -> Result<UserRecoveryProfile> {
        let profile = sqlx::query_as::<_, UserRecoveryProfile>(
            r#"
            INSERT INTO user_recovery_profiles (
                user_id,
                preferred_recovery_activities,
                available_equipment,
                dietary_preferences,
                sleep_schedule,
                stress_triggers,
                effective_techniques,
                completion_stats
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(json!([]))
        .bind(json!([]))
        .bind(json!(DietaryPreferences::default()))
        .bind(json!(SleepSchedule::default()))
        .bind(json!([]))
        .bind(json!([]))
        .bind(json!({}))
        .fetch_one(&self.db)
        .await
        .context("Failed to create default recovery profile")?;

        Ok(profile)
    }

    /// Update user recovery profile with new preferences
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        updates: UpdateProfileRequest,
    ) -> Result<UserRecoveryProfile> {
        // Ensure profile exists
        let mut profile = self.get_profile(user_id).await?;

        // Update fields if provided
        if let Some(activities) = updates.preferred_recovery_activities {
            profile.preferred_recovery_activities = json!(activities);
        }

        if let Some(equipment) = updates.available_equipment {
            profile.available_equipment = json!(equipment);
        }

        if let Some(dietary) = updates.dietary_preferences {
            profile.dietary_preferences = json!(dietary);
        }

        if let Some(sleep) = updates.sleep_schedule {
            profile.sleep_schedule = json!(sleep);
        }

        if let Some(triggers) = updates.stress_triggers {
            profile.stress_triggers = json!(triggers);
        }

        // Save updated profile
        let updated = sqlx::query_as::<_, UserRecoveryProfile>(
            r#"
            UPDATE user_recovery_profiles
            SET preferred_recovery_activities = $2,
                available_equipment = $3,
                dietary_preferences = $4,
                sleep_schedule = $5,
                stress_triggers = $6,
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&profile.preferred_recovery_activities)
        .bind(&profile.available_equipment)
        .bind(&profile.dietary_preferences)
        .bind(&profile.sleep_schedule)
        .bind(&profile.stress_triggers)
        .fetch_one(&self.db)
        .await
        .context("Failed to update recovery profile")?;

        info!("Updated recovery profile for user {}", user_id);
        Ok(updated)
    }

    /// Get effective techniques for a user (sorted by effectiveness score)
    pub async fn get_effective_techniques(
        &self,
        user_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<EffectiveTechnique>> {
        let profile = self.get_profile(user_id).await?;

        let mut techniques: Vec<EffectiveTechnique> =
            serde_json::from_value(profile.effective_techniques)
                .unwrap_or_default();

        // Sort by effectiveness score descending
        techniques.sort_by(|a, b| {
            b.effectiveness_score
                .partial_cmp(&a.effectiveness_score)
                .unwrap()
        });

        // Apply limit if specified
        if let Some(limit) = limit {
            techniques.truncate(limit as usize);
        }

        Ok(techniques)
    }

    /// Update profile based on recommendation interaction
    pub async fn update_from_recommendation(
        &self,
        user_id: Uuid,
        recommendation: &UserRecommendation,
        rating: Option<i32>,
    ) -> Result<()> {
        let mut profile = self.get_profile(user_id).await?;

        // Update completion stats
        self.update_completion_stats(&mut profile, recommendation).await?;

        // If completed with good rating, add to effective techniques
        if recommendation.status == UserRecommendationStatus::Completed {
            if let Some(rating_value) = rating {
                if rating_value >= 4 {
                    self.update_effective_techniques(&mut profile, recommendation, rating_value)
                        .await?;
                }
            }
        }

        // Save updated profile
        sqlx::query(
            r#"
            UPDATE user_recovery_profiles
            SET effective_techniques = $2,
                completion_stats = $3,
                updated_at = NOW()
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .bind(&profile.effective_techniques)
        .bind(&profile.completion_stats)
        .execute(&self.db)
        .await
        .context("Failed to update profile from recommendation")?;

        info!(
            "Updated profile for user {} from recommendation interaction",
            user_id
        );
        Ok(())
    }

    /// Update completion statistics per category
    async fn update_completion_stats(
        &self,
        profile: &mut UserRecoveryProfile,
        recommendation: &UserRecommendation,
    ) -> Result<()> {
        // Get template details to know the category
        let template = sqlx::query!(
            r#"
            SELECT category FROM recommendation_templates WHERE id = $1
            "#,
            recommendation.recommendation_template_id
        )
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch recommendation template")?;

        if let Some(template) = template {
            let category = template.category;

            // Parse existing stats
            let mut stats: HashMap<String, serde_json::Value> =
                serde_json::from_value(profile.completion_stats.clone()).unwrap_or_default();

            // Get or create category stats
            let category_stats = stats.entry(category.clone()).or_insert_with(|| {
                json!({
                    "shown": 0,
                    "completed": 0,
                    "skipped": 0,
                    "total_ratings": 0,
                    "sum_ratings": 0
                })
            });

            // Update counters
            if let Some(stats_obj) = category_stats.as_object_mut() {
                // Increment shown count
                if let Some(shown) = stats_obj.get_mut("shown") {
                    if let Some(count) = shown.as_i64() {
                        *shown = json!(count + 1);
                    }
                }

                // Update completion/skip counters based on status
                match recommendation.status {
                    UserRecommendationStatus::Completed => {
                        if let Some(completed) = stats_obj.get_mut("completed") {
                            if let Some(count) = completed.as_i64() {
                                *completed = json!(count + 1);
                            }
                        }
                    }
                    UserRecommendationStatus::Skipped => {
                        if let Some(skipped) = stats_obj.get_mut("skipped") {
                            if let Some(count) = skipped.as_i64() {
                                *skipped = json!(count + 1);
                            }
                        }
                    }
                    _ => {}
                }
            }

            profile.completion_stats = json!(stats);
        }

        Ok(())
    }

    /// Update effective techniques list
    async fn update_effective_techniques(
        &self,
        profile: &mut UserRecoveryProfile,
        recommendation: &UserRecommendation,
        rating: i32,
    ) -> Result<()> {
        // Get template details
        let template = sqlx::query!(
            r#"
            SELECT category, title FROM recommendation_templates WHERE id = $1
            "#,
            recommendation.recommendation_template_id
        )
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch recommendation template")?;

        if let Some(template) = template {
            // Parse existing effective techniques
            let mut techniques: Vec<EffectiveTechnique> =
                serde_json::from_value(profile.effective_techniques.clone()).unwrap_or_default();

            // Find existing technique or create new one
            if let Some(existing) = techniques
                .iter_mut()
                .find(|t| t.template_id == recommendation.recommendation_template_id)
            {
                // Update existing technique
                existing.completion_count += 1;
                existing.total_shown += 1;
                existing.completion_rate =
                    existing.completion_count as f64 / existing.total_shown as f64;

                // Update rating
                let total_rating_sum = existing.average_rating * existing.total_ratings as f64;
                existing.total_ratings += 1;
                existing.average_rating =
                    (total_rating_sum + rating as f64) / existing.total_ratings as f64;

                existing.last_completed = Some(Utc::now());

                // Recalculate effectiveness score
                existing.effectiveness_score = calculate_profile_effectiveness_score(
                    existing.completion_rate,
                    existing.average_rating,
                    existing.total_shown,
                );
            } else {
                // Add new technique
                let new_technique = EffectiveTechnique {
                    template_id: recommendation.recommendation_template_id,
                    category: template.category,
                    title: template.title,
                    completion_count: 1,
                    total_shown: 1,
                    completion_rate: 1.0,
                    average_rating: rating as f64,
                    total_ratings: 1,
                    last_completed: Some(Utc::now()),
                    effectiveness_score: calculate_profile_effectiveness_score(1.0, rating as f64, 1),
                };
                techniques.push(new_technique);
            }

            // Limit to top 50 techniques to prevent unbounded growth
            if techniques.len() > 50 {
                techniques.sort_by(|a, b| {
                    b.effectiveness_score
                        .partial_cmp(&a.effectiveness_score)
                        .unwrap()
                });
                techniques.truncate(50);
            }

            profile.effective_techniques = json!(techniques);
        }

        Ok(())
    }

    /// Get profile insights and patterns
    pub async fn get_insights(&self, user_id: Uuid) -> Result<ProfileInsightsResponse> {
        let profile = self.get_profile(user_id).await?;

        // Get all user recommendations for pattern analysis
        let recommendations = sqlx::query_as::<_, UserRecommendation>(
            r#"
            SELECT * FROM user_recommendations
            WHERE user_id = $1
            ORDER BY shown_at DESC
            LIMIT 100
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch user recommendations")?;

        // Generate insights
        let insights = self.generate_insights(&profile, &recommendations).await?;

        // Identify patterns
        let patterns = self.identify_patterns(&profile, &recommendations).await?;

        // Create summary
        let summary = self.create_recommendations_summary(&recommendations).await?;

        Ok(ProfileInsightsResponse {
            insights,
            patterns,
            recommendations_summary: summary,
        })
    }

    /// Generate insights from profile data
    async fn generate_insights(
        &self,
        profile: &UserRecoveryProfile,
        recommendations: &[UserRecommendation],
    ) -> Result<Vec<ProfileRecoveryInsight>> {
        let mut insights = Vec::new();

        // Parse effective techniques
        let techniques: Vec<EffectiveTechnique> =
            serde_json::from_value(profile.effective_techniques.clone()).unwrap_or_default();

        // Insight: Most effective category
        if let Some(top_technique) = techniques.first() {
            if top_technique.effectiveness_score > 0.7 {
                insights.push(ProfileRecoveryInsight {
                    insight_type: "effective_category".to_string(),
                    title: format!("{} works well for you", top_technique.category),
                    description: format!(
                        "You have a {:.0}% completion rate with {} recommendations and rate them {:.1}/5 on average.",
                        top_technique.completion_rate * 100.0,
                        top_technique.category,
                        top_technique.average_rating
                    ),
                    confidence: top_technique.effectiveness_score,
                    actionable: true,
                });
            }
        }

        // Insight: Completion rate
        let completed_count = recommendations
            .iter()
            .filter(|r| r.status == UserRecommendationStatus::Completed)
            .count();
        let total_count = recommendations.len();

        if total_count > 5 {
            let completion_rate = completed_count as f64 / total_count as f64;

            if completion_rate > 0.7 {
                insights.push(ProfileRecoveryInsight {
                    insight_type: "high_engagement".to_string(),
                    title: "High engagement with recommendations".to_string(),
                    description: format!(
                        "You've completed {}% of recommended recovery activities. Keep up the great work!",
                        (completion_rate * 100.0) as i32
                    ),
                    confidence: 0.9,
                    actionable: false,
                });
            } else if completion_rate < 0.3 {
                insights.push(ProfileRecoveryInsight {
                    insight_type: "low_engagement".to_string(),
                    title: "Consider trying more recovery activities".to_string(),
                    description: format!(
                        "You've completed only {}% of recommendations. Try starting with easier, shorter activities.",
                        (completion_rate * 100.0) as i32
                    ),
                    confidence: 0.8,
                    actionable: true,
                });
            }
        }

        // Insight: Preferred equipment
        let equipment: Vec<String> =
            serde_json::from_value(profile.available_equipment.clone()).unwrap_or_default();

        if equipment.is_empty() && total_count > 3 {
            insights.push(ProfileRecoveryInsight {
                insight_type: "equipment_suggestion".to_string(),
                title: "Add equipment to your profile".to_string(),
                description: "Specifying available equipment helps us recommend more suitable recovery activities.".to_string(),
                confidence: 0.7,
                actionable: true,
            });
        }

        Ok(insights)
    }

    /// Identify patterns in user behavior
    async fn identify_patterns(
        &self,
        _profile: &UserRecoveryProfile,
        recommendations: &[UserRecommendation],
    ) -> Result<Vec<ProfileRecoveryPattern>> {
        let mut patterns = Vec::new();

        // Pattern: Consistent completion
        let completed_count = recommendations
            .iter()
            .filter(|r| r.status == UserRecommendationStatus::Completed)
            .count();

        if completed_count >= 5 {
            let recent_completed = recommendations
                .iter()
                .take(10)
                .filter(|r| r.status == UserRecommendationStatus::Completed)
                .count();

            if recent_completed >= 7 {
                patterns.push(ProfileRecoveryPattern {
                    pattern_type: "consistent_engagement".to_string(),
                    description: "You consistently complete recovery recommendations".to_string(),
                    frequency: "High".to_string(),
                    suggestions: vec![
                        "Continue your excellent recovery habits".to_string(),
                        "Consider trying more challenging recovery activities".to_string(),
                    ],
                });
            }
        }

        // Pattern: Time-of-day preferences (would need time data)
        // This is a placeholder for future enhancement

        Ok(patterns)
    }

    /// Create summary of recommendation interactions
    async fn create_recommendations_summary(
        &self,
        recommendations: &[UserRecommendation],
    ) -> Result<RecommendationsSummary> {
        let total_shown = recommendations.len() as i32;
        let total_completed = recommendations
            .iter()
            .filter(|r| r.status == UserRecommendationStatus::Completed)
            .count() as i32;
        let total_skipped = recommendations
            .iter()
            .filter(|r| r.status == UserRecommendationStatus::Skipped)
            .count() as i32;

        let overall_completion_rate = if total_shown > 0 {
            total_completed as f64 / total_shown as f64
        } else {
            0.0
        };

        // Calculate average rating
        let ratings: Vec<i32> = recommendations
            .iter()
            .filter_map(|r| r.effectiveness_rating)
            .collect();

        let average_rating = if !ratings.is_empty() {
            Some(ratings.iter().sum::<i32>() as f64 / ratings.len() as f64)
        } else {
            None
        };

        // Most effective category would require joining with templates
        // Placeholder for now
        let most_effective_category = None;

        Ok(RecommendationsSummary {
            total_shown,
            total_completed,
            total_skipped,
            overall_completion_rate,
            most_effective_category,
            preferred_time_of_day: None,
            average_rating,
        })
    }
}
