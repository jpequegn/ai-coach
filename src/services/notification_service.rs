use chrono::{DateTime, Utc, Duration, TimeZone};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::{
    Notification, NotificationType, NotificationCategory, NotificationPriority,
    DeliveryChannel, DeliveryStatus, CreateNotificationRequest,
    NotificationPreferences, NotificationTemplate, NotificationBatch, BatchType,
    NotificationMetrics, NotificationTypeMetrics, ChannelMetrics,
    UpdateNotificationPreferencesRequest,
};

#[derive(Debug)]
pub struct NotificationService {
    db: PgPool,
    email_service: Option<EmailService>,
    push_service: Option<PushNotificationService>,
}

impl NotificationService {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            email_service: Some(EmailService::new()),
            push_service: Some(PushNotificationService::new()),
        }
    }

    /// Create a new notification
    pub async fn create_notification(
        &self,
        request: CreateNotificationRequest,
    ) -> Result<Notification, NotificationError> {
        let now = Utc::now();
        let scheduled_at = request.scheduled_at.unwrap_or(now);

        // Check user notification preferences
        let preferences = self.get_user_preferences(request.user_id).await?;
        let filtered_channels = self.filter_channels_by_preferences(
            &request.delivery_channels,
            &request.notification_type,
            &preferences,
        );

        if filtered_channels.is_empty() {
            return Err(NotificationError::AllChannelsDisabled);
        }

        let notification = Notification {
            id: Uuid::new_v4(),
            user_id: request.user_id,
            notification_type: request.notification_type.clone(),
            category: self.get_category_for_type(&request.notification_type),
            priority: self.get_priority_for_type(&request.notification_type),
            title: request.title,
            message: request.message,
            data: request.data,
            scheduled_at,
            sent_at: None,
            read_at: None,
            delivery_channels: filtered_channels,
            delivery_status: DeliveryStatus::Scheduled,
            expires_at: request.expires_at,
            created_at: now,
            updated_at: now,
        };

        // Store in database (mock implementation)
        // In real implementation, this would use sqlx to insert into notifications table
        tracing::info!("Created notification {} for user {}", notification.id, notification.user_id);

        Ok(notification)
    }

    /// Schedule notifications based on user's training plan and preferences
    pub async fn schedule_training_reminders(&self, user_id: Uuid) -> Result<Vec<Notification>, NotificationError> {
        let preferences = self.get_user_preferences(user_id).await?;
        if !preferences.workout_reminders {
            return Ok(vec![]);
        }

        let mut notifications = vec![];

        // Mock: Get upcoming workouts from training plan
        let upcoming_workouts = self.get_upcoming_workouts(user_id).await?;

        for workout in upcoming_workouts {
            let reminder_time = workout.scheduled_at - Duration::minutes(preferences.workout_reminder_advance_minutes as i64);

            // Check if reminder time is not in quiet hours
            if self.is_in_quiet_hours(&reminder_time, &preferences) {
                continue;
            }

            let notification = self.create_notification(CreateNotificationRequest {
                user_id,
                notification_type: NotificationType::WorkoutReminder,
                title: format!("Workout Reminder: {}", workout.name),
                message: format!("Your {} workout is scheduled in {} minutes",
                    workout.workout_type, preferences.workout_reminder_advance_minutes),
                data: Some(json!({
                    "workout_id": workout.id,
                    "workout_type": workout.workout_type,
                    "duration_minutes": workout.duration_minutes
                })),
                scheduled_at: Some(reminder_time),
                delivery_channels: vec![DeliveryChannel::InApp, DeliveryChannel::Email],
                expires_at: Some(workout.scheduled_at + Duration::hours(1)),
            }).await?;

            notifications.push(notification);
        }

        Ok(notifications)
    }

    /// Create performance improvement notification
    pub async fn create_performance_alert(
        &self,
        user_id: Uuid,
        alert_type: PerformanceAlertType,
        data: serde_json::Value,
    ) -> Result<Notification, NotificationError> {
        let preferences = self.get_user_preferences(user_id).await?;

        let (notification_type, title, message) = match alert_type {
            PerformanceAlertType::FitnessImprovement => {
                if !preferences.fitness_improvement_alerts {
                    return Err(NotificationError::NotificationDisabled);
                }
                (
                    NotificationType::FitnessImprovement,
                    "Fitness Improvement Detected!".to_string(),
                    "Your fitness has improved significantly over the past week. Keep up the great work!".to_string(),
                )
            },
            PerformanceAlertType::GoalAchievement => {
                if !preferences.goal_achievement_alerts {
                    return Err(NotificationError::NotificationDisabled);
                }
                (
                    NotificationType::GoalAchievement,
                    "Goal Achievement!".to_string(),
                    "Congratulations! You've achieved one of your training goals.".to_string(),
                )
            },
            PerformanceAlertType::PerformanceDecline => {
                if !preferences.performance_decline_alerts {
                    return Err(NotificationError::NotificationDisabled);
                }
                (
                    NotificationType::PerformanceDecline,
                    "Performance Decline Notice".to_string(),
                    "We've noticed a decline in your recent performance. Consider adjusting your training plan.".to_string(),
                )
            },
        };

        self.create_notification(CreateNotificationRequest {
            user_id,
            notification_type,
            title,
            message,
            data: Some(data),
            scheduled_at: Some(Utc::now()),
            delivery_channels: vec![DeliveryChannel::InApp, DeliveryChannel::Email],
            expires_at: Some(Utc::now() + Duration::days(7)),
        }).await
    }

    /// Create health and safety alerts
    pub async fn create_health_alert(
        &self,
        user_id: Uuid,
        alert_type: HealthAlertType,
        data: serde_json::Value,
    ) -> Result<Notification, NotificationError> {
        let preferences = self.get_user_preferences(user_id).await?;

        let (notification_type, title, message, priority) = match alert_type {
            HealthAlertType::OvertrainingRisk => {
                if !preferences.overtraining_risk_alerts {
                    return Err(NotificationError::NotificationDisabled);
                }
                (
                    NotificationType::OvertrainingRisk,
                    "Overtraining Risk Detected".to_string(),
                    "Your training load has been very high recently. Consider taking a recovery day.".to_string(),
                    NotificationPriority::High,
                )
            },
            HealthAlertType::NegativeTsb => {
                if !preferences.negative_tsb_alerts {
                    return Err(NotificationError::NotificationDisabled);
                }
                (
                    NotificationType::NegativeTsbAlert,
                    "High Fatigue Alert".to_string(),
                    "Your Training Stress Balance has been negative for several days. Recovery is recommended.".to_string(),
                    NotificationPriority::Medium,
                )
            },
            HealthAlertType::InjuryRisk => {
                if !preferences.injury_risk_alerts {
                    return Err(NotificationError::NotificationDisabled);
                }
                (
                    NotificationType::InjuryRisk,
                    "Injury Risk Warning".to_string(),
                    "Based on your training load and recovery patterns, there's an elevated injury risk.".to_string(),
                    NotificationPriority::Critical,
                )
            },
        };

        let mut notification = self.create_notification(CreateNotificationRequest {
            user_id,
            notification_type,
            title,
            message,
            data: Some(data),
            scheduled_at: Some(Utc::now()),
            delivery_channels: vec![DeliveryChannel::InApp, DeliveryChannel::Email, DeliveryChannel::WebPush],
            expires_at: Some(Utc::now() + Duration::days(3)),
        }).await?;

        notification.priority = priority;
        Ok(notification)
    }

    /// Create motivation and engagement notifications
    pub async fn create_motivation_notification(
        &self,
        user_id: Uuid,
        motivation_type: MotivationNotificationType,
        data: serde_json::Value,
    ) -> Result<Notification, NotificationError> {
        let preferences = self.get_user_preferences(user_id).await?;

        let (notification_type, title, message) = match motivation_type {
            MotivationNotificationType::WeeklyProgressSummary => {
                if !preferences.weekly_progress_summaries {
                    return Err(NotificationError::NotificationDisabled);
                }
                (
                    NotificationType::WeeklyProgressSummary,
                    "Your Weekly Progress".to_string(),
                    "Here's a summary of your training progress this week.".to_string(),
                )
            },
            MotivationNotificationType::AchievementBadge => {
                if !preferences.achievement_badge_alerts {
                    return Err(NotificationError::NotificationDisabled);
                }
                (
                    NotificationType::AchievementBadge,
                    "New Achievement Unlocked!".to_string(),
                    "You've earned a new achievement badge!".to_string(),
                )
            },
            MotivationNotificationType::TrainingStreak => {
                if !preferences.training_streak_alerts {
                    return Err(NotificationError::NotificationDisabled);
                }
                (
                    NotificationType::TrainingStreak,
                    "Training Streak Achievement!".to_string(),
                    "Congratulations on maintaining your training streak!".to_string(),
                )
            },
        };

        self.create_notification(CreateNotificationRequest {
            user_id,
            notification_type,
            title,
            message,
            data: Some(data),
            scheduled_at: Some(Utc::now()),
            delivery_channels: vec![DeliveryChannel::InApp, DeliveryChannel::Email],
            expires_at: Some(Utc::now() + Duration::days(30)),
        }).await
    }

    /// Send scheduled notifications
    pub async fn send_scheduled_notifications(&self) -> Result<u32, NotificationError> {
        let now = Utc::now();
        let notifications = self.get_scheduled_notifications(now).await?;
        let mut sent_count = 0;

        for notification in notifications {
            match self.send_notification(&notification).await {
                Ok(_) => {
                    sent_count += 1;
                    self.mark_notification_sent(notification.id, now).await?;
                },
                Err(e) => {
                    tracing::error!("Failed to send notification {}: {}", notification.id, e);
                    self.mark_notification_failed(notification.id).await?;
                }
            }
        }

        Ok(sent_count)
    }

    /// Send a single notification through all its delivery channels
    async fn send_notification(&self, notification: &Notification) -> Result<(), NotificationError> {
        for channel in &notification.delivery_channels {
            match channel {
                DeliveryChannel::Email => {
                    if let Some(ref email_service) = self.email_service {
                        email_service.send_email_notification(notification).await?;
                    }
                },
                DeliveryChannel::WebPush => {
                    if let Some(ref push_service) = self.push_service {
                        push_service.send_push_notification(notification).await?;
                    }
                },
                DeliveryChannel::InApp => {
                    // In-app notifications are stored in database and displayed in UI
                    // No external service needed
                },
                DeliveryChannel::Sms => {
                    // Future implementation
                    tracing::warn!("SMS notifications not yet implemented");
                },
            }
        }
        Ok(())
    }

    /// Get user notification preferences
    pub async fn get_user_preferences(&self, user_id: Uuid) -> Result<NotificationPreferences, NotificationError> {
        // Mock implementation - in real code, this would query the database
        Ok(NotificationPreferences {
            user_id,
            ..Default::default()
        })
    }

    /// Update user notification preferences
    pub async fn update_user_preferences(
        &self,
        user_id: Uuid,
        updates: UpdateNotificationPreferencesRequest,
    ) -> Result<NotificationPreferences, NotificationError> {
        let mut preferences = self.get_user_preferences(user_id).await?;

        // Apply updates
        if let Some(workout_reminders) = updates.workout_reminders {
            preferences.workout_reminders = workout_reminders;
        }
        if let Some(advance_minutes) = updates.workout_reminder_advance_minutes {
            preferences.workout_reminder_advance_minutes = advance_minutes;
        }
        if let Some(rest_day_reminders) = updates.rest_day_reminders {
            preferences.rest_day_reminders = rest_day_reminders;
        }
        if let Some(ftp_test_reminders) = updates.ftp_test_reminders {
            preferences.ftp_test_reminders = ftp_test_reminders;
        }
        if let Some(fitness_improvement_alerts) = updates.fitness_improvement_alerts {
            preferences.fitness_improvement_alerts = fitness_improvement_alerts;
        }
        if let Some(goal_achievement_alerts) = updates.goal_achievement_alerts {
            preferences.goal_achievement_alerts = goal_achievement_alerts;
        }
        if let Some(performance_decline_alerts) = updates.performance_decline_alerts {
            preferences.performance_decline_alerts = performance_decline_alerts;
        }
        if let Some(overtraining_risk_alerts) = updates.overtraining_risk_alerts {
            preferences.overtraining_risk_alerts = overtraining_risk_alerts;
        }
        if let Some(negative_tsb_alerts) = updates.negative_tsb_alerts {
            preferences.negative_tsb_alerts = negative_tsb_alerts;
        }
        if let Some(injury_risk_alerts) = updates.injury_risk_alerts {
            preferences.injury_risk_alerts = injury_risk_alerts;
        }
        if let Some(weekly_progress_summaries) = updates.weekly_progress_summaries {
            preferences.weekly_progress_summaries = weekly_progress_summaries;
        }
        if let Some(achievement_badge_alerts) = updates.achievement_badge_alerts {
            preferences.achievement_badge_alerts = achievement_badge_alerts;
        }
        if let Some(training_streak_alerts) = updates.training_streak_alerts {
            preferences.training_streak_alerts = training_streak_alerts;
        }
        if let Some(email_enabled) = updates.email_enabled {
            preferences.email_enabled = email_enabled;
        }
        if let Some(web_push_enabled) = updates.web_push_enabled {
            preferences.web_push_enabled = web_push_enabled;
        }
        if let Some(in_app_enabled) = updates.in_app_enabled {
            preferences.in_app_enabled = in_app_enabled;
        }
        if let Some(quiet_hours_start) = updates.quiet_hours_start {
            preferences.quiet_hours_start = quiet_hours_start;
        }
        if let Some(quiet_hours_end) = updates.quiet_hours_end {
            preferences.quiet_hours_end = quiet_hours_end;
        }
        if let Some(timezone) = updates.timezone {
            preferences.timezone = timezone;
        }
        if let Some(batch_notifications) = updates.batch_notifications {
            preferences.batch_notifications = batch_notifications;
        }
        if let Some(batch_interval_minutes) = updates.batch_interval_minutes {
            preferences.batch_interval_minutes = batch_interval_minutes;
        }

        preferences.updated_at = Utc::now();

        // Mock: Save to database
        tracing::info!("Updated notification preferences for user {}", user_id);

        Ok(preferences)
    }

    // Helper methods

    fn get_category_for_type(&self, notification_type: &NotificationType) -> NotificationCategory {
        match notification_type {
            NotificationType::WorkoutReminder |
            NotificationType::RestDayReminder |
            NotificationType::FtpTestSuggestion => NotificationCategory::Training,

            NotificationType::FitnessImprovement |
            NotificationType::GoalAchievement |
            NotificationType::PerformanceDecline => NotificationCategory::Performance,

            NotificationType::OvertrainingRisk |
            NotificationType::NegativeTsbAlert |
            NotificationType::InjuryRisk => NotificationCategory::Health,

            NotificationType::WeeklyProgressSummary |
            NotificationType::AchievementBadge |
            NotificationType::TrainingStreak => NotificationCategory::Motivation,

            NotificationType::SystemMaintenance |
            NotificationType::SecurityAlert => NotificationCategory::System,
        }
    }

    fn get_priority_for_type(&self, notification_type: &NotificationType) -> NotificationPriority {
        match notification_type {
            NotificationType::InjuryRisk |
            NotificationType::SecurityAlert => NotificationPriority::Critical,

            NotificationType::OvertrainingRisk |
            NotificationType::NegativeTsbAlert |
            NotificationType::SystemMaintenance => NotificationPriority::High,

            NotificationType::WorkoutReminder |
            NotificationType::FitnessImprovement |
            NotificationType::GoalAchievement |
            NotificationType::PerformanceDecline => NotificationPriority::Medium,

            _ => NotificationPriority::Low,
        }
    }

    fn filter_channels_by_preferences(
        &self,
        channels: &[DeliveryChannel],
        notification_type: &NotificationType,
        preferences: &NotificationPreferences,
    ) -> Vec<DeliveryChannel> {
        channels.iter()
            .filter(|channel| match channel {
                DeliveryChannel::Email => preferences.email_enabled,
                DeliveryChannel::WebPush => preferences.web_push_enabled,
                DeliveryChannel::InApp => preferences.in_app_enabled,
                DeliveryChannel::Sms => false, // Not yet implemented
            })
            .cloned()
            .collect()
    }

    fn is_in_quiet_hours(&self, time: &DateTime<Utc>, preferences: &NotificationPreferences) -> bool {
        // This is a simplified implementation
        // In real code, you'd parse the timezone and quiet hours properly
        let hour = time.hour();
        let quiet_start: u32 = preferences.quiet_hours_start.split(':').next()
            .and_then(|h| h.parse().ok()).unwrap_or(22);
        let quiet_end: u32 = preferences.quiet_hours_end.split(':').next()
            .and_then(|h| h.parse().ok()).unwrap_or(7);

        if quiet_start < quiet_end {
            hour >= quiet_start && hour < quiet_end
        } else {
            hour >= quiet_start || hour < quiet_end
        }
    }

    // Mock helper methods (in real implementation, these would query the database)

    async fn get_upcoming_workouts(&self, user_id: Uuid) -> Result<Vec<MockWorkout>, NotificationError> {
        // Mock implementation
        Ok(vec![])
    }

    async fn get_scheduled_notifications(&self, before: DateTime<Utc>) -> Result<Vec<Notification>, NotificationError> {
        // Mock implementation
        Ok(vec![])
    }

    async fn mark_notification_sent(&self, notification_id: Uuid, sent_at: DateTime<Utc>) -> Result<(), NotificationError> {
        // Mock implementation
        tracing::info!("Marked notification {} as sent at {}", notification_id, sent_at);
        Ok(())
    }

    async fn mark_notification_failed(&self, notification_id: Uuid) -> Result<(), NotificationError> {
        // Mock implementation
        tracing::error!("Marked notification {} as failed", notification_id);
        Ok(())
    }
}

