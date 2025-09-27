use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use axum_extra::extract::WithRejection;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use chrono;

use crate::auth::{AuthService, Claims};
use crate::models::{StructuredWorkoutRecommendation, SportType, TrainingZone};
use crate::services::WorkoutRecommendationService;

/// Query parameters for workout recommendations
#[derive(Debug, Deserialize)]
pub struct WorkoutRecommendationQuery {
    pub sport_type: Option<String>,
    pub max_duration_minutes: Option<u32>,
    pub preferred_intensity: Option<String>,
    pub target_date: Option<String>, // ISO date string
    pub equipment: Option<String>,   // Comma-separated list
    pub goals: Option<String>,       // Comma-separated list
    pub recent_workouts: Option<String>, // Comma-separated workout types
}

/// Response wrapper for workout recommendations
#[derive(Debug, Serialize)]
pub struct WorkoutRecommendationResponse {
    pub recommendation: StructuredWorkoutRecommendation,
    pub success: bool,
    pub message: String,
}

/// Response for workout recommendation alternatives
#[derive(Debug, Serialize)]
pub struct WorkoutAlternativesResponse {
    pub alternatives: Vec<StructuredWorkoutRecommendation>,
    pub success: bool,
    pub message: String,
}

/// API Error response
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

/// Workout feedback for improving recommendations
#[derive(Debug, Deserialize)]
pub struct WorkoutFeedback {
    pub recommendation_id: Uuid,
    pub completed: bool,
    pub actual_duration_minutes: Option<u32>,
    pub perceived_difficulty: Option<u8>, // 1-10 scale
    pub energy_level_after: Option<u8>,   // 1-10 scale
    pub enjoyment: Option<u8>,            // 1-10 scale
    pub notes: Option<String>,
}

/// Shared state for workout recommendation API
#[derive(Clone)]
pub struct WorkoutAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub workout_service: WorkoutRecommendationService,
}

/// Create workout recommendation routes
pub fn workout_recommendation_routes(db: PgPool, auth_service: AuthService) -> Router {
    let workout_service = WorkoutRecommendationService::new(db.clone());

    let shared_state = WorkoutAppState {
        db,
        auth_service,
        workout_service,
    };

    Router::new()
        .route("/test", get(test_handler))
        // .route("/recommendation", get(get_workout_recommendation))
        // .route("/alternatives/:recommendation_id", get(get_workout_alternatives))
        // .route("/feedback", post(submit_workout_feedback))
        // .route("/templates", get(get_workout_templates))
        // .route("/zones/:sport_type", get(get_training_zones))
        .with_state(shared_state)
}

/// Get personalized workout recommendation
async fn get_workout_recommendation(
    State(state): State<WorkoutAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<WorkoutRecommendationQuery>,
) -> Result<Json<WorkoutRecommendationResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", &format!("Invalid user ID format: {}", e))),
        )
    })?;

    // Parse sport type
    let sport_type = match query.sport_type.as_deref() {
        Some("cycling") => SportType::Cycling,
        Some("running") => SportType::Running,
        Some("swimming") => SportType::Swimming,
        Some("triathlon") => SportType::Triathlon,
        _ => SportType::Cycling, // Default
    };

    // Parse target date
    let target_date = query.target_date
        .and_then(|date_str| chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok());

    // Parse equipment list
    let available_equipment = query.equipment
        .map(|eq| eq.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    // Parse goals list
    let goals = query.goals
        .map(|g| g.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    // Parse recent workouts
    let recent_workouts = query.recent_workouts
        .map(|rw| rw.split(',').map(|s| s.trim().to_string()).collect());

    let request = crate::services::workout_recommendation_service::WorkoutRecommendationRequest {
        user_id,
        sport_type,
        target_date,
        max_duration_minutes: query.max_duration_minutes,
        preferred_intensity: query.preferred_intensity,
        available_equipment,
        goals,
        recent_workouts,
    };

    match state.workout_service.get_structured_workout_recommendation(request).await {
        Ok(recommendation) => {
            tracing::info!("Generated workout recommendation for user {}", user_id);
            Ok(Json(WorkoutRecommendationResponse {
                recommendation,
                success: true,
                message: "Workout recommendation generated successfully".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to generate workout recommendation: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("WORKOUT_RECOMMENDATION_ERROR", &format!("Failed to generate workout recommendation: {}", e))),
            ))
        }
    }
}

/// Get alternative workout options for a recommendation
async fn get_workout_alternatives(
    State(_state): State<WorkoutAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(recommendation_id): Path<Uuid>,
) -> Result<Json<WorkoutAlternativesResponse>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    // In a full implementation, we would:
    // 1. Fetch the original recommendation from database
    // 2. Generate new alternatives based on different parameters
    // 3. Return the alternatives

    // For now, return a placeholder response
    Ok(Json(WorkoutAlternativesResponse {
        alternatives: vec![],
        success: true,
        message: "Alternative workout recommendations".to_string(),
    }))
}

