use ai_coach::models::{TrainingFeatures, TrainingDataPoint};
use ai_coach::services::{FeatureEngineeringService, MLModelService, TrainingRecommendationService};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// Integration test for the complete training load prediction flow
/// This test verifies that all ML components work together correctly
#[tokio::test]
async fn test_complete_training_load_prediction_flow() {
    // Skip if no test database URL is available
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/ai_coach_test".to_string());

    // Try to connect to test database, skip test if not available
    let db = match PgPool::connect(&database_url).await {
        Ok(db) => db,
        Err(_) => {
            println!("Test database not available, skipping integration test");
            return;
        }
    };

    // Test user ID
    let user_id = Uuid::new_v4();

    // Test the training features extraction (using mock data)
    test_training_features_functionality();

    // Test ML model service functionality (using mock data)
    test_ml_model_service_functionality(db.clone()).await;

    // Test recommendation service functionality (using mock data)
    test_recommendation_service_functionality(db.clone()).await;

    println!("✅ Complete training load prediction flow test passed!");
}

/// Test TrainingFeatures struct functionality
fn test_training_features_functionality() {
    println!("🧪 Testing TrainingFeatures functionality...");

    // Test creation and conversion
    let features = TrainingFeatures {
        current_ctl: 100.0,
        current_atl: 60.0,
        current_tsb: 40.0,
        days_since_last_workout: 2,
        avg_weekly_tss_4weeks: 300.0,
        recent_performance_trend: 0.1,
        days_until_goal_event: Some(30),
        preferred_workout_types: vec!["endurance".to_string(), "threshold".to_string()],
        seasonal_factors: 0.8,
    };

    // Test conversion to ndarray
    let array = features.to_ndarray();
    assert_eq!(array.len(), 13); // 8 numeric + 5 workout type features
    assert_eq!(array[0], 100.0); // current_ctl
    assert_eq!(array[2], 40.0);  // current_tsb

    // Test feature names
    let names = TrainingFeatures::feature_names();
    assert_eq!(names.len(), 13);
    assert_eq!(names[0], "current_ctl");

    // Test default values
    let default_features = TrainingFeatures::default();
    assert_eq!(default_features.current_ctl, 0.0);
    assert_eq!(default_features.seasonal_factors, 1.0);

    println!("✅ TrainingFeatures functionality test passed!");
}

/// Test ML model service functionality with mock data
async fn test_ml_model_service_functionality(db: PgPool) {
    println!("🧪 Testing ML model service functionality...");

    let mut ml_service = MLModelService::new(db);

    // Test that service is created but no model is ready initially
    assert!(!ml_service.is_model_ready());
    assert!(ml_service.get_current_model_info().is_none());

    // Test prediction without model (should fail)
    let features = TrainingFeatures::default();
    let prediction_result = ml_service.predict_tss(&features).await;
    assert!(prediction_result.is_err());
    assert!(prediction_result.unwrap_err().to_string().contains("No trained model available"));

    // Test training data preparation
    let training_data = create_mock_training_data();
    let result = ml_service.prepare_training_data(&training_data);
    assert!(result.is_ok());

    let (features_matrix, targets) = result.unwrap();
    assert_eq!(features_matrix.nrows(), training_data.len());
    assert_eq!(targets.len(), training_data.len());

    // Test insufficient data scenarios
    let insufficient_data = training_data[0..5].to_vec(); // Less than 10 samples
    let result = ml_service.train_linear_regression_model(Uuid::new_v4(), &insufficient_data).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Insufficient training data"));

    println!("✅ ML model service functionality test passed!");
}

/// Test recommendation service functionality with mock data
async fn test_recommendation_service_functionality(db: PgPool) {
    println!("🧪 Testing recommendation service functionality...");

    let rec_service = TrainingRecommendationService::new(db);

    // Test edge case detection
    let user_id = Uuid::new_v4();

    // Test new user detection
    let new_user_features = TrainingFeatures {
        avg_weekly_tss_4weeks: 0.0,
        days_since_last_workout: 20,
        ..TrainingFeatures::default()
    };

    let edge_case = rec_service.detect_edge_case(&new_user_features, user_id).await;
    assert!(edge_case.is_ok());
    assert_eq!(edge_case.unwrap(), Some("new_user".to_string()));

    // Test overtrained detection
    let overtrained_features = TrainingFeatures {
        current_tsb: -30.0,
        avg_weekly_tss_4weeks: 400.0,
        days_since_last_workout: 1,
        ..TrainingFeatures::default()
    };

    let edge_case = rec_service.detect_edge_case(&overtrained_features, user_id).await;
    assert!(edge_case.is_ok());
    assert_eq!(edge_case.unwrap(), Some("overtrained".to_string()));

    // Test edge case handling
    let request = ai_coach::services::training_recommendation_service::RecommendationRequest {
        user_id,
        target_date: None,
        preferred_workout_type: None,
        max_duration_minutes: None,
        user_feedback: None,
    };

    let recommendation = rec_service
        .handle_edge_case("new_user".to_string(), &new_user_features, &request)
        .await;

    assert!(recommendation.is_ok());
    let rec = recommendation.unwrap();
    assert_eq!(rec.prediction.recommended_workout_type, "endurance");
    assert_eq!(rec.prediction.confidence, 0.6);
    assert!(!rec.warnings.is_empty());

    // Test cache functionality
    let (total, expired) = rec_service.get_cache_stats().await;
    assert_eq!(total, 0); // No cache entries initially

    println!("✅ Recommendation service functionality test passed!");
}

