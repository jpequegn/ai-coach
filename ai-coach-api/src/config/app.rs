use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub environment: String,
    pub log_level: String,
    pub jwt_secret: String,
    pub oura_client_id: Option<String>,
    pub oura_client_secret: Option<String>,
    pub oura_redirect_uri: Option<String>,
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

        // Oura OAuth configuration (optional)
        let oura_client_id = env::var("OURA_CLIENT_ID").ok();
        let oura_client_secret = env::var("OURA_CLIENT_SECRET").ok();
        let oura_redirect_uri = env::var("OURA_REDIRECT_URI").ok();

        Ok(AppConfig {
            host,
            port,
            environment,
            log_level,
            jwt_secret,
            oura_client_id,
            oura_client_secret,
            oura_redirect_uri,
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