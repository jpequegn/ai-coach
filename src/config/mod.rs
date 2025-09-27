use anyhow::Result;
use sqlx::PgPool;
use std::env;

/// Application configuration
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
}

impl AppConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()?,
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key".to_string()),
        })
    }

    /// Get server address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

impl DatabaseConfig {
    /// Create database configuration from environment
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/ai_coach".to_string()),
            max_connections: env::var("DB_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,
        })
    }

    /// Create database connection pool
    pub async fn create_pool(&self) -> Result<PgPool> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(self.max_connections)
            .connect(&self.url)
            .await?;

        Ok(pool)
    }
}