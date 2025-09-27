use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{TrainingPlan, CreateTrainingPlan, UpdateTrainingPlan};

pub struct TrainingPlanService {
    db: PgPool,
}

impl TrainingPlanService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create_plan(&self, plan_data: CreateTrainingPlan) -> Result<TrainingPlan> {
        let plan = sqlx::query_as!(
            TrainingPlan,
            r#"
            INSERT INTO training_plans (user_id, goal, start_date, end_date, plan_data, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $6)
            RETURNING id, user_id, goal, start_date, end_date, plan_data, status, created_at, updated_at
            "#,
            plan_data.user_id,
            plan_data.goal,
            plan_data.start_date,
            plan_data.end_date,
            plan_data.plan_data,
            Utc::now()
        )
        .fetch_one(&self.db)
        .await?;

        Ok(plan)
    }

    pub async fn get_plan_by_id(&self, plan_id: Uuid) -> Result<Option<TrainingPlan>> {
        let plan = sqlx::query_as!(
            TrainingPlan,
            "SELECT id, user_id, goal, start_date, end_date, plan_data, status, created_at, updated_at FROM training_plans WHERE id = $1",
            plan_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(plan)
    }

    pub async fn get_plans_by_user_id(&self, user_id: Uuid) -> Result<Vec<TrainingPlan>> {
        let plans = sqlx::query_as!(
            TrainingPlan,
            "SELECT id, user_id, goal, start_date, end_date, plan_data, status, created_at, updated_at FROM training_plans WHERE user_id = $1 ORDER BY created_at DESC",
            user_id
        )
        .fetch_all(&self.db)
        .await?;

        Ok(plans)
    }

    pub async fn update_plan(&self, plan_id: Uuid, plan_data: UpdateTrainingPlan) -> Result<Option<TrainingPlan>> {
        let now = Utc::now();

        let plan = sqlx::query_as!(
            TrainingPlan,
            r#"
            UPDATE training_plans
            SET goal = COALESCE($2, goal),
                start_date = COALESCE($3, start_date),
                end_date = COALESCE($4, end_date),
                plan_data = COALESCE($5, plan_data),
                status = COALESCE($6, status),
                updated_at = $7
            WHERE id = $1
            RETURNING id, user_id, goal, start_date, end_date, plan_data, status, created_at, updated_at
            "#,
            plan_id,
            plan_data.goal,
            plan_data.start_date,
            plan_data.end_date,
            plan_data.plan_data,
            plan_data.status,
            now
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(plan)
    }

    pub async fn delete_plan(&self, plan_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM training_plans WHERE id = $1",
            plan_id
        )
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}