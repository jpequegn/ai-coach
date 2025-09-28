use chrono::{DateTime, Utc, Duration};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};
use uuid::Uuid;

use crate::models::{
    NotificationType, DeliveryChannel, CreateNotificationRequest,
};
use crate::services::{
    NotificationService,
    notification_service::{PerformanceAlertType, HealthAlertType, MotivationNotificationType},
    TrainingAnalysisService, PerformanceInsightsService,
};

#[derive(Debug)]
pub struct NotificationScheduler {
    notification_service: Arc<NotificationService>,
    training_analysis_service: Arc<TrainingAnalysisService>,
    performance_insights_service: Arc<PerformanceInsightsService>,
    db: PgPool,
}

impl NotificationScheduler {
    pub fn new(
        notification_service: Arc<NotificationService>,
        training_analysis_service: Arc<TrainingAnalysisService>,
        performance_insights_service: Arc<PerformanceInsightsService>,
        db: PgPool,
    ) -> Self {
        Self {
            notification_service,
            training_analysis_service,
            performance_insights_service,
            db,
        }
    }

    /// Start the notification scheduler
    pub async fn start(&self) {
        let scheduler = Arc::new(self.clone());

        // Spawn different scheduler tasks
        let scheduler_clone = scheduler.clone();
        tokio::spawn(async move {
            scheduler_clone.run_scheduled_notifications().await;
        });

        let scheduler_clone = scheduler.clone();
        tokio::spawn(async move {
            scheduler_clone.run_training_reminder_check().await;
        });

        let scheduler_clone = scheduler.clone();
        tokio::spawn(async move {
            scheduler_clone.run_performance_monitoring().await;
        });

        let scheduler_clone = scheduler.clone();
        tokio::spawn(async move {
            scheduler_clone.run_health_monitoring().await;
        });

        let scheduler_clone = scheduler.clone();
        tokio::spawn(async move {
            scheduler_clone.run_motivation_scheduler().await;
        });

        tracing::info!("Notification scheduler started");
    }

