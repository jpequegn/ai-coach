use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use axum_extra::extract::WithRejection;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{NaiveDate, DateTime, Utc};

use crate::auth::{AuthService, Claims};
use crate::models::{
    Goal, CreateGoalRequest, UpdateGoalRequest, CreateGoalProgressRequest,
    GoalProgressSummary, GoalRecommendation, GoalProgress
};
use crate::services::GoalService;

// Goal models are now imported from crate::models

#[derive(Debug, Deserialize)]
pub struct GoalQuery {
    pub status: Option<String>,
    pub goal_type: Option<String>,
    pub priority: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct GoalResponse {
    pub goal: Goal,
    pub progress_percentage: Option<f64>,
    pub days_remaining: Option<i64>,
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
pub struct GoalsAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub goal_service: GoalService,
}

pub fn goals_routes(db: PgPool, auth_service: AuthService) -> Router {
    let goal_service = GoalService::new(db.clone());
    let shared_state = GoalsAppState {
        db,
        auth_service,
        goal_service,
    };

    Router::new()
        .route("/", get(get_goals).post(create_goal))
        .route("/:goal_id", get(get_goal).put(update_goal).delete(delete_goal))
        .route("/:goal_id/progress", post(add_progress).get(get_goal_progress))
        .route("/events", get(get_event_goals))
        .route("/summary", get(get_goals_summary))
        .with_state(shared_state)
}

/// Get all goals for the authenticated user
pub async fn get_goals(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<GoalQuery>,
) -> Result<Json<Vec<Goal>>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let goals = state.goal_service
        .get_goals_by_user(user_id, query.status, query.goal_type, query.priority, query.limit, query.offset)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get goals: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve goals")))
        })?;

    Ok(Json(goals))
}

/// Get a specific goal
pub async fn get_goal(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(goal_id): Path<Uuid>,
) -> Result<Json<GoalResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let goal = state.goal_service
        .get_goal_by_id(goal_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get goal: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve goal")))
        })?;

    let goal = goal.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ApiError::new("GOAL_NOT_FOUND", "Goal not found")))
    })?;

    let progress_percentage = goal.target_value.and_then(|target| {
        goal.current_value.map(|current| ((current / target) * 100.0).min(100.0))
    });

    let days_remaining = goal.target_date.map(|target| {
        (target - chrono::Local::now().naive_local().date()).num_days()
    });

    Ok(Json(GoalResponse {
        goal,
        progress_percentage,
        days_remaining,
        success: true,
    }))
}

/// Create a new goal
pub async fn create_goal(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<CreateGoalRequest>,
) -> Result<Json<GoalResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    // Validate request
    if request.title.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_TITLE", "Goal title cannot be empty")),
        ));
    }

    let goal = state.goal_service
        .create_goal(user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create goal: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to create goal")))
        })?;

    let progress_percentage = goal.target_value.and_then(|target| {
        goal.current_value.map(|current| ((current / target) * 100.0).min(100.0))
    });

    let days_remaining = goal.target_date.map(|target| {
        (target - chrono::Local::now().naive_local().date()).num_days()
    });

    Ok(Json(GoalResponse {
        goal,
        progress_percentage,
        days_remaining,
        success: true,
    }))
}

/// Update an existing goal
pub async fn update_goal(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(goal_id): Path<Uuid>,
    Json(request): Json<UpdateGoalRequest>,
) -> Result<Json<GoalResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let goal = state.goal_service
        .update_goal(goal_id, user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update goal: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to update goal")))
        })?;

    let goal = goal.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ApiError::new("GOAL_NOT_FOUND", "Goal not found")))
    })?;

    let progress_percentage = goal.target_value.and_then(|target| {
        goal.current_value.map(|current| ((current / target) * 100.0).min(100.0))
    });

    let days_remaining = goal.target_date.map(|target| {
        (target - chrono::Local::now().naive_local().date()).num_days()
    });

    Ok(Json(GoalResponse {
        goal,
        progress_percentage,
        days_remaining,
        success: true,
    }))
}

/// Delete a goal
pub async fn delete_goal(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(goal_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let deleted = state.goal_service
        .delete_goal(goal_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete goal: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to delete goal")))
        })?;

    if !deleted {
        return Err((StatusCode::NOT_FOUND, Json(ApiError::new("GOAL_NOT_FOUND", "Goal not found"))));
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Goal deleted successfully"
    })))
}

/// Add progress to a goal
pub async fn add_progress(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(goal_id): Path<Uuid>,
    Json(request): Json<CreateGoalProgressRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let progress = state.goal_service
        .add_progress(goal_id, user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add progress: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to add progress")))
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Progress added successfully",
        "progress": progress
    })))
}

/// Get goal progress history
pub async fn get_goal_progress(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(goal_id): Path<Uuid>,
) -> Result<Json<GoalProgressSummary>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let progress_summary = state.goal_service
        .get_goal_progress(goal_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get goal progress: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve progress")))
        })?;

    Ok(Json(progress_summary))
}

/// Get event-specific goals
pub async fn get_event_goals(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<Vec<Goal>>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let goals = state.goal_service
        .get_goals_by_user(user_id, None, Some("event".to_string()), None, None, None)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get event goals: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve event goals")))
        })?;

    Ok(Json(goals))
}

/// Get goals summary for dashboard
pub async fn get_goals_summary(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let summary = state.goal_service
        .get_goals_summary(user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get goals summary: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve goals summary")))
        })?;

    Ok(Json(summary))
}