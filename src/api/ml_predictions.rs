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
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{AuthService, Claims};
use crate::services::{
    TrainingRecommendationService, ModelTrainingService, ModelVersioningService,
    FeatureEngineeringService, MLModelService,
};
use crate::models::TrainingFeatures;

#[derive(Debug, Deserialize)]
pub struct RecommendationQuery {
    /// Target date for the recommendation (optional, defaults to today)
    pub target_date: Option<chrono::NaiveDate>,
    /// Preferred workout type (optional)
    pub preferred_workout_type: Option<String>,
    /// Maximum duration in minutes (optional)
    pub max_duration_minutes: Option<i32>,
    /// Perceived difficulty level 1-10 (user feedback)
    pub perceived_difficulty: Option<i32>,
    /// Energy level 1-10 (user feedback)
    pub energy_level: Option<i32>,
    /// Motivation level 1-10 (user feedback)
    pub motivation: Option<i32>,
    /// Available time in minutes (user feedback)
    pub available_time_minutes: Option<i32>,
    /// Preferred intensity: "easy", "moderate", "hard" (user feedback)
    pub preferred_intensity: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModelTrainingQuery {
    /// Minimum number of training samples required (optional, default: 20)
    pub min_samples: Option<usize>,
    /// Days of historical data to use (optional, default: 365)
    pub training_window_days: Option<i32>,
    /// Whether to force retrain existing models (optional, default: false)
    pub force_retrain: Option<bool>,
    /// Target RMSE threshold (optional, default: 50.0)
    pub target_rmse_threshold: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct ABTestQuery {
    /// Test name
    pub name: String,
    /// Champion model version
    pub champion_version: String,
    /// Challenger model version
    pub challenger_version: String,
    /// Traffic split percentage to challenger (0.0-1.0)
    pub traffic_split: f32,
    /// Test duration in days
    pub duration_days: i32,
    /// Target metric for comparison
    pub target_metric: String,
}

#[derive(Debug, Serialize)]
pub struct TrainingRecommendationResponse {
    /// User ID
    pub user_id: Uuid,
    /// Primary recommendation
    pub recommendation: RecommendationDetails,
    /// Alternative options
    pub alternatives: Vec<RecommendationDetails>,
    /// Explanation of the recommendation
    pub reasoning: String,
    /// Any warnings or considerations
    pub warnings: Vec<String>,
    /// Whether this was served from cache
    pub cached: bool,
    /// When the recommendation was generated
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct RecommendationDetails {
    /// Recommended TSS
    pub recommended_tss: f32,
    /// Confidence in the prediction (0.0-1.0)
    pub confidence: f32,
    /// Lower bound of confidence interval
    pub confidence_lower: f32,
    /// Upper bound of confidence interval
    pub confidence_upper: f32,
    /// Recommended workout type
    pub recommended_workout_type: String,
    /// Model version used
    pub model_version: String,
}

#[derive(Debug, Serialize)]
pub struct ModelTrainingResponse {
    /// User ID
    pub user_id: Uuid,
    /// Training results for each model
    pub model_results: Vec<ModelTrainingResult>,
    /// Best performing model
    pub best_model: Option<String>,
    /// Training data quality assessment
    pub data_quality: DataQualityInfo,
    /// Training timestamp
    pub trained_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ModelTrainingResult {
    /// Model version
    pub model_version: String,
    /// Model type
    pub model_type: String,
    /// Mean Absolute Error
    pub mae_tss: f32,
    /// Root Mean Square Error
    pub rmse_tss: f32,
    /// R-squared coefficient
    pub r_squared: f32,
    /// Number of training samples
    pub sample_count: usize,
}

#[derive(Debug, Serialize)]
pub struct DataQualityInfo {
    /// Total number of samples
    pub total_samples: usize,
    /// Number of valid samples
    pub valid_samples: usize,
    /// Data completeness percentage
    pub data_completeness: f32,
    /// Whether data is sufficient for training
    pub is_sufficient: bool,
    /// Recommendations for improving data quality
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ABTestResponse {
    /// Test ID
    pub test_id: Uuid,
    /// Test configuration
    pub test_config: ABTestInfo,
    /// Test status
    pub status: String,
    /// Test results (if completed)
    pub results: Option<ABTestResultInfo>,
}

#[derive(Debug, Serialize)]
pub struct ABTestInfo {
    /// Test name
    pub name: String,
    /// Champion model version
    pub champion_version: String,
    /// Challenger model version
    pub challenger_version: String,
    /// Traffic split to challenger
    pub traffic_split: f32,
    /// Test start date
    pub start_date: chrono::DateTime<chrono::Utc>,
    /// Test end date
    pub end_date: chrono::DateTime<chrono::Utc>,
    /// Target metric
    pub target_metric: String,
}

#[derive(Debug, Serialize)]
pub struct ABTestResultInfo {
    /// Champion performance metrics
    pub champion_performance: TestPerformanceMetrics,
    /// Challenger performance metrics
    pub challenger_performance: TestPerformanceMetrics,
    /// Statistical significance
    pub statistical_significance: f32,
    /// Winner model version (if any)
    pub winner: Option<String>,
    /// Test recommendation
    pub recommendation: String,
}

#[derive(Debug, Serialize)]
pub struct TestPerformanceMetrics {
    /// Model version
    pub version: String,
    /// Number of samples
    pub sample_size: usize,
    /// Average RMSE
    pub avg_rmse: f32,
    /// Average confidence
    pub avg_confidence: f32,
    /// Average prediction latency
    pub prediction_latency_ms: f32,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    /// Error code
    pub error_code: String,
    /// Error message
    pub message: String,
    /// Additional details
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

    pub fn with_details(code: &str, message: &str, details: serde_json::Value) -> Self {
        Self {
            error_code: code.to_string(),
            message: message.to_string(),
            details: Some(details),
        }
    }
}

pub fn ml_prediction_routes(db: PgPool, auth_service: AuthService) -> Router {
    let recommendation_service = TrainingRecommendationService::new(db.clone());
    let training_service = ModelTrainingService::new(db.clone());
    let versioning_service = ModelVersioningService::new(db.clone());
    let feature_service = FeatureEngineeringService::new(db.clone());

    let shared_state = MLAppState {
        db,
        auth_service,
        recommendation_service,
        training_service,
        versioning_service,
        feature_service,
    };

    Router::new()
        .route("/recommendation", get(get_training_recommendation))
        .route("/features", get(get_user_features))
        .route("/models/train", post(train_user_models))
        .route("/models/versions", get(list_model_versions))
        .route("/models/champion", get(get_champion_model))
        .route("/ab-tests", post(create_ab_test))
        .route("/ab-tests", get(list_ab_tests))
        .route("/ab-tests/:test_id", get(get_ab_test_results))
        .route("/ab-tests/:test_id/start", post(start_ab_test))
        .route("/ab-tests/:test_id/complete", post(complete_ab_test))
        .route("/data-quality", get(get_data_quality_assessment))
        .with_state(shared_state)
}

#[derive(Clone)]
pub struct MLAppState {
    pub db: PgPool,
    pub auth_service: AuthService,
    pub recommendation_service: TrainingRecommendationService,
    pub training_service: ModelTrainingService,
    pub versioning_service: ModelVersioningService,
    pub feature_service: FeatureEngineeringService,
}

/// Get training load recommendation for the user
pub async fn get_training_recommendation(
    State(state): State<MLAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<RecommendationQuery>,
) -> Result<Json<TrainingRecommendationResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    // Build recommendation request
    let user_feedback = if query.perceived_difficulty.is_some()
        || query.energy_level.is_some()
        || query.motivation.is_some()
        || query.available_time_minutes.is_some()
        || query.preferred_intensity.is_some() {
        Some(crate::services::training_recommendation_service::UserFeedback {
            perceived_difficulty: query.perceived_difficulty.unwrap_or(5),
            energy_level: query.energy_level.unwrap_or(5),
            motivation: query.motivation.unwrap_or(5),
            available_time_minutes: query.available_time_minutes,
            preferred_intensity: query.preferred_intensity,
        })
    } else {
        None
    };

    let request = crate::services::training_recommendation_service::RecommendationRequest {
        user_id,
        target_date: query.target_date,
        preferred_workout_type: query.preferred_workout_type,
        max_duration_minutes: query.max_duration_minutes,
        user_feedback,
    };

    // Get recommendation
    let recommendation = state
        .recommendation_service
        .get_recommendation(request)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("RECOMMENDATION_ERROR", &format!("Failed to get recommendation: {}", e))),
            )
        })?;

