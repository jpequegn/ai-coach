use anyhow::Result;
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::auth::password::hash_password;
use crate::models::{CreateUser, UpdateUser, User, UserResponse, DietaryPreferences, SleepSchedule};

#[derive(Clone)]
pub struct UserService {
    db: PgPool,
}

impl UserService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create_user(&self, user_data: CreateUser) -> Result<UserResponse> {
        let password_hash = hash_password(&user_data.password)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

        // Start a transaction to ensure user and profile are created together
        let mut tx = self.db.begin().await?;

        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (email, password_hash, timezone, active, created_at, updated_at)
            VALUES ($1, $2, 'UTC', true, $3, $3)
            RETURNING id as "id!", email as "email!", password_hash as "password_hash!", timezone as "timezone!", active as "active!", created_at as "created_at!", updated_at as "updated_at!"
            "#,
            user_data.email,
            password_hash,
            Utc::now()
        )
        .fetch_one(&mut *tx)
        .await?;

        // Create default recovery profile for the new user
        sqlx::query!(
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
            "#,
            user.id,
            json!([]),
            json!([]),
            json!(DietaryPreferences::default()),
            json!(SleepSchedule::default()),
            json!([]),
            json!([]),
            json!({})
        )
        .execute(&mut *tx)
        .await?;

        // Commit transaction
        tx.commit().await?;

        info!("Created user {} with default recovery profile", user.id);

        Ok(UserResponse {
            id: user.id,
            email: user.email,
            timezone: user.timezone,
            active: user.active,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<UserResponse>> {
        let user = sqlx::query_as!(
            User,
            "SELECT id as \"id!\", email as \"email!\", password_hash as \"password_hash!\", timezone as \"timezone!\", active as \"active!\", created_at as \"created_at!\", updated_at as \"updated_at!\" FROM users WHERE id = $1",
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(user.map(|u| UserResponse {
            id: u.id,
            email: u.email,
            timezone: u.timezone,
            active: u.active,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }))
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<UserResponse>> {
        let user = sqlx::query_as!(
            User,
            "SELECT id as \"id!\", email as \"email!\", password_hash as \"password_hash!\", timezone as \"timezone!\", active as \"active!\", created_at as \"created_at!\", updated_at as \"updated_at!\" FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(user.map(|u| UserResponse {
            id: u.id,
            email: u.email,
            timezone: u.timezone,
            active: u.active,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }))
    }

    pub async fn update_user(&self, user_id: Uuid, user_data: UpdateUser) -> Result<Option<UserResponse>> {
        let now = Utc::now();

        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET email = COALESCE($2, email),
                updated_at = $3
            WHERE id = $1
            RETURNING id as "id!", email as "email!", password_hash as "password_hash!", timezone as "timezone!", active as "active!", created_at as "created_at!", updated_at as "updated_at!"
            "#,
            user_id,
            user_data.email,
            now
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(user.map(|u| UserResponse {
            id: u.id,
            email: u.email,
            timezone: u.timezone,
            active: u.active,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }))
    }

    pub async fn delete_user(&self, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM users WHERE id = $1",
            user_id
        )
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_users(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<UserResponse>> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        let users = sqlx::query_as!(
            User,
            "SELECT id as \"id!\", email as \"email!\", password_hash as \"password_hash!\", timezone as \"timezone!\", active as \"active!\", created_at as \"created_at!\", updated_at as \"updated_at!\" FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            limit,
            offset
        )
        .fetch_all(&self.db)
        .await?;

        Ok(users.into_iter().map(|u| UserResponse {
            id: u.id,
            email: u.email,
            timezone: u.timezone,
            active: u.active,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }).collect())
    }
}