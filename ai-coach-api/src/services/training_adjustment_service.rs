use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Training adjustment service for recovery-based workout modifications
pub struct TrainingAdjustmentService {
    db: PgPool,
}

/// TSS adjustment recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TssAdjustment {
    pub original_tss: f64,
    pub recommended_tss: f64,
    pub adjustment_factor: f64,
    pub explanation: String,
    pub reasoning: Vec<String>,
}

/// Workout modification suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutModification {
    pub modification_type: String, // reduce_intensity, reduce_volume, swap_workout, rest_day
    pub original_workout: Option<WorkoutSummary>,
    pub suggested_workout: Option<WorkoutSummary>,
    pub reasoning: String,
}

/// Workout summary for modification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutSummary {
    pub workout_type: String,
    pub tss: f64,
    pub duration_minutes: i32,
    pub intensity_factor: Option<f64>,
}

/// Rest day recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestDayRecommendation {
    pub should_rest: bool,
    pub confidence: f64, // 0.0-1.0
    pub reasoning: String,
    pub alternative_action: Option<String>,
}

impl TrainingAdjustmentService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Calculate TSS adjustment based on recovery score
    pub async fn calculate_daily_tss_adjustment(
        &self,
        user_id: Uuid,
        target_date: NaiveDate,
        planned_tss: f64,
    ) -> Result<TssAdjustment> {
        // Get recovery score for the target date
        let recovery_score = sqlx::query!(
            r#"
            SELECT
                readiness_score,
                recovery_status,
                recommended_tss_adjustment,
                hrv_trend,
                sleep_quality_score,
                rhr_deviation
            FROM recovery_scores
            WHERE user_id = $1 AND score_date = $2
            "#,
            user_id,
            target_date
        )
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch recovery score")?;

        let score = match recovery_score {
            Some(s) => s,
            None => {
                // No recovery data - return no adjustment
                return Ok(TssAdjustment {
                    original_tss: planned_tss,
                    recommended_tss: planned_tss,
                    adjustment_factor: 1.0,
                    explanation: "No recovery data available - proceeding with planned workout".to_string(),
                    reasoning: vec!["Recovery data not yet available for today".to_string()],
                });
            }
        };

        // Use the calculated TSS adjustment from recovery score, or calculate based on readiness
        let adjustment_factor = score.recommended_tss_adjustment.unwrap_or_else(|| {
            // Calculate adjustment based on readiness score
            if score.readiness_score >= 80.0 {
                1.1 // Can increase by 10%
            } else if score.readiness_score >= 70.0 {
                1.0 // No adjustment
            } else if score.readiness_score >= 60.0 {
                0.95 // Reduce by 5%
            } else if score.readiness_score >= 50.0 {
                0.85 // Reduce by 15%
            } else if score.readiness_score >= 40.0 {
                0.7 // Reduce by 30%
            } else if score.readiness_score >= 30.0 {
                0.5 // Reduce by 50%
            } else {
                0.3 // Reduce by 70% or consider rest
            }
        });

        let recommended_tss = planned_tss * adjustment_factor;

        // Build reasoning
        let mut reasoning = Vec::new();

        reasoning.push(format!(
            "Readiness score: {:.1}/100 ({})",
            score.readiness_score, score.recovery_status
        ));

        if let Some(sleep_quality) = score.sleep_quality_score {
            reasoning.push(format!("Sleep quality: {:.1}/100", sleep_quality));
        }

        reasoning.push(format!("HRV trend: {}", score.hrv_trend));

        if let Some(rhr_dev) = score.rhr_deviation {
            if rhr_dev.abs() > 5.0 {
                reasoning.push(format!(
                    "Resting HR {} baseline by {:.1}%",
                    if rhr_dev > 0.0 { "above" } else { "below" },
                    rhr_dev.abs()
                ));
            }
        }

        let explanation = if adjustment_factor > 1.0 {
            format!(
                "Excellent recovery! You can increase training load by {:.0}%",
                (adjustment_factor - 1.0) * 100.0
            )
        } else if adjustment_factor == 1.0 {
            "Good recovery - proceed with planned workout".to_string()
        } else if adjustment_factor >= 0.8 {
            format!(
                "Moderate recovery - consider reducing intensity by {:.0}%",
                (1.0 - adjustment_factor) * 100.0
            )
        } else if adjustment_factor >= 0.5 {
            format!(
                "Poor recovery - strongly recommend reducing load by {:.0}%",
                (1.0 - adjustment_factor) * 100.0
            )
        } else {
            format!(
                "Critical recovery - consider rest day or very light activity ({:.0}% reduction)",
                (1.0 - adjustment_factor) * 100.0
            )
        };

        Ok(TssAdjustment {
            original_tss: planned_tss,
            recommended_tss,
            adjustment_factor,
            explanation,
            reasoning,
        })
    }

    /// Suggest workout modification based on recovery
    pub async fn suggest_workout_modification(
        &self,
        user_id: Uuid,
        target_date: NaiveDate,
        planned_workout: WorkoutSummary,
    ) -> Result<WorkoutModification> {
        let adjustment = self
            .calculate_daily_tss_adjustment(user_id, target_date, planned_workout.tss)
            .await?;

        // Determine modification type based on adjustment factor
        let modification = if adjustment.adjustment_factor < 0.5 {
            // Critical - suggest rest day
            WorkoutModification {
                modification_type: "rest_day".to_string(),
                original_workout: Some(planned_workout.clone()),
                suggested_workout: None,
                reasoning: format!(
                    "Recovery is critical (readiness < 30). Rest is strongly recommended. {}",
                    adjustment.explanation
                ),
            }
        } else if adjustment.adjustment_factor < 0.8 {
            // Poor - reduce intensity significantly
            let mut modified = planned_workout.clone();
            modified.tss = adjustment.recommended_tss;
            if let Some(if_val) = modified.intensity_factor {
                modified.intensity_factor = Some(if_val * 0.8); // Reduce intensity
            }
            modified.duration_minutes = (modified.duration_minutes as f64 * 0.9) as i32; // Slight duration reduction

            WorkoutModification {
                modification_type: "reduce_intensity".to_string(),
                original_workout: Some(planned_workout),
                suggested_workout: Some(modified),
                reasoning: format!(
                    "Recovery is poor. Reduce intensity to allow for better adaptation. {}",
                    adjustment.explanation
                ),
            }
        } else if adjustment.adjustment_factor < 1.0 {
            // Moderate - reduce volume
            let mut modified = planned_workout.clone();
            modified.tss = adjustment.recommended_tss;
            modified.duration_minutes = (modified.duration_minutes as f64 * adjustment.adjustment_factor) as i32;

            WorkoutModification {
                modification_type: "reduce_volume".to_string(),
                original_workout: Some(planned_workout),
                suggested_workout: Some(modified),
                reasoning: format!(
                    "Recovery is moderate. Reduce volume to manage training stress. {}",
                    adjustment.explanation
                ),
            }
        } else {
            // Good or excellent - no modification or can increase
            WorkoutModification {
                modification_type: "no_change".to_string(),
                original_workout: Some(planned_workout),
                suggested_workout: None,
                reasoning: format!("Recovery is good. Proceed as planned. {}", adjustment.explanation),
            }
        };

        Ok(modification)
    }

    /// Determine if a rest day should be scheduled
    pub async fn should_schedule_rest_day(
        &self,
        user_id: Uuid,
        target_date: NaiveDate,
    ) -> Result<RestDayRecommendation> {
        // Get recovery score for target date
        let recovery_score = sqlx::query_scalar!(
            r#"
            SELECT readiness_score
            FROM recovery_scores
            WHERE user_id = $1 AND score_date = $2
            "#,
            user_id,
            target_date
        )
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch recovery score")?;

        let readiness = match recovery_score {
            Some(score) => score,
            None => {
                return Ok(RestDayRecommendation {
                    should_rest: false,
                    confidence: 0.0,
                    reasoning: "No recovery data available".to_string(),
                    alternative_action: Some("Proceed with light workout and monitor how you feel".to_string()),
                });
            }
        };

        // Check for consecutive poor recovery days
        let poor_days_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM recovery_scores
            WHERE user_id = $1
              AND score_date >= $2
              AND score_date < $3
              AND readiness_score < 50
            "#,
            user_id,
            target_date - chrono::Duration::days(3),
            target_date
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to count poor recovery days")?;

        // Check last rest day
        let days_since_rest = sqlx::query_scalar!(
            r#"
            SELECT MIN(score_date)
            FROM recovery_scores
            WHERE user_id = $1
              AND score_date < $2
              AND readiness_score >= 80
            ORDER BY score_date DESC
            LIMIT 1
            "#,
            user_id,
            target_date
        )
        .fetch_optional(&self.db)
        .await
        .context("Failed to find last rest day")?;

        let days_without_good_recovery = match days_since_rest {
            Some(last_good_day) => (target_date - last_good_day).num_days(),
            None => 7, // Assume a week if no data
        };

        // Decision logic
        let should_rest = readiness < 30.0
            || (readiness < 40.0 && poor_days_count.unwrap_or(0) >= 3)
            || days_without_good_recovery > 6;

        let confidence = if readiness < 30.0 {
            0.95 // Very confident
        } else if poor_days_count.unwrap_or(0) >= 3 {
            0.85
        } else if days_without_good_recovery > 6 {
            0.75
        } else {
            0.5
        };

        let reasoning = if readiness < 30.0 {
            format!("Critical recovery (readiness: {:.1}/100). Rest is essential to prevent overtraining.", readiness)
        } else if poor_days_count.unwrap_or(0) >= 3 {
            format!(
                "Poor recovery for {} consecutive days (current: {:.1}/100). Rest day recommended.",
                poor_days_count.unwrap_or(0),
                readiness
            )
        } else if days_without_good_recovery > 6 {
            format!(
                "{} days without good recovery. Rest day recommended to allow full recovery.",
                days_without_good_recovery
            )
        } else {
            format!("Recovery is adequate (readiness: {:.1}/100). Rest not required.", readiness)
        };

        let alternative_action = if should_rest {
            Some("Active recovery: light walk, yoga, or stretching".to_string())
        } else {
            None
        };

        Ok(RestDayRecommendation {
            should_rest,
            confidence,
            reasoning,
            alternative_action,
        })
    }
}