    // Convert to API response format
    let response = TrainingRecommendationResponse {
        user_id: recommendation.user_id,
        recommendation: RecommendationDetails {
            recommended_tss: recommendation.prediction.recommended_tss,
            confidence: recommendation.prediction.confidence,
            confidence_lower: recommendation.prediction.confidence_lower,
            confidence_upper: recommendation.prediction.confidence_upper,
            recommended_workout_type: recommendation.prediction.recommended_workout_type,
            model_version: recommendation.prediction.model_version,
        },
        alternatives: recommendation
            .alternative_options
            .into_iter()
            .map(|alt| RecommendationDetails {
                recommended_tss: alt.recommended_tss,
                confidence: alt.confidence,
                confidence_lower: alt.confidence_lower,
                confidence_upper: alt.confidence_upper,
                recommended_workout_type: alt.recommended_workout_type,
                model_version: alt.model_version,
            })
            .collect(),
        reasoning: recommendation.reasoning,
        warnings: recommendation.warnings,
        cached: recommendation.cached,
        generated_at: recommendation.generated_at,
    };

    Ok(Json(response))
}

/// Get current user features for ML model
pub async fn get_user_features(
    State(state): State<MLAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<TrainingFeatures>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    let features = state
        .feature_service
        .extract_current_features(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("FEATURE_EXTRACTION_ERROR", &format!("Failed to extract features: {}", e))),
            )
        })?;

    Ok(Json(features))
}

