use chrono::{DateTime, NaiveTime, Utc, Duration};
use uuid::Uuid;
use ai_coach::models::*;
use ai_coach::services::NotificationService;

// Import test utilities
use crate::common::MockDataGenerator;

#[cfg(test)]
mod notification_service_tests {
    use super::*;

    #[test]
    fn test_notification_priority_assignment() {
        // Test notification priority assignment based on type
        let priority_mappings = vec![
            (NotificationType::OvertrainingRisk, NotificationPriority::Critical),
            (NotificationType::InjuryRisk, NotificationPriority::Critical),
            (NotificationType::SecurityAlert, NotificationPriority::Critical),
            (NotificationType::GoalAchievement, NotificationPriority::High),
            (NotificationType::FitnessImprovement, NotificationPriority::High),
            (NotificationType::WorkoutReminder, NotificationPriority::Medium),
            (NotificationType::WeeklyProgressSummary, NotificationPriority::Medium),
            (NotificationType::TrainingStreak, NotificationPriority::Low),
            (NotificationType::AchievementBadge, NotificationPriority::Low),
        ];

        for (notification_type, expected_priority) in priority_mappings {
            let calculated_priority = get_priority_for_type(&notification_type);
            assert_eq!(calculated_priority, expected_priority,
                "Priority for {:?} should be {:?}", notification_type, expected_priority);
        }
    }

    #[test]
    fn test_notification_category_assignment() {
        // Test notification category assignment based on type
        let category_mappings = vec![
            (NotificationType::WorkoutReminder, NotificationCategory::Training),
            (NotificationType::RestDayReminder, NotificationCategory::Training),
            (NotificationType::FtpTestSuggestion, NotificationCategory::Training),
            (NotificationType::FitnessImprovement, NotificationCategory::Performance),
            (NotificationType::GoalAchievement, NotificationCategory::Performance),
            (NotificationType::PerformanceDecline, NotificationCategory::Performance),
            (NotificationType::OvertrainingRisk, NotificationCategory::Health),
            (NotificationType::InjuryRisk, NotificationCategory::Health),
            (NotificationType::NegativeTsbAlert, NotificationCategory::Health),
            (NotificationType::WeeklyProgressSummary, NotificationCategory::Motivation),
            (NotificationType::TrainingStreak, NotificationCategory::Motivation),
            (NotificationType::AchievementBadge, NotificationCategory::Motivation),
            (NotificationType::SystemMaintenance, NotificationCategory::System),
            (NotificationType::SecurityAlert, NotificationCategory::System),
        ];

        for (notification_type, expected_category) in category_mappings {
            let calculated_category = get_category_for_type(&notification_type);
            assert_eq!(calculated_category, expected_category,
                "Category for {:?} should be {:?}", notification_type, expected_category);
        }
    }

