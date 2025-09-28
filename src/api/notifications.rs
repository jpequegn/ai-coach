use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use axum_extra::extract::WithRejection;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::auth::{AuthService, Claims};
use crate::models::{
    Notification, NotificationType, NotificationCategory, NotificationPriority,
    DeliveryChannel, DeliveryStatus, CreateNotificationRequest,
    NotificationPreferences, UpdateNotificationPreferencesRequest,
    NotificationMetrics,
};
use crate::services::{NotificationService, notification_service::{NotificationError, PerformanceAlertType, HealthAlertType, MotivationNotificationType}};

#[derive(Debug, Deserialize)]
pub struct NotificationQuery {
    pub category: Option<String>,
    pub status: Option<String>,
    pub unread_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNotificationApiRequest {
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub delivery_channels: Vec<DeliveryChannel>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct MarkAsReadRequest {
    pub notification_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct TestNotificationRequest {
    pub notification_type: NotificationType,
    pub delivery_channels: Vec<DeliveryChannel>,
}

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub notification: Notification,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct NotificationsListResponse {
    pub notifications: Vec<Notification>,
    pub total_count: i64,
    pub unread_count: i64,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct NotificationPreferencesResponse {
    pub preferences: NotificationPreferences,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct NotificationMetricsResponse {
    pub metrics: NotificationMetrics,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error_code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            error_code: code.to_string(),
            message: message.to_string(),
            details: None,
        }
    }
}

#[derive(Clone)]
pub struct NotificationAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub notification_service: NotificationService,
}

pub fn notification_routes(db: PgPool, auth_service: AuthService) -> Router {
    let notification_service = NotificationService::new(db.clone());

    let shared_state = NotificationAppState {
        db,
        auth_service,
        notification_service,
    };

    Router::new()
        .route("/", get(get_notifications).post(create_notification))
        .route("/unread", get(get_unread_notifications))
        .route("/mark-read", post(mark_notifications_as_read))
        .route("/:notification_id", get(get_notification).delete(delete_notification))
        .route("/:notification_id/read", post(mark_notification_as_read))
        .route("/preferences", get(get_notification_preferences).put(update_notification_preferences))
        .route("/test", post(send_test_notification))
        .route("/metrics", get(get_notification_metrics))
        .route("/schedule/training-reminders", post(schedule_training_reminders))
        .route("/alerts/performance", post(create_performance_alert))
        .route("/alerts/health", post(create_health_alert))
        .route("/alerts/motivation", post(create_motivation_alert))
        .with_state(shared_state)
}

/// Get all notifications for the authenticated user
pub async fn get_notifications(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<NotificationQuery>,
) -> Result<Json<NotificationsListResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    // Mock implementation - in real code, this would query the database
    let notifications = vec![];
    let total_count = 0;
    let unread_count = 0;

    tracing::info!("Retrieved {} notifications for user {}", notifications.len(), user_id);

    Ok(Json(NotificationsListResponse {
        notifications,
        total_count,
        unread_count,
        success: true,
    }))
}

/// Get unread notifications for the authenticated user
pub async fn get_unread_notifications(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<NotificationsListResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    // Mock implementation
    let notifications = vec![];
    let unread_count = notifications.len() as i64;

    Ok(Json(NotificationsListResponse {
        notifications,
        total_count: unread_count,
        unread_count,
        success: true,
    }))
}

/// Get a specific notification
pub async fn get_notification(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(notification_id): Path<Uuid>,
) -> Result<Json<NotificationResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    // Mock notification
    let notification = Notification {
        id: notification_id,
        user_id,
        notification_type: NotificationType::WorkoutReminder,
        category: NotificationCategory::Training,
        priority: NotificationPriority::Medium,
        title: "Workout Reminder".to_string(),
        message: "Your workout is scheduled in 60 minutes".to_string(),
        data: None,
        scheduled_at: Utc::now(),
        sent_at: Some(Utc::now()),
        read_at: None,
        delivery_channels: vec![DeliveryChannel::InApp, DeliveryChannel::Email],
        delivery_status: DeliveryStatus::Delivered,
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Ok(Json(NotificationResponse {
        notification,
        success: true,
    }))
}

/// Create a new notification
pub async fn create_notification(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<CreateNotificationApiRequest>,
) -> Result<Json<NotificationResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    let create_request = CreateNotificationRequest {
        user_id,
        notification_type: request.notification_type,
        title: request.title,
        message: request.message,
        data: request.data,
        scheduled_at: request.scheduled_at,
        delivery_channels: request.delivery_channels,
        expires_at: request.expires_at,
    };

    match state.notification_service.create_notification(create_request).await {
        Ok(notification) => {
            tracing::info!("Created notification {} for user {}", notification.id, user_id);
            Ok(Json(NotificationResponse {
                notification,
                success: true,
            }))
        },
        Err(e) => {
            tracing::error!("Failed to create notification: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("NOTIFICATION_CREATION_FAILED", &format!("Failed to create notification: {}", e))),
            ))
        }
    }
}

/// Mark notifications as read
pub async fn mark_notifications_as_read(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<MarkAsReadRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    // Mock implementation
    tracing::info!("Marked {} notifications as read for user {}", request.notification_ids.len(), user_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "marked_count": request.notification_ids.len()
    })))
}

/// Mark a single notification as read
pub async fn mark_notification_as_read(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(notification_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    tracing::info!("Marked notification {} as read for user {}", notification_id, user_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Notification marked as read"
    })))
}

/// Delete a notification
pub async fn delete_notification(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(notification_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    tracing::info!("Deleted notification {} for user {}", notification_id, user_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Notification deleted"
    })))
}