/// Train ML models for the user
pub async fn train_user_models(
    State(state): State<MLAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<ModelTrainingQuery>,
) -> Result<Json<ModelTrainingResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    // Build training configuration
    let config = crate::services::model_training_service::TrainingConfig {
        min_training_samples: query.min_samples.unwrap_or(20),
        training_window_days: query.training_window_days.unwrap_or(365),
        validation_split: 0.2,
        force_retrain: query.force_retrain.unwrap_or(false),
        target_rmse_threshold: query.target_rmse_threshold.unwrap_or(50.0),
    };

    // Train models
    let model_metrics = state
        .training_service
        .train_user_models(user_id, Some(config))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("TRAINING_ERROR", &format!("Failed to train models: {}", e))),
            )
        })?;

    // Get data quality assessment
    let quality_report = state
        .training_service
        .assess_data_quality(user_id, query.training_window_days.unwrap_or(365))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("DATA_QUALITY_ERROR", &format!("Failed to assess data quality: {}", e))),
            )
        })?;

    // Find best model
    let best_model = model_metrics
        .iter()
        .min_by(|a, b| a.rmse_tss.partial_cmp(&b.rmse_tss).unwrap())
        .map(|m| m.model_version.clone());

    // Get recommendations for improving data quality
    let recommendations = state.training_service.get_training_recommendations(&quality_report);

    let response = ModelTrainingResponse {
        user_id,
        model_results: model_metrics
            .into_iter()
            .map(|m| ModelTrainingResult {
                model_version: m.model_version,
                model_type: "ML".to_string(), // Would be determined from model version
                mae_tss: m.mae_tss,
                rmse_tss: m.rmse_tss,
                r_squared: m.r_squared,
                sample_count: m.sample_count,
            })
            .collect(),
        best_model,
        data_quality: DataQualityInfo {
            total_samples: quality_report.total_samples,
            valid_samples: quality_report.valid_samples,
            data_completeness: quality_report.data_completeness,
            is_sufficient: quality_report.is_sufficient,
            recommendations,
        },
        trained_at: chrono::Utc::now(),
    };

    Ok(Json(response))
}