    /// Run scheduled notifications every minute
    async fn run_scheduled_notifications(&self) {
        let mut interval = interval(TokioDuration::from_secs(60)); // Every minute

        loop {
            interval.tick().await;

            match self.notification_service.send_scheduled_notifications().await {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!("Sent {} scheduled notifications", count);
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to send scheduled notifications: {}", e);
                }
            }
        }
    }

    /// Check for training reminders every 15 minutes
    async fn run_training_reminder_check(&self) {
        let mut interval = interval(TokioDuration::from_secs(15 * 60)); // Every 15 minutes

        loop {
            interval.tick().await;

            let users = self.get_active_users().await.unwrap_or_default();

            for user_id in users {
                match self.notification_service.schedule_training_reminders(user_id).await {
                    Ok(notifications) => {
                        if !notifications.is_empty() {
                            tracing::info!("Scheduled {} training reminders for user {}", notifications.len(), user_id);
                        }
                    },
                    Err(e) => {
                        tracing::error!("Failed to schedule training reminders for user {}: {}", user_id, e);
                    }
                }
            }
        }
    }

    /// Monitor performance metrics every hour
    async fn run_performance_monitoring(&self) {
        let mut interval = interval(TokioDuration::from_secs(60 * 60)); // Every hour

        loop {
            interval.tick().await;

            let users = self.get_active_users().await.unwrap_or_default();

            for user_id in users {
                // Check for fitness improvements
                if let Ok(improvement_detected) = self.check_fitness_improvement(user_id).await {
                    if improvement_detected {
                        let data = json!({
                            "improvement_percentage": 5.2,
                            "period_days": 7,
                            "metric": "FTP"
                        });

                        if let Err(e) = self.notification_service
                            .create_performance_alert(user_id, PerformanceAlertType::FitnessImprovement, data)
                            .await
                        {
                            tracing::error!("Failed to create fitness improvement alert for user {}: {}", user_id, e);
                        }
                    }
                }

                // Check for goal achievements
                if let Ok(goals_achieved) = self.check_goal_achievements(user_id).await {
                    for goal in goals_achieved {
                        let data = json!({
                            "goal_id": goal.id,
                            "goal_title": goal.title,
                            "achievement_date": Utc::now()
                        });

                        if let Err(e) = self.notification_service
                            .create_performance_alert(user_id, PerformanceAlertType::GoalAchievement, data)
                            .await
                        {
                            tracing::error!("Failed to create goal achievement alert for user {}: {}", user_id, e);
                        }
                    }
                }

                // Check for performance decline
                if let Ok(decline_detected) = self.check_performance_decline(user_id).await {
                    if decline_detected {
                        let data = json!({
                            "decline_percentage": -8.5,
                            "period_days": 14,
                            "recommendation": "Consider reducing training intensity"
                        });

                        if let Err(e) = self.notification_service
                            .create_performance_alert(user_id, PerformanceAlertType::PerformanceDecline, data)
                            .await
                        {
                            tracing::error!("Failed to create performance decline alert for user {}: {}", user_id, e);
                        }
                    }
                }
            }
        }
    }

    /// Monitor health and safety metrics every 2 hours
    async fn run_health_monitoring(&self) {
        let mut interval = interval(TokioDuration::from_secs(2 * 60 * 60)); // Every 2 hours

        loop {
            interval.tick().await;

            let users = self.get_active_users().await.unwrap_or_default();

            for user_id in users {
                // Check for overtraining risk
                if let Ok(overtraining_risk) = self.check_overtraining_risk(user_id).await {
                    if overtraining_risk {
                        let data = json!({
                            "risk_level": "High",
                            "training_load_ratio": 1.8,
                            "recommendation": "Take 2-3 recovery days",
                            "detected_at": Utc::now()
                        });

                        if let Err(e) = self.notification_service
                            .create_health_alert(user_id, HealthAlertType::OvertrainingRisk, data)
                            .await
                        {
                            tracing::error!("Failed to create overtraining risk alert for user {}: {}", user_id, e);
                        }
                    }
                }

                // Check for negative TSB
                if let Ok(negative_tsb_days) = self.check_negative_tsb(user_id).await {
                    if negative_tsb_days >= 5 {
                        let data = json!({
                            "consecutive_days": negative_tsb_days,
                            "current_tsb": -35.2,
                            "recommendation": "Focus on recovery activities",
                            "detected_at": Utc::now()
                        });

                        if let Err(e) = self.notification_service
                            .create_health_alert(user_id, HealthAlertType::NegativeTsb, data)
                            .await
                        {
                            tracing::error!("Failed to create negative TSB alert for user {}: {}", user_id, e);
                        }
                    }
                }

                // Check for injury risk
                if let Ok(injury_risk) = self.check_injury_risk(user_id).await {
                    if injury_risk {
                        let data = json!({
                            "risk_factors": ["High training load", "Poor recovery", "Load spikes"],
                            "risk_score": 0.78,
                            "recommendation": "Consider medical evaluation",
                            "detected_at": Utc::now()
                        });

                        if let Err(e) = self.notification_service
                            .create_health_alert(user_id, HealthAlertType::InjuryRisk, data)
                            .await
                        {
                            tracing::error!("Failed to create injury risk alert for user {}: {}", user_id, e);
                        }
                    }
                }
            }
        }
    }

    /// Send motivation notifications at scheduled intervals
    async fn run_motivation_scheduler(&self) {
        let mut interval = interval(TokioDuration::from_secs(24 * 60 * 60)); // Every 24 hours

        loop {
            interval.tick().await;

            let users = self.get_active_users().await.unwrap_or_default();

            for user_id in users {
                // Check if it's time for weekly progress summary (every Monday)
                if self.is_monday() {
                    let weekly_data = self.generate_weekly_progress_data(user_id).await.unwrap_or_default();
                    let data = json!({
                        "week_summary": weekly_data,
                        "generated_at": Utc::now()
                    });

                    if let Err(e) = self.notification_service
                        .create_motivation_notification(user_id, MotivationNotificationType::WeeklyProgressSummary, data)
                        .await
                    {
                        tracing::error!("Failed to create weekly progress summary for user {}: {}", user_id, e);
                    }
                }

                // Check for achievement badges
                if let Ok(new_achievements) = self.check_new_achievements(user_id).await {
                    for achievement in new_achievements {
                        let data = json!({
                            "achievement_id": achievement.id,
                            "achievement_name": achievement.name,
                            "description": achievement.description,
                            "earned_at": Utc::now()
                        });

                        if let Err(e) = self.notification_service
                            .create_motivation_notification(user_id, MotivationNotificationType::AchievementBadge, data)
                            .await
                        {
                            tracing::error!("Failed to create achievement badge notification for user {}: {}", user_id, e);
                        }
                    }
                }

                // Check for training streaks
                if let Ok(streak_length) = self.check_training_streak(user_id).await {
                    if streak_length > 0 && streak_length % 7 == 0 { // Every week
                        let data = json!({
                            "streak_length": streak_length,
                            "streak_type": "daily_training",
                            "achievement_level": if streak_length >= 30 { "gold" } else if streak_length >= 14 { "silver" } else { "bronze" },
                            "earned_at": Utc::now()
                        });

                        if let Err(e) = self.notification_service
                            .create_motivation_notification(user_id, MotivationNotificationType::TrainingStreak, data)
                            .await
                        {
                            tracing::error!("Failed to create training streak notification for user {}: {}", user_id, e);
                        }
                    }
                }
            }
        }
    }

    // Helper methods for analysis and checks

    async fn get_active_users(&self) -> Result<Vec<Uuid>, Box<dyn std::error::Error + Send + Sync>> {
        // Mock implementation - in real code, this would query the database for active users
        Ok(vec![])
    }

    async fn check_fitness_improvement(&self, user_id: Uuid) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Mock implementation - would analyze training data for fitness improvements
        Ok(false)
    }

    async fn check_goal_achievements(&self, user_id: Uuid) -> Result<Vec<MockGoal>, Box<dyn std::error::Error + Send + Sync>> {
        // Mock implementation - would check if any goals have been achieved
        Ok(vec![])
    }

    async fn check_performance_decline(&self, user_id: Uuid) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Mock implementation - would analyze performance trends
        Ok(false)
    }

    async fn check_overtraining_risk(&self, user_id: Uuid) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Mock implementation - would analyze training load and recovery metrics
        Ok(false)
    }

    async fn check_negative_tsb(&self, user_id: Uuid) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
        // Mock implementation - would check consecutive days of negative TSB
        Ok(0)
    }

    async fn check_injury_risk(&self, user_id: Uuid) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Mock implementation - would analyze injury risk factors
        Ok(false)
    }

    async fn generate_weekly_progress_data(&self, user_id: Uuid) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // Mock implementation - would generate weekly progress summary
        Ok(json!({
            "total_sessions": 5,
            "total_duration_hours": 8.5,
            "total_tss": 420,
            "average_intensity": 0.75,
            "improvement_areas": ["Endurance", "Recovery"]
        }))
    }

    async fn check_new_achievements(&self, user_id: Uuid) -> Result<Vec<MockAchievement>, Box<dyn std::error::Error + Send + Sync>> {
        // Mock implementation - would check for new achievements
        Ok(vec![])
    }

    async fn check_training_streak(&self, user_id: Uuid) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
        // Mock implementation - would calculate current training streak
        Ok(0)
    }

    fn is_monday(&self) -> bool {
        Utc::now().weekday() == chrono::Weekday::Mon
    }
}

impl Clone for NotificationScheduler {
    fn clone(&self) -> Self {
        Self {
            notification_service: self.notification_service.clone(),
            training_analysis_service: self.training_analysis_service.clone(),
            performance_insights_service: self.performance_insights_service.clone(),
            db: self.db.clone(),
        }
    }
}

// Mock types for demonstration
#[derive(Debug)]
struct MockGoal {
    id: Uuid,
    title: String,
}

#[derive(Debug)]
struct MockAchievement {
    id: Uuid,
    name: String,
    description: String,
}