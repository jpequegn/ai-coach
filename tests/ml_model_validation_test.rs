use ndarray::{Array1, Array2, array};
use uuid::Uuid;
use chrono::{Utc, Duration};

use ai_coach::models::*;
use ai_coach::services::{MLModelService, FeatureEngineeringService};
use crate::common::{TestDatabase, MockDataGenerator, DatabaseTestHelpers};

#[cfg(test)]
mod ml_model_validation_tests {
    use super::*;

    /// Helper function to create training data with known patterns
    fn create_synthetic_training_data(samples: usize) -> Vec<TrainingDataPoint> {
        let mut data = Vec::new();

        for i in 0..samples {
            let ctl = 50.0 + (i as f32 * 2.0);
            let atl = 30.0 + (i as f32 * 1.5);
            let tsb = ctl - atl;

            // Create predictable pattern: TSS = CTL * 0.8 + noise
            let base_tss = ctl * 0.8;
            let noise = (i as f32 % 10.0) - 5.0; // Small noise
            let actual_tss = base_tss + noise;

            let features = TrainingFeatures {
                current_ctl: ctl,
                current_atl: atl,
                current_tsb: tsb,
                days_since_last_workout: (i % 7) as i32,
                avg_weekly_tss_4weeks: ctl * 7.0,
                recent_performance_trend: 0.1,
                days_until_goal_event: Some(30),
                preferred_workout_types: vec!["endurance".to_string()],
                seasonal_factors: 1.0,
            };

            data.push(TrainingDataPoint {
                features,
                actual_tss,
                actual_workout_type: "endurance".to_string(),
                performance_outcome: Some(7.0),
                recovery_rating: Some(6.0),
                workout_date: Utc::now() - Duration::days(i as i64),
            });
        }

        data
    }

    /// Helper function to create diverse training data for robustness testing
    fn create_diverse_training_data(samples: usize) -> Vec<TrainingDataPoint> {
        let mut data = Vec::new();
        let workout_types = ["endurance", "threshold", "vo2max", "recovery", "strength"];

        for i in 0..samples {
            let ctl = 30.0 + (i as f32 * 3.0) % 150.0; // Vary CTL widely
            let atl = 20.0 + (i as f32 * 2.0) % 100.0; // Vary ATL widely
            let tsb = ctl - atl;

            let workout_type = workout_types[i % workout_types.len()];
            let type_multiplier = match workout_type {
                "endurance" => 0.7,
                "threshold" => 1.0,
                "vo2max" => 1.3,
                "recovery" => 0.4,
                "strength" => 0.6,
                _ => 0.8,
            };

            let actual_tss = ctl * type_multiplier + (i as f32 % 20.0) - 10.0;

            let features = TrainingFeatures {
                current_ctl: ctl,
                current_atl: atl,
                current_tsb: tsb,
                days_since_last_workout: (i % 14) as i32,
                avg_weekly_tss_4weeks: ctl * 7.0,
                recent_performance_trend: ((i as f32 % 20.0) - 10.0) / 10.0, // -1.0 to 1.0
                days_until_goal_event: if i % 3 == 0 { Some((i % 100) as i32) } else { None },
                preferred_workout_types: vec![workout_type.to_string()],
                seasonal_factors: 0.8 + (i as f32 % 10.0) / 25.0, // 0.8 to 1.2
            };

            data.push(TrainingDataPoint {
                features,
                actual_tss,
                actual_workout_type: workout_type.to_string(),
                performance_outcome: Some(5.0 + (i as f32 % 10.0) / 2.0), // 5.0 to 10.0
                recovery_rating: Some(4.0 + (i as f32 % 12.0) / 2.0), // 4.0 to 10.0
                workout_date: Utc::now() - Duration::days(i as i64),
            });
        }

        data
    }

