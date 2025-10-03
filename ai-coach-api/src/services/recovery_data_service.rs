use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    CreateHrvReadingRequest, CreateRestingHrRequest, CreateSleepDataRequest, DataSource,
    HrvReading, HrvReadingResponse, HrvReadingsListResponse, RecoveryBaseline,
    RecoveryBaselineResponse, RecoveryDataQuery, RestingHrData, RestingHrListResponse,
    RestingHrResponse, SleepData, SleepDataListResponse, SleepDataResponse,
};

pub struct RecoveryDataService {
    db: PgPool,
}

impl RecoveryDataService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    // ========================================================================
    // HRV Operations
    // ========================================================================

    pub async fn create_hrv_reading(
        &self,
        user_id: Uuid,
        request: CreateHrvReadingRequest,
    ) -> Result<HrvReading> {
        let measurement_timestamp = request.measurement_timestamp.unwrap_or_else(Utc::now);
        let measurement_date = measurement_timestamp.date_naive();

        let reading = sqlx::query_as!(
            HrvReading,
            r#"
            INSERT INTO hrv_readings (
                user_id, measurement_date, measurement_timestamp,
                rmssd, sdnn, pnn50, source, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id, user_id, measurement_date, measurement_timestamp,
                rmssd, sdnn, pnn50, source,
                metadata as "metadata: sqlx::types::Json<serde_json::Value>",
                created_at
            "#,
            user_id,
            measurement_date,
            measurement_timestamp,
            request.rmssd,
            request.sdnn,
            request.pnn50,
            DataSource::Manual.as_str(),
            request.metadata.map(sqlx::types::Json)
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to create HRV reading")?;

        Ok(reading)
    }

    pub async fn get_hrv_readings(
        &self,
        user_id: Uuid,
        query: RecoveryDataQuery,
    ) -> Result<HrvReadingsListResponse> {
        let limit = query.limit.unwrap_or(100);
        let page = query.page.unwrap_or(1);
        let offset = (page - 1) * limit;

        let mut sql = String::from(
            r#"
            SELECT
                id, user_id, measurement_date, measurement_timestamp,
                rmssd, sdnn, pnn50, source, metadata, created_at
            FROM hrv_readings
            WHERE user_id = $1
            "#,
        );

        let mut params_count = 1;
        if query.from_date.is_some() {
            params_count += 1;
            sql.push_str(&format!(" AND measurement_date >= ${}", params_count));
        }
        if query.to_date.is_some() {
            params_count += 1;
            sql.push_str(&format!(" AND measurement_date <= ${}", params_count));
        }

        sql.push_str(" ORDER BY measurement_timestamp DESC");
        sql.push_str(&format!(" LIMIT ${} OFFSET ${}", params_count + 1, params_count + 2));

        let mut query_builder = sqlx::query_as::<_, HrvReading>(&sql).bind(user_id);

        if let Some(from_date) = query.from_date {
            query_builder = query_builder.bind(from_date);
        }
        if let Some(to_date) = query.to_date {
            query_builder = query_builder.bind(to_date);
        }

        query_builder = query_builder.bind(limit).bind(offset);

        let readings = query_builder
            .fetch_all(&self.db)
            .await
            .context("Failed to fetch HRV readings")?;

        // Get total count
        let total = self.get_hrv_count(user_id, query.from_date, query.to_date).await?;

        Ok(HrvReadingsListResponse {
            readings: readings.into_iter().map(|r| r.into()).collect(),
            total,
            page,
            page_size: limit,
        })
    }

    async fn get_hrv_count(
        &self,
        user_id: Uuid,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Result<i64> {
        let count = if let (Some(from), Some(to)) = (from_date, to_date) {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM hrv_readings WHERE user_id = $1 AND measurement_date >= $2 AND measurement_date <= $3",
                user_id,
                from,
                to
            )
            .fetch_one(&self.db)
            .await?
        } else if let Some(from) = from_date {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM hrv_readings WHERE user_id = $1 AND measurement_date >= $2",
                user_id,
                from
            )
            .fetch_one(&self.db)
            .await?
        } else if let Some(to) = to_date {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM hrv_readings WHERE user_id = $1 AND measurement_date <= $2",
                user_id,
                to
            )
            .fetch_one(&self.db)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM hrv_readings WHERE user_id = $1",
                user_id
            )
            .fetch_one(&self.db)
            .await?
        };

        Ok(count.unwrap_or(0))
    }

    // ========================================================================
    // Sleep Operations
    // ========================================================================

    pub async fn create_sleep_data(
        &self,
        user_id: Uuid,
        request: CreateSleepDataRequest,
    ) -> Result<SleepData> {
        let sleep_date = request.sleep_date.unwrap_or_else(|| Utc::now().date_naive());

        let data = sqlx::query_as!(
            SleepData,
            r#"
            INSERT INTO sleep_data (
                user_id, sleep_date, total_sleep_hours, deep_sleep_hours,
                rem_sleep_hours, light_sleep_hours, awake_hours,
                sleep_efficiency, sleep_latency_minutes, bedtime, wake_time,
                source, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING
                id, user_id, sleep_date, total_sleep_hours, deep_sleep_hours,
                rem_sleep_hours, light_sleep_hours, awake_hours,
                sleep_efficiency, sleep_latency_minutes, bedtime, wake_time, source,
                metadata as "metadata: sqlx::types::Json<serde_json::Value>",
                created_at
            "#,
            user_id,
            sleep_date,
            request.total_sleep_hours,
            request.deep_sleep_hours,
            request.rem_sleep_hours,
            request.light_sleep_hours,
            request.awake_hours,
            request.sleep_efficiency,
            request.sleep_latency_minutes,
            request.bedtime,
            request.wake_time,
            DataSource::Manual.as_str(),
            request.metadata.map(sqlx::types::Json)
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to create sleep data")?;

        Ok(data)
    }

    pub async fn get_sleep_data(
        &self,
        user_id: Uuid,
        query: RecoveryDataQuery,
    ) -> Result<SleepDataListResponse> {
        let limit = query.limit.unwrap_or(100);
        let page = query.page.unwrap_or(1);
        let offset = (page - 1) * limit;

        let mut sql = String::from(
            r#"
            SELECT
                id, user_id, sleep_date, total_sleep_hours, deep_sleep_hours,
                rem_sleep_hours, light_sleep_hours, awake_hours,
                sleep_efficiency, sleep_latency_minutes, bedtime, wake_time,
                source, metadata, created_at
            FROM sleep_data
            WHERE user_id = $1
            "#,
        );

        let mut params_count = 1;
        if query.from_date.is_some() {
            params_count += 1;
            sql.push_str(&format!(" AND sleep_date >= ${}", params_count));
        }
        if query.to_date.is_some() {
            params_count += 1;
            sql.push_str(&format!(" AND sleep_date <= ${}", params_count));
        }

        sql.push_str(" ORDER BY sleep_date DESC");
        sql.push_str(&format!(" LIMIT ${} OFFSET ${}", params_count + 1, params_count + 2));

        let mut query_builder = sqlx::query_as::<_, SleepData>(&sql).bind(user_id);

        if let Some(from_date) = query.from_date {
            query_builder = query_builder.bind(from_date);
        }
        if let Some(to_date) = query.to_date {
            query_builder = query_builder.bind(to_date);
        }

        query_builder = query_builder.bind(limit).bind(offset);

        let sleep_records = query_builder
            .fetch_all(&self.db)
            .await
            .context("Failed to fetch sleep data")?;

        // Get total count
        let total = self.get_sleep_count(user_id, query.from_date, query.to_date).await?;

        Ok(SleepDataListResponse {
            sleep_records: sleep_records.into_iter().map(|r| r.into()).collect(),
            total,
            page,
            page_size: limit,
        })
    }

    async fn get_sleep_count(
        &self,
        user_id: Uuid,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Result<i64> {
        let count = if let (Some(from), Some(to)) = (from_date, to_date) {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM sleep_data WHERE user_id = $1 AND sleep_date >= $2 AND sleep_date <= $3",
                user_id,
                from,
                to
            )
            .fetch_one(&self.db)
            .await?
        } else if let Some(from) = from_date {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM sleep_data WHERE user_id = $1 AND sleep_date >= $2",
                user_id,
                from
            )
            .fetch_one(&self.db)
            .await?
        } else if let Some(to) = to_date {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM sleep_data WHERE user_id = $1 AND sleep_date <= $2",
                user_id,
                to
            )
            .fetch_one(&self.db)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM sleep_data WHERE user_id = $1",
                user_id
            )
            .fetch_one(&self.db)
            .await?
        };

        Ok(count.unwrap_or(0))
    }

    // ========================================================================
    // Resting HR Operations
    // ========================================================================

    pub async fn create_resting_hr(
        &self,
        user_id: Uuid,
        request: CreateRestingHrRequest,
    ) -> Result<RestingHrData> {
        let measurement_timestamp = request.measurement_timestamp.unwrap_or_else(Utc::now);
        let measurement_date = measurement_timestamp.date_naive();

        let data = sqlx::query_as!(
            RestingHrData,
            r#"
            INSERT INTO resting_hr_data (
                user_id, measurement_date, measurement_timestamp,
                resting_hr, source, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id, user_id, measurement_date, measurement_timestamp,
                resting_hr, source,
                metadata as "metadata: sqlx::types::Json<serde_json::Value>",
                created_at
            "#,
            user_id,
            measurement_date,
            measurement_timestamp,
            request.resting_hr,
            DataSource::Manual.as_str(),
            request.metadata.map(sqlx::types::Json)
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to create resting HR data")?;

        Ok(data)
    }

    pub async fn get_resting_hr_data(
        &self,
        user_id: Uuid,
        query: RecoveryDataQuery,
    ) -> Result<RestingHrListResponse> {
        let limit = query.limit.unwrap_or(100);
        let page = query.page.unwrap_or(1);
        let offset = (page - 1) * limit;

        let mut sql = String::from(
            r#"
            SELECT
                id, user_id, measurement_date, measurement_timestamp,
                resting_hr, source, metadata, created_at
            FROM resting_hr_data
            WHERE user_id = $1
            "#,
        );

        let mut params_count = 1;
        if query.from_date.is_some() {
            params_count += 1;
            sql.push_str(&format!(" AND measurement_date >= ${}", params_count));
        }
        if query.to_date.is_some() {
            params_count += 1;
            sql.push_str(&format!(" AND measurement_date <= ${}", params_count));
        }

        sql.push_str(" ORDER BY measurement_timestamp DESC");
        sql.push_str(&format!(" LIMIT ${} OFFSET ${}", params_count + 1, params_count + 2));

        let mut query_builder = sqlx::query_as::<_, RestingHrData>(&sql).bind(user_id);

        if let Some(from_date) = query.from_date {
            query_builder = query_builder.bind(from_date);
        }
        if let Some(to_date) = query.to_date {
            query_builder = query_builder.bind(to_date);
        }

        query_builder = query_builder.bind(limit).bind(offset);

        let readings = query_builder
            .fetch_all(&self.db)
            .await
            .context("Failed to fetch resting HR data")?;

        // Get total count
        let total = self.get_resting_hr_count(user_id, query.from_date, query.to_date).await?;

        Ok(RestingHrListResponse {
            readings: readings.into_iter().map(|r| r.into()).collect(),
            total,
            page,
            page_size: limit,
        })
    }

    async fn get_resting_hr_count(
        &self,
        user_id: Uuid,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Result<i64> {
        let count = if let (Some(from), Some(to)) = (from_date, to_date) {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM resting_hr_data WHERE user_id = $1 AND measurement_date >= $2 AND measurement_date <= $3",
                user_id,
                from,
                to
            )
            .fetch_one(&self.db)
            .await?
        } else if let Some(from) = from_date {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM resting_hr_data WHERE user_id = $1 AND measurement_date >= $2",
                user_id,
                from
            )
            .fetch_one(&self.db)
            .await?
        } else if let Some(to) = to_date {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM resting_hr_data WHERE user_id = $1 AND measurement_date <= $2",
                user_id,
                to
            )
            .fetch_one(&self.db)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM resting_hr_data WHERE user_id = $1",
                user_id
            )
            .fetch_one(&self.db)
            .await?
        };

        Ok(count.unwrap_or(0))
    }

    // ========================================================================
    // Baseline Operations
    // ========================================================================

    pub async fn get_or_calculate_baseline(
        &self,
        user_id: Uuid,
    ) -> Result<Option<RecoveryBaselineResponse>> {
        // Try to get existing baseline
        if let Some(baseline) = self.get_baseline(user_id).await? {
            return Ok(Some(baseline.into()));
        }

        // Calculate new baseline
        self.calculate_baseline(user_id).await?;

        // Return the newly calculated baseline
        Ok(self.get_baseline(user_id).await?.map(|b| b.into()))
    }

    async fn get_baseline(&self, user_id: Uuid) -> Result<Option<RecoveryBaseline>> {
        let baseline = sqlx::query_as!(
            RecoveryBaseline,
            r#"
            SELECT id, user_id, hrv_baseline_rmssd, rhr_baseline,
                   typical_sleep_hours, calculated_at, data_points_count,
                   created_at, updated_at
            FROM recovery_baselines
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch recovery baseline")?;

        Ok(baseline)
    }

    pub async fn calculate_baseline(&self, user_id: Uuid) -> Result<()> {
        let thirty_days_ago = (Utc::now() - chrono::Duration::days(30)).date_naive();

        // Calculate HRV baseline (30-day average)
        let hrv_baseline = sqlx::query_scalar!(
            "SELECT AVG(rmssd) FROM hrv_readings WHERE user_id = $1 AND measurement_date >= $2",
            user_id,
            thirty_days_ago
        )
        .fetch_one(&self.db)
        .await?;

        // Calculate RHR baseline (30-day average)
        let rhr_baseline = sqlx::query_scalar!(
            "SELECT AVG(resting_hr) FROM resting_hr_data WHERE user_id = $1 AND measurement_date >= $2",
            user_id,
            thirty_days_ago
        )
        .fetch_one(&self.db)
        .await?;

        // Calculate typical sleep hours (30-day average)
        let typical_sleep = sqlx::query_scalar!(
            "SELECT AVG(total_sleep_hours) FROM sleep_data WHERE user_id = $1 AND sleep_date >= $2",
            user_id,
            thirty_days_ago
        )
        .fetch_one(&self.db)
        .await?;

        // Count data points
        let data_points_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(DISTINCT measurement_date) as "count!"
            FROM (
                SELECT measurement_date FROM hrv_readings WHERE user_id = $1 AND measurement_date >= $2
                UNION
                SELECT measurement_date FROM resting_hr_data WHERE user_id = $1 AND measurement_date >= $2
                UNION
                SELECT sleep_date as measurement_date FROM sleep_data WHERE user_id = $1 AND sleep_date >= $2
            ) dates
            "#,
            user_id,
            thirty_days_ago
        )
        .fetch_one(&self.db)
        .await?;

        // Insert or update baseline
        sqlx::query!(
            r#"
            INSERT INTO recovery_baselines (
                user_id, hrv_baseline_rmssd, rhr_baseline, typical_sleep_hours,
                calculated_at, data_points_count
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id)
            DO UPDATE SET
                hrv_baseline_rmssd = EXCLUDED.hrv_baseline_rmssd,
                rhr_baseline = EXCLUDED.rhr_baseline,
                typical_sleep_hours = EXCLUDED.typical_sleep_hours,
                calculated_at = EXCLUDED.calculated_at,
                data_points_count = EXCLUDED.data_points_count,
                updated_at = NOW()
            "#,
            user_id,
            hrv_baseline,
            rhr_baseline,
            typical_sleep,
            Utc::now(),
            data_points_count as i32
        )
        .execute(&self.db)
        .await
        .context("Failed to calculate recovery baseline")?;

        Ok(())
    }
}
