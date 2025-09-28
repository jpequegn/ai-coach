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
use chrono::NaiveDate;

use crate::auth::{AuthService, Claims};
use crate::models::{
    Event, EventPlan, CreateEventRequest, UpdateEventRequest, CreateEventPlanRequest,
    EventCalendar, EventConflict, EventRecommendation
};
use crate::services::EventService;

#[derive(Debug, Deserialize)]
pub struct EventQuery {
    pub sport: Option<String>,
    pub event_type: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub event: Event,
    pub days_until_event: Option<i64>,
    pub event_plan: Option<EventPlan>,
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
pub struct EventsAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub event_service: EventService,
}

pub fn events_routes(db: PgPool, auth_service: AuthService) -> Router {
    let event_service = EventService::new(db.clone());
    let shared_state = EventsAppState {
        db,
        auth_service,
        event_service,
    };

    Router::new()
        .route("/", get(get_events).post(create_event))
        .route("/:event_id", get(get_event).put(update_event).delete(delete_event))
        .route("/:event_id/plan", post(create_event_plan).get(get_event_plan))
        .route("/calendar", get(get_event_calendar))
        .route("/conflicts", get(get_event_conflicts))
        .route("/recommendations", get(get_event_recommendations))
        .with_state(shared_state)
}

/// Get all events for the authenticated user
pub async fn get_events(
    State(state): State<EventsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<EventQuery>,
) -> Result<Json<Vec<Event>>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let events = state.event_service
        .get_events_by_user(user_id, query.limit, query.offset)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get events: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve events")))
        })?;

    Ok(Json(events))
}

/// Get a specific event
pub async fn get_event(
    State(state): State<EventsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<EventResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let event = state.event_service
        .get_event_by_id(event_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get event: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve event")))
        })?;

    let event = event.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ApiError::new("EVENT_NOT_FOUND", "Event not found")))
    })?;

    let days_until_event = Some((event.event_date - chrono::Local::now().naive_local().date()).num_days());

    // Try to get event plan
    let event_plan = state.event_service
        .get_event_plan(event_id, user_id)
        .await
        .unwrap_or(None);

    Ok(Json(EventResponse {
        event,
        days_until_event,
        event_plan,
        success: true,
    }))
}

/// Create a new event
pub async fn create_event(
    State(state): State<EventsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<CreateEventRequest>,
) -> Result<Json<EventResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    // Validate request
    if request.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_NAME", "Event name cannot be empty")),
        ));
    }

    let event = state.event_service
        .create_event(user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create event: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to create event")))
        })?;

    let days_until_event = Some((event.event_date - chrono::Local::now().naive_local().date()).num_days());

    Ok(Json(EventResponse {
        event,
        days_until_event,
        event_plan: None,
        success: true,
    }))
}

/// Update an existing event
pub async fn update_event(
    State(state): State<EventsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(event_id): Path<Uuid>,
    Json(request): Json<UpdateEventRequest>,
) -> Result<Json<EventResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let event = state.event_service
        .update_event(event_id, user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update event: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to update event")))
        })?;

    let event = event.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ApiError::new("EVENT_NOT_FOUND", "Event not found")))
    })?;

    let days_until_event = Some((event.event_date - chrono::Local::now().naive_local().date()).num_days());

    // Try to get event plan
    let event_plan = state.event_service
        .get_event_plan(event_id, user_id)
        .await
        .unwrap_or(None);

    Ok(Json(EventResponse {
        event,
        days_until_event,
        event_plan,
        success: true,
    }))
}

/// Delete an event
pub async fn delete_event(
    State(state): State<EventsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let deleted = state.event_service
        .delete_event(event_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete event: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to delete event")))
        })?;

    if !deleted {
        return Err((StatusCode::NOT_FOUND, Json(ApiError::new("EVENT_NOT_FOUND", "Event not found"))));
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Event deleted successfully"
    })))
}

/// Create an event training plan
pub async fn create_event_plan(
    State(state): State<EventsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(event_id): Path<Uuid>,
    Json(request): Json<CreateEventPlanRequest>,
) -> Result<Json<EventPlan>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let event_plan = state.event_service
        .create_event_plan(event_id, user_id, request)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create event plan: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to create event plan")))
        })?;

    Ok(Json(event_plan))
}

/// Get an event training plan
pub async fn get_event_plan(
    State(state): State<EventsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<EventPlan>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let event_plan = state.event_service
        .get_event_plan(event_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get event plan: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve event plan")))
        })?;

    let event_plan = event_plan.ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(ApiError::new("EVENT_PLAN_NOT_FOUND", "Event plan not found")))
    })?;

    Ok(Json(event_plan))
}

/// Get event calendar with conflicts and recommendations
pub async fn get_event_calendar(
    State(state): State<EventsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<CalendarQuery>,
) -> Result<Json<EventCalendar>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    let calendar = state.event_service
        .get_event_calendar(user_id, query.start_date, query.end_date)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get event calendar: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve event calendar")))
        })?;

    Ok(Json(calendar))
}

/// Get event conflicts
pub async fn get_event_conflicts(
    State(state): State<EventsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<Vec<EventConflict>>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    // Get calendar for next 6 months to detect conflicts
    let start_date = chrono::Local::now().naive_local().date();
    let end_date = start_date + chrono::Duration::days(180);

    let calendar = state.event_service
        .get_event_calendar(user_id, start_date, end_date)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get event calendar for conflicts: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve conflicts")))
        })?;

    Ok(Json(calendar.conflicts))
}

/// Get event recommendations
pub async fn get_event_recommendations(
    State(state): State<EventsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<Vec<EventRecommendation>>, (StatusCode, Json<ApiError>)> {
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new("INVALID_USER_ID", "Invalid user ID")))
    })?;

    // Get calendar for next 6 months to generate recommendations
    let start_date = chrono::Local::now().naive_local().date();
    let end_date = start_date + chrono::Duration::days(180);

    let calendar = state.event_service
        .get_event_calendar(user_id, start_date, end_date)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get event calendar for recommendations: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError::new("DATABASE_ERROR", "Failed to retrieve recommendations")))
        })?;

    Ok(Json(calendar.recommendations))
}