    #[tokio::test]
    async fn test_linear_regression_model_training() {
        let test_db = TestDatabase::new().await;
        let mut ml_service = MLModelService::new(test_db.pool.clone());

        let user_id = Uuid::new_v4();
        let training_data = create_synthetic_training_data(50);

        // Test successful model training
        let result = ml_service
            .train_linear_regression_model(user_id, &training_data)
            .await;

        assert!(result.is_ok(), "Linear regression training should succeed");

        let metrics = result.unwrap();

        // Validate metrics are within reasonable bounds
        assert!(metrics.mae_tss >= 0.0, "MAE should be non-negative");
        assert!(metrics.rmse_tss >= 0.0, "RMSE should be non-negative");
        assert!(metrics.r_squared >= 0.0 && metrics.r_squared <= 1.0, "R-squared should be between 0 and 1");
        assert_eq!(metrics.sample_count, 10, "Test set should have 20% of samples (10)");
        assert!(!metrics.model_version.is_empty(), "Model version should not be empty");

        // For synthetic data with clear pattern, we should get good metrics
        assert!(metrics.r_squared > 0.7, "R-squared should be > 0.7 for synthetic data");
        assert!(metrics.rmse_tss < 20.0, "RMSE should be reasonably low for synthetic data");
    }

    #[tokio::test]
    async fn test_random_forest_model_training() {
        let test_db = TestDatabase::new().await;
        let mut ml_service = MLModelService::new(test_db.pool.clone());

        let user_id = Uuid::new_v4();
        let training_data = create_diverse_training_data(100); // More data for Random Forest

        // Test successful model training
        let result = ml_service
            .train_random_forest_model(user_id, &training_data)
            .await;

        assert!(result.is_ok(), "Random forest training should succeed");

        let metrics = result.unwrap();

        // Validate metrics
        assert!(metrics.mae_tss >= 0.0, "MAE should be non-negative");
        assert!(metrics.rmse_tss >= 0.0, "RMSE should be non-negative");
        assert!(metrics.r_squared >= 0.0 && metrics.r_squared <= 1.0, "R-squared should be between 0 and 1");
        assert_eq!(metrics.sample_count, 20, "Test set should have 20% of samples (20)");
        assert!(!metrics.model_version.is_empty(), "Model version should not be empty");
        assert!(metrics.model_version.starts_with("forest_v"), "Model version should indicate forest type");
    }

    #[tokio::test]
    async fn test_insufficient_training_data() {
        let test_db = TestDatabase::new().await;
        let mut ml_service = MLModelService::new(test_db.pool.clone());

        let user_id = Uuid::new_v4();

        // Test linear regression with insufficient data
        let small_data = create_synthetic_training_data(5);
        let result = ml_service
            .train_linear_regression_model(user_id, &small_data)
            .await;

        assert!(result.is_err(), "Should fail with insufficient data");
        assert!(result.unwrap_err().to_string().contains("Insufficient training data"));

        // Test random forest with insufficient data
        let medium_data = create_synthetic_training_data(15);
        let result = ml_service
            .train_random_forest_model(user_id, &medium_data)
            .await;

        assert!(result.is_err(), "Should fail with insufficient data for Random Forest");
        assert!(result.unwrap_err().to_string().contains("Insufficient training data"));
    }

