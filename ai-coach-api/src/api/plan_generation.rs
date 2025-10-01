use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
    Router,
};
use axum_extra::extract::WithRejection;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{AuthService, Claims};
use crate::models::{
    GeneratedPlan, PlanGenerationRequest, UserTrainingPreferences, TrainingConstraints,
    PlanAdaptation, PlanAlternative, CoachingInsight, AdaptationType
};
use crate::services::PlanGenerationService;

#[derive(Debug, Deserialize)]
pub struct PlanQuery {
    pub status: Option<String>,
    pub plan_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AdaptPlanRequest {
    pub adaptation_type: AdaptationType,
    pub trigger_reason: String,
}

#[derive(Debug, Serialize)]
pub struct PlanResponse {
    pub plan: GeneratedPlan,
    pub weeks_remaining: Option<i64>,
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
pub struct PlanGenerationAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub plan_generation_service: PlanGenerationService,
}

pub fn plan_generation_routes(db: PgPool, auth_service: AuthService) -> Router {
    let plan_generation_service = PlanGenerationService::new(db.clone());
    let shared_state = PlanGenerationAppState {
        db,
        auth_service,
        plan_generation_service,
    };

    Router::new()
        .route("/", get(get_plans).post(generate_plan))
        .route("/:plan_id", get(get_plan))
        .route("/:plan_id/adapt", post(adapt_plan))
        .route("/:plan_id/alternatives", get(get_alternatives).post(generate_alternatives))
        .route("/:plan_id/insights", get(get_insights).post(generate_insights))
        .route("/preferences", get(get_preferences).put(update_preferences))
        .route("/constraints", get(get_constraints).put(update_constraints))
        .with_state(shared_state)
}

/// Get all generated plans for the authenticated user
pub async fn get_plans(
    State(state): State<PlanGenerationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<PlanQuery>,
) -> Result<Json<Vec<GeneratedPlan>>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    // This is a simplified implementation - would need to add filtering logic
    // For now, return empty array as we'd need to implement get_plans_by_user in the service
    Ok(Json(vec![]))
}

/// Get a specific generated plan
pub async fn get_plan(
    State(state): State<PlanGenerationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<PlanResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let plan = state.plan_generation_service
        .get_plan_by_id(plan_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get plan: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve plan")))
        })?;

    let plan = plan.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ApiError::new("PLAN_NOT_FOUND", "Plan not found")))
    })?;

    let weeks_remaining = Some((plan.end_date - chrono::Local::now().naive_local().date()).num_weeks());

    Ok(Json(PlanResponse {
        plan,
        weeks_remaining,
        success: true,
    }))
}

/// Generate a new training plan
pub async fn generate_plan(
    State(state): State<PlanGenerationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<PlanGenerationRequest>,
) -> Result<Json<PlanResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    // Validate request
    if request.goals.is_empty() && request.events.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_REQUEST", "At least one goal or event must be specified")),
        ));
    }

    let plan = state.plan_generation_service
        .generate_plan(user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate plan: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("GENERATION_ERROR", "Failed to generate plan")))
        })?;

    let weeks_remaining = Some((plan.end_date - chrono::Local::now().naive_local().date()).num_weeks());

    Ok(Json(PlanResponse {
        plan,
        weeks_remaining,
        success: true,
    }))
}

/// Adapt an existing plan
pub async fn adapt_plan(
    State(state): State<PlanGenerationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(plan_id): Path<Uuid>,
    Json(request): Json<AdaptPlanRequest>,
) -> Result<Json<PlanResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let plan = state.plan_generation_service
        .adapt_plan(plan_id, user_id, request.trigger_reason, request.adaptation_type)
        .await
        .map_err(|e| {
            tracing::error!("Failed to adapt plan: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("ADAPTATION_ERROR", "Failed to adapt plan")))
        })?;

    let weeks_remaining = Some((plan.end_date - chrono::Local::now().naive_local().date()).num_weeks());

    Ok(Json(PlanResponse {
        plan,
        weeks_remaining,
        success: true,
    }))
}

/// Get plan alternatives
pub async fn get_alternatives(
    State(state): State<PlanGenerationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Vec<PlanAlternative>>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    // Verify plan exists and belongs to user
    let plan = state.plan_generation_service
        .get_plan_by_id(plan_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify plan: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to verify plan")))
        })?;

    if plan.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(ApiError::new("PLAN_NOT_FOUND", "Plan not found"))));
    }

    // For now, return empty array as we'd need to implement get_alternatives_by_plan in the service
    Ok(Json(vec![]))
}

/// Generate plan alternatives
pub async fn generate_alternatives(
    State(state): State<PlanGenerationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Vec<PlanAlternative>>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let alternatives = state.plan_generation_service
        .generate_alternatives(plan_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate alternatives: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("GENERATION_ERROR", "Failed to generate alternatives")))
        })?;

    Ok(Json(alternatives))
}

/// Get coaching insights for a plan
pub async fn get_insights(
    State(state): State<PlanGenerationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Vec<CoachingInsight>>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    // Verify plan exists and belongs to user
    let plan = state.plan_generation_service
        .get_plan_by_id(plan_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify plan: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to verify plan")))
        })?;

    if plan.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(ApiError::new("PLAN_NOT_FOUND", "Plan not found"))));
    }

    // For now, return empty array as we'd need to implement get_insights_by_plan in the service
    Ok(Json(vec![]))
}

/// Generate coaching insights for a plan
pub async fn generate_insights(
    State(state): State<PlanGenerationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Vec<CoachingInsight>>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let insights = state.plan_generation_service
        .generate_coaching_insights(plan_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to generate insights: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("GENERATION_ERROR", "Failed to generate insights")))
        })?;

    Ok(Json(insights))
}

/// Get user training preferences
pub async fn get_preferences(
    State(state): State<PlanGenerationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<UserTrainingPreferences>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    // Get preferences (this will return defaults if none exist)
    let preferences = state.plan_generation_service
        .get_user_preferences(user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get preferences: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve preferences")))
        })?;

    Ok(Json(preferences))
}

/// Update user training preferences
pub async fn update_preferences(
    State(state): State<PlanGenerationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(preferences): Json<UserTrainingPreferences>,
) -> Result<Json<UserTrainingPreferences>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let updated_preferences = state.plan_generation_service
        .update_user_preferences(user_id, preferences)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update preferences: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to update preferences")))
        })?;

    Ok(Json(updated_preferences))
}

/// Get user training constraints
pub async fn get_constraints(
    State(state): State<PlanGenerationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<TrainingConstraints>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    // Get constraints (this will return defaults if none exist)
    let constraints = state.plan_generation_service
        .get_user_constraints(user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get constraints: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve constraints")))
        })?;

    Ok(Json(constraints))
}

/// Update user training constraints
pub async fn update_constraints(
    State(state): State<PlanGenerationAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(constraints): Json<TrainingConstraints>,
) -> Result<Json<TrainingConstraints>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let updated_constraints = state.plan_generation_service
        .update_user_constraints(user_id, constraints)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update constraints: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to update constraints")))
        })?;

    Ok(Json(updated_constraints))
}