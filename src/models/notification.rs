use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub notification_type: NotificationType,
    pub category: NotificationCategory,
    pub priority: NotificationPriority,
    pub title: String,
    pub message: String,
    pub data: Option<serde_json::Value>, // Additional structured data
    pub scheduled_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub read_at: Option<DateTime<Utc>>,
    pub delivery_channels: Vec<DeliveryChannel>,
    pub delivery_status: DeliveryStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "notification_type", rename_all = "snake_case")]
pub enum NotificationType {
    // Training Reminders
    WorkoutReminder,
    RestDayReminder,
    FtpTestSuggestion,

    // Performance Alerts
    FitnessImprovement,
    GoalAchievement,
    PerformanceDecline,

    // Health and Safety Alerts
    OvertrainingRisk,
    NegativeTsbAlert,
    InjuryRisk,

    // Motivation and Engagement
    WeeklyProgressSummary,
    AchievementBadge,
    TrainingStreak,

    // System Notifications
    SystemMaintenance,
    SecurityAlert,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "notification_category", rename_all = "snake_case")]
pub enum NotificationCategory {
    Training,
    Performance,
    Health,
    Motivation,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "notification_priority", rename_all = "snake_case")]
pub enum NotificationPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "delivery_channel", rename_all = "snake_case")]
pub enum DeliveryChannel {
    InApp,
    Email,
    WebPush,
    Sms, // Future implementation
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "delivery_status", rename_all = "snake_case")]
pub enum DeliveryStatus {
    Scheduled,
    Sent,
    Delivered,
    Failed,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateNotificationRequest {
    pub user_id: Uuid,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub delivery_channels: Vec<DeliveryChannel>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub user_id: Uuid,

    // Training Reminder Preferences
    pub workout_reminders: bool,
    pub workout_reminder_advance_minutes: i32, // How many minutes before workout
    pub rest_day_reminders: bool,
    pub ftp_test_reminders: bool,

    // Performance Alert Preferences
    pub fitness_improvement_alerts: bool,
    pub goal_achievement_alerts: bool,
    pub performance_decline_alerts: bool,

    // Health and Safety Alert Preferences
    pub overtraining_risk_alerts: bool,
    pub negative_tsb_alerts: bool,
    pub injury_risk_alerts: bool,

    // Motivation and Engagement Preferences
    pub weekly_progress_summaries: bool,
    pub achievement_badge_alerts: bool,
    pub training_streak_alerts: bool,

    // Delivery Channel Preferences
    pub email_enabled: bool,
    pub web_push_enabled: bool,
    pub in_app_enabled: bool,

    // Timing Preferences
    pub quiet_hours_start: String, // HH:MM format
    pub quiet_hours_end: String,   // HH:MM format
    pub timezone: String,

    // Batching Preferences
    pub batch_notifications: bool,
    pub batch_interval_minutes: i32, // How often to send batched notifications

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateNotificationPreferencesRequest {
    pub workout_reminders: Option<bool>,
    pub workout_reminder_advance_minutes: Option<i32>,
    pub rest_day_reminders: Option<bool>,
    pub ftp_test_reminders: Option<bool>,
    pub fitness_improvement_alerts: Option<bool>,
    pub goal_achievement_alerts: Option<bool>,
    pub performance_decline_alerts: Option<bool>,
    pub overtraining_risk_alerts: Option<bool>,
    pub negative_tsb_alerts: Option<bool>,
    pub injury_risk_alerts: Option<bool>,
    pub weekly_progress_summaries: Option<bool>,
    pub achievement_badge_alerts: Option<bool>,
    pub training_streak_alerts: Option<bool>,
    pub email_enabled: Option<bool>,
    pub web_push_enabled: Option<bool>,
    pub in_app_enabled: Option<bool>,
    pub quiet_hours_start: Option<String>,
    pub quiet_hours_end: Option<String>,
    pub timezone: Option<String>,
    pub batch_notifications: Option<bool>,
    pub batch_interval_minutes: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationTemplate {
    pub id: Uuid,
    pub notification_type: NotificationType,
    pub title_template: String,
    pub message_template: String,
    pub default_channels: Vec<DeliveryChannel>,
    pub default_priority: NotificationPriority,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationBatch {
    pub id: Uuid,
    pub user_id: Uuid,
    pub notification_ids: Vec<Uuid>,
    pub batch_type: BatchType,
    pub title: String,
    pub summary: String,
    pub scheduled_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub delivery_channels: Vec<DeliveryChannel>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "batch_type", rename_all = "snake_case")]
pub enum BatchType {
    Hourly,
    Daily,
    Weekly,
    Custom,
}

// Analytics and metrics
#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationMetrics {
    pub user_id: Option<Uuid>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_sent: i64,
    pub total_delivered: i64,
    pub total_read: i64,
    pub delivery_rate: f64,
    pub read_rate: f64,
    pub by_type: std::collections::HashMap<NotificationType, NotificationTypeMetrics>,
    pub by_channel: std::collections::HashMap<DeliveryChannel, ChannelMetrics>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationTypeMetrics {
    pub sent: i64,
    pub delivered: i64,
    pub read: i64,
    pub delivery_rate: f64,
    pub read_rate: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelMetrics {
    pub sent: i64,
    pub delivered: i64,
    pub failed: i64,
    pub delivery_rate: f64,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            workout_reminders: true,
            workout_reminder_advance_minutes: 60,
            rest_day_reminders: true,
            ftp_test_reminders: true,
            fitness_improvement_alerts: true,
            goal_achievement_alerts: true,
            performance_decline_alerts: true,
            overtraining_risk_alerts: true,
            negative_tsb_alerts: true,
            injury_risk_alerts: true,
            weekly_progress_summaries: true,
            achievement_badge_alerts: true,
            training_streak_alerts: true,
            email_enabled: true,
            web_push_enabled: true,
            in_app_enabled: true,
            quiet_hours_start: "22:00".to_string(),
            quiet_hours_end: "07:00".to_string(),
            timezone: "UTC".to_string(),
            batch_notifications: true,
            batch_interval_minutes: 60,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}