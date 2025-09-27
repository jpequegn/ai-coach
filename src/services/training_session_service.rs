use anyhow::Result;
use chrono::{Utc, NaiveDate};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{TrainingSession, CreateTrainingSession, UpdateTrainingSession, SessionSummary};

pub struct TrainingSessionService {
    db: PgPool,
}

impl TrainingSessionService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create_session(&self, session_data: CreateTrainingSession) -> Result<TrainingSession> {
        let session = sqlx::query_as!(
            TrainingSession,
            r#"
            INSERT INTO training_sessions (user_id, date, trainrs_data, uploaded_file_path, session_type, duration_seconds, distance_meters, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
            RETURNING id, user_id, date, trainrs_data, uploaded_file_path, session_type, duration_seconds, distance_meters, created_at, updated_at
            "#,
            session_data.user_id,
            session_data.date,
            session_data.trainrs_data,
            session_data.uploaded_file_path,
            session_data.session_type,
            session_data.duration_seconds,
            session_data.distance_meters,
            Utc::now()
        )
        .fetch_one(&self.db)
        .await?;

        Ok(session)
    }

    pub async fn get_session_by_id(&self, session_id: Uuid) -> Result<Option<TrainingSession>> {
        let session = sqlx::query_as!(
            TrainingSession,
            "SELECT id, user_id, date, trainrs_data, uploaded_file_path, session_type, duration_seconds, distance_meters, created_at, updated_at FROM training_sessions WHERE id = $1",
            session_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(session)
    }

    pub async fn get_sessions_by_user_id(&self, user_id: Uuid, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<TrainingSession>> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        let sessions = sqlx::query_as!(
            TrainingSession,
            "SELECT id, user_id, date, trainrs_data, uploaded_file_path, session_type, duration_seconds, distance_meters, created_at, updated_at FROM training_sessions WHERE user_id = $1 ORDER BY date DESC LIMIT $2 OFFSET $3",
            user_id,
            limit,
            offset
        )
        .fetch_all(&self.db)
        .await?;

        Ok(sessions)
    }

    pub async fn get_sessions_by_date_range(&self, user_id: Uuid, start_date: NaiveDate, end_date: NaiveDate) -> Result<Vec<TrainingSession>> {
        let sessions = sqlx::query_as!(
            TrainingSession,
            "SELECT id, user_id, date, trainrs_data, uploaded_file_path, session_type, duration_seconds, distance_meters, created_at, updated_at FROM training_sessions WHERE user_id = $1 AND date >= $2 AND date <= $3 ORDER BY date ASC",
            user_id,
            start_date,
            end_date
        )
        .fetch_all(&self.db)
        .await?;

        Ok(sessions)
    }

    pub async fn update_session(&self, session_id: Uuid, session_data: UpdateTrainingSession) -> Result<Option<TrainingSession>> {
        let now = Utc::now();

        let session = sqlx::query_as!(
            TrainingSession,
            r#"
            UPDATE training_sessions
            SET date = COALESCE($2, date),
                trainrs_data = COALESCE($3, trainrs_data),
                uploaded_file_path = COALESCE($4, uploaded_file_path),
                session_type = COALESCE($5, session_type),
                duration_seconds = COALESCE($6, duration_seconds),
                distance_meters = COALESCE($7, distance_meters),
                updated_at = $8
            WHERE id = $1
            RETURNING id, user_id, date, trainrs_data, uploaded_file_path, session_type, duration_seconds, distance_meters, created_at, updated_at
            "#,
            session_id,
            session_data.date,
            session_data.trainrs_data,
            session_data.uploaded_file_path,
            session_data.session_type,
            session_data.duration_seconds,
            session_data.distance_meters,
            now
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(session)
    }

    pub async fn delete_session(&self, session_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM training_sessions WHERE id = $1",
            session_id
        )
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_session_summary(&self, user_id: Uuid, start_date: Option<NaiveDate>, end_date: Option<NaiveDate>) -> Result<SessionSummary> {
        let mut query = "SELECT COUNT(*) as total_sessions, SUM(duration_seconds) as total_duration, SUM(distance_meters) as total_distance, AVG(duration_seconds) as average_duration FROM training_sessions WHERE user_id = $1".to_string();

        let mut query_params = vec![user_id.to_string()];

        if let Some(start) = start_date {
            query.push_str(" AND date >= $2");
            query_params.push(start.to_string());
        }

        if let Some(end) = end_date {
            if start_date.is_some() {
                query.push_str(" AND date <= $3");
            } else {
                query.push_str(" AND date <= $2");
            }
            query_params.push(end.to_string());
        }

        let summary_row = sqlx::query!(
            &query,
            user_id
        )
        .fetch_one(&self.db)
        .await?;

        let session_types = sqlx::query_scalar!(
            "SELECT DISTINCT session_type FROM training_sessions WHERE user_id = $1 AND session_type IS NOT NULL",
            user_id
        )
        .fetch_all(&self.db)
        .await?;

        Ok(SessionSummary {
            total_sessions: summary_row.total_sessions.unwrap_or(0),
            total_duration: summary_row.total_duration,
            total_distance: summary_row.total_distance,
            average_duration: summary_row.average_duration,
            session_types,
        })
    }
}