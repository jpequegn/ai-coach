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
pub struct TrainingPlan {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub goal_event: Option<String>,
    pub plan_type: PlanType,
    pub status: PlanStatus,
    pub weeks: Vec<TrainingWeek>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanType {
    Base,
    Build,
    Peak,
    Race,
    Maintenance,
    Custom,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Draft,
    Active,
    Completed,
    Paused,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrainingWeek {
    pub week_number: u8,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub focus: String,
    pub target_tss: f64,
    pub target_hours: f64,
    pub workouts: Vec<PlannedWorkout>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlannedWorkout {
    pub id: Uuid,
    pub day_of_week: u8,
    pub workout_type: String,
    pub duration_minutes: u32,
    pub intensity: String,
    pub description: String,
    pub target_tss: Option<f64>,
    pub completed: bool,
    pub actual_workout_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePlanRequest {
    pub name: String,
    pub description: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub goal_event: Option<String>,
    pub plan_type: PlanType,
    pub weekly_hours_available: f64,
    pub current_fitness_level: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlanRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub end_date: Option<NaiveDate>,
    pub status: Option<PlanStatus>,
}

#[derive(Debug, Serialize)]
pub struct CoachingRecommendation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub recommendation_type: RecommendationType,
    pub priority: Priority,
    pub title: String,
    pub description: String,
    pub action_items: Vec<String>,
    pub rationale: String,
    pub expected_benefit: String,
    pub created_at: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationType {
    Training,
    Recovery,
    Nutrition,
    Equipment,
    Technique,
    Goal,
    Health,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Serialize)]
pub struct AdaptivePlanResponse {
    pub plan_id: Uuid,
    pub adaptations: Vec<PlanAdaptation>,
    pub current_status: PlanProgress,
    pub recommendations: Vec<String>,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct PlanAdaptation {
    pub date: NaiveDate,
    pub adaptation_type: String,
    pub reason: String,
    pub changes: Vec<String>,
    pub impact: String,
}

#[derive(Debug, Serialize)]
pub struct PlanProgress {
    pub completion_percentage: f64,
    pub adherence_rate: f64,
    pub tss_achievement: f64,
    pub current_week: u8,
    pub total_weeks: u8,
    pub on_track: bool,
}

#[derive(Debug, Serialize)]
pub struct CoachingInsight {
    pub insight_type: String,
    pub title: String,
    pub description: String,
    pub data_points: Vec<DataPoint>,
    pub confidence: f64,
    pub action_required: bool,
}

#[derive(Debug, Serialize)]
pub struct DataPoint {
    pub metric: String,
    pub value: f64,
    pub unit: String,
    pub trend: String,
}

#[derive(Debug, Deserialize)]
pub struct PlanQuery {
    pub status: Option<String>,
    pub include_completed: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
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
pub struct CoachingAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
}

pub fn coaching_routes(db: PgPool, auth_service: AuthService) -> Router {
    let shared_state = CoachingAppState {
        db,
        auth_service,
    };

    Router::new()
        .route("/plans", get(get_training_plans).post(create_training_plan))
        .route("/plans/:plan_id", get(get_training_plan).put(update_training_plan).delete(delete_training_plan))
        .route("/plans/:plan_id/adapt", post(adapt_training_plan))
        .route("/plans/:plan_id/progress", get(get_plan_progress))
        .route("/plans/:plan_id/workouts", get(get_plan_workouts))
        .route("/recommendations", get(get_recommendations))
        .route("/recommendations/:recommendation_id/dismiss", post(dismiss_recommendation))
        .route("/insights", get(get_coaching_insights))
        .route("/insights/weekly-summary", get(get_weekly_summary))
        .route("/guidance/next-workout", get(get_next_workout_guidance))
        .route("/guidance/recovery", get(get_recovery_guidance))
        .with_state(shared_state)
}

/// Get all training plans for the user
pub async fn get_training_plans(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<PlanQuery>,
) -> Result<Json<Vec<TrainingPlan>>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    // Mock response
    let plan = TrainingPlan {
        id: Uuid::new_v4(),
        user_id: Uuid::parse_str(&user_id).unwrap_or_default(),
        name: "Spring Marathon Training".to_string(),
        description: "16-week marathon preparation plan".to_string(),
        start_date: chrono::Local::now().naive_local().date(),
        end_date: chrono::Local::now().naive_local().date() + chrono::Duration::weeks(16),
        goal_event: Some("City Marathon 2025".to_string()),
        plan_type: PlanType::Race,
        status: PlanStatus::Active,
        weeks: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Ok(Json(vec![plan]))
}

/// Get a specific training plan
pub async fn get_training_plan(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<TrainingPlan>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    // Create sample weeks
    let mut weeks = vec![];
    for week_num in 1..=4 {
        let start = chrono::Local::now().naive_local().date() + chrono::Duration::weeks(week_num - 1);
        weeks.push(TrainingWeek {
            week_number: week_num as u8,
            start_date: start,
            end_date: start + chrono::Duration::days(6),
            focus: match week_num {
                1 => "Base Building".to_string(),
                2 => "Endurance Focus".to_string(),
                3 => "Intensity Introduction".to_string(),
                _ => "Recovery Week".to_string(),
            },
            target_tss: 350.0 + (week_num as f64 * 25.0),
            target_hours: 8.0 + (week_num as f64 * 0.5),
            workouts: vec![],
            notes: None,
        });
    }

    let plan = TrainingPlan {
        id: plan_id,
        user_id: Uuid::parse_str(&user_id).unwrap_or_default(),
        name: "Custom Training Plan".to_string(),
        description: "Personalized training plan based on your goals".to_string(),
        start_date: chrono::Local::now().naive_local().date(),
        end_date: chrono::Local::now().naive_local().date() + chrono::Duration::weeks(12),
        goal_event: Some("Target Event".to_string()),
        plan_type: PlanType::Build,
        status: PlanStatus::Active,
        weeks,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Ok(Json(plan))
}

/// Create a new training plan
pub async fn create_training_plan(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Json(request): Json<CreatePlanRequest>,
) -> Result<Json<TrainingPlan>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    // Validate dates
    if request.end_date <= request.start_date {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_DATES", "End date must be after start date")),
        ));
    }

    let weeks_count = ((request.end_date - request.start_date).num_days() / 7) as u8;
    let mut weeks = vec![];

    for week_num in 1..=weeks_count.min(16) {
        let start = request.start_date + chrono::Duration::weeks((week_num - 1) as i64);
        weeks.push(TrainingWeek {
            week_number: week_num,
            start_date: start,
            end_date: start + chrono::Duration::days(6),
            focus: "TBD".to_string(),
            target_tss: 350.0,
            target_hours: request.weekly_hours_available,
            workouts: vec![],
            notes: None,
        });
    }

    let plan = TrainingPlan {
        id: Uuid::new_v4(),
        user_id: Uuid::parse_str(&user_id).unwrap_or_default(),
        name: request.name,
        description: request.description,
        start_date: request.start_date,
        end_date: request.end_date,
        goal_event: request.goal_event,
        plan_type: request.plan_type,
        status: PlanStatus::Active,
        weeks,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    tracing::info!("Created training plan {} for user {}", plan.id, user_id);

    Ok(Json(plan))
}

/// Update a training plan
pub async fn update_training_plan(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(plan_id): Path<Uuid>,
    Json(request): Json<UpdatePlanRequest>,
) -> Result<Json<TrainingPlan>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    tracing::info!("Updating training plan {} for user {}", plan_id, user_id);

    // Return mock updated plan
    let plan = TrainingPlan {
        id: plan_id,
        user_id: Uuid::parse_str(&user_id).unwrap_or_default(),
        name: request.name.unwrap_or_else(|| "Updated Plan".to_string()),
        description: request.description.unwrap_or_else(|| "Updated description".to_string()),
        start_date: chrono::Local::now().naive_local().date(),
        end_date: request.end_date.unwrap_or_else(|| chrono::Local::now().naive_local().date() + chrono::Duration::weeks(12)),
        goal_event: None,
        plan_type: PlanType::Custom,
        status: request.status.unwrap_or(PlanStatus::Active),
        weeks: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Ok(Json(plan))
}

/// Delete a training plan
pub async fn delete_training_plan(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    tracing::info!("Deleting training plan {} for user {}", plan_id, user_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Training plan deleted successfully"
    })))
}