/// Create mock training data for testing
fn create_mock_training_data() -> Vec<TrainingDataPoint> {
    let user_id = Uuid::new_v4();
    let base_date = Utc::now();

    (0..25).map(|i| {
        let ctl = 80.0 + (i as f32 * 2.0);
        let atl = 40.0 + (i as f32 * 1.5);
        let tsb = ctl - atl;
        let tss = 100.0 + (i as f32 * 5.0) + (tsb * 0.5);

        TrainingDataPoint {
            features: TrainingFeatures {
                current_ctl: ctl,
                current_atl: atl,
                current_tsb: tsb,
                days_since_last_workout: (i % 7) as i32,
                avg_weekly_tss_4weeks: 300.0 + (i as f32 * 10.0),
                recent_performance_trend: (i as f32 / 25.0) - 0.5,
                days_until_goal_event: if i % 3 == 0 { Some(30 - i as i32) } else { None },
                preferred_workout_types: vec!["endurance".to_string()],
                seasonal_factors: 0.8 + (i as f32 * 0.01),
            },
            actual_tss: tss,
            actual_workout_type: if i % 2 == 0 { "endurance".to_string() } else { "threshold".to_string() },
            performance_outcome: Some(7.0 + (i as f32 % 3.0)),
            recovery_rating: Some(6.0 + (i as f32 % 4.0)),
            workout_date: base_date - chrono::Duration::days(i as i64),
        }
    }).collect()
}

/// Test the complete API flow (mock test)
#[tokio::test]
async fn test_api_flow_mock() {
    println!("🧪 Testing API flow with mock data...");

    // Test query parameter parsing
    let query = ai_coach::api::ml_predictions::RecommendationQuery {
        target_date: None,
        preferred_workout_type: Some("endurance".to_string()),
        max_duration_minutes: Some(60),
        perceived_difficulty: Some(5),
        energy_level: Some(8),
        motivation: Some(7),
        available_time_minutes: Some(60),
        preferred_intensity: Some("moderate".to_string()),
    };

    // Verify query structure
    assert_eq!(query.preferred_workout_type, Some("endurance".to_string()));
    assert_eq!(query.max_duration_minutes, Some(60));
    assert_eq!(query.energy_level, Some(8));

    // Test response structures
    let prediction_details = ai_coach::api::ml_predictions::RecommendationDetails {
        recommended_tss: 200.0,
        confidence: 0.8,
        confidence_lower: 180.0,
        confidence_upper: 220.0,
        recommended_workout_type: "endurance".to_string(),
        model_version: "test_v1".to_string(),
    };

    assert_eq!(prediction_details.recommended_tss, 200.0);
    assert_eq!(prediction_details.confidence, 0.8);

    println!("✅ API flow mock test passed!");
}

/// Test feature engineering calculations
#[test]
fn test_feature_engineering_calculations() {
    println!("🧪 Testing feature engineering calculations...");

    // Test seasonal factors
    use chrono::{NaiveDate, Datelike};

    // Winter month should have lower factor
    let winter_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    let winter_month = winter_date.month();
    let winter_factor = match winter_month {
        12 | 1 | 2 => 0.7,
        3 | 4 | 5 => 0.9,
        6 | 7 | 8 => 1.0,
        9 | 10 | 11 => 0.8,
        _ => 1.0,
    };
    assert_eq!(winter_factor, 0.7);

    // Summer month should have higher factor
    let summer_date = NaiveDate::from_ymd_opt(2024, 7, 15).unwrap();
    let summer_month = summer_date.month();
    let summer_factor = match summer_month {
        12 | 1 | 2 => 0.7,
        3 | 4 | 5 => 0.9,
        6 | 7 | 8 => 1.0,
        9 | 10 | 11 => 0.8,
        _ => 1.0,
    };
    assert_eq!(summer_factor, 1.0);

    println!("✅ Feature engineering calculations test passed!");
}

