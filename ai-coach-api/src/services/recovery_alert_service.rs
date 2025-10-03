use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    RecoveryAlert, RecoveryAlertPreferences, RecoveryAlertRule, RecoveryScore,
    Severity,
};
use crate::services::NotificationService;

const ALERT_COOLDOWN_HOURS: i64 = 24;

pub struct RecoveryAlertService {
    db: PgPool,
    notification_service: NotificationService,
}

impl RecoveryAlertService {
    pub fn new(db: PgPool, notification_service: NotificationService) -> Self {
        Self {
            db,
            notification_service,
        }
    }

    /// Evaluate alerts for a recovery score
    pub async fn evaluate_alerts(&self, user_id: Uuid, score: &RecoveryScore) -> Result<Vec<RecoveryAlert>> {
        let mut alerts = Vec::new();

        // Get user alert preferences
        let preferences = self.get_or_create_preferences(user_id).await?;

        if !preferences.enabled {
            return Ok(alerts);
        }

        // Define alert rules
        let rules = self.get_alert_rules(&preferences);

        for rule in rules {
            if self.should_trigger_alert(user_id, score, &rule, &preferences).await? {
                // Check cooldown
                if self.is_in_cooldown(user_id, &rule.alert_type).await? {
                    tracing::debug!(
                        "Alert {} for user {} is in cooldown",
                        rule.alert_type,
                        user_id
                    );
                    continue;
                }

                // Create alert
                let alert = self.create_alert(user_id, score, &rule).await?;

                // Send notification
                self.send_alert_notification(&alert, &preferences).await?;

                alerts.push(alert);
            }
        }

        Ok(alerts)
    }

    /// Get alert history for a user
    pub async fn get_alert_history(
        &self,
        user_id: Uuid,
        limit: i64,
        include_acknowledged: bool,
    ) -> Result<Vec<RecoveryAlert>> {
        let alerts = if include_acknowledged {
            sqlx::query_as!(
                RecoveryAlert,
                r#"
                SELECT id, user_id, alert_type, severity, recovery_score_id,
                       message, recommendations, acknowledged_at, created_at
                FROM recovery_alerts
                WHERE user_id = $1
                ORDER BY created_at DESC
                LIMIT $2
                "#,
                user_id,
                limit
            )
            .fetch_all(&self.db)
            .await?
        } else {
            sqlx::query_as!(
                RecoveryAlert,
                r#"
                SELECT id, user_id, alert_type, severity, recovery_score_id,
                       message, recommendations, acknowledged_at, created_at
                FROM recovery_alerts
                WHERE user_id = $1 AND acknowledged_at IS NULL
                ORDER BY created_at DESC
                LIMIT $2
                "#,
                user_id,
                limit
            )
            .fetch_all(&self.db)
            .await?
        };

        Ok(alerts)
    }

