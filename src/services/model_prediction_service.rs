use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{ModelPrediction, CreateModelPrediction, UpdateModelPrediction};

pub struct ModelPredictionService {
    db: PgPool,
}

impl ModelPredictionService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create_prediction(&self, pred_data: CreateModelPrediction) -> Result<ModelPrediction> {
        let prediction = sqlx::query_as!(
            ModelPrediction,
            r#"
            INSERT INTO model_predictions (user_id, prediction_type, data, confidence, model_version, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $6)
            RETURNING id, user_id, prediction_type, data, confidence, model_version, created_at, updated_at
            "#,
            pred_data.user_id,
            pred_data.prediction_type,
            pred_data.data,
            pred_data.confidence,
            pred_data.model_version,
            Utc::now()
        )
        .fetch_one(&self.db)
        .await?;

        Ok(prediction)
    }

    pub async fn get_prediction_by_id(&self, pred_id: Uuid) -> Result<Option<ModelPrediction>> {
        let prediction = sqlx::query_as!(
            ModelPrediction,
            "SELECT id, user_id, prediction_type, data, confidence, model_version, created_at, updated_at FROM model_predictions WHERE id = $1",
            pred_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(prediction)
    }

    pub async fn get_predictions_by_user_id(&self, user_id: Uuid, prediction_type: Option<String>, limit: Option<i64>) -> Result<Vec<ModelPrediction>> {
        let limit = limit.unwrap_or(50);

        let predictions = match prediction_type {
            Some(pred_type) => {
                sqlx::query_as!(
                    ModelPrediction,
                    "SELECT id, user_id, prediction_type, data, confidence, model_version, created_at, updated_at FROM model_predictions WHERE user_id = $1 AND prediction_type = $2 ORDER BY created_at DESC LIMIT $3",
                    user_id,
                    pred_type,
                    limit
                )
                .fetch_all(&self.db)
                .await?
            },
            None => {
                sqlx::query_as!(
                    ModelPrediction,
                    "SELECT id, user_id, prediction_type, data, confidence, model_version, created_at, updated_at FROM model_predictions WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2",
                    user_id,
                    limit
                )
                .fetch_all(&self.db)
                .await?
            }
        };

        Ok(predictions)
    }

    pub async fn update_prediction(&self, pred_id: Uuid, pred_data: UpdateModelPrediction) -> Result<Option<ModelPrediction>> {
        let now = Utc::now();

        let prediction = sqlx::query_as!(
            ModelPrediction,
            r#"
            UPDATE model_predictions
            SET prediction_type = COALESCE($2, prediction_type),
                data = COALESCE($3, data),
                confidence = COALESCE($4, confidence),
                model_version = COALESCE($5, model_version),
                updated_at = $6
            WHERE id = $1
            RETURNING id, user_id, prediction_type, data, confidence, model_version, created_at, updated_at
            "#,
            pred_id,
            pred_data.prediction_type,
            pred_data.data,
            pred_data.confidence,
            pred_data.model_version,
            now
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(prediction)
    }

    pub async fn delete_prediction(&self, pred_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM model_predictions WHERE id = $1",
            pred_id
        )
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}