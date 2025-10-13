use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::{PgPool, Row};
use tracing::{info, warn};
use uuid::Uuid;

use crate::models::{
    calculate_effectiveness_score, determine_effectiveness_trend, update_effectiveness_with_ema,
    CategoryAnalytics, EffectivenessAnalytics, EffectivenessFilter, ProcessOutcomesResult,
    RecommendationOutcome, SystemAnalytics, TemplatePerformance, UserRecommendation,
    UserRecommendationStatus,
};

pub struct RecommendationEffectivenessService {
    db: PgPool,
}

impl RecommendationEffectivenessService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Track outcome for a completed recommendation
    ///
    /// Creates an outcome record when a recommendation is completed.
    /// Calculates completion time and stores baseline recovery score.
    pub async fn track_outcome(
        &self,
        user_recommendation: &UserRecommendation,
        baseline_recovery_score_id: Option<Uuid>,
        baseline_recovery_score: f64,
    ) -> Result<RecommendationOutcome> {
        let completed_at = user_recommendation
            .completed_at
            .ok_or_else(|| anyhow::anyhow!("Recommendation not completed"))?;

        let completion_time_hours =
            (completed_at - user_recommendation.shown_at).num_minutes() as f64 / 60.0;

        let outcome = sqlx::query_as!(
            RecommendationOutcome,
            r#"
            INSERT INTO recommendation_outcomes (
                user_recommendation_id,
                user_id,
                recommendation_template_id,
                shown_at,
                completed_at,
                completion_time_hours,
                baseline_recovery_score_id,
                baseline_recovery_score,
                user_rating,
                user_feedback,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING
                id, user_recommendation_id, user_id, recommendation_template_id,
                shown_at, completed_at, completion_time_hours,
                baseline_recovery_score_id, baseline_recovery_score,
                next_day_recovery_score_id, next_day_recovery_score,
                recovery_improvement, user_rating, user_feedback,
                effectiveness_score, metadata, created_at, updated_at
            "#,
            user_recommendation.id,
            user_recommendation.user_id,
            user_recommendation.recommendation_template_id,
            user_recommendation.shown_at,
            completed_at,
            completion_time_hours,
            baseline_recovery_score_id,
            baseline_recovery_score,
            user_recommendation.effectiveness_rating,
            user_recommendation.user_feedback,
            json!({})
        )
        .fetch_one(&self.db)
        .await?;

        info!(
            "Created outcome record for recommendation {} (template {})",
            user_recommendation.id, user_recommendation.recommendation_template_id
        );

        Ok(outcome)
    }

    /// Update outcome with next-day recovery score
    ///
    /// Called 24 hours after completion to update with next day's recovery score.
    /// Calculates effectiveness and updates template effectiveness score.
    pub async fn update_outcome_with_next_day_score(
        &self,
        outcome_id: Uuid,
        next_day_recovery_score_id: Option<Uuid>,
        next_day_recovery_score: f64,
    ) -> Result<RecommendationOutcome> {
        // Get existing outcome
        let outcome = sqlx::query_as!(
            RecommendationOutcome,
            r#"
            SELECT
                id, user_recommendation_id, user_id, recommendation_template_id,
                shown_at, completed_at, completion_time_hours,
                baseline_recovery_score_id, baseline_recovery_score,
                next_day_recovery_score_id, next_day_recovery_score,
                recovery_improvement, user_rating, user_feedback,
                effectiveness_score, metadata, created_at, updated_at
            FROM recommendation_outcomes
            WHERE id = $1
            "#,
            outcome_id
        )
        .fetch_one(&self.db)
        .await?;

        // Calculate improvement and effectiveness
        let recovery_improvement = next_day_recovery_score - outcome.baseline_recovery_score;
        let effectiveness_score = calculate_effectiveness_score(
            outcome.baseline_recovery_score,
            next_day_recovery_score,
            outcome.user_rating,
            outcome.completion_time_hours,
        );

        // Update outcome record
        let updated_outcome = sqlx::query_as!(
            RecommendationOutcome,
            r#"
            UPDATE recommendation_outcomes
            SET
                next_day_recovery_score_id = $2,
                next_day_recovery_score = $3,
                recovery_improvement = $4,
                effectiveness_score = $5,
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, user_recommendation_id, user_id, recommendation_template_id,
                shown_at, completed_at, completion_time_hours,
                baseline_recovery_score_id, baseline_recovery_score,
                next_day_recovery_score_id, next_day_recovery_score,
                recovery_improvement, user_rating, user_feedback,
                effectiveness_score, metadata, created_at, updated_at
            "#,
            outcome_id,
            next_day_recovery_score_id,
            next_day_recovery_score,
            recovery_improvement,
            effectiveness_score
        )
        .fetch_one(&self.db)
        .await?;

        // Update template effectiveness
        self.update_template_effectiveness(outcome.recommendation_template_id, effectiveness_score)
            .await?;

        info!(
            "Updated outcome {} with effectiveness score {:.2}",
            outcome_id, effectiveness_score
        );

        Ok(updated_outcome)
    }

    /// Update template effectiveness using exponential moving average
    ///
    /// Uses 30% weight for new outcome, 70% for existing average.
    pub async fn update_template_effectiveness(
        &self,
        template_id: Uuid,
        new_effectiveness: f64,
    ) -> Result<()> {
        // Get current effectiveness
        let current = sqlx::query!(
            "SELECT effectiveness_score FROM recommendation_templates WHERE id = $1",
            template_id
        )
        .fetch_one(&self.db)
        .await?;

        // Calculate new effectiveness with EMA
        let updated_effectiveness =
            update_effectiveness_with_ema(current.effectiveness_score, new_effectiveness);

        // Update template
        sqlx::query!(
            r#"
            UPDATE recommendation_templates
            SET effectiveness_score = $2, updated_at = NOW()
            WHERE id = $1
            "#,
            template_id,
            updated_effectiveness
        )
        .execute(&self.db)
        .await?;

        info!(
            "Updated template {} effectiveness: {:.2} -> {:.2}",
            template_id, current.effectiveness_score, updated_effectiveness
        );

        Ok(())
    }

    /// Process outcomes for recommendations completed 24 hours ago
    ///
    /// Background job that runs daily to:
    /// 1. Find outcomes needing next-day scores
    /// 2. Fetch latest recovery scores for those users
    /// 3. Calculate effectiveness
    /// 4. Update template scores
    /// 5. Flag underperforming templates
    pub async fn process_daily_outcomes(&self) -> Result<ProcessOutcomesResult> {
        let twenty_four_hours_ago = Utc::now() - Duration::hours(24);
        let twenty_five_hours_ago = Utc::now() - Duration::hours(25);

        // Find outcomes completed 24-25 hours ago without next-day scores
        let pending_outcomes = sqlx::query_as!(
            RecommendationOutcome,
            r#"
            SELECT
                id, user_recommendation_id, user_id, recommendation_template_id,
                shown_at, completed_at, completion_time_hours,
                baseline_recovery_score_id, baseline_recovery_score,
                next_day_recovery_score_id, next_day_recovery_score,
                recovery_improvement, user_rating, user_feedback,
                effectiveness_score, metadata, created_at, updated_at
            FROM recommendation_outcomes
            WHERE completed_at BETWEEN $1 AND $2
              AND next_day_recovery_score IS NULL
            "#,
            twenty_five_hours_ago,
            twenty_four_hours_ago
        )
        .fetch_all(&self.db)
        .await?;

        let mut processed_count = 0u64;
        let mut updated_templates = Vec::new();
        let mut errors = Vec::new();

        for outcome in pending_outcomes {
            // Get latest recovery score for user
            match self.get_latest_recovery_score(outcome.user_id).await {
                Ok(Some((score_id, score_value))) => {
                    match self
                        .update_outcome_with_next_day_score(
                            outcome.id,
                            Some(score_id),
                            score_value,
                        )
                        .await
                    {
                        Ok(_) => {
                            processed_count += 1;
                            if !updated_templates.contains(&outcome.recommendation_template_id) {
                                updated_templates.push(outcome.recommendation_template_id);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to update outcome {}: {}", outcome.id, e);
                            errors.push(format!("Outcome {}: {}", outcome.id, e));
                        }
                    }
                }
                Ok(None) => {
                    warn!("No recovery score found for user {}", outcome.user_id);
                    errors.push(format!(
                        "No recovery score for user {}",
                        outcome.user_id
                    ));
                }
                Err(e) => {
                    warn!(
                        "Failed to get recovery score for user {}: {}",
                        outcome.user_id, e
                    );
                    errors.push(format!("User {}: {}", outcome.user_id, e));
                }
            }
        }

        // Flag underperforming templates
        let flagged_for_review = self.flag_underperforming_templates().await?;

        info!(
            "Processed {} outcomes, updated {} templates, flagged {} for review",
            processed_count,
            updated_templates.len(),
            flagged_for_review.len()
        );

        Ok(ProcessOutcomesResult {
            processed_count,
            updated_templates,
            flagged_for_review,
            errors,
        })
    }

    /// Get latest recovery score for a user
    async fn get_latest_recovery_score(&self, user_id: Uuid) -> Result<Option<(Uuid, f64)>> {
        let result = sqlx::query!(
            r#"
            SELECT id, overall_readiness
            FROM recovery_scores
            WHERE user_id = $1
            ORDER BY date DESC
            LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(result.map(|r| (r.id, r.overall_readiness)))
    }

    /// Get effectiveness analytics for a template or category
    pub async fn get_effectiveness_analytics(
        &self,
        filter: EffectivenessFilter,
    ) -> Result<Vec<EffectivenessAnalytics>> {
        let mut query = r#"
            WITH outcome_stats AS (
                SELECT
                    ro.recommendation_template_id,
                    rt.category::text as category,
                    COUNT(*) as completions,
                    AVG(ro.user_rating) as avg_rating,
                    AVG(ro.recovery_improvement) as avg_improvement,
                    AVG(ro.effectiveness_score) as avg_effectiveness
                FROM recommendation_outcomes ro
                JOIN recommendation_templates rt ON rt.id = ro.recommendation_template_id
                WHERE 1=1
        "#
        .to_string();

        if filter.template_id.is_some() {
            query.push_str(" AND ro.recommendation_template_id = $1");
        }
        if filter.category.is_some() {
            query.push_str(" AND rt.category::text = $2");
        }
        if filter.from_date.is_some() {
            query.push_str(" AND ro.completed_at >= $3");
        }
        if filter.to_date.is_some() {
            query.push_str(" AND ro.completed_at <= $4");
        }

        query.push_str(
            r#"
                GROUP BY ro.recommendation_template_id, rt.category
            ),
            recent_stats AS (
                SELECT
                    ro.recommendation_template_id,
                    COUNT(*) as recent_completions,
                    AVG(ro.user_rating) as recent_avg_rating
                FROM recommendation_outcomes ro
                WHERE ro.completed_at >= NOW() - INTERVAL '30 days'
                GROUP BY ro.recommendation_template_id
            ),
            suggestion_stats AS (
                SELECT
                    recommendation_template_id,
                    COUNT(*) as total_suggestions
                FROM user_recommendations
                GROUP BY recommendation_template_id
            )
            SELECT
                os.recommendation_template_id as template_id,
                os.category,
                COALESCE(ss.total_suggestions, 0) as total_suggestions,
                os.completions as total_completions,
                CASE
                    WHEN COALESCE(ss.total_suggestions, 0) > 0
                    THEN os.completions::float / ss.total_suggestions
                    ELSE 0
                END as completion_rate,
                os.avg_rating,
                os.avg_improvement as avg_recovery_improvement,
                os.avg_effectiveness,
                COALESCE(rs.recent_completions, 0) as last_30_days_completions,
                rs.recent_avg_rating as last_30_days_avg_rating
            FROM outcome_stats os
            LEFT JOIN recent_stats rs ON rs.recommendation_template_id = os.recommendation_template_id
            LEFT JOIN suggestion_stats ss ON ss.recommendation_template_id = os.recommendation_template_id
            ORDER BY os.avg_effectiveness DESC NULLS LAST
        "#,
        );

        // Execute query with parameters
        let mut query_builder = sqlx::query(&query);

        if let Some(template_id) = filter.template_id {
            query_builder = query_builder.bind(template_id);
        }
        if let Some(category) = filter.category {
            query_builder = query_builder.bind(category);
        }
        if let Some(from_date) = filter.from_date {
            query_builder = query_builder.bind(from_date);
        }
        if let Some(to_date) = filter.to_date {
            query_builder = query_builder.bind(to_date);
        }

        let rows = query_builder.fetch_all(&self.db).await?;

        let analytics = rows
            .iter()
            .map(|row| {
                let template_id: Option<Uuid> = row.try_get("template_id").ok();
                let category: Option<String> = row.try_get("category").ok();
                let total_suggestions: i64 = row.try_get("total_suggestions").unwrap_or(0);
                let total_completions: i64 = row.try_get("total_completions").unwrap_or(0);
                let completion_rate: f64 = row.try_get("completion_rate").unwrap_or(0.0);
                let avg_user_rating: Option<f64> = row.try_get("avg_rating").ok();
                let avg_recovery_improvement: Option<f64> =
                    row.try_get("avg_recovery_improvement").ok();
                let avg_effectiveness_score: Option<f64> = row.try_get("avg_effectiveness").ok();
                let last_30_days_completions: i64 =
                    row.try_get("last_30_days_completions").unwrap_or(0);
                let last_30_days_avg_rating: Option<f64> =
                    row.try_get("last_30_days_avg_rating").ok();

                let effectiveness_trend = determine_effectiveness_trend(
                    last_30_days_avg_rating,
                    avg_user_rating,
                );

                EffectivenessAnalytics {
                    template_id,
                    category,
                    total_suggestions,
                    total_completions,
                    completion_rate,
                    avg_user_rating,
                    avg_recovery_improvement,
                    avg_effectiveness_score,
                    effectiveness_trend,
                    last_30_days_completions,
                    last_30_days_avg_rating,
                }
            })
            .collect();

        Ok(analytics)
    }

    /// Get system-wide recommendation analytics
    pub async fn get_system_analytics(&self) -> Result<SystemAnalytics> {
        // Overall stats
        let overall = sqlx::query!(
            r#"
            SELECT
                COUNT(DISTINCT ur.id) as total_shown,
                COUNT(DISTINCT ro.id) as total_completed,
                AVG(ro.user_rating) as avg_rating,
                AVG(ro.effectiveness_score) as avg_effectiveness
            FROM user_recommendations ur
            LEFT JOIN recommendation_outcomes ro ON ro.user_recommendation_id = ur.id
            "#
        )
        .fetch_one(&self.db)
        .await?;

        let total_recommendations_shown = overall.total_shown.unwrap_or(0);
        let total_recommendations_completed = overall.total_completed.unwrap_or(0);
        let overall_completion_rate = if total_recommendations_shown > 0 {
            total_recommendations_completed as f64 / total_recommendations_shown as f64
        } else {
            0.0
        };

        // Category analytics
        let category_rows = sqlx::query!(
            r#"
            SELECT
                rt.category::text as category,
                COUNT(DISTINCT ro.id) as completions,
                COUNT(DISTINCT ur.id) as total_suggestions,
                AVG(ro.user_rating) as avg_rating,
                AVG(ro.effectiveness_score) as avg_effectiveness
            FROM recommendation_templates rt
            LEFT JOIN user_recommendations ur ON ur.recommendation_template_id = rt.id
            LEFT JOIN recommendation_outcomes ro ON ro.recommendation_template_id = rt.id
            GROUP BY rt.category
            "#
        )
        .fetch_all(&self.db)
        .await?;

        let category_analytics = category_rows
            .iter()
            .map(|row| {
                let completions = row.completions.unwrap_or(0);
                let total = row.total_suggestions.unwrap_or(0);
                let completion_rate = if total > 0 {
                    completions as f64 / total as f64
                } else {
                    0.0
                };

                CategoryAnalytics {
                    category: row.category.clone().unwrap_or_default(),
                    completions,
                    completion_rate,
                    avg_rating: row.avg_rating,
                    avg_effectiveness: row.avg_effectiveness,
                }
            })
            .collect();

        // Top performing templates
        let top_performing = self.get_top_performing_templates(10).await?;

        // Underperforming templates
        let underperforming = self.get_underperforming_templates().await?;

        Ok(SystemAnalytics {
            total_recommendations_shown,
            total_recommendations_completed,
            overall_completion_rate,
            overall_avg_rating: overall.avg_rating,
            overall_avg_effectiveness: overall.avg_effectiveness,
            category_analytics,
            top_performing_templates: top_performing,
            underperforming_templates: underperforming,
        })
    }

    /// Get top performing templates
    async fn get_top_performing_templates(&self, limit: i32) -> Result<Vec<TemplatePerformance>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                rt.id,
                rt.title,
                rt.category::text as category,
                COUNT(DISTINCT ro.id) as completions,
                COUNT(DISTINCT ur.id) as total_suggestions,
                AVG(ro.user_rating) as avg_rating,
                AVG(ro.effectiveness_score) as avg_effectiveness
            FROM recommendation_templates rt
            LEFT JOIN user_recommendations ur ON ur.recommendation_template_id = rt.id
            LEFT JOIN recommendation_outcomes ro ON ro.recommendation_template_id = rt.id
            WHERE rt.is_active = true
            GROUP BY rt.id, rt.title, rt.category
            HAVING COUNT(DISTINCT ro.id) >= 5
            ORDER BY AVG(ro.effectiveness_score) DESC NULLS LAST
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        Ok(self.map_to_template_performance(rows))
    }

    /// Get underperforming templates that need review
    async fn get_underperforming_templates(&self) -> Result<Vec<TemplatePerformance>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                rt.id,
                rt.title,
                rt.category::text as category,
                COUNT(DISTINCT ro.id) as completions,
                COUNT(DISTINCT ur.id) as total_suggestions,
                AVG(ro.user_rating) as avg_rating,
                AVG(ro.effectiveness_score) as avg_effectiveness
            FROM recommendation_templates rt
            LEFT JOIN user_recommendations ur ON ur.recommendation_template_id = rt.id
            LEFT JOIN recommendation_outcomes ro ON ro.recommendation_template_id = rt.id
            WHERE rt.is_active = true
            GROUP BY rt.id, rt.title, rt.category
            HAVING
                (COUNT(DISTINCT ro.id) >= 5 AND AVG(ro.effectiveness_score) < 0.3)
                OR (COUNT(DISTINCT ur.id) >= 10 AND COUNT(DISTINCT ro.id)::float / COUNT(DISTINCT ur.id) < 0.1)
                OR (COUNT(DISTINCT ro.id) >= 5 AND AVG(ro.user_rating) < 2.0)
            ORDER BY AVG(ro.effectiveness_score) ASC NULLS FIRST
            "#
        )
        .fetch_all(&self.db)
        .await?;

        Ok(self.map_to_template_performance(rows))
    }

    /// Flag underperforming templates as inactive
    async fn flag_underperforming_templates(&self) -> Result<Vec<Uuid>> {
        let flagged = sqlx::query!(
            r#"
            UPDATE recommendation_templates rt
            SET is_active = false, updated_at = NOW()
            FROM (
                SELECT
                    rt.id
                FROM recommendation_templates rt
                LEFT JOIN user_recommendations ur ON ur.recommendation_template_id = rt.id
                LEFT JOIN recommendation_outcomes ro ON ro.recommendation_template_id = rt.id
                WHERE rt.is_active = true
                  AND ro.completed_at >= NOW() - INTERVAL '30 days'
                GROUP BY rt.id
                HAVING
                    (COUNT(DISTINCT ro.id) >= 10 AND AVG(ro.effectiveness_score) < 0.3)
                    OR (AVG(ro.user_rating) < 2.0 AND COUNT(DISTINCT ro.id) >= 10)
            ) underperforming
            WHERE rt.id = underperforming.id
            RETURNING rt.id
            "#
        )
        .fetch_all(&self.db)
        .await?;

        Ok(flagged.iter().map(|r| r.id).collect())
    }

    /// Helper to map query results to TemplatePerformance
    fn map_to_template_performance(
        &self,
        rows: Vec<sqlx::postgres::PgRow>,
    ) -> Vec<TemplatePerformance> {
        rows.iter()
            .map(|row| {
                let template_id: Uuid = row.get("id");
                let title: String = row.get("title");
                let category: String = row.get("category");
                let completions: i64 = row.try_get("completions").unwrap_or(0);
                let total_suggestions: i64 = row.try_get("total_suggestions").unwrap_or(0);
                let avg_rating: Option<f64> = row.try_get("avg_rating").ok();
                let avg_effectiveness: Option<f64> = row.try_get("avg_effectiveness").ok();

                let completion_rate = if total_suggestions > 0 {
                    completions as f64 / total_suggestions as f64
                } else {
                    0.0
                };

                let needs_review = (completions >= 5 && avg_effectiveness.unwrap_or(1.0) < 0.3)
                    || (total_suggestions >= 10 && completion_rate < 0.1)
                    || (completions >= 5 && avg_rating.unwrap_or(5.0) < 2.0);

                TemplatePerformance {
                    template_id,
                    title,
                    category,
                    completions,
                    completion_rate,
                    avg_rating,
                    avg_effectiveness,
                    needs_review,
                }
            })
            .collect()
    }
}
