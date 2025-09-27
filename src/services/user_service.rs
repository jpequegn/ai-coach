use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::password::hash_password;
use crate::models::{CreateUser, UpdateUser, UserResponse};

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

        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (email, password_hash, created_at, updated_at)
            VALUES ($1, $2, $3, $3)
            RETURNING id, email, password_hash, created_at, updated_at
            "#,
            user_data.email,
            password_hash,
            Utc::now()
        )
        .fetch_one(&self.db)
        .await?;

        Ok(UserResponse {
            id: user.id,
            email: user.email,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<UserResponse>> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, created_at, updated_at FROM users WHERE id = $1",
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(user.map(|u| UserResponse {
            id: u.id,
            email: u.email,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }))
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<UserResponse>> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, created_at, updated_at FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(user.map(|u| UserResponse {
            id: u.id,
            email: u.email,
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
            RETURNING id, email, password_hash, created_at, updated_at
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
            "SELECT id, email, password_hash, created_at, updated_at FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            limit,
            offset
        )
        .fetch_all(&self.db)
        .await?;

        Ok(users.into_iter().map(|u| UserResponse {
            id: u.id,
            email: u.email,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }).collect())
    }
}