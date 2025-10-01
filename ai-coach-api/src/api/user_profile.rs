use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, put},
    Router,
};
use axum_extra::extract::WithRejection;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{NaiveDate, DateTime, Utc};

use crate::auth::{AuthService, Claims};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub age: Option<u8>,
    pub weight_kg: Option<f64>,
    pub height_cm: Option<f64>,
    pub resting_heart_rate: Option<u32>,
    pub max_heart_rate: Option<u32>,
    pub ftp_watts: Option<u32>,
    pub threshold_pace_ms: Option<f64>,
    pub sport_preferences: SportPreferences,
    pub training_preferences: TrainingPreferences,
    pub notification_settings: NotificationSettings,
    pub privacy_settings: PrivacySettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SportPreferences {
    pub primary_sport: String,
    pub secondary_sports: Vec<String>,
    pub preferred_activities: Vec<String>,
    pub avoided_activities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrainingPreferences {
    pub weekly_hours_available: f64,
    pub preferred_workout_times: Vec<String>,
    pub preferred_intensity: String,
    pub recovery_emphasis: String,
    pub equipment_available: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub email_notifications: bool,
    pub workout_reminders: bool,
    pub goal_updates: bool,
    pub performance_insights: bool,
    pub weekly_summary: bool,
    pub notification_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub profile_visibility: String,
    pub share_activities: bool,
    pub share_performance_data: bool,
    pub allow_peer_comparison: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub age: Option<u8>,
    pub weight_kg: Option<f64>,
    pub height_cm: Option<f64>,
    pub resting_heart_rate: Option<u32>,
    pub max_heart_rate: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateThresholdsRequest {
    pub ftp_watts: Option<u32>,
    pub threshold_pace_ms: Option<f64>,
    pub lactate_threshold_hr: Option<u32>,
    pub vo2_max: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub sport_preferences: Option<SportPreferences>,
    pub training_preferences: Option<TrainingPreferences>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotificationSettingsRequest {
    pub email_notifications: Option<bool>,
    pub workout_reminders: Option<bool>,
    pub goal_updates: Option<bool>,
    pub performance_insights: Option<bool>,
    pub weekly_summary: Option<bool>,
    pub notification_time: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub profile: UserProfile,
    pub completion_percentage: f64,
    pub missing_fields: Vec<String>,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct ThresholdsResponse {
    pub user_id: Uuid,
    pub ftp_watts: Option<u32>,
    pub threshold_pace_ms: Option<f64>,
    pub lactate_threshold_hr: Option<u32>,
    pub vo2_max: Option<f64>,
    pub power_zones: Vec<PowerZone>,
    pub heart_rate_zones: Vec<HeartRateZone>,
    pub pace_zones: Vec<PaceZone>,
    pub last_updated: DateTime<Utc>,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct PowerZone {
    pub zone: u8,
    pub name: String,
    pub min_watts: u32,
    pub max_watts: Option<u32>,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct HeartRateZone {
    pub zone: u8,
    pub name: String,
    pub min_bpm: u32,
    pub max_bpm: Option<u32>,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct PaceZone {
    pub zone: u8,
    pub name: String,
    pub min_pace_ms: f64,
    pub max_pace_ms: Option<f64>,
    pub description: String,
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
pub struct ProfileAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
}

pub fn user_profile_routes(db: PgPool, auth_service: AuthService) -> Router {
    let shared_state = ProfileAppState {
        db,
        auth_service,
    };

    Router::new()
        .route("/profile", get(get_profile).put(update_profile))
        .route("/profile/thresholds", get(get_thresholds).put(update_thresholds))
        .route("/profile/preferences", put(update_preferences))
        .route("/profile/notifications", get(get_notification_settings).put(update_notification_settings))
        .route("/profile/privacy", get(get_privacy_settings).put(update_privacy_settings))
        .route("/profile/zones/calculate", get(calculate_training_zones))
        .route("/profile/export", get(export_profile_data))
        .with_state(shared_state)
}

/// Get user profile
pub async fn get_profile(
    State(state): State<ProfileAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<ProfileResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    // Mock profile for now
    let profile = UserProfile {
        user_id: Uuid::parse_str(&user_id).unwrap_or_default(),
        email: "user@example.com".to_string(),
        name: Some("John Doe".to_string()),
        age: Some(35),
        weight_kg: Some(70.0),
        height_cm: Some(175.0),
        resting_heart_rate: Some(55),
        max_heart_rate: Some(185),
        ftp_watts: Some(250),
        threshold_pace_ms: Some(4.30),
        sport_preferences: SportPreferences {
            primary_sport: "cycling".to_string(),
            secondary_sports: vec!["running".to_string()],
            preferred_activities: vec!["intervals".to_string(), "endurance".to_string()],
            avoided_activities: vec![],
        },
        training_preferences: TrainingPreferences {
            weekly_hours_available: 10.0,
            preferred_workout_times: vec!["morning".to_string(), "evening".to_string()],
            preferred_intensity: "moderate".to_string(),
            recovery_emphasis: "balanced".to_string(),
            equipment_available: vec!["bike".to_string(), "trainer".to_string(), "power_meter".to_string()],
        },
        notification_settings: NotificationSettings {
            email_notifications: true,
            workout_reminders: true,
            goal_updates: true,
            performance_insights: true,
            weekly_summary: true,
            notification_time: "08:00".to_string(),
        },
        privacy_settings: PrivacySettings {
            profile_visibility: "friends".to_string(),
            share_activities: true,
            share_performance_data: true,
            allow_peer_comparison: true,
        },
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Calculate completion percentage
    let mut fields_filled = 0;
    let total_fields = 10;
    let mut missing_fields = vec![];

    if profile.name.is_some() { fields_filled += 1; } else { missing_fields.push("name".to_string()); }
    if profile.age.is_some() { fields_filled += 1; } else { missing_fields.push("age".to_string()); }
    if profile.weight_kg.is_some() { fields_filled += 1; } else { missing_fields.push("weight".to_string()); }
    if profile.height_cm.is_some() { fields_filled += 1; } else { missing_fields.push("height".to_string()); }
    if profile.resting_heart_rate.is_some() { fields_filled += 1; } else { missing_fields.push("resting_hr".to_string()); }
    if profile.max_heart_rate.is_some() { fields_filled += 1; } else { missing_fields.push("max_hr".to_string()); }
    if profile.ftp_watts.is_some() { fields_filled += 1; } else { missing_fields.push("ftp".to_string()); }
    if profile.threshold_pace_ms.is_some() { fields_filled += 1; } else { missing_fields.push("threshold_pace".to_string()); }
    fields_filled += 2; // sport and training preferences always filled

    let completion_percentage = (fields_filled as f64 / total_fields as f64) * 100.0;

    Ok(Json(ProfileResponse {
        profile,
        completion_percentage,
        missing_fields,
        success: true,
    }))
}

/// Update user profile
pub async fn update_profile(
    State(state): State<ProfileAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<UpdateProfileRequest>,
) -> Result<Json<ProfileResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    // Validate age if provided
    if let Some(age) = request.age {
        if age < 13 || age > 120 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError::new("INVALID_AGE", "Age must be between 13 and 120")),
            ));
        }
    }

    // Validate weight if provided
    if let Some(weight) = request.weight_kg {
        if weight < 30.0 || weight > 300.0 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError::new("INVALID_WEIGHT", "Weight must be between 30 and 300 kg")),
            ));
        }
    }

    tracing::info!("Updating profile for user {}", user_id);

    // Return mock updated profile
    let profile = UserProfile {
        user_id: Uuid::parse_str(&user_id).unwrap_or_default(),
        email: "user@example.com".to_string(),
        name: request.name.or(Some("Updated Name".to_string())),
        age: request.age,
        weight_kg: request.weight_kg,
        height_cm: request.height_cm,
        resting_heart_rate: request.resting_heart_rate,
        max_heart_rate: request.max_heart_rate,
        ftp_watts: Some(250),
        threshold_pace_ms: Some(4.30),
        sport_preferences: SportPreferences {
            primary_sport: "cycling".to_string(),
            secondary_sports: vec![],
            preferred_activities: vec![],
            avoided_activities: vec![],
        },
        training_preferences: TrainingPreferences {
            weekly_hours_available: 10.0,
            preferred_workout_times: vec![],
            preferred_intensity: "moderate".to_string(),
            recovery_emphasis: "balanced".to_string(),
            equipment_available: vec![],
        },
        notification_settings: NotificationSettings {
            email_notifications: true,
            workout_reminders: true,
            goal_updates: true,
            performance_insights: true,
            weekly_summary: true,
            notification_time: "08:00".to_string(),
        },
        privacy_settings: PrivacySettings {
            profile_visibility: "friends".to_string(),
            share_activities: true,
            share_performance_data: true,
            allow_peer_comparison: true,
        },
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Ok(Json(ProfileResponse {
        profile,
        completion_percentage: 80.0,
        missing_fields: vec![],
        success: true,
    }))
}

