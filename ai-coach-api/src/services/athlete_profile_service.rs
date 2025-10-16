use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{AthleteProfile, CreateAthleteProfile, UpdateAthleteProfile};

#[derive(Clone)]
pub struct AthleteProfileService {
    db: SqlitePool,
}

impl AthleteProfileService {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn create_profile(&self, profile_data: CreateAthleteProfile) -> Result<AthleteProfile> {
        let now = Utc::now();
        let zones_json = profile_data.zones.map(|v| v.to_string());

        let profile = sqlx::query_as::<_, AthleteProfile>(
            r#"
            INSERT INTO athlete_profiles (user_id, sport, ftp, lthr, max_heart_rate, threshold_pace, zones, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
            RETURNING id, user_id, sport, ftp, lthr, max_heart_rate, threshold_pace, zones, created_at, updated_at
            "#
        )
        .bind(&profile_data.user_id)
        .bind(&profile_data.sport)
        .bind(profile_data.ftp)
        .bind(profile_data.lthr)
        .bind(profile_data.max_heart_rate)
        .bind(profile_data.threshold_pace)
        .bind(zones_json)
        .bind(now)
        .fetch_one(&self.db)
        .await?;

        Ok(profile)
    }

    pub async fn get_profile_by_id(&self, profile_id: Uuid) -> Result<Option<AthleteProfile>> {
        let profile = sqlx::query_as::<_, AthleteProfile>(
            "SELECT id, user_id, sport, ftp, lthr, max_heart_rate, threshold_pace, zones, created_at, updated_at FROM athlete_profiles WHERE id = $1"
        )
        .bind(&profile_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(profile)
    }

    pub async fn get_profile_by_user_id(&self, user_id: Uuid) -> Result<Option<AthleteProfile>> {
        let profile = sqlx::query_as::<_, AthleteProfile>(
            "SELECT id, user_id, sport, ftp, lthr, max_heart_rate, threshold_pace, zones, created_at, updated_at FROM athlete_profiles WHERE user_id = $1"
        )
        .bind(&user_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(profile)
    }

    pub async fn get_profiles_by_sport(&self, sport: &str) -> Result<Vec<AthleteProfile>> {
        let profiles = sqlx::query_as::<_, AthleteProfile>(
            "SELECT id, user_id, sport, ftp, lthr, max_heart_rate, threshold_pace, zones, created_at, updated_at FROM athlete_profiles WHERE sport = $1"
        )
        .bind(sport)
        .fetch_all(&self.db)
        .await?;

        Ok(profiles)
    }

    pub async fn update_profile(&self, profile_id: Uuid, profile_data: UpdateAthleteProfile) -> Result<Option<AthleteProfile>> {
        let now = Utc::now();
        let zones_json = profile_data.zones.map(|v| v.to_string());

        let profile = sqlx::query_as::<_, AthleteProfile>(
            r#"
            UPDATE athlete_profiles
            SET sport = COALESCE($2, sport),
                ftp = COALESCE($3, ftp),
                lthr = COALESCE($4, lthr),
                max_heart_rate = COALESCE($5, max_heart_rate),
                threshold_pace = COALESCE($6, threshold_pace),
                zones = COALESCE($7, zones),
                updated_at = $8
            WHERE id = $1
            RETURNING id, user_id, sport, ftp, lthr, max_heart_rate, threshold_pace, zones, created_at, updated_at
            "#
        )
        .bind(&profile_id)
        .bind(&profile_data.sport)
        .bind(profile_data.ftp)
        .bind(profile_data.lthr)
        .bind(profile_data.max_heart_rate)
        .bind(profile_data.threshold_pace)
        .bind(zones_json)
        .bind(now)
        .fetch_optional(&self.db)
        .await?;

        Ok(profile)
    }

    pub async fn delete_profile(&self, profile_id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM athlete_profiles WHERE id = $1")
            .bind(&profile_id)
            .execute(&self.db)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list_profiles(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<AthleteProfile>> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        let profiles = sqlx::query_as::<_, AthleteProfile>(
            "SELECT id, user_id, sport, ftp, lthr, max_heart_rate, threshold_pace, zones, created_at, updated_at FROM athlete_profiles ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        Ok(profiles)
    }
}