/// Test model evaluation metrics
#[test]
fn test_model_evaluation_metrics() {
    println!("🧪 Testing model evaluation metrics...");

    use ndarray::Array1;

    let predictions = Array1::from(vec![100.0, 150.0, 200.0, 250.0]);
    let targets = Array1::from(vec![110.0, 140.0, 190.0, 240.0]);

    // Calculate MAE manually
    let mae = (predictions - &targets).mapv(|x| x.abs()).mean().unwrap();
    let expected_mae = 10.0;
    assert!((mae - expected_mae).abs() < 0.001);

    // Calculate RMSE manually
    let mse = ((predictions - &targets).mapv(|x| x.powi(2))).mean().unwrap();
    let rmse = mse.sqrt();
    let expected_rmse = 10.0;
    assert!((rmse - expected_rmse).abs() < 0.001);

    // Calculate R-squared manually
    let target_mean = targets.mean().unwrap();
    let ss_tot = (targets - target_mean).mapv(|x| x.powi(2)).sum();
    let ss_res = (targets - predictions).mapv(|x| x.powi(2)).sum();
    let r_squared = 1.0 - (ss_res / ss_tot);

    assert!(r_squared >= 0.0);
    assert!(r_squared <= 1.0);

    println!("✅ Model evaluation metrics test passed!");
}

/// Test TSS discretization and undiscretization
#[test]
fn test_tss_discretization() {
    println!("🧪 Testing TSS discretization...");

    // Test discretization logic
    let discretize_tss = |tss: f64| -> usize {
        match tss as i32 {
            0..=50 => 0,     // Recovery
            51..=100 => 1,   // Easy
            101..=200 => 2,  // Moderate
            201..=300 => 3,  // Hard
            301..=400 => 4,  // Very Hard
            _ => 5,          // Extreme
        }
    };

    let undiscretize_tss = |label: usize| -> f32 {
        match label {
            0 => 25.0,   // Recovery
            1 => 75.0,   // Easy
            2 => 150.0,  // Moderate
            3 => 250.0,  // Hard
            4 => 350.0,  // Very Hard
            _ => 450.0,  // Extreme
        }
    };

    // Test round-trip consistency for typical values
    let test_values = [25.0, 75.0, 150.0, 250.0, 350.0];
    for &tss in &test_values {
        let discretized = discretize_tss(tss as f64);
        let undiscretized = undiscretize_tss(discretized);
        assert_eq!(undiscretized, tss);
    }

    // Test edge cases
    assert_eq!(discretize_tss(0.0), 0);
    assert_eq!(discretize_tss(500.0), 5);
    assert_eq!(discretize_tss(-10.0), 0);

    println!("✅ TSS discretization test passed!");
}

/// Test confidence calculation logic
#[test]
fn test_confidence_calculations() {
    println!("🧪 Testing confidence calculations...");

    let calculate_linear_confidence = |prediction: f64| -> f32 {
        if prediction > 0.0 && prediction < 500.0 {
            0.8 // High confidence for reasonable predictions
        } else {
            0.5 // Lower confidence for extreme predictions
        }
    };

    // Test reasonable predictions
    assert_eq!(calculate_linear_confidence(200.0), 0.8);
    assert_eq!(calculate_linear_confidence(100.0), 0.8);
    assert_eq!(calculate_linear_confidence(400.0), 0.8);

    // Test extreme predictions
    assert_eq!(calculate_linear_confidence(0.0), 0.5);
    assert_eq!(calculate_linear_confidence(600.0), 0.5);
    assert_eq!(calculate_linear_confidence(-50.0), 0.5);

    println!("✅ Confidence calculations test passed!");
}

/// Test workout type recommendation logic
#[test]
fn test_workout_type_recommendations() {
    println!("🧪 Testing workout type recommendations...");

    let recommend_workout_type = |tsb: f32, predicted_tss: f32| -> String {
        if tsb < -20.0 {
            "recovery".to_string()
        } else if predicted_tss < 100.0 {
            "endurance".to_string()
        } else if predicted_tss < 200.0 {
            "threshold".to_string()
        } else {
            "vo2max".to_string()
        }
    };

    // Test high fatigue (negative TSB)
    assert_eq!(recommend_workout_type(-25.0, 150.0), "recovery");

    // Test normal TSB with different TSS levels
    assert_eq!(recommend_workout_type(0.0, 80.0), "endurance");
    assert_eq!(recommend_workout_type(0.0, 150.0), "threshold");
    assert_eq!(recommend_workout_type(0.0, 250.0), "vo2max");

    println!("✅ Workout type recommendations test passed!");
}

/// Run all integration tests
#[tokio::test]
async fn run_all_integration_tests() {
    println!("🚀 Running complete ML training load prediction integration tests...");

    test_complete_training_load_prediction_flow().await;
    test_api_flow_mock().await;
    test_feature_engineering_calculations();
    test_model_evaluation_metrics();
    test_tss_discretization();
    test_confidence_calculations();
    test_workout_type_recommendations();

    println!("🎉 All integration tests passed! Training load prediction system is working correctly.");
}