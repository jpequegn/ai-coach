use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::{Notification, NotificationType, NotificationPreferences};

#[derive(Debug)]
pub struct EmailNotificationService {
    templates: HashMap<NotificationType, EmailTemplate>,
    smtp_config: SmtpConfig,
}

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
}

#[derive(Debug, Clone)]
pub struct EmailTemplate {
    pub subject_template: String,
    pub text_template: String,
    pub html_template: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailContext {
    pub user_name: String,
    pub notification_title: String,
    pub notification_message: String,
    pub notification_data: Option<serde_json::Value>,
    pub app_name: String,
    pub app_url: String,
    pub unsubscribe_url: String,
    pub current_date: String,
}

impl EmailNotificationService {
    pub fn new(smtp_config: SmtpConfig) -> Self {
        let mut templates = HashMap::new();

        // Initialize email templates for each notification type
        templates.insert(
            NotificationType::WorkoutReminder,
            EmailTemplate {
                subject_template: "🏃‍♂️ Workout Reminder: {{notification_title}}".to_string(),
                text_template: include_str!("../templates/email/workout_reminder.txt").to_string(),
                html_template: include_str!("../templates/email/workout_reminder.html").to_string(),
            }
        );

        templates.insert(
            NotificationType::FitnessImprovement,
            EmailTemplate {
                subject_template: "🎉 Great News: {{notification_title}}".to_string(),
                text_template: include_str!("../templates/email/fitness_improvement.txt").to_string(),
                html_template: include_str!("../templates/email/fitness_improvement.html").to_string(),
            }
        );

        templates.insert(
            NotificationType::GoalAchievement,
            EmailTemplate {
                subject_template: "🏆 Goal Achievement: {{notification_title}}".to_string(),
                text_template: include_str!("../templates/email/goal_achievement.txt").to_string(),
                html_template: include_str!("../templates/email/goal_achievement.html").to_string(),
            }
        );

        templates.insert(
            NotificationType::OvertrainingRisk,
            EmailTemplate {
                subject_template: "⚠️ Health Alert: {{notification_title}}".to_string(),
                text_template: include_str!("../templates/email/overtraining_risk.txt").to_string(),
                html_template: include_str!("../templates/email/overtraining_risk.html").to_string(),
            }
        );

        templates.insert(
            NotificationType::WeeklyProgressSummary,
            EmailTemplate {
                subject_template: "📊 Your Weekly Training Summary".to_string(),
                text_template: include_str!("../templates/email/weekly_summary.txt").to_string(),
                html_template: include_str!("../templates/email/weekly_summary.html").to_string(),
            }
        );

        Self {
            templates,
            smtp_config,
        }
    }

    pub async fn send_notification_email(
        &self,
        notification: &Notification,
        user_email: &str,
        user_name: &str,
        user_preferences: &NotificationPreferences,
    ) -> Result<(), EmailError> {
        if !user_preferences.email_enabled {
            return Err(EmailError::EmailDisabled);
        }

        let template = self.templates.get(&notification.notification_type)
            .ok_or(EmailError::TemplateNotFound)?;

        let context = self.create_email_context(notification, user_name)?;

        let subject = self.render_template(&template.subject_template, &context)?;
        let text_body = self.render_template(&template.text_template, &context)?;
        let html_body = self.render_template(&template.html_template, &context)?;

        self.send_email(
            user_email,
            &subject,
            &text_body,
            &html_body,
        ).await?;

        tracing::info!("Sent email notification {} to {}", notification.id, user_email);
        Ok(())
    }

    fn create_email_context(&self, notification: &Notification, user_name: &str) -> Result<EmailContext, EmailError> {
        Ok(EmailContext {
            user_name: user_name.to_string(),
            notification_title: notification.title.clone(),
            notification_message: notification.message.clone(),
            notification_data: notification.data.clone(),
            app_name: "AI Coach".to_string(),
            app_url: "https://ai-coach.app".to_string(),
            unsubscribe_url: format!("https://ai-coach.app/unsubscribe?user_id={}", notification.user_id),
            current_date: Utc::now().format("%B %d, %Y").to_string(),
        })
    }

