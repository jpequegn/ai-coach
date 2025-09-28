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

#[derive(Debug, Serialize, Deserialize)]
pub struct Goal {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: String,
    pub goal_type: GoalType,
    pub target_value: Option<f64>,
    pub current_value: Option<f64>,
    pub unit: Option<String>,
    pub target_date: Option<NaiveDate>,
    pub status: GoalStatus,
    pub priority: GoalPriority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalType {
    Performance,
    Fitness,
    Weight,
    Event,
    Training,
    Custom,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    Active,
    Completed,
    Paused,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Deserialize)]
pub struct CreateGoalRequest {
    pub title: String,
    pub description: String,
    pub goal_type: GoalType,
    pub target_value: Option<f64>,
    pub unit: Option<String>,
    pub target_date: Option<NaiveDate>,
    pub priority: GoalPriority,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGoalRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub target_value: Option<f64>,
    pub current_value: Option<f64>,
    pub target_date: Option<NaiveDate>,
    pub status: Option<GoalStatus>,
    pub priority: Option<GoalPriority>,
}

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
pub struct GoalProgressResponse {
    pub goal_id: Uuid,
    pub progress_entries: Vec<ProgressEntry>,
    pub trend: TrendDirection,
    pub projected_completion_date: Option<NaiveDate>,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct ProgressEntry {
    pub date: NaiveDate,
    pub value: f64,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Improving,
    Stable,
    Declining,
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct AddProgressRequest {
    pub value: f64,
    pub date: Option<NaiveDate>,
    pub note: Option<String>,
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
}

pub fn goals_routes(db: PgPool, auth_service: AuthService) -> Router {
    let shared_state = GoalsAppState {
        db,
        auth_service,
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
    let user_id = claims.sub;

    // Build dynamic query based on filters
    let mut sql = "SELECT * FROM goals WHERE user_id = $1".to_string();
    let mut params_count = 2;

    if let Some(status) = &query.status {
        sql.push_str(&format!(" AND status = ${}", params_count));
        params_count += 1;
    }

    if let Some(goal_type) = &query.goal_type {
        sql.push_str(&format!(" AND goal_type = ${}", params_count));
        params_count += 1;
    }

    if let Some(priority) = &query.priority {
        sql.push_str(&format!(" AND priority = ${}", params_count));
        params_count += 1;
    }

    sql.push_str(" ORDER BY priority DESC, target_date ASC");

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);
    sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    // For now, return a mock response
    Ok(Json(vec![]))
}

/// Get a specific goal
pub async fn get_goal(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(goal_id): Path<Uuid>,
) -> Result<Json<GoalResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    // Mock response for now
    let goal = Goal {
        id: goal_id,
        user_id: Uuid::parse_str(&user_id).unwrap_or_default(),
        title: "Complete Marathon".to_string(),
        description: "Run a marathon in under 4 hours".to_string(),
        goal_type: GoalType::Event,
        target_value: Some(240.0),
        current_value: Some(270.0),
        unit: Some("minutes".to_string()),
        target_date: Some(chrono::Local::now().naive_local().date()),
        status: GoalStatus::Active,
        priority: GoalPriority::High,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

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
    let user_id = claims.sub;

    // Validate request
    if request.title.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_TITLE", "Goal title cannot be empty")),
        ));
    }

    // Create mock goal for now
    let goal = Goal {
        id: Uuid::new_v4(),
        user_id: Uuid::parse_str(&user_id).unwrap_or_default(),
        title: request.title,
        description: request.description,
        goal_type: request.goal_type,
        target_value: request.target_value,
        current_value: Some(0.0),
        unit: request.unit,
        target_date: request.target_date,
        status: GoalStatus::Active,
        priority: request.priority,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let days_remaining = goal.target_date.map(|target| {
        (target - chrono::Local::now().naive_local().date()).num_days()
    });

    Ok(Json(GoalResponse {
        goal,
        progress_percentage: Some(0.0),
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
    let user_id = claims.sub;

    // Mock response for now
    let goal = Goal {
        id: goal_id,
        user_id: Uuid::parse_str(&user_id).unwrap_or_default(),
        title: request.title.unwrap_or_else(|| "Updated Goal".to_string()),
        description: request.description.unwrap_or_else(|| "Updated description".to_string()),
        goal_type: GoalType::Performance,
        target_value: request.target_value,
        current_value: request.current_value,
        unit: Some("units".to_string()),
        target_date: request.target_date,
        status: request.status.unwrap_or(GoalStatus::Active),
        priority: request.priority.unwrap_or(GoalPriority::Medium),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

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
    let user_id = claims.sub;

    tracing::info!("Deleting goal {} for user {}", goal_id, user_id);

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
    Json(request): Json<AddProgressRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    let date = request.date.unwrap_or_else(|| chrono::Local::now().naive_local().date());

    tracing::info!("Adding progress {} to goal {} on {}", request.value, goal_id, date);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Progress added successfully",
        "new_value": request.value,
        "date": date
    })))
}

/// Get goal progress history
pub async fn get_goal_progress(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(goal_id): Path<Uuid>,
) -> Result<Json<GoalProgressResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    // Mock progress entries
    let progress_entries = vec![
        ProgressEntry {
            date: chrono::Local::now().naive_local().date() - chrono::Duration::days(30),
            value: 10.0,
            note: Some("Started training".to_string()),
        },
        ProgressEntry {
            date: chrono::Local::now().naive_local().date() - chrono::Duration::days(15),
            value: 25.0,
            note: Some("Good progress".to_string()),
        },
        ProgressEntry {
            date: chrono::Local::now().naive_local().date(),
            value: 40.0,
            note: Some("Halfway there!".to_string()),
        },
    ];

    Ok(Json(GoalProgressResponse {
        goal_id,
        progress_entries,
        trend: TrendDirection::Improving,
        projected_completion_date: Some(chrono::Local::now().naive_local().date() + chrono::Duration::days(30)),
        success: true,
    }))
}

/// Get event-specific goals
pub async fn get_event_goals(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<Vec<Goal>>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    // Return goals filtered by event type
    Ok(Json(vec![]))
}

/// Get goals summary for dashboard
pub async fn get_goals_summary(
    State(state): State<GoalsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    Ok(Json(serde_json::json!({
        "total_goals": 5,
        "active_goals": 3,
        "completed_goals": 2,
        "completion_rate": 40.0,
        "upcoming_deadlines": [
            {
                "goal_id": Uuid::new_v4(),
                "title": "Marathon Training",
                "days_remaining": 15
            }
        ],
        "recent_achievements": [
            {
                "goal_id": Uuid::new_v4(),
                "title": "5K Personal Best",
                "completed_at": chrono::Utc::now() - chrono::Duration::days(7)
            }
        ],
        "success": true
    })))
}