    /// Acknowledge an alert
    pub async fn acknowledge_alert(&self, alert_id: Uuid, user_id: Uuid) -> Result<RecoveryAlert> {
        let alert = sqlx::query_as!(
            RecoveryAlert,
            r#"
            UPDATE recovery_alerts
            SET acknowledged_at = NOW()
            WHERE id = $1 AND user_id = $2 AND acknowledged_at IS NULL
            RETURNING id, user_id, alert_type, severity, recovery_score_id,
                      message, recommendations, acknowledged_at, created_at
            "#,
            alert_id,
            user_id
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to acknowledge alert or alert not found")?;

        Ok(alert)
    }

    /// Get user alert preferences
    pub async fn get_preferences(&self, user_id: Uuid) -> Result<RecoveryAlertPreferences> {
        self.get_or_create_preferences(user_id).await
    }

    /// Update user alert preferences
    pub async fn update_preferences(
        &self,
        user_id: Uuid,
        enabled: Option<bool>,
        push_notifications: Option<bool>,
        email_notifications: Option<bool>,
        poor_recovery_threshold: Option<f64>,
        critical_recovery_threshold: Option<f64>,
    ) -> Result<RecoveryAlertPreferences> {
        let prefs = sqlx::query_as!(
            RecoveryAlertPreferences,
            r#"
            UPDATE recovery_alert_preferences
            SET enabled = COALESCE($2, enabled),
                push_notifications = COALESCE($3, push_notifications),
                email_notifications = COALESCE($4, email_notifications),
                poor_recovery_threshold = COALESCE($5, poor_recovery_threshold),
                critical_recovery_threshold = COALESCE($6, critical_recovery_threshold),
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING id, user_id, enabled, push_notifications, email_notifications,
                      poor_recovery_threshold, critical_recovery_threshold,
                      created_at, updated_at
            "#,
            user_id,
            enabled,
            push_notifications,
            email_notifications,
            poor_recovery_threshold,
            critical_recovery_threshold
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to update alert preferences")?;

        Ok(prefs)
    }

    // ========================================================================
    // Private Helper Methods
    // ========================================================================

    async fn get_or_create_preferences(&self, user_id: Uuid) -> Result<RecoveryAlertPreferences> {
        // Try to get existing preferences
        let existing = sqlx::query_as!(
            RecoveryAlertPreferences,
            r#"
            SELECT id, user_id, enabled, push_notifications, email_notifications,
                   poor_recovery_threshold, critical_recovery_threshold,
                   created_at, updated_at
            FROM recovery_alert_preferences
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        if let Some(prefs) = existing {
            return Ok(prefs);
        }

        // Create default preferences
        let prefs = sqlx::query_as!(
            RecoveryAlertPreferences,
            r#"
            INSERT INTO recovery_alert_preferences (user_id)
            VALUES ($1)
            RETURNING id, user_id, enabled, push_notifications, email_notifications,
                      poor_recovery_threshold, critical_recovery_threshold,
                      created_at, updated_at
            "#,
            user_id
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to create default alert preferences")?;

        Ok(prefs)
    }

    fn get_alert_rules(&self, preferences: &RecoveryAlertPreferences) -> Vec<RecoveryAlertRule> {
        vec![
            // Critical recovery status
            RecoveryAlertRule {
                alert_type: "critical_recovery".to_string(),
                severity: Severity::Critical,
                condition: Box::new(move |score: &RecoveryScore| {
                    score.readiness_score < preferences.critical_recovery_threshold
                }),
                message: "Critical recovery status detected".to_string(),
                recommendation: "Take a complete rest day. Avoid any strenuous activity. Prioritize sleep and hydration.".to_string(),
            },
            // Poor recovery (consecutive days)
            RecoveryAlertRule {
                alert_type: "consecutive_poor_recovery".to_string(),
                severity: Severity::Warning,
                condition: Box::new(move |score: &RecoveryScore| {
                    score.readiness_score < preferences.poor_recovery_threshold
                }),
                message: "Poor recovery detected".to_string(),
                recommendation: "Consider reducing training intensity by 30-50%. Focus on recovery activities.".to_string(),
            },
            // Declining HRV trend
            RecoveryAlertRule {
                alert_type: "declining_hrv".to_string(),
                severity: Severity::Warning,
                condition: Box::new(|score: &RecoveryScore| {
                    score.hrv_trend == "declining"
                }),
                message: "Your HRV has been trending downward".to_string(),
                recommendation: "This indicates increased stress or fatigue. Consider taking a recovery day.".to_string(),
            },
            // High training strain with poor recovery
            RecoveryAlertRule {
                alert_type: "high_strain_poor_recovery".to_string(),
                severity: Severity::Warning,
                condition: Box::new(|score: &RecoveryScore| {
                    score.training_strain.unwrap_or(0.0) > 1300.0 && score.readiness_score < 60.0
                }),
                message: "High training strain combined with poor recovery".to_string(),
                recommendation: "Risk of overtraining. Take a rest day and reassess your training load.".to_string(),
            },
            // Poor sleep quality
            RecoveryAlertRule {
                alert_type: "poor_sleep".to_string(),
                severity: Severity::Info,
                condition: Box::new(|score: &RecoveryScore| {
                    score.sleep_quality_score.unwrap_or(100.0) < 60.0
                }),
                message: "Sleep quality is below optimal".to_string(),
                recommendation: "Aim for 8+ hours of quality sleep. Review sleep hygiene practices.".to_string(),
            },
        ]
    }

    async fn should_trigger_alert(
        &self,
        user_id: Uuid,
        score: &RecoveryScore,
        rule: &RecoveryAlertRule,
        preferences: &RecoveryAlertPreferences,
    ) -> Result<bool> {
        // Check if rule condition is met
        if !(rule.condition)(score) {
            return Ok(false);
        }

        // For consecutive poor recovery, check if we have 3 consecutive days
        if rule.alert_type == "consecutive_poor_recovery" {
            let consecutive_count = self.count_consecutive_poor_days(user_id, score.score_date).await?;
            if consecutive_count < 3 {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn count_consecutive_poor_days(
        &self,
        user_id: Uuid,
        end_date: chrono::NaiveDate,
    ) -> Result<i32> {
        let start_date = end_date - Duration::days(7);

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

        let mut consecutive = 0;
        for score in scores {
            if score.readiness_score < 40.0 {
                consecutive += 1;
            } else {
                break;
            }
        }

        Ok(consecutive)
    }

    async fn is_in_cooldown(&self, user_id: Uuid, alert_type: &str) -> Result<bool> {
        let cooldown_time = Utc::now() - Duration::hours(ALERT_COOLDOWN_HOURS);

        let recent_alert = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM recovery_alerts
            WHERE user_id = $1 AND alert_type = $2 AND created_at > $3
            "#,
            user_id,
            alert_type,
            cooldown_time
        )
        .fetch_one(&self.db)
        .await?;

        Ok(recent_alert > 0)
    }

    async fn create_alert(
        &self,
        user_id: Uuid,
        score: &RecoveryScore,
        rule: &RecoveryAlertRule,
    ) -> Result<RecoveryAlert> {
        let recommendations = serde_json::json!([{
            "priority": rule.severity.priority_level(),
            "category": "recovery",
            "message": rule.message.clone(),
            "action": rule.recommendation.clone()
        }]);

        let alert = sqlx::query_as!(
            RecoveryAlert,
            r#"
            INSERT INTO recovery_alerts (
                user_id, alert_type, severity, recovery_score_id,
                message, recommendations
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, user_id, alert_type, severity, recovery_score_id,
                      message, recommendations, acknowledged_at, created_at
            "#,
            user_id,
            rule.alert_type,
            rule.severity.as_str(),
            score.id,
            rule.message,
            sqlx::types::Json(recommendations)
        )
        .fetch_one(&self.db)
        .await
        .context("Failed to create recovery alert")?;

        tracing::info!(
            "Created recovery alert {} for user {}: {}",
            rule.alert_type,
            user_id,
            rule.message
        );

        Ok(alert)
    }

    async fn send_alert_notification(
        &self,
        alert: &RecoveryAlert,
        preferences: &RecoveryAlertPreferences,
    ) -> Result<()> {
        let severity = Severity::from_str(&alert.severity);

        // Critical alerts always send push notifications
        if severity == Severity::Critical || preferences.push_notifications {
            self.notification_service
                .send_push_notification(
                    alert.user_id,
                    "Recovery Alert".to_string(),
                    alert.message.clone(),
                )
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to send push notification: {}", e);
                });
        }

        // Send email for warnings and critical (if enabled)
        if (severity == Severity::Warning || severity == Severity::Critical)
            && preferences.email_notifications
        {
            self.notification_service
                .send_email_notification(
                    alert.user_id,
                    "Recovery Alert".to_string(),
                    alert.message.clone(),
                )
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to send email notification: {}", e);
                });
        }

        Ok(())
    }
}
