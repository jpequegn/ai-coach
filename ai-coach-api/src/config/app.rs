use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub environment: String,
    pub log_level: String,
    pub jwt_secret: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .unwrap_or(3000);
        let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        let jwt_secret = env::var("JWT_SECRET")
            .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());

        Ok(AppConfig {
            host,
            port,
            environment,
            log_level,
            jwt_secret,
        })
    }

    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}