// Supporting types and services

#[derive(Debug)]
pub enum PerformanceAlertType {
    FitnessImprovement,
    GoalAchievement,
    PerformanceDecline,
}

#[derive(Debug)]
pub enum HealthAlertType {
    OvertrainingRisk,
    NegativeTsb,
    InjuryRisk,
}

#[derive(Debug)]
pub enum MotivationNotificationType {
    WeeklyProgressSummary,
    AchievementBadge,
    TrainingStreak,
}

#[derive(Debug)]
struct MockWorkout {
    id: Uuid,
    name: String,
    workout_type: String,
    scheduled_at: DateTime<Utc>,
    duration_minutes: u32,
}

#[derive(Debug)]
struct EmailService;

impl EmailService {
    fn new() -> Self {
        Self
    }

    async fn send_email_notification(&self, notification: &Notification) -> Result<(), NotificationError> {
        // Mock email sending
        tracing::info!("Sending email notification {} to user {}", notification.id, notification.user_id);
        Ok(())
    }
}

#[derive(Debug)]
struct PushNotificationService;

impl PushNotificationService {
    fn new() -> Self {
        Self
    }

    async fn send_push_notification(&self, notification: &Notification) -> Result<(), NotificationError> {
        // Mock push notification sending
        tracing::info!("Sending push notification {} to user {}", notification.id, notification.user_id);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("All delivery channels are disabled for this notification type")]
    AllChannelsDisabled,
    #[error("This notification type is disabled for the user")]
    NotificationDisabled,
    #[error("Email service error: {0}")]
    EmailService(String),
    #[error("Push notification service error: {0}")]
    PushService(String),
    #[error("Invalid notification data: {0}")]
    InvalidData(String),
}