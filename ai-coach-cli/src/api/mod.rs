use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::Config;

mod error;
mod retry;

pub use error::ApiError;
pub use retry::RetryConfig;

/// Login request payload
#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response from API
#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserInfo,
}

/// User information
#[derive(Debug, Deserialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
}

/// Token refresh request
#[derive(Debug, Serialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Token refresh response
#[derive(Debug, Deserialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

/// API client for communicating with AI Coach backend
pub struct ApiClient {
    client: Client,
    base_url: String,
    config: Arc<Mutex<Config>>,
    retry_config: RetryConfig,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(config: Config) -> Result<Self> {
        let timeout = Duration::from_secs(config.api.timeout_seconds);
        let base_url = config.api.base_url.clone();

        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url,
            config: Arc::new(Mutex::new(config)),
            retry_config: RetryConfig::default(),
        })
    }

    /// Create a new API client with custom retry configuration
    pub fn with_retry_config(config: Config, retry_config: RetryConfig) -> Result<Self> {
        let timeout = Duration::from_secs(config.api.timeout_seconds);
        let base_url = config.api.base_url.clone();

        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url,
            config: Arc::new(Mutex::new(config)),
            retry_config,
        })
    }

    /// Login to AI Coach API
    pub async fn login(&self, username: &str, password: &str) -> Result<LoginResponse> {
        let url = format!("{}/api/v1/auth/login", self.base_url);
        let username = username.to_string();
        let password = password.to_string();

        tracing::debug!("Logging in as {}", username);

        // Use retry logic for login request
        self.retry_config
            .execute(|| async {
                let request = LoginRequest {
                    username: username.clone(),
                    password: password.clone(),
                };

                let response = self.client
                    .post(&url)
                    .json(&request)
                    .send()
                    .await
                    .context("Failed to send login request")?;

                let status = response.status();

                if status.is_success() {
                    let login_response: LoginResponse = response
                        .json()
                        .await
                        .context("Failed to parse login response")?;

                    // Save tokens to config
                    {
                        let mut config = self.config.lock().unwrap();
                        config.set_tokens(
                            login_response.access_token.clone(),
                            login_response.refresh_token.clone(),
                        );
                        config.save()?;
                    }

                    tracing::info!("Successfully logged in as {}", username);
                    Ok(login_response)
                } else {
                    let error_text = response.text().await.unwrap_or_default();
                    Err(ApiError::from_status(status, error_text).into())
                }
            })
            .await
    }

    /// Refresh access token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<RefreshTokenResponse> {
        let url = format!("{}/api/v1/auth/refresh", self.base_url);

        let request = RefreshTokenRequest {
            refresh_token: refresh_token.to_string(),
        };

        tracing::debug!("Refreshing access token");

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send refresh token request")?;

        let status = response.status();

        if status.is_success() {
            let refresh_response: RefreshTokenResponse = response
                .json()
                .await
                .context("Failed to parse refresh response")?;

            tracing::info!("Successfully refreshed access token");
            Ok(refresh_response)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::from_status(status, error_text).into())
        }
    }

    /// Get current user information
    pub async fn whoami(&self) -> Result<UserInfo> {
        let url = format!("{}/api/v1/auth/me", self.base_url);

        let config = self.config.lock().unwrap();
        if !config.is_authenticated() {
            return Err(anyhow::anyhow!("Not logged in"));
        }

        let token = config.auth.token.clone();
        drop(config); // Release lock

        tracing::debug!("Fetching current user information");

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to send whoami request")?;

        let status = response.status();

        if status.is_success() {
            let user_info: UserInfo = response
                .json()
                .await
                .context("Failed to parse user info response")?;

            tracing::info!("Retrieved user info for {}", user_info.username);
            Ok(user_info)
        } else if status == StatusCode::UNAUTHORIZED {
            Err(anyhow::anyhow!("Authentication token is invalid or expired"))
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::from_status(status, error_text).into())
        }
    }

    /// Refresh access token and save to config
    async fn try_refresh_token(&self) -> Result<String> {
        let refresh_token = {
            let config = self.config.lock().unwrap();
            if config.auth.refresh_token.is_empty() {
                return Err(anyhow::anyhow!("No refresh token available"));
            }
            config.auth.refresh_token.clone()
        };

        tracing::debug!("Attempting to refresh access token");

        let refresh_response = self.refresh_token(&refresh_token).await?;

        // Save new tokens to config
        {
            let mut config = self.config.lock().unwrap();
            config.set_tokens(
                refresh_response.access_token.clone(),
                refresh_response.refresh_token.clone(),
            );
            config.save()?;
        }

        tracing::info!("Successfully refreshed and saved access token");
        Ok(refresh_response.access_token)
    }

    /// Make an authenticated GET request with automatic token refresh
    pub async fn get(&self, path: &str) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.base_url, path);

        let token = {
            let config = self.config.lock().unwrap();
            if !config.is_authenticated() {
                return Err(anyhow::anyhow!("Not logged in"));
            }
            config.auth.token.clone()
        };

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to send GET request")?;

        // If we get 401, try to refresh token and retry once
        if response.status() == StatusCode::UNAUTHORIZED {
            tracing::debug!("Received 401, attempting token refresh");

            let new_token = self.try_refresh_token().await?;

            // Retry with new token
            let response = self.client
                .get(&url)
                .header("Authorization", format!("Bearer {}", new_token))
                .send()
                .await
                .context("Failed to retry GET request after token refresh")?;

            return Ok(response);
        }

        Ok(response)
    }

    /// Make an authenticated POST request with automatic token refresh
    pub async fn post<T: Serialize>(&self, path: &str, body: &T) -> Result<reqwest::Response> {
        let url = format!("{}{}", self.base_url, path);

        let token = {
            let config = self.config.lock().unwrap();
            if !config.is_authenticated() {
                return Err(anyhow::anyhow!("Not logged in"));
            }
            config.auth.token.clone()
        };

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(body)
            .send()
            .await
            .context("Failed to send POST request")?;

        // If we get 401, try to refresh token and retry once
        if response.status() == StatusCode::UNAUTHORIZED {
            tracing::debug!("Received 401, attempting token refresh");

            let new_token = self.try_refresh_token().await?;

            // Retry with new token
            let response = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", new_token))
                .json(body)
                .send()
                .await
                .context("Failed to retry POST request after token refresh")?;

            return Ok(response);
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_creation() {
        let config = Config::default();
        let client = ApiClient::new(config);
        assert!(client.is_ok());
    }
}