/// Submit feedback on a completed workout
async fn submit_workout_feedback(
    State(_state): State<WorkoutAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(feedback): Json<WorkoutFeedback>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    // In a full implementation, we would:
    // 1. Store the feedback in the database
    // 2. Use it to improve future recommendations
    // 3. Update user preferences and model training data

    tracing::info!("Received workout feedback for recommendation {}", feedback.recommendation_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Workout feedback submitted successfully"
    })))
}

/// Get available workout templates
async fn get_workout_templates(
    State(_state): State<WorkoutAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<WorkoutTemplateQuery>,
) -> Result<Json<WorkoutTemplatesResponse>, (StatusCode, Json<ApiError>)> {
    // In a full implementation, we would fetch templates from database
    // For now, return basic templates

    let sport_filter = query.sport_type.as_deref().unwrap_or("all");

    let templates = vec![
        WorkoutTemplateInfo {
            id: "endurance_cycling".to_string(),
            name: "Endurance Ride".to_string(),
            sport_type: "cycling".to_string(),
            description: "Steady aerobic base building ride".to_string(),
            estimated_duration_minutes: 90,
            difficulty_score: 4.0,
            primary_energy_system: "aerobic".to_string(),
        },
        WorkoutTemplateInfo {
            id: "threshold_intervals".to_string(),
            name: "Threshold Intervals".to_string(),
            sport_type: "cycling".to_string(),
            description: "High-intensity threshold training".to_string(),
            estimated_duration_minutes: 75,
            difficulty_score: 7.5,
            primary_energy_system: "anaerobic_lactic".to_string(),
        },
        WorkoutTemplateInfo {
            id: "recovery_run".to_string(),
            name: "Recovery Run".to_string(),
            sport_type: "running".to_string(),
            description: "Easy recovery pace run".to_string(),
            estimated_duration_minutes: 45,
            difficulty_score: 2.0,
            primary_energy_system: "aerobic".to_string(),
        },
    ];

    let filtered_templates = if sport_filter == "all" {
        templates
    } else {
        templates.into_iter()
            .filter(|t| t.sport_type == sport_filter)
            .collect()
    };

    Ok(Json(WorkoutTemplatesResponse {
        templates: filtered_templates,
        success: true,
        message: "Workout templates retrieved successfully".to_string(),
    }))
}

/// Get training zones for a specific sport
async fn get_training_zones(
    State(_state): State<WorkoutAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Path(sport_type): Path<String>,
) -> Result<Json<TrainingZonesResponse>, (StatusCode, Json<ApiError>)> {

    let zones = match sport_type.as_str() {
        "cycling" => TrainingZone::cycling_zones(),
        "running" => TrainingZone::running_zones(),
        _ => TrainingZone::cycling_zones(), // Default to cycling
    };

    Ok(Json(TrainingZonesResponse {
        sport_type,
        zones,
        success: true,
        message: "Training zones retrieved successfully".to_string(),
    }))
}

/// Get current champion model - copied from ml_predictions.rs
pub async fn test_handler(
    State(state): State<WorkoutAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let response = serde_json::json!({
        "champion_version": "test",
        "timestamp": chrono::Utc::now()
    });

    Ok(Json(response))
}

/// Query parameters for workout templates
#[derive(Debug, Deserialize)]
pub struct WorkoutTemplateQuery {
    pub sport_type: Option<String>,
    pub difficulty_min: Option<f32>,
    pub difficulty_max: Option<f32>,
    pub duration_min: Option<u32>,
    pub duration_max: Option<u32>,
}

/// Response for workout templates
#[derive(Debug, Serialize)]
pub struct WorkoutTemplatesResponse {
    pub templates: Vec<WorkoutTemplateInfo>,
    pub success: bool,
    pub message: String,
}

/// Simplified workout template info for API responses
#[derive(Debug, Serialize)]
pub struct WorkoutTemplateInfo {
    pub id: String,
    pub name: String,
    pub sport_type: String,
    pub description: String,
    pub estimated_duration_minutes: u32,
    pub difficulty_score: f32,
    pub primary_energy_system: String,
}

/// Response for training zones
#[derive(Debug, Serialize)]
pub struct TrainingZonesResponse {
    pub sport_type: String,
    pub zones: Vec<crate::models::TrainingZone>,
    pub success: bool,
    pub message: String,
}