/// Adapt training plan based on progress
pub async fn adapt_training_plan(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<AdaptivePlanResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    let adaptations = vec![
        PlanAdaptation {
            date: chrono::Local::now().naive_local().date(),
            adaptation_type: "Load Adjustment".to_string(),
            reason: "Higher than expected fatigue".to_string(),
            changes: vec![
                "Reduced Thursday interval intensity".to_string(),
                "Added recovery day on Friday".to_string(),
            ],
            impact: "Better recovery while maintaining fitness gains".to_string(),
        },
    ];

    let current_status = PlanProgress {
        completion_percentage: 45.0,
        adherence_rate: 88.0,
        tss_achievement: 92.0,
        current_week: 6,
        total_weeks: 12,
        on_track: true,
    };

    let recommendations = vec![
        "Consider an easy week next week for better adaptation".to_string(),
        "Focus on sleep and nutrition this week".to_string(),
    ];

    Ok(Json(AdaptivePlanResponse {
        plan_id,
        adaptations,
        current_status,
        recommendations,
        success: true,
    }))
}

/// Get plan progress
pub async fn get_plan_progress(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<PlanProgress>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    Ok(Json(PlanProgress {
        completion_percentage: 65.0,
        adherence_rate: 85.0,
        tss_achievement: 88.0,
        current_week: 8,
        total_weeks: 12,
        on_track: true,
    }))
}

