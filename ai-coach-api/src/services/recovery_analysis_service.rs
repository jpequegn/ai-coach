use anyhow::{Context, Result};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    HrvReading, HrvTrend, KeyFactor, RecoveryBaseline, RecoveryDataPoint, RecoveryInsight,
    RecoveryInsightsResponse, RecoveryPattern, RecoveryScore, RecoveryStatus,
    RecoveryStatusResponse, RecoveryTrendsResponse, Recommendation, RestingHrData, SleepData,
};

const MODEL_VERSION: &str = "1.0.0-simple";

pub struct RecoveryAnalysisService {
    db: PgPool,
}

impl RecoveryAnalysisService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Calculate daily recovery score for a user
    pub async fn calculate_daily_recovery(
        &self,
        user_id: Uuid,
        target_date: NaiveDate,
    ) -> Result<RecoveryScore> {
        // Fetch recent data
        let hrv_data = self.fetch_recent_hrv(user_id, target_date, 7).await?;
        let sleep_data = self.fetch_recent_sleep(user_id, target_date, 3).await?;
        let rhr_data = self.fetch_recent_rhr(user_id, target_date, 7).await?;
        let baseline = self.fetch_baseline(user_id).await?;

        // Calculate HRV trend and deviation
        let (hrv_trend, hrv_deviation) = self.calculate_hrv_metrics(&hrv_data, &baseline);

        // Calculate sleep quality score
        let sleep_quality_score = self.calculate_sleep_quality(&sleep_data);

        // Calculate RHR deviation
        let rhr_deviation = self.calculate_rhr_deviation(&rhr_data, &baseline);

        // Calculate recovery adequacy (simple formula)
        let recovery_adequacy = self.calculate_recovery_adequacy(
            hrv_deviation,
            sleep_quality_score,
            rhr_deviation,
        );

        // Calculate overall readiness score
        let readiness_score = self.calculate_readiness_score(
            hrv_deviation,
            sleep_quality_score,
            rhr_deviation,
            recovery_adequacy,
        );

        // Determine recovery status
        let recovery_status = RecoveryStatus::from_score(readiness_score);

        // Calculate recommended TSS adjustment
        let recommended_tss_adjustment = self.calculate_tss_adjustment(readiness_score);

        // Create or update recovery score
        let score = self
            .upsert_recovery_score(
                user_id,
                target_date,
                readiness_score,
                hrv_trend.clone(),
                hrv_deviation,
                sleep_quality_score,
                recovery_adequacy,
                rhr_deviation,
                None, // training_strain - requires training data
                recovery_status.clone(),
                recommended_tss_adjustment,
            )
            .await?;

        Ok(score)
    }

    /// Get current recovery status for a user
    pub async fn get_recovery_status(&self, user_id: Uuid) -> Result<Option<RecoveryStatusResponse>> {
        let today = Utc::now().date_naive();

        // Try to get today's score, or calculate it if missing
        let score = match self.get_recovery_score(user_id, today).await? {
            Some(s) => s,
            None => self.calculate_daily_recovery(user_id, today).await?,
        };

        let recommendations = self.generate_recommendations(&score).await?;

        Ok(Some(RecoveryStatusResponse {
            date: score.score_date,
            readiness_score: score.readiness_score,
            recovery_status: score.recovery_status,
            hrv_trend: score.hrv_trend,
            hrv_deviation: score.hrv_deviation,
            sleep_quality: score.sleep_quality_score,
            recovery_adequacy: score.recovery_adequacy,
            rhr_deviation: score.rhr_deviation,
            recommended_tss_adjustment: score.recommended_tss_adjustment,
            recommendations,
        }))
    }

    /// Get recovery trends over a period
    pub async fn get_recovery_trends(
        &self,
        user_id: Uuid,
        period_days: i32,
    ) -> Result<RecoveryTrendsResponse> {
        let end_date = Utc::now().date_naive();
        let start_date = end_date - Duration::days(period_days as i64);

        let scores = self.get_recovery_scores_range(user_id, start_date, end_date).await?;

        if scores.is_empty() {
            return Ok(RecoveryTrendsResponse {
                period_days,
                average_readiness: 0.0,
                trend_direction: "insufficient_data".to_string(),
                data_points: vec![],
                patterns: vec![],
            });
        }

        let average_readiness = scores.iter().map(|s| s.readiness_score).sum::<f64>() / scores.len() as f64;

        let trend_direction = self.determine_trend_direction(&scores);

        let data_points = scores
            .iter()
            .map(|s| RecoveryDataPoint {
                date: s.score_date,
                readiness_score: s.readiness_score,
                recovery_status: s.recovery_status.clone(),
            })
            .collect();

        let patterns = self.detect_patterns(&scores).await?;

        Ok(RecoveryTrendsResponse {
            period_days,
            average_readiness,
            trend_direction,
            data_points,
            patterns,
        })
    }

    /// Get recovery insights for a user
    pub async fn get_recovery_insights(&self, user_id: Uuid) -> Result<RecoveryInsightsResponse> {
        let current_score = match self.get_recovery_status(user_id).await? {
            Some(s) => s,
            None => {
                return Ok(RecoveryInsightsResponse {
                    insights: vec![],
                    key_factors: vec![],
                    suggestions: vec!["Insufficient data to generate insights. Please log recovery data regularly.".to_string()],
                });
            }
        };

        let baseline = self.fetch_baseline(user_id).await?;

        let mut insights = Vec::new();
        let mut key_factors = Vec::new();
        let mut suggestions = Vec::new();

        // HRV insights
        if let Some(hrv_dev) = current_score.hrv_deviation {
            if let Some(baseline_hrv) = baseline.hrv_baseline_rmssd {
                key_factors.push(KeyFactor {
                    factor: "HRV".to_string(),
                    current_value: baseline_hrv * (1.0 + hrv_dev / 100.0),
                    baseline_value: baseline_hrv,
                    deviation_percent: hrv_dev,
                });

                if current_score.hrv_trend == "declining" {
                    insights.push(RecoveryInsight {
                        category: "HRV".to_string(),
                        title: "Declining Heart Rate Variability".to_string(),
                        description: "Your HRV has been trending downward, indicating increased stress or fatigue.".to_string(),
                        impact: "negative".to_string(),
                    });
                    suggestions.push("Consider reducing training intensity and prioritizing recovery activities.".to_string());
                }
            }
        }

        // Sleep insights
        if let Some(sleep_quality) = current_score.sleep_quality {
            if sleep_quality < 70.0 {
                insights.push(RecoveryInsight {
                    category: "Sleep".to_string(),
                    title: "Below Optimal Sleep Quality".to_string(),
                    description: format!("Your sleep quality score is {:.1}/100, which is below optimal.", sleep_quality),
                    impact: "negative".to_string(),
                });
                suggestions.push("Aim for 7-9 hours of quality sleep. Consider improving sleep hygiene.".to_string());
            }
        }

        // Overall recovery insights
        if current_score.readiness_score < 50.0 {
            insights.push(RecoveryInsight {
                category: "Recovery".to_string(),
                title: "Poor Recovery Status".to_string(),
                description: "Your overall recovery is below optimal. Consider taking a rest day.".to_string(),
                impact: "negative".to_string(),
            });
            suggestions.push("Take a complete rest day or engage in very light active recovery.".to_string());
        } else if current_score.readiness_score >= 85.0 {
            insights.push(RecoveryInsight {
                category: "Recovery".to_string(),
                title: "Excellent Recovery".to_string(),
                description: "Your recovery is optimal. You're ready for high-intensity training.".to_string(),
                impact: "positive".to_string(),
            });
            suggestions.push("This is a good day for high-intensity or long-duration training.".to_string());
        }

        Ok(RecoveryInsightsResponse {
            insights,
            key_factors,
            suggestions,
        })
    }

    // ========================================================================
    // Private Helper Methods
    // ========================================================================

    async fn fetch_recent_hrv(
        &self,
        user_id: Uuid,
        target_date: NaiveDate,
        days: i64,
    ) -> Result<Vec<HrvReading>> {
        let start_date = target_date - Duration::days(days);

        let readings = sqlx::query_as!(
            HrvReading,
            r#"
            SELECT
                id, user_id, measurement_date, measurement_timestamp,
                rmssd, sdnn, pnn50, source,
                metadata as "metadata: sqlx::types::Json<serde_json::Value>",
                created_at
            FROM hrv_readings
            WHERE user_id = $1 AND measurement_date >= $2 AND measurement_date <= $3
            ORDER BY measurement_date DESC
            "#,
            user_id,
            start_date,
            target_date
        )
        .fetch_all(&self.db)
        .await?;

        Ok(readings)
    }

    async fn fetch_recent_sleep(
        &self,
        user_id: Uuid,
        target_date: NaiveDate,
        days: i64,
    ) -> Result<Vec<SleepData>> {
        let start_date = target_date - Duration::days(days);

        let data = sqlx::query_as!(
            SleepData,
            r#"
            SELECT
                id, user_id, sleep_date, total_sleep_hours, deep_sleep_hours,
                rem_sleep_hours, light_sleep_hours, awake_hours,
                sleep_efficiency, sleep_latency_minutes, bedtime, wake_time, source,
                metadata as "metadata: sqlx::types::Json<serde_json::Value>",
                created_at
            FROM sleep_data
            WHERE user_id = $1 AND sleep_date >= $2 AND sleep_date <= $3
            ORDER BY sleep_date DESC
            "#,
            user_id,
            start_date,
            target_date
        )
        .fetch_all(&self.db)
        .await?;

        Ok(data)
    }

    async fn fetch_recent_rhr(
        &self,
        user_id: Uuid,
        target_date: NaiveDate,
        days: i64,
    ) -> Result<Vec<RestingHrData>> {
        let start_date = target_date - Duration::days(days);

        let data = sqlx::query_as!(
            RestingHrData,
            r#"
            SELECT
                id, user_id, measurement_date, measurement_timestamp,
                resting_hr, source,
                metadata as "metadata: sqlx::types::Json<serde_json::Value>",
                created_at
            FROM resting_hr_data
            WHERE user_id = $1 AND measurement_date >= $2 AND measurement_date <= $3
            ORDER BY measurement_date DESC
            "#,
            user_id,
            start_date,
            target_date
        )
        .fetch_all(&self.db)
        .await?;

        Ok(data)
    }

    async fn fetch_baseline(&self, user_id: Uuid) -> Result<RecoveryBaseline> {
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
        .await?
        .unwrap_or_else(|| RecoveryBaseline {
            id: Uuid::new_v4(),
            user_id,
            hrv_baseline_rmssd: None,
            rhr_baseline: None,
            typical_sleep_hours: None,
            calculated_at: Utc::now(),
            data_points_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        Ok(baseline)
    }

    fn calculate_hrv_metrics(
        &self,
        hrv_data: &[HrvReading],
        baseline: &RecoveryBaseline,
    ) -> (HrvTrend, Option<f64>) {
        if hrv_data.is_empty() {
            return (HrvTrend::InsufficientData, None);
        }

        let recent_avg = hrv_data.iter().map(|h| h.rmssd).sum::<f64>() / hrv_data.len() as f64;

        let deviation = if let Some(baseline_hrv) = baseline.hrv_baseline_rmssd {
            Some(((recent_avg - baseline_hrv) / baseline_hrv) * 100.0)
        } else {
            None
        };

        // Simple trend detection: compare first half to second half
        let trend = if hrv_data.len() >= 4 {
            let mid = hrv_data.len() / 2;
            let recent_half: Vec<_> = hrv_data[..mid].to_vec();
            let older_half: Vec<_> = hrv_data[mid..].to_vec();

            let recent_avg_half = recent_half.iter().map(|h| h.rmssd).sum::<f64>() / recent_half.len() as f64;
            let older_avg_half = older_half.iter().map(|h| h.rmssd).sum::<f64>() / older_half.len() as f64;

            let change_percent = ((recent_avg_half - older_avg_half) / older_avg_half) * 100.0;

            if change_percent > 5.0 {
                HrvTrend::Improving
            } else if change_percent < -5.0 {
                HrvTrend::Declining
            } else {
                HrvTrend::Stable
            }
        } else {
            HrvTrend::Stable
        };

        (trend, deviation)
    }

    fn calculate_sleep_quality(&self, sleep_data: &[SleepData]) -> Option<f64> {
        if sleep_data.is_empty() {
            return None;
        }

        let recent = &sleep_data[0];
        let mut score = 50.0; // Base score

        // Hours score (optimal 7-9 hours)
        if recent.total_sleep_hours >= 7.0 && recent.total_sleep_hours <= 9.0 {
            score += 25.0;
        } else if recent.total_sleep_hours >= 6.0 && recent.total_sleep_hours <= 10.0 {
            score += 15.0;
        }

        // Efficiency score
        if let Some(efficiency) = recent.sleep_efficiency {
            score += (efficiency / 100.0) * 25.0;
        }

        Some(score.min(100.0))
    }

    fn calculate_rhr_deviation(
        &self,
        rhr_data: &[RestingHrData],
        baseline: &RecoveryBaseline,
    ) -> Option<f64> {
        if rhr_data.is_empty() {
            return None;
        }

        let recent_avg = rhr_data.iter().map(|r| r.resting_hr).sum::<f64>() / rhr_data.len() as f64;

        if let Some(baseline_rhr) = baseline.rhr_baseline {
            Some(((recent_avg - baseline_rhr) / baseline_rhr) * 100.0)
        } else {
            None
        }
    }

    fn calculate_recovery_adequacy(
        &self,
        hrv_deviation: Option<f64>,
        sleep_quality: Option<f64>,
        rhr_deviation: Option<f64>,
    ) -> Option<f64> {
        let mut components = 0;
        let mut total_score = 0.0;

        if let Some(hrv_dev) = hrv_deviation {
            components += 1;
            // Positive HRV deviation is good
            total_score += 50.0 + (hrv_dev * 0.5).min(50.0).max(-50.0);
        }

        if let Some(sleep) = sleep_quality {
            components += 1;
            total_score += sleep;
        }

        if let Some(rhr_dev) = rhr_deviation {
            components += 1;
            // Negative RHR deviation is good (lower resting HR)
            total_score += 50.0 - (rhr_dev * 0.5).min(50.0).max(-50.0);
        }

        if components > 0 {
            Some((total_score / components as f64).max(0.0).min(100.0))
        } else {
            None
        }
    }

    fn calculate_readiness_score(
        &self,
        hrv_deviation: Option<f64>,
        sleep_quality: Option<f64>,
        rhr_deviation: Option<f64>,
        recovery_adequacy: Option<f64>,
    ) -> f64 {
        if let Some(adequacy) = recovery_adequacy {
            adequacy
        } else if let Some(sleep) = sleep_quality {
            sleep
        } else {
            50.0 // Default neutral score
        }
    }

    fn calculate_tss_adjustment(&self, readiness_score: f64) -> Option<f64> {
        let adjustment = if readiness_score >= 85.0 {
            1.1 // Increase by 10%
        } else if readiness_score >= 70.0 {
            1.0 // No change
        } else if readiness_score >= 50.0 {
            0.9 // Reduce by 10%
        } else if readiness_score >= 30.0 {
            0.7 // Reduce by 30%
        } else {
            0.5 // Reduce by 50%
        };

        Some(adjustment)
    }

    async fn upsert_recovery_score(
        &self,
        user_id: Uuid,
        score_date: NaiveDate,
        readiness_score: f64,
        hrv_trend: HrvTrend,
        hrv_deviation: Option<f64>,
        sleep_quality_score: Option<f64>,
        recovery_adequacy: Option<f64>,
        rhr_deviation: Option<f64>,
        training_strain: Option<f64>,
        recovery_status: RecoveryStatus,
        recommended_tss_adjustment: Option<f64>,
    ) -> Result<RecoveryScore> {
        let score = sqlx::query_as!(
            RecoveryScore,
            r#"
            INSERT INTO recovery_scores (
                user_id, score_date, readiness_score, hrv_trend, hrv_deviation,
                sleep_quality_score, recovery_adequacy, rhr_deviation, training_strain,
                recovery_status, recommended_tss_adjustment, model_version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (user_id, score_date)
            DO UPDATE SET
                readiness_score = EXCLUDED.readiness_score,
                hrv_trend = EXCLUDED.hrv_trend,
                hrv_deviation = EXCLUDED.hrv_deviation,
                sleep_quality_score = EXCLUDED.sleep_quality_score,
                recovery_adequacy = EXCLUDED.recovery_adequacy,
                rhr_deviation = EXCLUDED.rhr_deviation,
                training_strain = EXCLUDED.training_strain,
                recovery_status = EXCLUDED.recovery_status,
                recommended_tss_adjustment = EXCLUDED.recommended_tss_adjustment,
                model_version = EXCLUDED.model_version,
                calculated_at = NOW(),
                updated_at = NOW()
            RETURNING
                id, user_id, score_date, readiness_score, hrv_trend, hrv_deviation,
                sleep_quality_score, recovery_adequacy, rhr_deviation, training_strain,
                recovery_status, recommended_tss_adjustment, calculated_at, model_version,
                created_at, updated_at
            "#,
            user_id,
            score_date,
            readiness_score,
            hrv_trend.as_str(),
            hrv_deviation,
            sleep_quality_score,
            recovery_adequacy,
            rhr_deviation,
            training_strain,
            recovery_status.as_str(),
            recommended_tss_adjustment,
            MODEL_VERSION
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to upsert recovery score")?;

        Ok(score)
    }

    async fn get_recovery_score(&self, user_id: Uuid, date: NaiveDate) -> Result<Option<RecoveryScore>> {
        let score = sqlx::query_as!(
            RecoveryScore,
            r#"
            SELECT id, user_id, score_date, readiness_score, hrv_trend, hrv_deviation,
                   sleep_quality_score, recovery_adequacy, rhr_deviation, training_strain,
                   recovery_status, recommended_tss_adjustment, calculated_at, model_version,
                   created_at, updated_at
            FROM recovery_scores
            WHERE user_id = $1 AND score_date = $2
            "#,
            user_id,
            date
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(score)
    }

    async fn get_recovery_scores_range(
        &self,
        user_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<RecoveryScore>> {
        let scores = sqlx::query_as!(
            RecoveryScore,
            r#"
            SELECT id, user_id, score_date, readiness_score, hrv_trend, hrv_deviation,
                   sleep_quality_score, recovery_adequacy, rhr_deviation, training_strain,
                   recovery_status, recommended_tss_adjustment, calculated_at, model_version,
                   created_at, updated_at
            FROM recovery_scores
            WHERE user_id = $1 AND score_date >= $2 AND score_date <= $3
            ORDER BY score_date DESC
            "#,
            user_id,
            start_date,
            end_date
        )
        .fetch_all(&self.db)
        .await?;

        Ok(scores)
    }

    fn determine_trend_direction(&self, scores: &[RecoveryScore]) -> String {
        if scores.len() < 3 {
            return "insufficient_data".to_string();
        }

        let mid = scores.len() / 2;
        let recent: Vec<_> = scores[..mid].to_vec();
        let older: Vec<_> = scores[mid..].to_vec();

        let recent_avg = recent.iter().map(|s| s.readiness_score).sum::<f64>() / recent.len() as f64;
        let older_avg = older.iter().map(|s| s.readiness_score).sum::<f64>() / older.len() as f64;

        let change = recent_avg - older_avg;

        if change > 5.0 {
            "improving".to_string()
        } else if change < -5.0 {
            "declining".to_string()
        } else {
            "stable".to_string()
        }
    }

    async fn detect_patterns(&self, scores: &[RecoveryScore]) -> Result<Vec<RecoveryPattern>> {
        let mut patterns = Vec::new();

        // Simple pattern: consecutive poor recovery
        let consecutive_poor = scores
            .windows(3)
            .filter(|w| w.iter().all(|s| s.readiness_score < 50.0))
            .count();

        if consecutive_poor > 0 {
            patterns.push(RecoveryPattern {
                pattern_type: "consecutive_poor_recovery".to_string(),
                description: format!("Detected {} instances of 3+ consecutive poor recovery days", consecutive_poor),
                confidence: 0.8,
            });
        }

        // Simple pattern: weekend recovery
        let weekend_scores: Vec<_> = scores
            .iter()
            .filter(|s| {
                let weekday = s.score_date.weekday();
                weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun
            })
            .collect();

        if !weekend_scores.is_empty() {
            let weekend_avg = weekend_scores.iter().map(|s| s.readiness_score).sum::<f64>() / weekend_scores.len() as f64;
            let overall_avg = scores.iter().map(|s| s.readiness_score).sum::<f64>() / scores.len() as f64;

            if weekend_avg > overall_avg + 10.0 {
                patterns.push(RecoveryPattern {
                    pattern_type: "weekend_recovery".to_string(),
                    description: "Recovery scores are significantly better on weekends".to_string(),
                    confidence: 0.7,
                });
            }
        }

        Ok(patterns)
    }

    async fn generate_recommendations(&self, score: &RecoveryScore) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        // HRV-based recommendations
        if score.hrv_trend == "declining" {
            recommendations.push(Recommendation {
                priority: "high".to_string(),
                category: "recovery".to_string(),
                message: "Your HRV is declining, indicating increased stress or fatigue".to_string(),
                action: "Consider taking a rest day or reducing training intensity by 30%".to_string(),
            });
        }

        // Sleep-based recommendations
        if let Some(sleep_quality) = score.sleep_quality_score {
            if sleep_quality < 70.0 {
                recommendations.push(Recommendation {
                    priority: "medium".to_string(),
                    category: "sleep".to_string(),
                    message: "Sleep quality is below optimal".to_string(),
                    action: "Aim for 8+ hours of quality sleep tonight. Consider improving sleep hygiene.".to_string(),
                });
            }
        }

        // Overall recovery recommendations
        if score.readiness_score < 30.0 {
            recommendations.push(Recommendation {
                priority: "critical".to_string(),
                category: "recovery".to_string(),
                message: "Critical recovery status detected".to_string(),
                action: "Take a complete rest day. Avoid any strenuous activity.".to_string(),
            });
        } else if score.readiness_score >= 85.0 {
            recommendations.push(Recommendation {
                priority: "low".to_string(),
                category: "training".to_string(),
                message: "Excellent recovery - you're ready for high-intensity training".to_string(),
                action: "This is a good day for your hardest workout of the week.".to_string(),
            });
        }

        // Default recommendation if none added
        if recommendations.is_empty() {
            recommendations.push(Recommendation {
                priority: "low".to_string(),
                category: "general".to_string(),
                message: "Recovery is within normal range".to_string(),
                action: "Continue with your planned training schedule.".to_string(),
            });
        }

        Ok(recommendations)
    }
}
