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
use chrono::{NaiveDate, DateTime, Utc};

use crate::auth::{AuthService, Claims};

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub period: Option<String>, // day, week, month, year
    pub metrics: Option<String>, // comma-separated list
    pub sport_type: Option<String>,
    pub aggregation: Option<String>, // sum, avg, max, min
}

#[derive(Debug, Serialize)]
pub struct PerformanceTrendsResponse {
    pub user_id: Uuid,
    pub period: String,
    pub trends: Vec<TrendData>,
    pub summary_statistics: SummaryStatistics,
    pub peak_performance: PeakPerformanceData,
    pub consistency_metrics: ConsistencyMetrics,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct TrendData {
    pub date: NaiveDate,
    pub metric_name: String,
    pub value: f64,
    pub change_from_previous: Option<f64>,
    pub moving_average_7d: Option<f64>,
    pub moving_average_30d: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SummaryStatistics {
    pub total_sessions: u32,
    pub total_duration_hours: f64,
    pub total_distance_km: f64,
    pub average_session_duration_minutes: f64,
    pub average_tss: f64,
    pub total_tss: f64,
    pub average_intensity_factor: f64,
}

#[derive(Debug, Serialize)]
pub struct PeakPerformanceData {
    pub best_5min_power: Option<PowerData>,
    pub best_20min_power: Option<PowerData>,
    pub best_60min_power: Option<PowerData>,
    pub fastest_5k: Option<RunData>,
    pub fastest_10k: Option<RunData>,
    pub longest_ride: Option<RideData>,
}

#[derive(Debug, Serialize)]
pub struct PowerData {
    pub watts: u32,
    pub watts_per_kg: f64,
    pub date: NaiveDate,
    pub activity_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct RunData {
    pub time_seconds: u32,
    pub pace_per_km: String,
    pub date: NaiveDate,
    pub activity_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct RideData {
    pub distance_km: f64,
    pub duration_hours: f64,
    pub average_power: u32,
    pub date: NaiveDate,
    pub activity_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ConsistencyMetrics {
    pub weeks_trained: u32,
    pub consistency_score: f64,
    pub average_sessions_per_week: f64,
    pub missed_planned_sessions: u32,
    pub streak_current_days: u32,
    pub streak_longest_days: u32,
}

#[derive(Debug, Serialize)]
pub struct ComparativeAnalysisResponse {
    pub user_metrics: UserMetrics,
    pub peer_comparison: PeerComparison,
    pub percentile_rankings: PercentileRankings,
    pub relative_improvements: RelativeImprovements,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct UserMetrics {
    pub user_id: Uuid,
    pub current_ftp: Option<u32>,
    pub current_vo2max: Option<f64>,
    pub current_ctl: f64,
    pub average_weekly_tss: f64,
    pub average_weekly_hours: f64,
}

#[derive(Debug, Serialize)]
pub struct PeerComparison {
    pub peer_group_size: u32,
    pub age_group: String,
    pub performance_level: String,
    pub relative_position: f64,
}

#[derive(Debug, Serialize)]
pub struct PercentileRankings {
    pub ftp_percentile: Option<f64>,
    pub volume_percentile: Option<f64>,
    pub consistency_percentile: Option<f64>,
    pub improvement_rate_percentile: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct RelativeImprovements {
    pub vs_3months_ago: ImprovementData,
    pub vs_6months_ago: ImprovementData,
    pub vs_1year_ago: ImprovementData,
}

#[derive(Debug, Serialize)]
pub struct ImprovementData {
    pub ftp_change_percent: Option<f64>,
    pub volume_change_percent: Option<f64>,
    pub ctl_change_percent: Option<f64>,
    pub weight_change_percent: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ModelConfidenceResponse {
    pub model_version: String,
    pub overall_confidence: f64,
    pub confidence_by_metric: Vec<MetricConfidence>,
    pub data_quality_score: f64,
    pub explanations: Vec<ConfidenceExplanation>,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct MetricConfidence {
    pub metric_name: String,
    pub confidence_score: f64,
    pub data_points_used: u32,
    pub reliability: String,
}

#[derive(Debug, Serialize)]
pub struct ConfidenceExplanation {
    pub factor: String,
    pub impact: String,
    pub description: String,
    pub recommendation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TrainingLoadResponse {
    pub current_load: TrainingLoadData,
    pub historical_load: Vec<TrainingLoadData>,
    pub load_balance: LoadBalance,
    pub recommendations: Vec<String>,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct TrainingLoadData {
    pub date: NaiveDate,
    pub acute_load: f64,
    pub chronic_load: f64,
    pub training_stress_balance: f64,
    pub load_ratio: f64,
    pub risk_level: String,
}

#[derive(Debug, Serialize)]
pub struct LoadBalance {
    pub current_state: String,
    pub optimal_range: LoadRange,
    pub current_ratio: f64,
    pub days_until_fresh: Option<u32>,
    pub days_until_peaked: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct LoadRange {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Serialize)]
pub struct ZoneDistributionResponse {
    pub period: String,
    pub total_time_hours: f64,
    pub zone_distribution: Vec<ZoneData>,
    pub polarization_index: f64,
    pub recommendations: Vec<String>,
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct ZoneData {
    pub zone: u8,
    pub zone_name: String,
    pub time_hours: f64,
    pub percentage: f64,
    pub target_percentage: Option<f64>,
    pub deviation: Option<f64>,
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
pub struct AnalyticsAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
}

pub fn analytics_routes(db: PgPool, auth_service: AuthService) -> Router {
    let shared_state = AnalyticsAppState {
        db,
        auth_service,
    };

    Router::new()
        .route("/trends", get(get_performance_trends))
        .route("/comparative", get(get_comparative_analysis))
        .route("/model-confidence", get(get_model_confidence))
        .route("/training-load", get(get_training_load))
        .route("/zone-distribution", get(get_zone_distribution))
        .route("/statistics/summary", get(get_summary_statistics))
        .route("/statistics/personal-records", get(get_personal_records))
        .route("/statistics/monthly", get(get_monthly_statistics))
        .route("/export", get(export_analytics_data))
        .with_state(shared_state)
}

/// Get performance trends over time
pub async fn get_performance_trends(
    State(state): State<AnalyticsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<PerformanceTrendsResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    let period = query.period.as_deref().unwrap_or("month");

    // Mock trend data
    let trends = vec![
        TrendData {
            date: chrono::Local::now().naive_local().date() - chrono::Duration::days(7),
            metric_name: "ftp".to_string(),
            value: 250.0,
            change_from_previous: Some(2.5),
            moving_average_7d: Some(248.0),
            moving_average_30d: Some(245.0),
        },
        TrendData {
            date: chrono::Local::now().naive_local().date(),
            metric_name: "ftp".to_string(),
            value: 255.0,
            change_from_previous: Some(5.0),
            moving_average_7d: Some(252.0),
            moving_average_30d: Some(248.0),
        },
    ];

    let summary_statistics = SummaryStatistics {
        total_sessions: 45,
        total_duration_hours: 67.5,
        total_distance_km: 1250.0,
        average_session_duration_minutes: 90.0,
        average_tss: 85.0,
        total_tss: 3825.0,
        average_intensity_factor: 0.75,
    };

    let peak_performance = PeakPerformanceData {
        best_5min_power: Some(PowerData {
            watts: 350,
            watts_per_kg: 5.0,
            date: chrono::Local::now().naive_local().date() - chrono::Duration::days(14),
            activity_id: Uuid::new_v4(),
        }),
        best_20min_power: Some(PowerData {
            watts: 280,
            watts_per_kg: 4.0,
            date: chrono::Local::now().naive_local().date() - chrono::Duration::days(7),
            activity_id: Uuid::new_v4(),
        }),
        best_60min_power: Some(PowerData {
            watts: 250,
            watts_per_kg: 3.57,
            date: chrono::Local::now().naive_local().date() - chrono::Duration::days(3),
            activity_id: Uuid::new_v4(),
        }),
        fastest_5k: None,
        fastest_10k: None,
        longest_ride: Some(RideData {
            distance_km: 150.0,
            duration_hours: 5.5,
            average_power: 180,
            date: chrono::Local::now().naive_local().date() - chrono::Duration::days(21),
            activity_id: Uuid::new_v4(),
        }),
    };

    let consistency_metrics = ConsistencyMetrics {
        weeks_trained: 12,
        consistency_score: 85.0,
        average_sessions_per_week: 3.75,
        missed_planned_sessions: 4,
        streak_current_days: 15,
        streak_longest_days: 42,
    };

    Ok(Json(PerformanceTrendsResponse {
        user_id,
        period: period.to_string(),
        trends,
        summary_statistics,
        peak_performance,
        consistency_metrics,
        success: true,
    }))
}

/// Get comparative analysis data
pub async fn get_comparative_analysis(
    State(state): State<AnalyticsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<ComparativeAnalysisResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub.parse::<Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("INVALID_USER_ID", "Invalid user ID format")),
        )
    })?;

    let user_metrics = UserMetrics {
        user_id,
        current_ftp: Some(255),
        current_vo2max: Some(55.0),
        current_ctl: 75.0,
        average_weekly_tss: 450.0,
        average_weekly_hours: 8.5,
    };

    let peer_comparison = PeerComparison {
        peer_group_size: 150,
        age_group: "35-39".to_string(),
        performance_level: "Cat 3".to_string(),
        relative_position: 65.0,
    };

    let percentile_rankings = PercentileRankings {
        ftp_percentile: Some(70.0),
        volume_percentile: Some(60.0),
        consistency_percentile: Some(85.0),
        improvement_rate_percentile: Some(75.0),
    };

    let relative_improvements = RelativeImprovements {
        vs_3months_ago: ImprovementData {
            ftp_change_percent: Some(5.0),
            volume_change_percent: Some(10.0),
            ctl_change_percent: Some(15.0),
            weight_change_percent: Some(-2.0),
        },
        vs_6months_ago: ImprovementData {
            ftp_change_percent: Some(8.0),
            volume_change_percent: Some(20.0),
            ctl_change_percent: Some(25.0),
            weight_change_percent: Some(-3.5),
        },
        vs_1year_ago: ImprovementData {
            ftp_change_percent: Some(12.0),
            volume_change_percent: Some(35.0),
            ctl_change_percent: Some(40.0),
            weight_change_percent: Some(-5.0),
        },
    };

    Ok(Json(ComparativeAnalysisResponse {
        user_metrics,
        peer_comparison,
        percentile_rankings,
        relative_improvements,
        success: true,
    }))
}

/// Get model confidence and explanations
pub async fn get_model_confidence(
    State(state): State<AnalyticsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<ModelConfidenceResponse>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    let confidence_by_metric = vec![
        MetricConfidence {
            metric_name: "FTP Prediction".to_string(),
            confidence_score: 0.85,
            data_points_used: 45,
            reliability: "High".to_string(),
        },
        MetricConfidence {
            metric_name: "Fatigue Assessment".to_string(),
            confidence_score: 0.78,
            data_points_used: 30,
            reliability: "Moderate".to_string(),
        },
        MetricConfidence {
            metric_name: "Performance Trajectory".to_string(),
            confidence_score: 0.92,
            data_points_used: 90,
            reliability: "Very High".to_string(),
        },
    ];

    let explanations = vec![
        ConfidenceExplanation {
            factor: "Data Completeness".to_string(),
            impact: "Positive".to_string(),
            description: "Training data is 95% complete for the last 3 months".to_string(),
            recommendation: None,
        },
        ConfidenceExplanation {
            factor: "Data Consistency".to_string(),
            impact: "Positive".to_string(),
            description: "Power and heart rate data show consistent patterns".to_string(),
            recommendation: None,
        },
        ConfidenceExplanation {
            factor: "Recent Changes".to_string(),
            impact: "Neutral".to_string(),
            description: "Recent training pattern changes may affect predictions".to_string(),
            recommendation: Some("Continue current training for 2 more weeks for better predictions".to_string()),
        },
    ];

    Ok(Json(ModelConfidenceResponse {
        model_version: "v2.1.0".to_string(),
        overall_confidence: 0.85,
        confidence_by_metric,
        data_quality_score: 0.92,
        explanations,
        success: true,
    }))
}

/// Get training load analysis
pub async fn get_training_load(
    State(state): State<AnalyticsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<TrainingLoadResponse>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    let current_load = TrainingLoadData {
        date: chrono::Local::now().naive_local().date(),
        acute_load: 550.0,
        chronic_load: 475.0,
        training_stress_balance: -15.0,
        load_ratio: 1.16,
        risk_level: "Optimal".to_string(),
    };

    let mut historical_load = vec![];
    for i in 1..=7 {
        historical_load.push(TrainingLoadData {
            date: chrono::Local::now().naive_local().date() - chrono::Duration::days(i),
            acute_load: 550.0 - (i as f64 * 10.0),
            chronic_load: 475.0 - (i as f64 * 5.0),
            training_stress_balance: -15.0 + (i as f64 * 2.0),
            load_ratio: 1.16 - (i as f64 * 0.02),
            risk_level: "Optimal".to_string(),
        });
    }

    let load_balance = LoadBalance {
        current_state: "Productive".to_string(),
        optimal_range: LoadRange { min: 0.8, max: 1.3 },
        current_ratio: 1.16,
        days_until_fresh: Some(3),
        days_until_peaked: Some(14),
    };

    let recommendations = vec![
        "Current load is optimal for performance gains".to_string(),
        "Consider a recovery week in 10-14 days".to_string(),
        "Maintain current training intensity".to_string(),
    ];

    Ok(Json(TrainingLoadResponse {
        current_load,
        historical_load,
        load_balance,
        recommendations,
        success: true,
    }))
}

/// Get training zone distribution
pub async fn get_zone_distribution(
    State(state): State<AnalyticsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<ZoneDistributionResponse>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    let period = query.period.as_deref().unwrap_or("month");

    let zone_distribution = vec![
        ZoneData {
            zone: 1,
            zone_name: "Recovery".to_string(),
            time_hours: 5.5,
            percentage: 15.0,
            target_percentage: Some(20.0),
            deviation: Some(-5.0),
        },
        ZoneData {
            zone: 2,
            zone_name: "Endurance".to_string(),
            time_hours: 18.5,
            percentage: 50.0,
            target_percentage: Some(55.0),
            deviation: Some(-5.0),
        },
        ZoneData {
            zone: 3,
            zone_name: "Tempo".to_string(),
            time_hours: 7.4,
            percentage: 20.0,
            target_percentage: Some(15.0),
            deviation: Some(5.0),
        },
        ZoneData {
            zone: 4,
            zone_name: "Threshold".to_string(),
            time_hours: 3.7,
            percentage: 10.0,
            target_percentage: Some(8.0),
            deviation: Some(2.0),
        },
        ZoneData {
            zone: 5,
            zone_name: "VO2 Max".to_string(),
            time_hours: 1.85,
            percentage: 5.0,
            target_percentage: Some(2.0),
            deviation: Some(3.0),
        },
    ];

    let recommendations = vec![
        "Increase Zone 2 time for better aerobic base".to_string(),
        "Current threshold work is appropriate".to_string(),
        "Consider adding more recovery time".to_string(),
    ];

    Ok(Json(ZoneDistributionResponse {
        period: period.to_string(),
        total_time_hours: 37.0,
        zone_distribution,
        polarization_index: 0.75,
        recommendations,
        success: true,
    }))
}

/// Get summary statistics
pub async fn get_summary_statistics(
    State(state): State<AnalyticsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    Ok(Json(serde_json::json!({
        "period": query.period.as_deref().unwrap_or("all_time"),
        "total_activities": 245,
        "total_duration_hours": 367.5,
        "total_distance_km": 8750.0,
        "total_elevation_gain_m": 45000,
        "average_speed_kmh": 23.8,
        "calories_burned": 185000,
        "success": true
    })))
}

/// Get personal records
pub async fn get_personal_records(
    State(state): State<AnalyticsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    Ok(Json(serde_json::json!({
        "power_records": {
            "5_seconds": { "watts": 850, "date": "2024-11-15" },
            "1_minute": { "watts": 450, "date": "2024-11-20" },
            "5_minutes": { "watts": 350, "date": "2024-11-22" },
            "20_minutes": { "watts": 280, "date": "2024-12-01" },
            "60_minutes": { "watts": 250, "date": "2024-12-05" }
        },
        "speed_records": {
            "max_speed_kmh": 65.5,
            "fastest_100km": "2:45:30",
            "fastest_50km": "1:18:45"
        },
        "distance_records": {
            "longest_ride_km": 180.0,
            "longest_ride_time": "6:15:00",
            "biggest_week_km": 450.0
        },
        "success": true
    })))
}

/// Get monthly statistics
pub async fn get_monthly_statistics(
    State(state): State<AnalyticsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    Ok(Json(serde_json::json!({
        "month": "December 2024",
        "activities": 18,
        "duration_hours": 27.5,
        "distance_km": 650.0,
        "elevation_gain_m": 3500,
        "average_tss": 82.0,
        "total_tss": 1476.0,
        "days_active": 15,
        "rest_days": 10,
        "comparison_to_previous_month": {
            "activities_change": "+2",
            "duration_change": "+3.5 hours",
            "distance_change": "+85 km",
            "tss_change": "+150"
        },
        "success": true
    })))
}

/// Export analytics data
pub async fn export_analytics_data(
    State(state): State<AnalyticsAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let _user_id = claims.sub;

    Ok(Json(serde_json::json!({
        "export_id": Uuid::new_v4(),
        "export_format": "json",
        "created_at": Utc::now(),
        "download_url": "/api/v1/analytics/export/download/{export_id}",
        "expires_at": Utc::now() + chrono::Duration::hours(24),
        "success": true
    })))
}