use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{CoachingRecommendation, CreateCoachingRecommendation, UpdateCoachingRecommendation};

pub struct CoachingRecommendationService {
    db: PgPool,
}

impl CoachingRecommendationService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create_recommendation(&self, rec_data: CreateCoachingRecommendation) -> Result<CoachingRecommendation> {
        let recommendation = sqlx::query_as!(
            CoachingRecommendation,
            r#"
            INSERT INTO coaching_recommendations (user_id, recommendation_type, content, confidence, metadata, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $6)
            RETURNING id, user_id, recommendation_type, content, confidence, metadata, applied, created_at, updated_at
            "#,
            rec_data.user_id,
            rec_data.recommendation_type,
            rec_data.content,
            rec_data.confidence,
            rec_data.metadata,
            Utc::now()
        )
        .fetch_one(&self.db)
        .await?;

        Ok(recommendation)
    }

    pub async fn get_recommendation_by_id(&self, rec_id: Uuid) -> Result<Option<CoachingRecommendation>> {
        let recommendation = sqlx::query_as!(
            CoachingRecommendation,
            "SELECT id, user_id, recommendation_type, content, confidence, metadata, applied, created_at, updated_at FROM coaching_recommendations WHERE id = $1",
            rec_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(recommendation)
    }

    pub async fn get_recommendations_by_user_id(&self, user_id: Uuid, limit: Option<i64>) -> Result<Vec<CoachingRecommendation>> {
        let limit = limit.unwrap_or(50);

        let recommendations = sqlx::query_as!(
            CoachingRecommendation,
            "SELECT id, user_id, recommendation_type, content, confidence, metadata, applied, created_at, updated_at FROM coaching_recommendations WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2",
            user_id,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        Ok(recommendations)
    }

    pub async fn update_recommendation(&self, rec_id: Uuid, rec_data: UpdateCoachingRecommendation) -> Result<Option<CoachingRecommendation>> {
        let now = Utc::now();

        let recommendation = sqlx::query_as!(
            CoachingRecommendation,
            r#"
            UPDATE coaching_recommendations
            SET recommendation_type = COALESCE($2, recommendation_type),
                content = COALESCE($3, content),
                confidence = COALESCE($4, confidence),
                metadata = COALESCE($5, metadata),
                applied = COALESCE($6, applied),
                updated_at = $7
            WHERE id = $1
            RETURNING id, user_id, recommendation_type, content, confidence, metadata, applied, created_at, updated_at
            "#,
            rec_id,
            rec_data.recommendation_type,
            rec_data.content,
            rec_data.confidence,
            rec_data.metadata,
            rec_data.applied,
            now
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(recommendation)
    }

    pub async fn delete_recommendation(&self, rec_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM coaching_recommendations WHERE id = $1",
            rec_id
        )
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}