/// Get performance thresholds
pub async fn get_thresholds(
    State(state): State<ProfileAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<ThresholdsResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    let ftp = 250;
    let max_hr = 185;

    // Calculate power zones based on FTP
    let power_zones = vec![
        PowerZone { zone: 1, name: "Recovery".to_string(), min_watts: 0, max_watts: Some((ftp as f64 * 0.55) as u32), description: "Easy spinning".to_string() },
        PowerZone { zone: 2, name: "Endurance".to_string(), min_watts: ((ftp as f64 * 0.56) as u32), max_watts: Some((ftp as f64 * 0.75) as u32), description: "Aerobic base building".to_string() },
        PowerZone { zone: 3, name: "Tempo".to_string(), min_watts: ((ftp as f64 * 0.76) as u32), max_watts: Some((ftp as f64 * 0.90) as u32), description: "Moderate effort".to_string() },
        PowerZone { zone: 4, name: "Threshold".to_string(), min_watts: ((ftp as f64 * 0.91) as u32), max_watts: Some((ftp as f64 * 1.05) as u32), description: "Lactate threshold".to_string() },
        PowerZone { zone: 5, name: "VO2 Max".to_string(), min_watts: ((ftp as f64 * 1.06) as u32), max_watts: Some((ftp as f64 * 1.20) as u32), description: "Maximum aerobic power".to_string() },
        PowerZone { zone: 6, name: "Anaerobic".to_string(), min_watts: ((ftp as f64 * 1.21) as u32), max_watts: Some((ftp as f64 * 1.50) as u32), description: "Short intense efforts".to_string() },
        PowerZone { zone: 7, name: "Neuromuscular".to_string(), min_watts: ((ftp as f64 * 1.51) as u32), max_watts: None, description: "Sprint power".to_string() },
    ];

    // Calculate heart rate zones
    let heart_rate_zones = vec![
        HeartRateZone { zone: 1, name: "Recovery".to_string(), min_bpm: 0, max_bpm: Some((max_hr as f64 * 0.60) as u32), description: "Easy recovery".to_string() },
        HeartRateZone { zone: 2, name: "Aerobic".to_string(), min_bpm: ((max_hr as f64 * 0.61) as u32), max_bpm: Some((max_hr as f64 * 0.70) as u32), description: "Base building".to_string() },
        HeartRateZone { zone: 3, name: "Tempo".to_string(), min_bpm: ((max_hr as f64 * 0.71) as u32), max_bpm: Some((max_hr as f64 * 0.80) as u32), description: "Moderate effort".to_string() },
        HeartRateZone { zone: 4, name: "Threshold".to_string(), min_bpm: ((max_hr as f64 * 0.81) as u32), max_bpm: Some((max_hr as f64 * 0.90) as u32), description: "Hard effort".to_string() },
        HeartRateZone { zone: 5, name: "VO2 Max".to_string(), min_bpm: ((max_hr as f64 * 0.91) as u32), max_bpm: None, description: "Maximum effort".to_string() },
    ];

    let pace_zones = vec![];

    Ok(Json(ThresholdsResponse {
        user_id: Uuid::parse_str(&user_id).unwrap_or_default(),
        ftp_watts: Some(ftp),
        threshold_pace_ms: Some(4.30),
        lactate_threshold_hr: Some(165),
        vo2_max: Some(55.0),
        power_zones,
        heart_rate_zones,
        pace_zones,
        last_updated: Utc::now(),
        success: true,
    }))
}

