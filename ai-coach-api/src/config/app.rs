use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub environment: String,
    pub log_level: String,
    pub jwt_secret: String,
    pub allowed_origins: Vec<String>,
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

        // JWT Secret validation
        let jwt_secret = match env::var("JWT_SECRET") {
            Ok(secret) => {
                // Validate minimum length for security
                if secret.len() < 32 {
                    return Err(anyhow::anyhow!(
                        "JWT_SECRET must be at least 32 characters long for security"
                    ));
                }
                secret
            }
            Err(_) => {
                // Only allow default in development
                if environment == "production" {
                    return Err(anyhow::anyhow!(
                        "JWT_SECRET environment variable is required in production"
                    ));
                }
                // Development-only fallback with clear warning
                eprintln!("⚠️  WARNING: Using insecure default JWT_SECRET for development");
                eprintln!("⚠️  Set JWT_SECRET environment variable for production!");
                "development-insecure-jwt-secret-change-this".to_string()
            }
        };

        // CORS allowed origins configuration
        let allowed_origins = env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| {
                if environment == "production" {
                    // Empty list in production requires explicit configuration
                    String::new()
                } else {
                    // Development defaults
                    "http://localhost:3000,http://localhost:5173,http://127.0.0.1:3000".to_string()
                }
            })
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().to_string())
            .collect();

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
            allowed_origins,
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