use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
    pub idle_timeout: Duration,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/ai_coach".to_string());

        let max_connections = env::var("DB_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "20".to_string())
            .parse()
            .unwrap_or(20);

        let min_connections = env::var("DB_MIN_CONNECTIONS")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5);

        let connect_timeout_secs = env::var("DB_CONNECT_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);

        let idle_timeout_secs = env::var("DB_IDLE_TIMEOUT")
            .unwrap_or_else(|_| "600".to_string())
            .parse()
            .unwrap_or(600);

        Ok(DatabaseConfig {
            database_url,
            max_connections,
            min_connections,
            connect_timeout: Duration::from_secs(connect_timeout_secs),
            idle_timeout: Duration::from_secs(idle_timeout_secs),
        })
    }

    pub async fn create_pool(&self) -> Result<PgPool> {
        let pool = PgPoolOptions::new()
            .max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .acquire_timeout(self.connect_timeout)
            .idle_timeout(Some(self.idle_timeout))
            .connect(&self.database_url)
            .await?;

        Ok(pool)
    }
}

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}