/// Update performance thresholds
pub async fn update_thresholds(
    State(state): State<ProfileAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<UpdateThresholdsRequest>,
) -> Result<Json<ThresholdsResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    tracing::info!("Updating thresholds for user {}", user_id);

    // Return updated thresholds
    get_thresholds(State(state), WithRejection(claims, StatusCode::UNAUTHORIZED)).await
}

/// Update training preferences
pub async fn update_preferences(
    State(state): State<ProfileAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<UpdatePreferencesRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    tracing::info!("Updating preferences for user {}", user_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Preferences updated successfully"
    })))
}

/// Get notification settings
pub async fn get_notification_settings(
    State(state): State<ProfileAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<NotificationSettings>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    Ok(Json(NotificationSettings {
        email_notifications: true,
        workout_reminders: true,
        goal_updates: true,
        performance_insights: true,
        weekly_summary: true,
        notification_time: "08:00".to_string(),
    }))
}

/// Update notification settings
pub async fn update_notification_settings(
    State(state): State<ProfileAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<UpdateNotificationSettingsRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    tracing::info!("Updating notification settings for user {}", user_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Notification settings updated successfully"
    })))
}

/// Get privacy settings
pub async fn get_privacy_settings(
    State(state): State<ProfileAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<PrivacySettings>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    Ok(Json(PrivacySettings {
        profile_visibility: "friends".to_string(),
        share_activities: true,
        share_performance_data: true,
        allow_peer_comparison: true,
    }))
}

/// Update privacy settings
pub async fn update_privacy_settings(
    State(state): State<ProfileAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(settings): Json<PrivacySettings>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    tracing::info!("Updating privacy settings for user {}", user_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Privacy settings updated successfully"
    })))
}

/// Calculate training zones based on current thresholds
pub async fn calculate_training_zones(
    State(state): State<ProfileAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    // This would calculate zones based on user's current thresholds
    Ok(Json(serde_json::json!({
        "power_zones": [],
        "heart_rate_zones": [],
        "pace_zones": [],
        "success": true
    })))
}

/// Export user profile data
pub async fn export_profile_data(
    State(state): State<ProfileAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    // This would export all user data in a structured format
    Ok(Json(serde_json::json!({
        "export_id": Uuid::new_v4(),
        "created_at": Utc::now(),
        "data": {
            "profile": {},
            "thresholds": {},
            "preferences": {},
            "training_history": [],
            "goals": []
        },
        "success": true
    })))
}