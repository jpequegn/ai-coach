use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};

/// Oura API v2 Client
///
/// Implements OAuth 2.0 flow and data synchronization with Oura Ring API.
/// Rate limit: 5,000 requests per day per user
/// API Documentation: https://cloud.ouraring.com/v2/docs
pub struct OuraApiClient {
    client: Client,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    base_url: String,
}

impl OuraApiClient {
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            client_id,
            client_secret,
            redirect_uri,
            base_url: "https://api.ouraring.com".to_string(),
        })
    }

    // ========================================================================
    // OAuth 2.0 Flow
    // ========================================================================

    /// Generate OAuth authorization URL
    pub fn get_authorization_url(&self, state: &str) -> String {
        format!(
            "https://cloud.ouraring.com/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope=daily%20heartrate%20workout%20tag%20personal%20session&state={}",
            self.client_id,
            urlencoding::encode(&self.redirect_uri),
            state
        )
    }

    /// Exchange authorization code for access token
    pub async fn exchange_code_for_token(
        &self,
        code: &str,
    ) -> Result<OuraTokenResponse> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.redirect_uri),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let response = self
            .client
            .post(format!("{}/oauth/token", self.base_url))
            .form(&params)
            .send()
            .await
            .context("Failed to send token request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Oura token exchange failed: {} - {}", status, error_text);
            anyhow::bail!("Failed to exchange code for token: {}", status);
        }

        let token_response = response
            .json::<OuraTokenResponse>()
            .await
            .context("Failed to parse token response")?;

        Ok(token_response)
    }

    /// Refresh access token
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<OuraTokenResponse> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let response = self
            .client
            .post(format!("{}/oauth/token", self.base_url))
            .form(&params)
            .send()
            .await
            .context("Failed to send refresh token request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Oura token refresh failed: {} - {}", status, error_text);
            anyhow::bail!("Failed to refresh token: {}", status);
        }

        let token_response = response
            .json::<OuraTokenResponse>()
            .await
            .context("Failed to parse refresh token response")?;

        Ok(token_response)
    }

    // ========================================================================
    // Data Retrieval Methods
    // ========================================================================

    /// Fetch daily sleep data
    pub async fn get_sleep_data(
        &self,
        access_token: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<OuraSleepData>> {
        let url = format!(
            "{}/v2/usercollection/daily_sleep?start_date={}&end_date={}",
            self.base_url, start_date, end_date
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .context("Failed to fetch sleep data")?;

        self.handle_rate_limit(&response).await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Oura sleep data request failed: {} - {}", status, error_text);
            anyhow::bail!("Failed to fetch sleep data: {}", status);
        }

        let sleep_response = response
            .json::<OuraSleepResponse>()
            .await
            .context("Failed to parse sleep data")?;

        Ok(sleep_response.data)
    }

    /// Fetch daily readiness data (includes HRV)
    pub async fn get_readiness_data(
        &self,
        access_token: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<OuraReadinessData>> {
        let url = format!(
            "{}/v2/usercollection/daily_readiness?start_date={}&end_date={}",
            self.base_url, start_date, end_date
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .context("Failed to fetch readiness data")?;

        self.handle_rate_limit(&response).await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Oura readiness data request failed: {} - {}", status, error_text);
            anyhow::bail!("Failed to fetch readiness data: {}", status);
        }

        let readiness_response = response
            .json::<OuraReadinessResponse>()
            .await
            .context("Failed to parse readiness data")?;

        Ok(readiness_response.data)
    }

    /// Fetch heart rate data
    pub async fn get_heart_rate_data(
        &self,
        access_token: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<OuraHeartRateData>> {
        let url = format!(
            "{}/v2/usercollection/heartrate?start_date={}&end_date={}",
            self.base_url, start_date, end_date
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .context("Failed to fetch heart rate data")?;

        self.handle_rate_limit(&response).await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Oura heart rate request failed: {} - {}", status, error_text);
            anyhow::bail!("Failed to fetch heart rate data: {}", status);
        }

        let hr_response = response
            .json::<OuraHeartRateResponse>()
            .await
            .context("Failed to parse heart rate data")?;

        Ok(hr_response.data)
    }

    // ========================================================================
    // Rate Limiting & Error Handling
    // ========================================================================

    async fn handle_rate_limit(&self, response: &reqwest::Response) -> Result<()> {
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            if let Some(retry_after) = response.headers().get("Retry-After") {
                let retry_seconds = retry_after
                    .to_str()
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(60);

                warn!(
                    "Oura API rate limit exceeded. Retry after {} seconds",
                    retry_seconds
                );

                anyhow::bail!(
                    "Rate limit exceeded. Retry after {} seconds",
                    retry_seconds
                );
            }
        }
        Ok(())
    }
}

// ============================================================================
// OAuth Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OuraTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: String,
}

