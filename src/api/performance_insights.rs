use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use axum_extra::extract::WithRejection;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{AuthService, Claims};
use crate::models::{PerformanceInsights, PerformanceInsightsRequest};
use crate::services::PerformanceInsightsService;

/// Query parameters for performance insights
#[derive(Debug, Deserialize)]
pub struct InsightsQuery {
    /// Analysis period in days (default: 90)
    pub period_days: Option<u32>,
    /// Include peer comparison data
    pub include_peer_comparison: Option<bool>,
    /// Include race time predictions
    pub include_predictions: Option<bool>,
    /// Focus areas (comma-separated: fitness,performance,goals,recovery)
    pub focus_areas: Option<String>,
}

/// Response wrapper for performance insights
#[derive(Debug, Serialize)]
pub struct InsightsResponse {
    pub insights: PerformanceInsights,
    pub success: bool,
    pub message: String,
}

/// Summary insights response for quick overview
#[derive(Debug, Serialize)]
pub struct InsightsSummaryResponse {
    pub user_id: Uuid,
    pub fitness_score: f64,
    pub performance_trend: String,
    pub consistency_score: f64,
    pub key_insights: Vec<String>,
    pub top_recommendations: Vec<String>,
    pub warnings: Vec<String>,
    pub success: bool,
}

/// Fitness trends response
#[derive(Debug, Serialize)]
pub struct FitnessTrendsResponse {
    pub current_ctl: f64,
    pub ctl_trend_6weeks: f64,
    pub ctl_trend_3months: f64,
    pub current_tsb: f64,
    pub fitness_trajectory: String,
    pub peak_fitness_date: Option<chrono::NaiveDate>,
    pub success: bool,
}

/// Performance comparison response
#[derive(Debug, Serialize)]
pub struct PerformanceComparisonResponse {
    pub peer_percentile: Option<f64>,
    pub age_group_percentile: Option<f64>,
    pub vs_last_year: Option<PerformanceComparisonData>,
    pub vs_best_year: Option<PerformanceComparisonData>,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct PerformanceComparisonData {
    pub fitness_change: f64,
    pub power_change: f64,
    pub volume_change: f64,
    pub consistency_change: f64,
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

/// Shared state for performance insights API
#[derive(Clone)]
pub struct InsightsAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub insights_service: PerformanceInsightsService,
}

/// Create performance insights routes
pub fn performance_insights_routes(db: PgPool, auth_service: AuthService) -> Router {
    let insights_service = PerformanceInsightsService::new(db.clone())
        .expect("Failed to create PerformanceInsightsService");

    let shared_state = InsightsAppState {
        db,
        auth_service,
        insights_service,
    };

    Router::new()
        .route("/insights", get(get_performance_insights))
        .route("/insights/summary", get(get_insights_summary))
        .route("/insights/fitness-trends", get(get_fitness_trends))
        .route("/insights/performance-comparison", get(get_performance_comparison))
        .route("/insights/recommendations", get(get_recommendations))
        .route("/insights/warnings", get(get_warnings))
        .with_state(shared_state)
}

/// Get comprehensive performance insights for the authenticated user
pub async fn get_performance_insights(
    State(state): State<InsightsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<InsightsQuery>,
) -> Result<Json<InsightsResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", &format!("Invalid user ID format: {}", e))),
        )
    })?;

    // Parse focus areas
    let focus_areas = query.focus_areas
        .map(|areas| areas.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let request = PerformanceInsightsRequest {
        user_id,
        period_days: query.period_days,
        include_peer_comparison: query.include_peer_comparison.unwrap_or(false),
        include_predictions: query.include_predictions.unwrap_or(true),
        focus_areas,
    };

    match state.insights_service.generate_insights(request).await {
        Ok(insights) => {
            tracing::info!("Generated performance insights for user {}", user_id);
            Ok(Json(InsightsResponse {
                insights,
                success: true,
                message: "Performance insights generated successfully".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to generate performance insights: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("INSIGHTS_GENERATION_ERROR", &format!("Failed to generate insights: {}", e))),
            ))
        }
    }
}

/// Get quick summary of key performance insights
pub async fn get_insights_summary(
    State(state): State<InsightsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<InsightsSummaryResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", &format!("Invalid user ID format: {}", e))),
        )
    })?;

    let request = PerformanceInsightsRequest {
        user_id,
        period_days: Some(30), // Short period for summary
        include_peer_comparison: false,
        include_predictions: false,
        focus_areas: vec!["fitness".to_string(), "performance".to_string()],
    };

    match state.insights_service.generate_insights(request).await {
        Ok(insights) => {
            let fitness_score = insights.fitness_trends.current_ctl;

            let performance_trend = if insights.performance_trends.power_trend_30days > 5.0 {
                "Improving".to_string()
            } else if insights.performance_trends.power_trend_30days < -5.0 {
                "Declining".to_string()
            } else {
                "Stable".to_string()
            };

            let key_insights: Vec<String> = insights.key_insights
                .iter()
                .take(3)
                .map(|insight| insight.message.clone())
                .collect();

            let top_recommendations: Vec<String> = insights.recommendations
                .iter()
                .take(3)
                .map(|rec| rec.action.clone())
                .collect();

            let warnings: Vec<String> = insights.warnings
                .iter()
                .map(|warning| warning.title.clone())
                .collect();

            Ok(Json(InsightsSummaryResponse {
                user_id,
                fitness_score,
                performance_trend,
                consistency_score: insights.training_consistency.weekly_consistency_score,
                key_insights,
                top_recommendations,
                warnings,
                success: true,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to generate insights summary: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("INSIGHTS_SUMMARY_ERROR", &format!("Failed to generate summary: {}", e))),
            ))
        }
    }
}