/// Get notification preferences
pub async fn get_notification_preferences(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<NotificationPreferencesResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    match state.notification_service.get_user_preferences(user_id).await {
        Ok(preferences) => {
            Ok(Json(NotificationPreferencesResponse {
                preferences,
                success: true,
            }))
        },
        Err(e) => {
            tracing::error!("Failed to get notification preferences: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("PREFERENCES_RETRIEVAL_FAILED", &format!("Failed to get preferences: {}", e))),
            ))
        }
    }
}

/// Update notification preferences
pub async fn update_notification_preferences(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<UpdateNotificationPreferencesRequest>,
) -> Result<Json<NotificationPreferencesResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    match state.notification_service.update_user_preferences(user_id, request).await {
        Ok(preferences) => {
            tracing::info!("Updated notification preferences for user {}", user_id);
            Ok(Json(NotificationPreferencesResponse {
                preferences,
                success: true,
            }))
        },
        Err(e) => {
            tracing::error!("Failed to update notification preferences: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("PREFERENCES_UPDATE_FAILED", &format!("Failed to update preferences: {}", e))),
            ))
        }
    }
}

/// Send a test notification
pub async fn send_test_notification(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<TestNotificationRequest>,
) -> Result<Json<NotificationResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    let create_request = CreateNotificationRequest {
        user_id,
        notification_type: request.notification_type.clone(),
        title: "Test Notification".to_string(),
        message: format!("This is a test notification of type {:?}", request.notification_type),
        data: Some(serde_json::json!({"test": true})),
        scheduled_at: Some(Utc::now()),
        delivery_channels: request.delivery_channels,
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
    };

    match state.notification_service.create_notification(create_request).await {
        Ok(notification) => {
            tracing::info!("Created test notification {} for user {}", notification.id, user_id);
            Ok(Json(NotificationResponse {
                notification,
                success: true,
            }))
        },
        Err(e) => {
            tracing::error!("Failed to create test notification: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("TEST_NOTIFICATION_FAILED", &format!("Failed to send test notification: {}", e))),
            ))
        }
    }
}

/// Get notification metrics
pub async fn get_notification_metrics(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<NotificationMetricsResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    // Mock metrics
    let metrics = NotificationMetrics {
        user_id: Some(user_id),
        period_start: Utc::now() - chrono::Duration::days(30),
        period_end: Utc::now(),
        total_sent: 45,
        total_delivered: 42,
        total_read: 38,
        delivery_rate: 93.3,
        read_rate: 84.4,
        by_type: std::collections::HashMap::new(),
        by_channel: std::collections::HashMap::new(),
    };

    Ok(Json(NotificationMetricsResponse {
        metrics,
        success: true,
    }))
}

/// Schedule training reminders for the user
pub async fn schedule_training_reminders(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    match state.notification_service.schedule_training_reminders(user_id).await {
        Ok(notifications) => {
            tracing::info!("Scheduled {} training reminders for user {}", notifications.len(), user_id);
            Ok(Json(serde_json::json!({
                "success": true,
                "scheduled_count": notifications.len(),
                "message": "Training reminders scheduled successfully"
            })))
        },
        Err(e) => {
            tracing::error!("Failed to schedule training reminders: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("TRAINING_REMINDERS_FAILED", &format!("Failed to schedule training reminders: {}", e))),
            ))
        }
    }
}

/// Create a performance alert
pub async fn create_performance_alert(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(data): Json<serde_json::Value>,
) -> Result<Json<NotificationResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    // For demonstration, we'll create a fitness improvement alert
    let alert_type = PerformanceAlertType::FitnessImprovement;

    match state.notification_service.create_performance_alert(user_id, alert_type, data).await {
        Ok(notification) => {
            tracing::info!("Created performance alert {} for user {}", notification.id, user_id);
            Ok(Json(NotificationResponse {
                notification,
                success: true,
            }))
        },
        Err(e) => {
            tracing::error!("Failed to create performance alert: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("PERFORMANCE_ALERT_FAILED", &format!("Failed to create performance alert: {}", e))),
            ))
        }
    }
}

/// Create a health alert
pub async fn create_health_alert(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(data): Json<serde_json::Value>,
) -> Result<Json<NotificationResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    // For demonstration, we'll create an overtraining risk alert
    let alert_type = HealthAlertType::OvertrainingRisk;

    match state.notification_service.create_health_alert(user_id, alert_type, data).await {
        Ok(notification) => {
            tracing::info!("Created health alert {} for user {}", notification.id, user_id);
            Ok(Json(NotificationResponse {
                notification,
                success: true,
            }))
        },
        Err(e) => {
            tracing::error!("Failed to create health alert: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("HEALTH_ALERT_FAILED", &format!("Failed to create health alert: {}", e))),
            ))
        }
    }
}

/// Create a motivation alert
pub async fn create_motivation_alert(
    State(state): State<NotificationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(data): Json<serde_json::Value>,
) -> Result<Json<NotificationResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    // For demonstration, we'll create a weekly progress summary
    let motivation_type = MotivationNotificationType::WeeklyProgressSummary;

    match state.notification_service.create_motivation_notification(user_id, motivation_type, data).await {
        Ok(notification) => {
            tracing::info!("Created motivation notification {} for user {}", notification.id, user_id);
            Ok(Json(NotificationResponse {
                notification,
                success: true,
            }))
        },
        Err(e) => {
            tracing::error!("Failed to create motivation notification: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("MOTIVATION_ALERT_FAILED", &format!("Failed to create motivation notification: {}", e))),
            ))
        }
    }
}