    fn render_template(&self, template: &str, context: &EmailContext) -> Result<String, EmailError> {
        let mut rendered = template.to_string();

        // Simple template rendering (in production, you'd use a proper template engine like Handlebars)
        rendered = rendered.replace("{{user_name}}", &context.user_name);
        rendered = rendered.replace("{{notification_title}}", &context.notification_title);
        rendered = rendered.replace("{{notification_message}}", &context.notification_message);
        rendered = rendered.replace("{{app_name}}", &context.app_name);
        rendered = rendered.replace("{{app_url}}", &context.app_url);
        rendered = rendered.replace("{{unsubscribe_url}}", &context.unsubscribe_url);
        rendered = rendered.replace("{{current_date}}", &context.current_date);

        // Handle notification data if present
        if let Some(data) = &context.notification_data {
            if let Ok(data_str) = serde_json::to_string_pretty(data) {
                rendered = rendered.replace("{{notification_data}}", &data_str);
            }
        }

        Ok(rendered)
    }

    async fn send_email(
        &self,
        to_email: &str,
        subject: &str,
        text_body: &str,
        html_body: &str,
    ) -> Result<(), EmailError> {
        // Mock email sending implementation
        // In production, you would use a library like lettre or an email service like SendGrid

        tracing::info!(
            "Sending email to: {}, subject: {}, config: {:?}",
            to_email,
            subject,
            self.smtp_config.host
        );

        // Simulate email sending delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(())
    }

    pub async fn send_batch_email(
        &self,
        notifications: &[Notification],
        user_email: &str,
        user_name: &str,
        user_preferences: &NotificationPreferences,
    ) -> Result<(), EmailError> {
        if !user_preferences.email_enabled {
            return Err(EmailError::EmailDisabled);
        }

        if notifications.is_empty() {
            return Ok(());
        }

        // Create a digest email with multiple notifications
        let subject = format!("📬 AI Coach Digest - {} new notifications", notifications.len());

        let mut text_body = format!("Hi {},\n\nYou have {} new notifications:\n\n", user_name, notifications.len());
        let mut html_body = format!(
            r#"<html><body><h2>Hi {},</h2><p>You have {} new notifications:</p><ul>"#,
            user_name, notifications.len()
        );

        for notification in notifications {
            text_body.push_str(&format!(
                "• {}: {}\n",
                notification.title,
                notification.message
            ));

            html_body.push_str(&format!(
                r#"<li><strong>{}:</strong> {}</li>"#,
                notification.title,
                notification.message
            ));
        }

        text_body.push_str(&format!(
            "\n\nBest regards,\nAI Coach Team\n\nUnsubscribe: https://ai-coach.app/unsubscribe?user_id={}"
        , notifications[0].user_id));

        html_body.push_str(&format!(
            r#"</ul><p>Best regards,<br>AI Coach Team</p><p><a href="https://ai-coach.app/unsubscribe?user_id={}">Unsubscribe</a></p></body></html>"#,
            notifications[0].user_id
        ));

        self.send_email(user_email, &subject, &text_body, &html_body).await?;

        tracing::info!("Sent batch email with {} notifications to {}", notifications.len(), user_email);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EmailError {
    #[error("Email notifications are disabled for this user")]
    EmailDisabled,
    #[error("Email template not found for notification type")]
    TemplateNotFound,
    #[error("Template rendering failed: {0}")]
    TemplateRenderingFailed(String),
    #[error("SMTP connection failed: {0}")]
    SmtpConnectionFailed(String),
    #[error("Email sending failed: {0}")]
    EmailSendingFailed(String),
    #[error("Invalid email address: {0}")]
    InvalidEmailAddress(String),
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 587,
            username: "ai-coach".to_string(),
            password: "password".to_string(),
            from_email: "noreply@ai-coach.app".to_string(),
            from_name: "AI Coach".to_string(),
        }
    }
}