    #[tokio::test]
    async fn test_model_prediction_accuracy() {
        let test_db = TestDatabase::new().await;
        let mut ml_service = MLModelService::new(test_db.pool.clone());

        let user_id = Uuid::new_v4();
        let training_data = create_synthetic_training_data(50);

        // Train the model
        ml_service
            .train_linear_regression_model(user_id, &training_data)
            .await
            .unwrap();

        // Test prediction on known pattern
        let test_features = TrainingFeatures {
            current_ctl: 100.0,
            current_atl: 75.0,
            current_tsb: 25.0,
            days_since_last_workout: 1,
            avg_weekly_tss_4weeks: 700.0,
            recent_performance_trend: 0.1,
            days_until_goal_event: Some(30),
            preferred_workout_types: vec!["endurance".to_string()],
            seasonal_factors: 1.0,
        };

        let prediction = ml_service.predict_tss(&test_features).await;
        assert!(prediction.is_ok(), "Prediction should succeed");

        let prediction = prediction.unwrap();

        // Expected TSS should be around CTL * 0.8 = 80.0 based on our training pattern
        let expected_tss = 80.0;
        let tolerance = 15.0; // Allow some error

        assert!(
            (prediction.recommended_tss - expected_tss).abs() < tolerance,
            "Prediction {} should be within {} of expected {}",
            prediction.recommended_tss,
            tolerance,
            expected_tss
        );

        assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0, "Confidence should be between 0 and 1");
        assert!(!prediction.model_version.is_empty(), "Model version should not be empty");
        assert!(!prediction.recommended_workout_type.is_empty(), "Workout type should not be empty");
    }

    #[tokio::test]
    async fn test_prediction_without_trained_model() {
        let test_db = TestDatabase::new().await;
        let ml_service = MLModelService::new(test_db.pool.clone());

        let test_features = TrainingFeatures::new();
        let result = ml_service.predict_tss(&test_features).await;

        assert!(result.is_err(), "Should fail when no model is trained");
        assert!(result.unwrap_err().to_string().contains("No trained model available"));
    }

    #[tokio::test]
    async fn test_feature_scaling() {
        use ai_coach::services::ml_model_service::FeatureScaler;

        // Create test data with different scales
        let features = Array2::from_shape_vec(
            (3, 2),
            vec![1.0, 100.0, 2.0, 200.0, 3.0, 300.0],
        ).unwrap();

        // Fit scaler
        let scaler = FeatureScaler::fit(&features);

        // Check means
        assert!((scaler.means[0] - 2.0).abs() < 1e-6, "Mean should be 2.0 for first feature");
        assert!((scaler.means[1] - 200.0).abs() < 1e-6, "Mean should be 200.0 for second feature");

        // Transform features
        let scaled = scaler.transform(&features);

        // Check that transformed features have mean close to 0
        let scaled_means = scaled.mean_axis(ndarray::Axis(0)).unwrap();
        assert!(scaled_means[0].abs() < 1e-10, "Scaled mean should be close to 0");
        assert!(scaled_means[1].abs() < 1e-10, "Scaled mean should be close to 0");

        // Test single feature transformation
        let single_feature = Array1::from(vec![2.0, 200.0]);
        let scaled_single = scaler.transform_single(&single_feature);

        // Should be close to 0 since it's the mean
        assert!(scaled_single[0].abs() < 1e-10, "Scaled single feature should be close to 0");
        assert!(scaled_single[1].abs() < 1e-10, "Scaled single feature should be close to 0");
    }

    #[tokio::test]
    async fn test_training_features_to_ndarray() {
        let features = TrainingFeatures {
            current_ctl: 100.0,
            current_atl: 75.0,
            current_tsb: 25.0,
            days_since_last_workout: 3,
            avg_weekly_tss_4weeks: 700.0,
            recent_performance_trend: 0.2,
            days_until_goal_event: Some(45),
            preferred_workout_types: vec!["endurance".to_string(), "threshold".to_string()],
            seasonal_factors: 0.9,
        };

        let ndarray = features.to_ndarray();

        // Check basic features
        assert_eq!(ndarray[0], 100.0); // current_ctl
        assert_eq!(ndarray[1], 75.0);  // current_atl
        assert_eq!(ndarray[2], 25.0);  // current_tsb
        assert_eq!(ndarray[3], 3.0);   // days_since_last_workout
        assert_eq!(ndarray[4], 700.0); // avg_weekly_tss_4weeks
        assert_eq!(ndarray[5], 0.2);   // recent_performance_trend
        assert_eq!(ndarray[6], 45.0);  // days_until_goal_event
        assert_eq!(ndarray[7], 0.9);   // seasonal_factors

        // Check one-hot encoding for workout types
        // endurance, threshold, vo2max, recovery, strength
        assert_eq!(ndarray[8], 1.0);   // endurance (present)
        assert_eq!(ndarray[9], 1.0);   // threshold (present)
        assert_eq!(ndarray[10], 0.0);  // vo2max (not present)
        assert_eq!(ndarray[11], 0.0);  // recovery (not present)
        assert_eq!(ndarray[12], 0.0);  // strength (not present)

        // Total length should be 8 basic features + 5 workout type features
        assert_eq!(ndarray.len(), 13);
    }

    #[tokio::test]
    async fn test_training_features_edge_cases() {
        // Test with None for days_until_goal_event
        let features = TrainingFeatures {
            current_ctl: 50.0,
            current_atl: 40.0,
            current_tsb: 10.0,
            days_since_last_workout: 0,
            avg_weekly_tss_4weeks: 350.0,
            recent_performance_trend: -0.5,
            days_until_goal_event: None, // This should be converted to -1
            preferred_workout_types: vec![], // Empty preferences
            seasonal_factors: 0.5,
        };

        let ndarray = features.to_ndarray();

        assert_eq!(ndarray[6], -1.0); // None should become -1

        // All workout type one-hot encodings should be 0
        for i in 8..13 {
            assert_eq!(ndarray[i], 0.0, "Workout type encoding at index {} should be 0", i);
        }
    }

    #[tokio::test]
    async fn test_model_metrics_validation() {
        // Test with known values
        let actual = Array1::from(vec![100.0, 120.0, 80.0, 90.0, 110.0]);
        let predicted = Array1::from(vec![95.0, 125.0, 85.0, 88.0, 115.0]);

        // Calculate metrics manually for validation
        let mae = (&actual - &predicted).mapv(|x| x.abs()).mean().unwrap();
        let mse = (&actual - &predicted).mapv(|x| x.powi(2)).mean().unwrap();
        let rmse = mse.sqrt();

        // Calculate R-squared
        let actual_mean = actual.mean().unwrap();
        let ss_tot = (&actual - actual_mean).mapv(|x| x.powi(2)).sum();
        let ss_res = (&actual - &predicted).mapv(|x| x.powi(2)).sum();
        let r_squared = 1.0 - (ss_res / ss_tot);

        // Validate calculations
        assert!((mae - 4.0).abs() < 0.1, "MAE should be approximately 4.0");
        assert!((rmse - 4.58).abs() < 0.1, "RMSE should be approximately 4.58");
        assert!(r_squared > 0.9, "R-squared should be > 0.9 for this close prediction");

        // Test metric bounds
        assert!(mae >= 0.0, "MAE should be non-negative");
        assert!(rmse >= 0.0, "RMSE should be non-negative");
        assert!(rmse >= mae, "RMSE should be >= MAE");
    }

    #[tokio::test]
    async fn test_model_overfitting_detection() {
        let test_db = TestDatabase::new().await;
        let mut ml_service = MLModelService::new(test_db.pool.clone());

        let user_id = Uuid::new_v4();

        // Create very small dataset that could lead to overfitting
        let training_data = create_synthetic_training_data(15);

        let result = ml_service
            .train_linear_regression_model(user_id, &training_data)
            .await;

        assert!(result.is_ok(), "Should still train with minimal data");

        let metrics = result.unwrap();

        // With very little data, we might see perfect or near-perfect metrics
        // which could indicate overfitting. Test set is only 3 samples (20% of 15)
        assert_eq!(metrics.sample_count, 3, "Should have 3 test samples");

        // Even with potential overfitting, metrics should be bounded
        assert!(metrics.r_squared <= 1.0, "R-squared should not exceed 1.0");
        assert!(metrics.mae_tss >= 0.0, "MAE should be non-negative");
    }

    #[tokio::test]
    async fn test_model_robustness_with_noise() {
        let test_db = TestDatabase::new().await;
        let mut ml_service = MLModelService::new(test_db.pool.clone());

        let user_id = Uuid::new_v4();

        // Create noisy training data
        let mut noisy_data = create_synthetic_training_data(50);

        // Add significant noise to the data
        for (i, point) in noisy_data.iter_mut().enumerate() {
            let noise = (i as f32 * 37.0) % 40.0 - 20.0; // Random-ish noise ±20
            point.actual_tss += noise;
        }

        let result = ml_service
            .train_linear_regression_model(user_id, &noisy_data)
            .await;

        assert!(result.is_ok(), "Should handle noisy data");

        let metrics = result.unwrap();

        // With noisy data, we expect worse metrics but still reasonable
        assert!(metrics.r_squared >= 0.0, "R-squared should be non-negative even with noise");
        assert!(metrics.rmse_tss > 5.0, "RMSE should be higher with noisy data");
        assert!(metrics.mae_tss > 3.0, "MAE should be higher with noisy data");
    }

    #[tokio::test]
    async fn test_model_comparison() {
        let test_db = TestDatabase::new().await;
        let mut ml_service = MLModelService::new(test_db.pool.clone());

        let user_id = Uuid::new_v4();
        let training_data = create_diverse_training_data(100);

        // Train linear regression
        let linear_metrics = ml_service
            .train_linear_regression_model(user_id, &training_data)
            .await
            .unwrap();

        // Train random forest on same data
        let forest_metrics = ml_service
            .train_random_forest_model(user_id, &training_data)
            .await
            .unwrap();

        // Both should produce valid metrics
        assert!(linear_metrics.mae_tss >= 0.0);
        assert!(forest_metrics.mae_tss >= 0.0);
        assert!(linear_metrics.r_squared >= 0.0 && linear_metrics.r_squared <= 1.0);
        assert!(forest_metrics.r_squared >= 0.0 && forest_metrics.r_squared <= 1.0);

        // Models should have different versions
        assert_ne!(linear_metrics.model_version, forest_metrics.model_version);
        assert!(linear_metrics.model_version.starts_with("linear_v"));
        assert!(forest_metrics.model_version.starts_with("forest_v"));

        println!("Linear Regression Metrics:");
        println!("  MAE: {:.2}", linear_metrics.mae_tss);
        println!("  RMSE: {:.2}", linear_metrics.rmse_tss);
        println!("  R²: {:.3}", linear_metrics.r_squared);

        println!("Random Forest Metrics:");
        println!("  MAE: {:.2}", forest_metrics.mae_tss);
        println!("  RMSE: {:.2}", forest_metrics.rmse_tss);
        println!("  R²: {:.3}", forest_metrics.r_squared);
    }

    #[tokio::test]
    async fn test_cross_validation_simulation() {
        let test_db = TestDatabase::new().await;

        let training_data = create_diverse_training_data(100);
        let fold_size = training_data.len() / 5; // 5-fold CV

        let mut all_metrics = Vec::new();

        // Simulate 5-fold cross validation
        for fold in 0..5 {
            let mut ml_service = MLModelService::new(test_db.pool.clone());
            let user_id = Uuid::new_v4();

            // Create training set (excluding current fold)
            let mut train_data = Vec::new();
            for (i, point) in training_data.iter().enumerate() {
                if i < fold * fold_size || i >= (fold + 1) * fold_size {
                    train_data.push(point.clone());
                }
            }

            if train_data.len() >= 10 {
                let result = ml_service
                    .train_linear_regression_model(user_id, &train_data)
                    .await;

                if let Ok(metrics) = result {
                    all_metrics.push(metrics);
                }
            }
        }

        assert!(!all_metrics.is_empty(), "Should have at least some successful folds");

        // Calculate average metrics across folds
        let avg_mae = all_metrics.iter().map(|m| m.mae_tss).sum::<f32>() / all_metrics.len() as f32;
        let avg_rmse = all_metrics.iter().map(|m| m.rmse_tss).sum::<f32>() / all_metrics.len() as f32;
        let avg_r2 = all_metrics.iter().map(|m| m.r_squared).sum::<f32>() / all_metrics.len() as f32;

        println!("Cross-Validation Results (avg of {} folds):", all_metrics.len());
        println!("  Avg MAE: {:.2}", avg_mae);
        println!("  Avg RMSE: {:.2}", avg_rmse);
        println!("  Avg R²: {:.3}", avg_r2);

        // Validate that cross-validation metrics are reasonable
        assert!(avg_mae >= 0.0, "Average MAE should be non-negative");
        assert!(avg_rmse >= avg_mae, "Average RMSE should be >= average MAE");
        assert!(avg_r2 >= 0.0 && avg_r2 <= 1.0, "Average R² should be between 0 and 1");
    }
}