    #[test]
    fn test_channel_filtering_by_preferences() {
        // Test notification channel filtering based on user preferences
        let preferences = NotificationPreferences {
            user_id: Uuid::new_v4(),
            workout_reminders: true,
            email_enabled: true,
            web_push_enabled: false,
            in_app_enabled: true,
            ..Default::default()
        };

        let requested_channels = vec![
            DeliveryChannel::Email,
            DeliveryChannel::WebPush,
            DeliveryChannel::InApp,
        ];

        let filtered = filter_channels_by_preferences(
            &requested_channels,
            &NotificationType::WorkoutReminder,
            &preferences,
        );

        // Should only include Email and InApp (WebPush disabled)
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&DeliveryChannel::Email));
        assert!(filtered.contains(&DeliveryChannel::InApp));
        assert!(!filtered.contains(&DeliveryChannel::WebPush));
    }

    #[test]
    fn test_notification_type_preferences() {
        // Test notification type preference filtering
        let preferences = NotificationPreferences {
            user_id: Uuid::new_v4(),
            workout_reminders: false, // Disabled
            goal_achievement_alerts: true,
            overtraining_risk_alerts: true,
            email_enabled: true,
            web_push_enabled: true,
            in_app_enabled: true,
            ..Default::default()
        };

        // Workout reminders should be filtered out
        assert!(!should_send_notification(&NotificationType::WorkoutReminder, &preferences));

        // Goal achievement should be allowed
        assert!(should_send_notification(&NotificationType::GoalAchievement, &preferences));

        // Overtraining risk should be allowed
        assert!(should_send_notification(&NotificationType::OvertrainingRisk, &preferences));
    }

    #[test]
    fn test_quiet_hours_checking() {
        // Test quiet hours checking logic
        let preferences = NotificationPreferences {
            user_id: Uuid::new_v4(),
            quiet_hours_start: "22:00".to_string(),
            quiet_hours_end: "07:00".to_string(),
            timezone: "UTC".to_string(),
            ..Default::default()
        };

        // Test times during quiet hours
        let quiet_time1 = create_datetime_with_time(23, 30); // 11:30 PM
        let quiet_time2 = create_datetime_with_time(2, 15);  // 2:15 AM
        let quiet_time3 = create_datetime_with_time(6, 45);  // 6:45 AM

        assert!(is_in_quiet_hours(&quiet_time1, &preferences));
        assert!(is_in_quiet_hours(&quiet_time2, &preferences));
        assert!(is_in_quiet_hours(&quiet_time3, &preferences));

        // Test times outside quiet hours
        let active_time1 = create_datetime_with_time(9, 0);   // 9:00 AM
        let active_time2 = create_datetime_with_time(15, 30); // 3:30 PM
        let active_time3 = create_datetime_with_time(21, 45); // 9:45 PM

        assert!(!is_in_quiet_hours(&active_time1, &preferences));
        assert!(!is_in_quiet_hours(&active_time2, &preferences));
        assert!(!is_in_quiet_hours(&active_time3, &preferences));
    }

    #[test]
    fn test_notification_batching_logic() {
        // Test notification batching logic
        let preferences = NotificationPreferences {
            user_id: Uuid::new_v4(),
            batch_notifications: true,
            batch_interval_minutes: 60,
            ..Default::default()
        };

        let notifications = vec![
            create_test_notification(NotificationType::TrainingStreak),
            create_test_notification(NotificationType::AchievementBadge),
            create_test_notification(NotificationType::WeeklyProgressSummary),
        ];

        let should_batch = should_batch_notifications(&notifications, &preferences);
        assert!(should_batch);

        // Test with critical notifications (should not batch)
        let critical_notifications = vec![
            create_test_notification(NotificationType::OvertrainingRisk),
            create_test_notification(NotificationType::InjuryRisk),
        ];

        let should_not_batch = should_batch_notifications(&critical_notifications, &preferences);
        assert!(!should_not_batch);
    }

    #[test]
    fn test_notification_expiration() {
        // Test notification expiration logic
        let now = Utc::now();
        let expired_notification = Notification {
            expires_at: Some(now - Duration::hours(1)),
            ..create_test_notification(NotificationType::WorkoutReminder)
        };

        let valid_notification = Notification {
            expires_at: Some(now + Duration::hours(1)),
            ..create_test_notification(NotificationType::WorkoutReminder)
        };

        let no_expiry_notification = Notification {
            expires_at: None,
            ..create_test_notification(NotificationType::GoalAchievement)
        };

        assert!(is_notification_expired(&expired_notification));
        assert!(!is_notification_expired(&valid_notification));
        assert!(!is_notification_expired(&no_expiry_notification));
    }

    #[test]
    fn test_delivery_retry_logic() {
        // Test delivery retry logic
        let failed_notification = Notification {
            delivery_status: DeliveryStatus::Failed,
            sent_at: Some(Utc::now() - Duration::minutes(30)),
            ..create_test_notification(NotificationType::WorkoutReminder)
        };

        let recently_failed = Notification {
            delivery_status: DeliveryStatus::Failed,
            sent_at: Some(Utc::now() - Duration::minutes(5)),
            ..create_test_notification(NotificationType::WorkoutReminder)
        };

        assert!(should_retry_delivery(&failed_notification));
        assert!(!should_retry_delivery(&recently_failed));
    }

    #[test]
    fn test_notification_template_processing() {
        // Test notification template processing
        let template = NotificationTemplate {
            id: Uuid::new_v4(),
            notification_type: NotificationType::WorkoutReminder,
            title_template: "Workout Reminder: {{workout_name}}".to_string(),
            message_template: "Your {{workout_type}} workout is scheduled for {{time}}".to_string(),
            default_channels: vec![DeliveryChannel::InApp, DeliveryChannel::Email],
            default_priority: NotificationPriority::Medium,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let context = std::collections::HashMap::from([
            ("workout_name".to_string(), "Threshold Training".to_string()),
            ("workout_type".to_string(), "Interval".to_string()),
            ("time".to_string(), "2:00 PM".to_string()),
        ]);

        let processed = process_template(&template, &context);

        assert_eq!(processed.title, "Workout Reminder: Threshold Training");
        assert_eq!(processed.message, "Your Interval workout is scheduled for 2:00 PM");
    }

    #[test]
    fn test_notification_metrics_calculation() {
        // Test notification metrics calculation
        let notifications = vec![
            Notification {
                delivery_status: DeliveryStatus::Delivered,
                read_at: Some(Utc::now()),
                ..create_test_notification(NotificationType::WorkoutReminder)
            },
            Notification {
                delivery_status: DeliveryStatus::Delivered,
                read_at: None,
                ..create_test_notification(NotificationType::GoalAchievement)
            },
            Notification {
                delivery_status: DeliveryStatus::Failed,
                read_at: None,
                ..create_test_notification(NotificationType::TrainingStreak)
            },
        ];

        let metrics = calculate_notification_metrics(&notifications);

        assert_eq!(metrics.total_sent, 3);
        assert_eq!(metrics.total_delivered, 2);
        assert_eq!(metrics.total_read, 1);
        assert_eq!(metrics.delivery_rate, 66.67); // 2/3 * 100, rounded
        assert_eq!(metrics.read_rate, 50.0); // 1/2 * 100
    }

    // Helper functions for testing

    fn get_priority_for_type(notification_type: &NotificationType) -> NotificationPriority {
        match notification_type {
            NotificationType::OvertrainingRisk
            | NotificationType::InjuryRisk
            | NotificationType::SecurityAlert => NotificationPriority::Critical,

            NotificationType::GoalAchievement
            | NotificationType::FitnessImprovement
            | NotificationType::PerformanceDecline
            | NotificationType::NegativeTsbAlert => NotificationPriority::High,

            NotificationType::WorkoutReminder
            | NotificationType::RestDayReminder
            | NotificationType::FtpTestSuggestion
            | NotificationType::WeeklyProgressSummary => NotificationPriority::Medium,

            NotificationType::TrainingStreak
            | NotificationType::AchievementBadge
            | NotificationType::SystemMaintenance => NotificationPriority::Low,
        }
    }

    fn get_category_for_type(notification_type: &NotificationType) -> NotificationCategory {
        match notification_type {
            NotificationType::WorkoutReminder
            | NotificationType::RestDayReminder
            | NotificationType::FtpTestSuggestion => NotificationCategory::Training,

            NotificationType::FitnessImprovement
            | NotificationType::GoalAchievement
            | NotificationType::PerformanceDecline => NotificationCategory::Performance,

            NotificationType::OvertrainingRisk
            | NotificationType::InjuryRisk
            | NotificationType::NegativeTsbAlert => NotificationCategory::Health,

            NotificationType::WeeklyProgressSummary
            | NotificationType::TrainingStreak
            | NotificationType::AchievementBadge => NotificationCategory::Motivation,

            NotificationType::SystemMaintenance
            | NotificationType::SecurityAlert => NotificationCategory::System,
        }
    }

    fn filter_channels_by_preferences(
        requested: &[DeliveryChannel],
        notification_type: &NotificationType,
        preferences: &NotificationPreferences,
    ) -> Vec<DeliveryChannel> {
        if !should_send_notification(notification_type, preferences) {
            return vec![];
        }

        requested
            .iter()
            .filter(|&channel| match channel {
                DeliveryChannel::Email => preferences.email_enabled,
                DeliveryChannel::WebPush => preferences.web_push_enabled,
                DeliveryChannel::InApp => preferences.in_app_enabled,
                DeliveryChannel::Sms => true, // Future implementation
            })
            .cloned()
            .collect()
    }

    fn should_send_notification(notification_type: &NotificationType, preferences: &NotificationPreferences) -> bool {
        match notification_type {
            NotificationType::WorkoutReminder => preferences.workout_reminders,
            NotificationType::RestDayReminder => preferences.rest_day_reminders,
            NotificationType::FtpTestSuggestion => preferences.ftp_test_reminders,
            NotificationType::FitnessImprovement => preferences.fitness_improvement_alerts,
            NotificationType::GoalAchievement => preferences.goal_achievement_alerts,
            NotificationType::PerformanceDecline => preferences.performance_decline_alerts,
            NotificationType::OvertrainingRisk => preferences.overtraining_risk_alerts,
            NotificationType::NegativeTsbAlert => preferences.negative_tsb_alerts,
            NotificationType::InjuryRisk => preferences.injury_risk_alerts,
            NotificationType::WeeklyProgressSummary => preferences.weekly_progress_summaries,
            NotificationType::AchievementBadge => preferences.achievement_badge_alerts,
            NotificationType::TrainingStreak => preferences.training_streak_alerts,
            // System notifications are always sent
            NotificationType::SystemMaintenance | NotificationType::SecurityAlert => true,
        }
    }

    fn is_in_quiet_hours(time: &DateTime<Utc>, preferences: &NotificationPreferences) -> bool {
        let time = time.time();
        let start = NaiveTime::parse_from_str(&preferences.quiet_hours_start, "%H:%M").unwrap_or_else(|_| NaiveTime::from_hms_opt(22, 0, 0).unwrap());
        let end = NaiveTime::parse_from_str(&preferences.quiet_hours_end, "%H:%M").unwrap_or_else(|_| NaiveTime::from_hms_opt(7, 0, 0).unwrap());

        if start <= end {
            // Quiet hours don't cross midnight
            time >= start && time <= end
        } else {
            // Quiet hours cross midnight
            time >= start || time <= end
        }
    }

    fn create_datetime_with_time(hour: u32, minute: u32) -> DateTime<Utc> {
        let today = chrono::Local::now().naive_local().date();
        let time = NaiveTime::from_hms_opt(hour, minute, 0).unwrap();
        let naive_datetime = today.and_time(time);
        Utc.from_local_datetime(&naive_datetime).unwrap()
    }

    fn create_test_notification(notification_type: NotificationType) -> Notification {
        Notification {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            notification_type,
            category: get_category_for_type(&notification_type),
            priority: get_priority_for_type(&notification_type),
            title: "Test Notification".to_string(),
            message: "Test message".to_string(),
            data: None,
            scheduled_at: Utc::now(),
            sent_at: None,
            read_at: None,
            delivery_channels: vec![DeliveryChannel::InApp],
            delivery_status: DeliveryStatus::Scheduled,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn should_batch_notifications(notifications: &[Notification], preferences: &NotificationPreferences) -> bool {
        if !preferences.batch_notifications {
            return false;
        }

        // Don't batch critical notifications
        !notifications.iter().any(|n| n.priority == NotificationPriority::Critical)
    }

    fn is_notification_expired(notification: &Notification) -> bool {
        if let Some(expires_at) = notification.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    fn should_retry_delivery(notification: &Notification) -> bool {
        if notification.delivery_status != DeliveryStatus::Failed {
            return false;
        }

        if let Some(sent_at) = notification.sent_at {
            // Retry after 15 minutes
            Utc::now() - sent_at > Duration::minutes(15)
        } else {
            true
        }
    }

    fn process_template(template: &NotificationTemplate, context: &std::collections::HashMap<String, String>) -> ProcessedNotification {
        let mut title = template.title_template.clone();
        let mut message = template.message_template.clone();

        for (key, value) in context {
            let placeholder = format!("{{{{{}}}}}", key);
            title = title.replace(&placeholder, value);
            message = message.replace(&placeholder, value);
        }

        ProcessedNotification { title, message }
    }

    fn calculate_notification_metrics(notifications: &[Notification]) -> NotificationMetrics {
        let total_sent = notifications.len() as i64;
        let total_delivered = notifications.iter().filter(|n| n.delivery_status == DeliveryStatus::Delivered).count() as i64;
        let total_read = notifications.iter().filter(|n| n.read_at.is_some()).count() as i64;

        let delivery_rate = if total_sent > 0 {
            ((total_delivered as f64 / total_sent as f64) * 100.0 * 100.0).round() / 100.0
        } else {
            0.0
        };

        let read_rate = if total_delivered > 0 {
            ((total_read as f64 / total_delivered as f64) * 100.0 * 100.0).round() / 100.0
        } else {
            0.0
        };

        NotificationMetrics {
            user_id: None,
            period_start: Utc::now() - Duration::days(7),
            period_end: Utc::now(),
            total_sent,
            total_delivered,
            total_read,
            delivery_rate,
            read_rate,
            by_type: std::collections::HashMap::new(),
            by_channel: std::collections::HashMap::new(),
        }
    }

    // Mock struct for testing template processing
    struct ProcessedNotification {
        title: String,
        message: String,
    }
}