// ============================================================================
// Oura API Data Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OuraSleepResponse {
    pub data: Vec<OuraSleepData>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OuraSleepData {
    pub id: String,
    pub day: NaiveDate,
    pub score: Option<i32>,
    pub timestamp: DateTime<Utc>,

    // Sleep contributors
    pub contributors: OuraSleepContributors,

    // Sleep periods
    pub average_breath: Option<f64>,
    pub average_heart_rate: Option<f64>,
    pub average_hrv: Option<f64>,
    pub awake_time: Option<i32>,
    pub bedtime_end: Option<DateTime<Utc>>,
    pub bedtime_start: Option<DateTime<Utc>>,
    pub day_end: Option<DateTime<Utc>>,
    pub day_start: Option<DateTime<Utc>>,
    pub deep_sleep_duration: Option<i32>,
    pub efficiency: Option<f64>,
    pub latency: Option<i32>,
    pub light_sleep_duration: Option<i32>,
    pub low_battery_alert: Option<bool>,
    pub lowest_heart_rate: Option<i32>,
    pub movement_30_sec: Option<String>,
    pub period: i32,
    pub rem_sleep_duration: Option<i32>,
    pub restless_periods: Option<i32>,
    pub sleep_phase_5_min: Option<String>,
    pub time_in_bed: Option<i32>,
    pub total_sleep_duration: Option<i32>,
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OuraSleepContributors {
    pub deep_sleep: Option<i32>,
    pub efficiency: Option<i32>,
    pub latency: Option<i32>,
    pub rem_sleep: Option<i32>,
    pub restfulness: Option<i32>,
    pub timing: Option<i32>,
    pub total_sleep: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OuraReadinessResponse {
    pub data: Vec<OuraReadinessData>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OuraReadinessData {
    pub id: String,
    pub day: NaiveDate,
    pub score: Option<i32>,
    pub timestamp: DateTime<Utc>,

    // Readiness contributors (includes HRV balance)
    pub contributors: OuraReadinessContributors,

    // Temperature deviation
    pub temperature_deviation: Option<f64>,
    pub temperature_trend_deviation: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OuraReadinessContributors {
    pub activity_balance: Option<i32>,
    pub body_temperature: Option<i32>,
    pub hrv_balance: Option<i32>,
    pub previous_day_activity: Option<i32>,
    pub previous_night: Option<i32>,
    pub recovery_index: Option<i32>,
    pub resting_heart_rate: Option<i32>,
    pub sleep_balance: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OuraHeartRateResponse {
    pub data: Vec<OuraHeartRateData>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OuraHeartRateData {
    pub bpm: i32,
    pub source: String,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_url_generation() {
        let client = OuraApiClient::new(
            "test_client_id".to_string(),
            "test_secret".to_string(),
            "http://localhost:3000/callback".to_string(),
        )
        .unwrap();

        let state = "random_state_123";
        let url = client.get_authorization_url(state);

        assert!(url.contains("test_client_id"));
        assert!(url.contains(state));
        assert!(url.contains("daily%20heartrate%20workout"));
    }

    #[test]
    fn test_client_creation() {
        let result = OuraApiClient::new(
            "client_id".to_string(),
            "client_secret".to_string(),
            "http://localhost:3000/callback".to_string(),
        );

        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.client_id, "client_id");
        assert_eq!(client.base_url, "https://api.ouraring.com");
    }
}