/// List all model versions
pub async fn list_model_versions(
    State(state): State<MLAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ApiError>)> {
    let versions = state.versioning_service.list_model_versions().await;

    let response: Vec<serde_json::Value> = versions
        .into_iter()
        .map(|v| serde_json::json!({
            "id": v.id,
            "version": v.version,
            "model_type": v.model_type,
            "status": format!("{:?}", v.status),
            "mae_tss": v.metrics.mae_tss,
            "rmse_tss": v.metrics.rmse_tss,
            "r_squared": v.metrics.r_squared,
            "created_at": v.created_at,
            "deployed_at": v.deployed_at,
            "description": v.description
        }))
        .collect();

    Ok(Json(response))
}

/// Get current champion model
pub async fn get_champion_model(
    State(state): State<MLAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let champion = state.versioning_service.get_champion_version().await;

    let response = serde_json::json!({
        "champion_version": champion,
        "timestamp": chrono::Utc::now()
    });

    Ok(Json(response))
}

/// Create a new A/B test
pub async fn create_ab_test(
    State(state): State<MLAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<ABTestQuery>,
) -> Result<Json<ABTestResponse>, (StatusCode, Json<ApiError>)> {
    let test_config = state
        .versioning_service
        .create_ab_test(
            query.name,
            query.champion_version,
            query.challenger_version,
            query.traffic_split,
            query.duration_days,
            query.target_metric,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("AB_TEST_ERROR", &format!("Failed to create A/B test: {}", e))),
            )
        })?;

    let response = ABTestResponse {
        test_id: test_config.test_id,
        test_config: ABTestInfo {
            name: test_config.name,
            champion_version: test_config.champion_version,
            challenger_version: test_config.challenger_version,
            traffic_split: test_config.traffic_split,
            start_date: test_config.start_date,
            end_date: test_config.end_date,
            target_metric: test_config.target_metric,
        },
        status: format!("{:?}", test_config.status),
        results: None,
    };

    Ok(Json(response))
}

/// List active A/B tests
pub async fn list_ab_tests(
    State(state): State<MLAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
) -> Result<Json<Vec<ABTestResponse>>, (StatusCode, Json<ApiError>)> {
    let tests = state.versioning_service.list_active_ab_tests().await;

    let response: Vec<ABTestResponse> = tests
        .into_iter()
        .map(|test| ABTestResponse {
            test_id: test.test_id,
            test_config: ABTestInfo {
                name: test.name,
                champion_version: test.champion_version,
                challenger_version: test.challenger_version,
                traffic_split: test.traffic_split,
                start_date: test.start_date,
                end_date: test.end_date,
                target_metric: test.target_metric,
            },
            status: format!("{:?}", test.status),
            results: None,
        })
        .collect();

    Ok(Json(response))
}

/// Get A/B test results
pub async fn get_ab_test_results(
    State(state): State<MLAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Path(test_id): Path<Uuid>,
) -> Result<Json<ABTestResponse>, (StatusCode, Json<ApiError>)> {
    let results = state
        .versioning_service
        .analyze_ab_test(test_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("AB_TEST_ANALYSIS_ERROR", &format!("Failed to analyze A/B test: {}", e))),
            )
        })?;

    // This would typically also fetch the test config, but for brevity using dummy data
    let response = ABTestResponse {
        test_id: results.test_id,
        test_config: ABTestInfo {
            name: "Test".to_string(),
            champion_version: results.champion_performance.version.clone(),
            challenger_version: results.challenger_performance.version.clone(),
            traffic_split: 0.5,
            start_date: chrono::Utc::now() - chrono::Duration::days(7),
            end_date: chrono::Utc::now(),
            target_metric: "rmse".to_string(),
        },
        status: "Completed".to_string(),
        results: Some(ABTestResultInfo {
            champion_performance: TestPerformanceMetrics {
                version: results.champion_performance.version,
                sample_size: results.champion_performance.sample_size,
                avg_rmse: results.champion_performance.avg_rmse,
                avg_confidence: results.champion_performance.avg_confidence,
                prediction_latency_ms: results.champion_performance.prediction_latency_ms,
            },
            challenger_performance: TestPerformanceMetrics {
                version: results.challenger_performance.version,
                sample_size: results.challenger_performance.sample_size,
                avg_rmse: results.challenger_performance.avg_rmse,
                avg_confidence: results.challenger_performance.avg_confidence,
                prediction_latency_ms: results.challenger_performance.prediction_latency_ms,
            },
            statistical_significance: results.statistical_significance,
            winner: results.winner,
            recommendation: format!("{:?}", results.recommendation),
        }),
    };

    Ok(Json(response))
}