/// Get plan workouts
pub async fn get_plan_workouts(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(plan_id): Path<Uuid>,
) -> Result<Json<Vec<PlannedWorkout>>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    let workouts = vec![
        PlannedWorkout {
            id: Uuid::new_v4(),
            day_of_week: 2, // Tuesday
            workout_type: "Intervals".to_string(),
            duration_minutes: 75,
            intensity: "High".to_string(),
            description: "5x5min @ threshold with 2min recovery".to_string(),
            target_tss: Some(85.0),
            completed: false,
            actual_workout_id: None,
        },
        PlannedWorkout {
            id: Uuid::new_v4(),
            day_of_week: 4, // Thursday
            workout_type: "Endurance".to_string(),
            duration_minutes: 90,
            intensity: "Moderate".to_string(),
            description: "Steady endurance ride in Zone 2".to_string(),
            target_tss: Some(70.0),
            completed: false,
            actual_workout_id: None,
        },
    ];

    Ok(Json(workouts))
}

/// Get coaching recommendations
pub async fn get_recommendations(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<Vec<CoachingRecommendation>>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    let recommendations = vec![
        CoachingRecommendation {
            id: Uuid::new_v4(),
            user_id: Uuid::parse_str(&user_id).unwrap_or_default(),
            recommendation_type: RecommendationType::Recovery,
            priority: Priority::High,
            title: "Take a Recovery Day".to_string(),
            description: "Your training load has been high. A recovery day will help adaptation.".to_string(),
            action_items: vec![
                "Skip today's planned workout".to_string(),
                "Do light stretching or yoga".to_string(),
                "Focus on hydration and nutrition".to_string(),
            ],
            rationale: "TSB is -25, indicating accumulated fatigue".to_string(),
            expected_benefit: "Improved performance in next hard workout".to_string(),
            created_at: Utc::now(),
            valid_until: Some(Utc::now() + chrono::Duration::days(2)),
        },
        CoachingRecommendation {
            id: Uuid::new_v4(),
            user_id: Uuid::parse_str(&user_id).unwrap_or_default(),
            recommendation_type: RecommendationType::Training,
            priority: Priority::Medium,
            title: "Add Threshold Work".to_string(),
            description: "Your threshold power could be improved with targeted training.".to_string(),
            action_items: vec![
                "Include 2x20min threshold intervals weekly".to_string(),
                "Monitor power output consistency".to_string(),
            ],
            rationale: "FTP progression has plateaued".to_string(),
            expected_benefit: "5-8% FTP improvement in 6 weeks".to_string(),
            created_at: Utc::now(),
            valid_until: Some(Utc::now() + chrono::Duration::days(7)),
        },
    ];

    Ok(Json(recommendations))
}