/// Get fitness trends data
pub async fn get_fitness_trends(
    State(state): State<InsightsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<InsightsQuery>,
) -> Result<Json<FitnessTrendsResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", &format!("Invalid user ID format: {}", e))),
        )
    })?;

    let request = PerformanceInsightsRequest {
        user_id,
        period_days: query.period_days,
        include_peer_comparison: false,
        include_predictions: false,
        focus_areas: vec!["fitness".to_string()],
    };

    match state.insights_service.generate_insights(request).await {
        Ok(insights) => {
            let fitness_trajectory = match insights.fitness_trends.fitness_trajectory {
                crate::models::FitnessTrajectory::Building => "Building",
                crate::models::FitnessTrajectory::Peaking => "Peaking",
                crate::models::FitnessTrajectory::Declining => "Declining",
                crate::models::FitnessTrajectory::Maintaining => "Maintaining",
            };

            Ok(Json(FitnessTrendsResponse {
                current_ctl: insights.fitness_trends.current_ctl,
                ctl_trend_6weeks: insights.fitness_trends.ctl_trend_6weeks,
                ctl_trend_3months: insights.fitness_trends.ctl_trend_3months,
                current_tsb: insights.fitness_trends.current_tsb,
                fitness_trajectory: fitness_trajectory.to_string(),
                peak_fitness_date: insights.fitness_trends.peak_fitness_date,
                success: true,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to get fitness trends: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("FITNESS_TRENDS_ERROR", &format!("Failed to get fitness trends: {}", e))),
            ))
        }
    }
}

/// Get performance comparison data
pub async fn get_performance_comparison(
    State(state): State<InsightsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<PerformanceComparisonResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", &format!("Invalid user ID format: {}", e))),
        )
    })?;

    let request = PerformanceInsightsRequest {
        user_id,
        period_days: Some(365), // Full year for comparison
        include_peer_comparison: true,
        include_predictions: false,
        focus_areas: vec!["performance".to_string()],
    };

    match state.insights_service.generate_insights(request).await {
        Ok(insights) => {
            let peer_percentile = insights.peer_comparison
                .as_ref()
                .map(|pc| pc.fitness_percentile);

            let age_group_percentile = insights.age_group_benchmarks
                .as_ref()
                .and_then(|agb| agb.power_percentile);

            let vs_last_year = insights.historical_comparison
                .as_ref()
                .map(|hc| PerformanceComparisonData {
                    fitness_change: hc.vs_last_year.fitness_change,
                    power_change: hc.vs_last_year.power_change,
                    volume_change: hc.vs_last_year.volume_change,
                    consistency_change: hc.vs_last_year.consistency_change,
                });

            let vs_best_year = insights.historical_comparison
                .as_ref()
                .map(|hc| PerformanceComparisonData {
                    fitness_change: hc.vs_best_year.fitness_change,
                    power_change: hc.vs_best_year.power_change,
                    volume_change: hc.vs_best_year.volume_change,
                    consistency_change: hc.vs_best_year.consistency_change,
                });

            Ok(Json(PerformanceComparisonResponse {
                peer_percentile,
                age_group_percentile,
                vs_last_year,
                vs_best_year,
                success: true,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to get performance comparison: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("PERFORMANCE_COMPARISON_ERROR", &format!("Failed to get comparison: {}", e))),
            ))
        }
    }
}

/// Get personalized recommendations
pub async fn get_recommendations(
    State(state): State<InsightsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", &format!("Invalid user ID format: {}", e))),
        )
    })?;

    let request = PerformanceInsightsRequest {
        user_id,
        period_days: Some(90),
        include_peer_comparison: false,
        include_predictions: false,
        focus_areas: vec!["training".to_string(), "recovery".to_string()],
    };

    match state.insights_service.generate_insights(request).await {
        Ok(insights) => {
            Ok(Json(serde_json::json!({
                "recommendations": insights.recommendations,
                "success": true,
                "message": "Recommendations retrieved successfully"
            })))
        }
        Err(e) => {
            tracing::error!("Failed to get recommendations: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("RECOMMENDATIONS_ERROR", &format!("Failed to get recommendations: {}", e))),
            ))
        }
    }
}

/// Get warnings and risk alerts
pub async fn get_warnings(
    State(state): State<InsightsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", &format!("Invalid user ID format: {}", e))),
        )
    })?;

    let request = PerformanceInsightsRequest {
        user_id,
        period_days: Some(30), // Recent data for warnings
        include_peer_comparison: false,
        include_predictions: false,
        focus_areas: vec!["recovery".to_string(), "training".to_string()],
    };

    match state.insights_service.generate_insights(request).await {
        Ok(insights) => {
            Ok(Json(serde_json::json!({
                "warnings": insights.warnings,
                "achievements": insights.achievements,
                "success": true,
                "message": "Warnings and achievements retrieved successfully"
            })))
        }
        Err(e) => {
            tracing::error!("Failed to get warnings: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("WARNINGS_ERROR", &format!("Failed to get warnings: {}", e))),
            ))
        }
    }
}