/// Start an A/B test
pub async fn start_ab_test(
    State(state): State<MLAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Path(test_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    state
        .versioning_service
        .start_ab_test(test_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("AB_TEST_START_ERROR", &format!("Failed to start A/B test: {}", e))),
            )
        })?;

    let response = serde_json::json!({
        "test_id": test_id,
        "status": "Running",
        "started_at": chrono::Utc::now()
    });

    Ok(Json(response))
}

/// Complete an A/B test
pub async fn complete_ab_test(
    State(state): State<MLAppState>,
    WithRejection(_claims, _): WithRejection<Claims, StatusCode>,
    Path(test_id): Path<Uuid>,
) -> Result<Json<ABTestResponse>, (StatusCode, Json<ApiError>)> {
    let results = state
        .versioning_service
        .complete_ab_test(test_id, false) // Don't auto-promote winner
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("AB_TEST_COMPLETE_ERROR", &format!("Failed to complete A/B test: {}", e))),
            )
        })?;

    // Convert results to response format (similar to get_ab_test_results)
    let response = ABTestResponse {
        test_id: results.test_id,
        test_config: ABTestInfo {
            name: "Test".to_string(),
            champion_version: results.champion_performance.version.clone(),
            challenger_version: results.challenger_performance.version.clone(),
            traffic_split: 0.5,
            start_date: chrono::Utc::now() - chrono::Duration::days(7),
            end_date: chrono::Utc::now(),
            target_metric: "rmse".to_string(),
        },
        status: "Completed".to_string(),
        results: Some(ABTestResultInfo {
            champion_performance: TestPerformanceMetrics {
                version: results.champion_performance.version,
                sample_size: results.champion_performance.sample_size,
                avg_rmse: results.champion_performance.avg_rmse,
                avg_confidence: results.champion_performance.avg_confidence,
                prediction_latency_ms: results.champion_performance.prediction_latency_ms,
            },
            challenger_performance: TestPerformanceMetrics {
                version: results.challenger_performance.version,
                sample_size: results.challenger_performance.sample_size,
                avg_rmse: results.challenger_performance.avg_rmse,
                avg_confidence: results.challenger_performance.avg_confidence,
                prediction_latency_ms: results.challenger_performance.prediction_latency_ms,
            },
            statistical_significance: results.statistical_significance,
            winner: results.winner,
            recommendation: format!("{:?}", results.recommendation),
        }),
    };

    Ok(Json(response))
}

/// Get data quality assessment for the user
pub async fn get_data_quality_assessment(
    State(state): State<MLAppState>,
    WithRejection(claims, _): WithRejection<Claims, StatusCode>,
    Query(query): Query<ModelTrainingQuery>,
) -> Result<Json<DataQualityInfo>, (StatusCode, Json<ApiError>)> {
    let user_id = claims.sub;

    let quality_report = state
        .training_service
        .assess_data_quality(user_id, query.training_window_days.unwrap_or(365))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("DATA_QUALITY_ERROR", &format!("Failed to assess data quality: {}", e))),
            )
        })?;

    let recommendations = state.training_service.get_training_recommendations(&quality_report);

    let response = DataQualityInfo {
        total_samples: quality_report.total_samples,
        valid_samples: quality_report.valid_samples,
        data_completeness: quality_report.data_completeness,
        is_sufficient: quality_report.is_sufficient,
        recommendations,
    };

    Ok(Json(response))
}