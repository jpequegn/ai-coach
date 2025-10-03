use anyhow::{Context, Result};
use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{DataSource, HrvReading, RestingHrData, SleepData, WearableConnection};
use crate::services::oura_api_client::{OuraApiClient, OuraTokenResponse};

pub struct OuraIntegrationService {
    db: PgPool,
    api_client: OuraApiClient,
}

impl OuraIntegrationService {
    pub fn new(
        db: PgPool,
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> Result<Self> {
        let api_client = OuraApiClient::new(client_id, client_secret, redirect_uri)?;

        Ok(Self { db, api_client })
    }

    // ========================================================================
    // OAuth Connection Management
    // ========================================================================

    /// Get OAuth authorization URL
    pub fn get_authorization_url(&self, user_id: Uuid) -> String {
        let state = format!("{}:oura", user_id);
        self.api_client.get_authorization_url(&state)
    }

    /// Handle OAuth callback and store connection
    pub async fn handle_oauth_callback(
        &self,
        user_id: Uuid,
        code: &str,
    ) -> Result<WearableConnection> {
        // Exchange code for tokens
        let token_response = self
            .api_client
            .exchange_code_for_token(code)
            .await
            .context("Failed to exchange authorization code")?;

        // Calculate token expiry
        let token_expires_at = Utc::now() + Duration::seconds(token_response.expires_in);

        // Store connection in database
        let connection = sqlx::query_as!(
            WearableConnection,
            r#"
            INSERT INTO wearable_connections (
                user_id, provider, access_token, refresh_token,
                token_expires_at, is_active
            )
            VALUES ($1, $2, $3, $4, $5, TRUE)
            ON CONFLICT (user_id, provider)
            DO UPDATE SET
                access_token = EXCLUDED.access_token,
                refresh_token = EXCLUDED.refresh_token,
                token_expires_at = EXCLUDED.token_expires_at,
                is_active = TRUE,
                updated_at = NOW()
            RETURNING
                id, user_id, provider, access_token, refresh_token,
                token_expires_at, provider_user_id, scopes, connected_at,
                last_sync_at, is_active,
                metadata as "metadata: sqlx::types::Json<serde_json::Value>",
                created_at, updated_at
            "#,
            user_id,
            DataSource::Oura.as_str(),
            token_response.access_token,
            token_response.refresh_token,
            token_expires_at
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to store Oura connection")?;

        tracing::info!("Oura connected for user {}", user_id);

        Ok(connection)
    }

    /// Refresh token if expired
    async fn ensure_valid_token(&self, connection: &mut WearableConnection) -> Result<()> {
        if let Some(expires_at) = connection.token_expires_at {
            if Utc::now() + Duration::minutes(5) >= expires_at {
                // Token expires soon, refresh it
                let refresh_token = connection
                    .refresh_token
                    .as_ref()
                    .context("No refresh token available")?;

                let token_response = self
                    .api_client
                    .refresh_access_token(refresh_token)
                    .await
                    .context("Failed to refresh token")?;

                let new_expires_at = Utc::now() + Duration::seconds(token_response.expires_in);

                // Update connection with new tokens
                sqlx::query!(
                    r#"
                    UPDATE wearable_connections
                    SET access_token = $1, refresh_token = $2,
                        token_expires_at = $3, updated_at = NOW()
                    WHERE id = $4
                    "#,
                    token_response.access_token,
                    token_response.refresh_token,
                    new_expires_at,
                    connection.id
                )
                .execute(&self.db)
                .await
                .context("Failed to update tokens")?;

                connection.access_token = Some(token_response.access_token);
                connection.refresh_token = Some(token_response.refresh_token);
                connection.token_expires_at = Some(new_expires_at);

                tracing::info!("Refreshed Oura token for user {}", connection.user_id);
            }
        }

        Ok(())
    }

    // ========================================================================
    // Data Synchronization
    // ========================================================================

    /// Sync all Oura data for a user
    pub async fn sync_user_data(&self, user_id: Uuid, days_back: i64) -> Result<SyncResult> {
        let mut connection = self.get_oura_connection(user_id).await?;
        self.ensure_valid_token(&mut connection).await?;

        let access_token = connection
            .access_token
            .as_ref()
            .context("No access token available")?;

        let end_date = Utc::now().date_naive();
        let start_date = end_date - Duration::days(days_back);

        let mut result = SyncResult::default();

        // Sync sleep data
        match self.sync_sleep_data(user_id, access_token, start_date, end_date).await {
            Ok(count) => result.sleep_records = count,
            Err(e) => {
                result.errors.push(format!("Sleep sync error: {}", e));
                tracing::error!("Failed to sync Oura sleep data: {}", e);
            }
        }

        // Sync HRV data
        match self.sync_hrv_data(user_id, access_token, start_date, end_date).await {
            Ok(count) => result.hrv_readings = count,
            Err(e) => {
                result.errors.push(format!("HRV sync error: {}", e));
                tracing::error!("Failed to sync Oura HRV data: {}", e);
            }
        }

        // Sync resting HR data
        match self.sync_resting_hr_data(user_id, access_token, start_date, end_date).await {
            Ok(count) => result.rhr_readings = count,
            Err(e) => {
                result.errors.push(format!("RHR sync error: {}", e));
                tracing::error!("Failed to sync Oura RHR data: {}", e);
            }
        }

        // Update last sync timestamp
        sqlx::query!(
            "UPDATE wearable_connections SET last_sync_at = NOW() WHERE id = $1",
            connection.id
        )
        .execute(&self.db)
        .await?;

        tracing::info!(
            "Oura sync completed for user {}: {} sleep, {} HRV, {} RHR",
            user_id,
            result.sleep_records,
            result.hrv_readings,
            result.rhr_readings
        );

        Ok(result)
    }

    async fn sync_sleep_data(
        &self,
        user_id: Uuid,
        access_token: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<usize> {
        let sleep_data = self
            .api_client
            .get_sleep_data(access_token, start_date, end_date)
            .await?;

        let mut synced_count = 0;

        for data in sleep_data {
            let total_sleep_hours = data.total_sleep_duration.map(|d| d as f64 / 3600.0);
            let deep_sleep_hours = data.deep_sleep_duration.map(|d| d as f64 / 3600.0);
            let rem_sleep_hours = data.rem_sleep_duration.map(|d| d as f64 / 3600.0);
            let light_sleep_hours = data.light_sleep_duration.map(|d| d as f64 / 3600.0);
            let awake_hours = data.awake_time.map(|d| d as f64 / 3600.0);

            if let Some(total) = total_sleep_hours {
                let result = sqlx::query!(
                    r#"
                    INSERT INTO sleep_data (
                        user_id, sleep_date, total_sleep_hours, deep_sleep_hours,
                        rem_sleep_hours, light_sleep_hours, awake_hours,
                        sleep_efficiency, sleep_latency_minutes, bedtime, wake_time,
                        source, metadata
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                    ON CONFLICT (user_id, sleep_date, source) DO NOTHING
                    "#,
                    user_id,
                    data.day,
                    total,
                    deep_sleep_hours,
                    rem_sleep_hours,
                    light_sleep_hours,
                    awake_hours,
                    data.efficiency.map(|e| e * 100.0), // Convert to percentage
                    data.latency.map(|l| l / 60), // Convert to minutes
                    data.bedtime_start,
                    data.bedtime_end,
                    DataSource::Oura.as_str(),
                    sqlx::types::Json(serde_json::json!({
                        "oura_sleep_id": data.id,
                        "score": data.score,
                        "average_hrv": data.average_hrv,
                        "average_heart_rate": data.average_heart_rate
                    }))
                )
                .execute(&self.db)
                .await?;

                if result.rows_affected() > 0 {
                    synced_count += 1;
                }
            }
        }

        Ok(synced_count)
    }

    async fn sync_hrv_data(
        &self,
        user_id: Uuid,
        access_token: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<usize> {
        let readiness_data = self
            .api_client
            .get_readiness_data(access_token, start_date, end_date)
            .await?;

        let mut synced_count = 0;

        for data in readiness_data {
            // Oura provides HRV balance score, we need to derive RMSSD
            // For now, we'll use readiness score as a proxy
            if let Some(hrv_balance) = data.contributors.hrv_balance {
                let rmssd = hrv_balance as f64; // This is a simplified mapping

                let result = sqlx::query!(
                    r#"
                    INSERT INTO hrv_readings (
                        user_id, measurement_date, measurement_timestamp,
                        rmssd, source, metadata
                    )
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (user_id, measurement_timestamp, source) DO NOTHING
                    "#,
                    user_id,
                    data.day,
                    data.timestamp,
                    rmssd,
                    DataSource::Oura.as_str(),
                    sqlx::types::Json(serde_json::json!({
                        "oura_readiness_id": data.id,
                        "readiness_score": data.score,
                        "hrv_balance": hrv_balance,
                        "temperature_deviation": data.temperature_deviation
                    }))
                )
                .execute(&self.db)
                .await?;

                if result.rows_affected() > 0 {
                    synced_count += 1;
                }
            }
        }

        Ok(synced_count)
    }

    async fn sync_resting_hr_data(
        &self,
        user_id: Uuid,
        access_token: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<usize> {
        let hr_data = self
            .api_client
            .get_heart_rate_data(access_token, start_date, end_date)
            .await?;

        let mut synced_count = 0;

        // Group by day and find minimum (resting) HR
        use std::collections::HashMap;
        let mut daily_min: HashMap<NaiveDate, (i32, chrono::DateTime<Utc>)> = HashMap::new();

        for reading in hr_data {
            let date = reading.timestamp.date_naive();
            daily_min
                .entry(date)
                .and_modify(|(min_bpm, _)| {
                    if reading.bpm < *min_bpm {
                        *min_bpm = reading.bpm;
                    }
                })
                .or_insert((reading.bpm, reading.timestamp));
        }

        for (date, (resting_hr, timestamp)) in daily_min {
            let result = sqlx::query!(
                r#"
                INSERT INTO resting_hr_data (
                    user_id, measurement_date, measurement_timestamp,
                    resting_hr, source, metadata
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (user_id, measurement_timestamp, source) DO NOTHING
                "#,
                user_id,
                date,
                timestamp,
                resting_hr as f64,
                DataSource::Oura.as_str(),
                sqlx::types::Json(serde_json::json!({
                    "derived_from_daily_minimum": true
                }))
            )
            .execute(&self.db)
            .await?;

            if result.rows_affected() > 0 {
                synced_count += 1;
            }
        }

        Ok(synced_count)
    }

    async fn get_oura_connection(&self, user_id: Uuid) -> Result<WearableConnection> {
        sqlx::query_as!(
            WearableConnection,
            r#"
            SELECT
                id, user_id, provider, access_token, refresh_token,
                token_expires_at, provider_user_id, scopes, connected_at,
                last_sync_at, is_active,
                metadata as "metadata: sqlx::types::Json<serde_json::Value>",
                created_at, updated_at
            FROM wearable_connections
            WHERE user_id = $1 AND provider = $2 AND is_active = TRUE
            "#,
            user_id,
            DataSource::Oura.as_str()
        )
        .fetch_one(&self.db)
        .await
        .context("Oura connection not found for user")
    }

    /// Disconnect Oura
    pub async fn disconnect(&self, user_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE wearable_connections
            SET is_active = FALSE, updated_at = NOW()
            WHERE user_id = $1 AND provider = $2
            "#,
            user_id,
            DataSource::Oura.as_str()
        )
        .execute(&self.db)
        .await?;

        tracing::info!("Oura disconnected for user {}", user_id);

        Ok(())
    }
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct SyncResult {
    pub sleep_records: usize,
    pub hrv_readings: usize,
    pub rhr_readings: usize,
    pub errors: Vec<String>,
}