/// Dismiss a recommendation
pub async fn dismiss_recommendation(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Path(recommendation_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    tracing::info!("Dismissing recommendation {} for user {}", recommendation_id, user_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Recommendation dismissed"
    })))
}

/// Get coaching insights
pub async fn get_coaching_insights(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<Vec<CoachingInsight>>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    let insights = vec![
        CoachingInsight {
            insight_type: "Performance".to_string(),
            title: "Power Output Improving".to_string(),
            description: "Your 20-minute power has increased by 5% over the last month".to_string(),
            data_points: vec![
                DataPoint {
                    metric: "20min_power".to_string(),
                    value: 280.0,
                    unit: "watts".to_string(),
                    trend: "up".to_string(),
                },
            ],
            confidence: 0.85,
            action_required: false,
        },
        CoachingInsight {
            insight_type: "Recovery".to_string(),
            title: "Recovery Time Increasing".to_string(),
            description: "You're taking longer to recover between hard sessions".to_string(),
            data_points: vec![
                DataPoint {
                    metric: "hrv_recovery".to_string(),
                    value: 48.0,
                    unit: "hours".to_string(),
                    trend: "up".to_string(),
                },
            ],
            confidence: 0.75,
            action_required: true,
        },
    ];

    Ok(Json(insights))
}

/// Get weekly summary
pub async fn get_weekly_summary(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    Ok(Json(serde_json::json!({
        "week_number": 8,
        "total_hours": 11.5,
        "total_tss": 680,
        "sessions_completed": 5,
        "sessions_planned": 6,
        "adherence_rate": 83.3,
        "key_workouts": [
            {
                "date": "2024-12-20",
                "type": "Threshold",
                "completed": true,
                "quality_score": 8.5
            }
        ],
        "highlights": [
            "New 20-minute power PR",
            "Consistent training all week"
        ],
        "areas_for_improvement": [
            "Missed Saturday long ride",
            "Hydration during long efforts"
        ],
        "next_week_focus": "Recovery week with focus on technique",
        "success": true
    })))
}

/// Get next workout guidance
pub async fn get_next_workout_guidance(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    Ok(Json(serde_json::json!({
        "workout": {
            "type": "Endurance",
            "duration_minutes": 90,
            "intensity": "Zone 2",
            "description": "Steady endurance ride focusing on aerobic efficiency"
        },
        "preparation": [
            "Ensure proper hydration before starting",
            "Have nutrition ready for during the ride",
            "Check weather conditions"
        ],
        "focus_points": [
            "Maintain steady power output",
            "Keep cadence between 85-95 rpm",
            "Stay in Zone 2 heart rate"
        ],
        "alternatives": [
            {
                "reason": "If feeling fatigued",
                "workout": "45min recovery ride in Zone 1"
            },
            {
                "reason": "If time constrained",
                "workout": "60min tempo ride with 3x10min efforts"
            }
        ],
        "success": true
    })))
}

/// Get recovery guidance
pub async fn get_recovery_guidance(
    State(state): State<CoachingAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    Ok(Json(serde_json::json!({
        "recovery_status": "Moderate",
        "recovery_score": 65,
        "recommendations": [
            {
                "category": "Sleep",
                "action": "Aim for 8-9 hours tonight",
                "rationale": "Recent sleep average is 6.5 hours"
            },
            {
                "category": "Nutrition",
                "action": "Increase protein intake to 1.6g/kg",
                "rationale": "Support muscle recovery from recent intensity"
            },
            {
                "category": "Activity",
                "action": "Light movement or stretching today",
                "rationale": "Active recovery promotes blood flow"
            }
        ],
        "estimated_full_recovery": "24-36 hours",
        "next_hard_workout": "Thursday",
        "success": true
    })))
}