use crate::models::{
    CompleteRecommendationRequest, HistoryFilter, RateRecommendationRequest,
    SkipRecommendationRequest, UserRecommendation, UserRecommendationStatus,
    UserRecommendationWithTemplate, RecommendationTemplate,
};
use crate::services::RecommendationEffectivenessService;
use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

pub struct RecommendationTrackingService {
    db: PgPool,
    effectiveness_service: Option<Arc<RecommendationEffectivenessService>>,
}

impl RecommendationTrackingService {
    pub fn new(db: PgPool) -> Self {
        Self {
            db: db.clone(),
            effectiveness_service: Some(Arc::new(RecommendationEffectivenessService::new(db))),
        }
    }

    /// Complete a recommendation
    pub async fn complete_recommendation(
        &self,
        recommendation_id: Uuid,
        user_id: Uuid,
        request: CompleteRecommendationRequest,
    ) -> Result<UserRecommendation> {
        let completed_at = request.completed_at.unwrap_or_else(Utc::now);

        let recommendation = sqlx::query_as::<_, UserRecommendation>(
            r#"
            UPDATE user_recommendations
            SET status = 'completed',
                completed_at = $3,
                updated_at = NOW()
            WHERE id = $1 AND user_id = $2 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(recommendation_id)
        .bind(user_id)
        .bind(completed_at)
        .fetch_one(&self.db)
        .await
        .context("Failed to complete recommendation")?;

        info!(
            "User {} completed recommendation {}",
            user_id, recommendation_id
        );

        // Track outcome for effectiveness measurement
        if let Some(effectiveness_service) = &self.effectiveness_service {
            // Get baseline recovery score at the time recommendation was shown
            if let Ok(Some((score_id, score_value))) = self.get_recovery_score_at_time(
                user_id,
                recommendation.shown_at,
            ).await {
                if let Err(e) = effectiveness_service
                    .track_outcome(&recommendation, Some(score_id), score_value)
                    .await
                {
                    warn!(
                        "Failed to track outcome for recommendation {}: {}",
                        recommendation_id, e
                    );
                }
            } else {
                warn!(
                    "No recovery score found for user {} at time {}",
                    user_id, recommendation.shown_at
                );
            }
        }

        Ok(recommendation)
    }

    /// Get recovery score closest to a specific time
    async fn get_recovery_score_at_time(
        &self,
        user_id: Uuid,
        target_time: chrono::DateTime<Utc>,
    ) -> Result<Option<(Uuid, f64)>> {
        let result = sqlx::query!(
            r#"
            SELECT id, overall_readiness
            FROM recovery_scores
            WHERE user_id = $1
              AND score_date <= $2
            ORDER BY score_date DESC
            LIMIT 1
            "#,
            user_id,
            target_time
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(result.map(|r| (r.id, r.overall_readiness)))
    }

    /// Skip a recommendation
    pub async fn skip_recommendation(
        &self,
        recommendation_id: Uuid,
        user_id: Uuid,
        request: SkipRecommendationRequest,
    ) -> Result<UserRecommendation> {
        let recommendation = sqlx::query_as::<_, UserRecommendation>(
            r#"
            UPDATE user_recommendations
            SET status = 'skipped',
                skip_reason = $3,
                skipped_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND user_id = $2 AND status = 'pending'
            RETURNING *
            "#,
        )
        .bind(recommendation_id)
        .bind(user_id)
        .bind(request.reason)
        .fetch_one(&self.db)
        .await
        .context("Failed to skip recommendation")?;

        info!(
            "User {} skipped recommendation {}",
            user_id, recommendation_id
        );

        Ok(recommendation)
    }

    /// Rate a recommendation
    pub async fn rate_recommendation(
        &self,
        recommendation_id: Uuid,
        user_id: Uuid,
        request: RateRecommendationRequest,
    ) -> Result<UserRecommendation> {
        // Validate rating range
        if request.rating < 1 || request.rating > 5 {
            anyhow::bail!("Rating must be between 1 and 5");
        }

        let recommendation = sqlx::query_as::<_, UserRecommendation>(
            r#"
            UPDATE user_recommendations
            SET effectiveness_rating = $3,
                user_feedback = $4,
                rated_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND user_id = $2
            RETURNING *
            "#,
        )
        .bind(recommendation_id)
        .bind(user_id)
        .bind(request.rating)
        .bind(request.feedback)
        .fetch_one(&self.db)
        .await
        .context("Failed to rate recommendation")?;

        info!(
            "User {} rated recommendation {} with {} stars",
            user_id, recommendation_id, request.rating
        );

        // Update template effectiveness score
        self.update_template_effectiveness(recommendation.recommendation_template_id)
            .await?;

        Ok(recommendation)
    }

    /// Get user recommendation history
    pub async fn get_user_history(
        &self,
        user_id: Uuid,
        filters: HistoryFilter,
    ) -> Result<Vec<UserRecommendationWithTemplate>> {
        let page = filters.page.unwrap_or(1);
        let page_size = filters.page_size.unwrap_or(20).min(100); // Max 100 per page
        let offset = (page - 1) * page_size;

        let mut query = String::from(
            r#"
            SELECT
                ur.*,
                rt.id as "template_id",
                rt.category as "template_category",
                rt.title as "template_title",
                rt.description as "template_description",
                rt.action as "template_action",
                rt.priority_default as "template_priority_default",
                rt.difficulty as "template_difficulty",
                rt.expected_impact as "template_expected_impact",
                rt.time_required_minutes as "template_time_required_minutes",
                rt.trigger_conditions as "template_trigger_conditions",
                rt.user_constraints as "template_user_constraints",
                rt.effectiveness_score as "template_effectiveness_score",
                rt.times_suggested as "template_times_suggested",
                rt.times_completed as "template_times_completed",
                rt.metadata as "template_metadata",
                rt.is_active as "template_is_active",
                rt.created_at as "template_created_at",
                rt.updated_at as "template_updated_at"
            FROM user_recommendations ur
            JOIN recommendation_templates rt ON ur.recommendation_template_id = rt.id
            WHERE ur.user_id = $1
            "#,
        );

        let mut bind_count = 2;
        if filters.status.is_some() {
            query.push_str(&format!(" AND ur.status = ${}", bind_count));
            bind_count += 1;
        }
        if filters.category.is_some() {
            query.push_str(&format!(" AND rt.category = ${}", bind_count));
            bind_count += 1;
        }
        if filters.from_date.is_some() {
            query.push_str(&format!(" AND ur.shown_at >= ${}", bind_count));
            bind_count += 1;
        }
        if filters.to_date.is_some() {
            query.push_str(&format!(" AND ur.shown_at <= ${}", bind_count));
            bind_count += 1;
        }

        query.push_str(&format!(
            " ORDER BY ur.shown_at DESC LIMIT ${} OFFSET ${}",
            bind_count,
            bind_count + 1
        ));

        let mut query_builder = sqlx::query(&query).bind(user_id);

        if let Some(status) = filters.status {
            query_builder = query_builder.bind(status.to_string());
        }
        if let Some(category) = filters.category {
            query_builder = query_builder.bind(category.to_string());
        }
        if let Some(from_date) = filters.from_date {
            query_builder = query_builder.bind(from_date);
        }
        if let Some(to_date) = filters.to_date {
            query_builder = query_builder.bind(to_date);
        }

        query_builder = query_builder.bind(page_size).bind(offset);

        let rows = query_builder.fetch_all(&self.db).await?;

        let results: Vec<UserRecommendationWithTemplate> = rows
            .iter()
            .map(|row| {
                let user_recommendation = UserRecommendation {
                    id: row.get("id"),
                    user_id: row.get("user_id"),
                    recommendation_template_id: row.get("recommendation_template_id"),
                    recovery_score_id: row.get("recovery_score_id"),
                    status: row.get("status"),
                    effectiveness_rating: row.get("effectiveness_rating"),
                    user_feedback: row.get("user_feedback"),
                    skip_reason: row.get("skip_reason"),
                    shown_at: row.get("shown_at"),
                    completed_at: row.get("completed_at"),
                    skipped_at: row.get("skipped_at"),
                    expired_at: row.get("expired_at"),
                    rated_at: row.get("rated_at"),
                    metadata: row.get("metadata"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                };

                let template = RecommendationTemplate {
                    id: row.get("template_id"),
                    category: row.get("template_category"),
                    title: row.get("template_title"),
                    description: row.get("template_description"),
                    action: row.get("template_action"),
                    priority_default: row.get("template_priority_default"),
                    difficulty: row.get("template_difficulty"),
                    expected_impact: row.get("template_expected_impact"),
                    time_required_minutes: row.get("template_time_required_minutes"),
                    trigger_conditions: row.get("template_trigger_conditions"),
                    user_constraints: row.get("template_user_constraints"),
                    effectiveness_score: row.get("template_effectiveness_score"),
                    times_suggested: row.get("template_times_suggested"),
                    times_completed: row.get("template_times_completed"),
                    metadata: row.get("template_metadata"),
                    is_active: row.get("template_is_active"),
                    created_at: row.get("template_created_at"),
                    updated_at: row.get("template_updated_at"),
                };

                UserRecommendationWithTemplate {
                    user_recommendation,
                    template,
                }
            })
            .collect();

        Ok(results)
    }

    /// Expire old recommendations (>7 days pending)
    pub async fn expire_old_recommendations(&self) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE user_recommendations
            SET status = 'expired',
                expired_at = NOW(),
                updated_at = NOW()
            WHERE status = 'pending'
              AND shown_at < NOW() - INTERVAL '7 days'
            "#,
        )
        .execute(&self.db)
        .await
        .context("Failed to expire old recommendations")?;

        let count = result.rows_affected();
        if count > 0 {
            info!("Expired {} old recommendations", count);
        }

        Ok(count)
    }

    /// Update template effectiveness score based on ratings
    async fn update_template_effectiveness(&self, template_id: Uuid) -> Result<()> {
        // Calculate exponential moving average of ratings
        let result = sqlx::query(
            r#"
            UPDATE recommendation_templates rt
            SET effectiveness_score = (
                SELECT COALESCE(
                    -- Exponential moving average: 0.7 * new_avg + 0.3 * old_score
                    0.7 * (AVG(effectiveness_rating::float) / 5.0) + 0.3 * rt.effectiveness_score,
                    rt.effectiveness_score
                )
                FROM user_recommendations
                WHERE recommendation_template_id = $1
                  AND effectiveness_rating IS NOT NULL
            ),
            times_completed = (
                SELECT COUNT(*)
                FROM user_recommendations
                WHERE recommendation_template_id = $1
                  AND status = 'completed'
            ),
            updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(template_id)
        .execute(&self.db)
        .await;

        match result {
            Ok(_) => {
                info!("Updated effectiveness score for template {}", template_id);
                Ok(())
            }
            Err(e) => {
                warn!(
                    "Failed to update effectiveness score for template {}: {}",
                    template_id, e
                );
                // Don't fail the rating operation if effectiveness update fails
                Ok(())
            }
        }
    }
}
