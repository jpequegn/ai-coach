use anyhow::{Result, anyhow};
use chrono::Utc;
use ndarray::{Array1, Array2};
use linfa::prelude::*;
use linfa_linear::LinearRegression;
use linfa_trees::RandomForest;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{TrainingFeatures, TrainingLoadPrediction, ModelMetrics, TrainingDataPoint};
use crate::services::FeatureEngineeringService;

/// Machine Learning model types
#[derive(Debug, Clone)]
pub enum ModelType {
    LinearRegression,
    RandomForest,
}

/// Trained ML model for TSS prediction
#[derive(Debug, Clone)]
pub struct TrainedModel {
    pub model_type: ModelType,
    pub model_version: String,
    pub linear_model: Option<LinearRegression<f64>>,
    pub forest_model: Option<RandomForest<f64, usize>>,
    pub feature_scaler: FeatureScaler,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Feature scaling for normalization
#[derive(Debug, Clone)]
pub struct FeatureScaler {
    pub means: Array1<f64>,
    pub stds: Array1<f64>,
}

impl FeatureScaler {
    /// Create a new scaler from training data
    pub fn fit(features: &Array2<f64>) -> Self {
        let means = features.mean_axis(ndarray::Axis(0)).unwrap();
        let stds = features.std_axis(ndarray::Axis(0), 0.0);

        Self { means, stds }
    }

    /// Transform features using fitted scaler
    pub fn transform(&self, features: &Array2<f64>) -> Array2<f64> {
        (features - &self.means) / &self.stds
    }

    /// Transform a single feature vector
    pub fn transform_single(&self, features: &Array1<f64>) -> Array1<f64> {
        (features - &self.means) / &self.stds
    }
}

/// Service for training and using ML models
#[derive(Clone)]
pub struct MLModelService {
    db: PgPool,
    feature_service: FeatureEngineeringService,
    current_model: Option<TrainedModel>,
}

impl MLModelService {
    /// Create a new MLModelService
    pub fn new(db: PgPool) -> Self {
        let feature_service = FeatureEngineeringService::new(db.clone());

        Self {
            db,
            feature_service,
            current_model: None,
        }
    }

    /// Train a linear regression model for TSS prediction
    pub async fn train_linear_regression_model(
        &mut self,
        user_id: Uuid,
        training_data: &[TrainingDataPoint],
    ) -> Result<ModelMetrics> {
        if training_data.len() < 10 {
            return Err(anyhow!("Insufficient training data: need at least 10 samples, got {}", training_data.len()));
        }

        // Prepare training data
        let (features, targets) = self.prepare_training_data(training_data)?;

        // Split into train/test
        let split_idx = (features.nrows() as f64 * 0.8) as usize;
        let (train_features, test_features) = features.view().split_at(ndarray::Axis(0), split_idx);
        let (train_targets, test_targets) = targets.view().split_at(ndarray::Axis(0), split_idx);

        // Fit feature scaler
        let scaler = FeatureScaler::fit(&train_features.to_owned());
        let scaled_train_features = scaler.transform(&train_features.to_owned());

        // Create dataset
        let train_dataset = Dataset::new(scaled_train_features, train_targets.to_owned());

        // Train linear regression
        let linear_model = LinearRegression::default().fit(&train_dataset)?;

        // Create trained model
        let model_version = format!("linear_v{}", Utc::now().timestamp());
        let trained_model = TrainedModel {
            model_type: ModelType::LinearRegression,
            model_version: model_version.clone(),
            linear_model: Some(linear_model),
            forest_model: None,
            feature_scaler: scaler,
            created_at: Utc::now(),
        };

        // Evaluate model
        let metrics = self.evaluate_model(&trained_model, &test_features.to_owned(), &test_targets.to_owned())?;

        // Store the model
        self.current_model = Some(trained_model);

        Ok(metrics)
    }

    /// Train a Random Forest model for more sophisticated predictions
    pub async fn train_random_forest_model(
        &mut self,
        user_id: Uuid,
        training_data: &[TrainingDataPoint],
    ) -> Result<ModelMetrics> {
        if training_data.len() < 20 {
            return Err(anyhow!("Insufficient training data for Random Forest: need at least 20 samples, got {}", training_data.len()));
        }

        // Prepare training data
        let (features, targets) = self.prepare_training_data(training_data)?;

        // Split into train/test
        let split_idx = (features.nrows() as f64 * 0.8) as usize;
        let (train_features, test_features) = features.view().split_at(ndarray::Axis(0), split_idx);
        let (train_targets, test_targets) = targets.view().split_at(ndarray::Axis(0), split_idx);

        // Fit feature scaler
        let scaler = FeatureScaler::fit(&train_features.to_owned());
        let scaled_train_features = scaler.transform(&train_features.to_owned());

        // Convert targets to discrete labels for Random Forest
        let train_labels: Array1<usize> = train_targets.mapv(|x| self.discretize_tss(x));

        // Create dataset
        let train_dataset = Dataset::new(scaled_train_features, train_labels);

        // Train Random Forest
        let forest_model = RandomForest::params()
            .n_trees(100)
            .max_depth(Some(10))
            .min_samples_split(5)
            .fit(&train_dataset)?;

        // Create trained model
        let model_version = format!("forest_v{}", Utc::now().timestamp());
        let trained_model = TrainedModel {
            model_type: ModelType::RandomForest,
            model_version: model_version.clone(),
            linear_model: None,
            forest_model: Some(forest_model),
            feature_scaler: scaler,
            created_at: Utc::now(),
        };

        // Evaluate model
        let metrics = self.evaluate_model(&trained_model, &test_features.to_owned(), &test_targets.to_owned())?;

        // Store the model
        self.current_model = Some(trained_model);

        Ok(metrics)
    }

    /// Make a TSS prediction using the current model
    pub async fn predict_tss(&self, features: &TrainingFeatures) -> Result<TrainingLoadPrediction> {
        let model = self.current_model.as_ref()
            .ok_or_else(|| anyhow!("No trained model available"))?;

        // Convert features to ndarray
        let feature_array = features.to_ndarray();
        let scaled_features = model.feature_scaler.transform_single(&feature_array);

        let (prediction, confidence) = match &model.model_type {
            ModelType::LinearRegression => {
                let linear_model = model.linear_model.as_ref()
                    .ok_or_else(|| anyhow!("Linear model not available"))?;

                let pred = linear_model.predict(&scaled_features.insert_axis(ndarray::Axis(0)))[0];
                let confidence = self.calculate_linear_confidence(pred);
                (pred as f32, confidence)
            }
            ModelType::RandomForest => {
                let forest_model = model.forest_model.as_ref()
                    .ok_or_else(|| anyhow!("Random Forest model not available"))?;

                let pred_label = forest_model.predict(&scaled_features.insert_axis(ndarray::Axis(0)))[0];
                let pred = self.undiscretize_tss(pred_label);
                let confidence = self.calculate_forest_confidence(&forest_model, &scaled_features);
                (pred, confidence)
            }
        };

        // Calculate confidence intervals (simplified approach)
        let confidence_interval = prediction * 0.1; // 10% confidence interval
        let confidence_lower = prediction - confidence_interval;
        let confidence_upper = prediction + confidence_interval;

        // Determine recommended workout type based on features and prediction
        let recommended_workout_type = self.recommend_workout_type(features, prediction);

        Ok(TrainingLoadPrediction {
            recommended_tss: prediction.max(0.0), // Ensure non-negative
            confidence,
            confidence_lower: confidence_lower.max(0.0),
            confidence_upper,
            model_version: model.model_version.clone(),
            recommended_workout_type,
            predicted_at: Utc::now(),
        })
    }

    /// Prepare training data from TrainingDataPoint vector
    fn prepare_training_data(&self, data: &[TrainingDataPoint]) -> Result<(Array2<f64>, Array1<f64>)> {
        let n_samples = data.len();
        let n_features = TrainingFeatures::feature_names().len();

        let mut features = Array2::zeros((n_samples, n_features));
        let mut targets = Array1::zeros(n_samples);

        for (i, point) in data.iter().enumerate() {
            let feature_array = point.features.to_ndarray();
            features.row_mut(i).assign(&feature_array);
            targets[i] = point.actual_tss as f64;
        }

        Ok((features, targets))
    }

    /// Evaluate model performance
    fn evaluate_model(
        &self,
        model: &TrainedModel,
        test_features: &Array2<f64>,
        test_targets: &Array1<f64>,
    ) -> Result<ModelMetrics> {
        let scaled_test_features = model.feature_scaler.transform(test_features);

        let predictions = match &model.model_type {
            ModelType::LinearRegression => {
                let linear_model = model.linear_model.as_ref()
                    .ok_or_else(|| anyhow!("Linear model not available"))?;
                linear_model.predict(&scaled_test_features)
            }
            ModelType::RandomForest => {
                let forest_model = model.forest_model.as_ref()
                    .ok_or_else(|| anyhow!("Random Forest model not available"))?;
                let pred_labels = forest_model.predict(&scaled_test_features);
                pred_labels.mapv(|label| self.undiscretize_tss(label) as f64)
            }
        };

        // Calculate metrics
        let mae = self.calculate_mae(&predictions, test_targets);
        let rmse = self.calculate_rmse(&predictions, test_targets);
        let r_squared = self.calculate_r_squared(&predictions, test_targets);

        Ok(ModelMetrics {
            mae_tss: mae as f32,
            rmse_tss: rmse as f32,
            r_squared: r_squared as f32,
            sample_count: test_targets.len(),
            model_version: model.model_version.clone(),
            evaluated_at: Utc::now(),
        })
    }

    /// Convert continuous TSS to discrete labels for Random Forest
    fn discretize_tss(&self, tss: f64) -> usize {
        match tss as i32 {
            0..=50 => 0,     // Recovery
            51..=100 => 1,   // Easy
            101..=200 => 2,  // Moderate
            201..=300 => 3,  // Hard
            301..=400 => 4,  // Very Hard
            _ => 5,          // Extreme
        }
    }

    /// Convert discrete labels back to TSS values
    fn undiscretize_tss(&self, label: usize) -> f32 {
        match label {
            0 => 25.0,   // Recovery
            1 => 75.0,   // Easy
            2 => 150.0,  // Moderate
            3 => 250.0,  // Hard
            4 => 350.0,  // Very Hard
            _ => 450.0,  // Extreme
        }
    }

    /// Calculate confidence for linear regression predictions
    fn calculate_linear_confidence(&self, prediction: f64) -> f32 {
        // Simplified confidence calculation
        // In practice, this would use prediction intervals from the model
        if prediction > 0.0 && prediction < 500.0 {
            0.8 // High confidence for reasonable predictions
        } else {
            0.5 // Lower confidence for extreme predictions
        }
    }

    /// Calculate confidence for Random Forest predictions
    fn calculate_forest_confidence(&self, _model: &RandomForest<f64, usize>, _features: &Array1<f64>) -> f32 {
        // Simplified confidence calculation
        // In practice, this would use ensemble variance or similar
        0.75
    }

    /// Recommend workout type based on features and predicted TSS
    fn recommend_workout_type(&self, features: &TrainingFeatures, predicted_tss: f32) -> String {
        // Simple heuristic for workout type recommendation
        if features.current_tsb < -20.0 {
            "recovery".to_string()
        } else if predicted_tss < 100.0 {
            "endurance".to_string()
        } else if predicted_tss < 200.0 {
            "threshold".to_string()
        } else {
            "vo2max".to_string()
        }
    }

    /// Calculate Mean Absolute Error
    fn calculate_mae(&self, predictions: &Array1<f64>, targets: &Array1<f64>) -> f64 {
        (predictions - targets).mapv(|x| x.abs()).mean().unwrap()
    }

    /// Calculate Root Mean Square Error
    fn calculate_rmse(&self, predictions: &Array1<f64>, targets: &Array1<f64>) -> f64 {
        ((predictions - targets).mapv(|x| x.powi(2)).mean().unwrap()).sqrt()
    }

    /// Calculate R-squared
    fn calculate_r_squared(&self, predictions: &Array1<f64>, targets: &Array1<f64>) -> f64 {
        let target_mean = targets.mean().unwrap();
        let ss_tot = (targets - target_mean).mapv(|x| x.powi(2)).sum();
        let ss_res = (targets - predictions).mapv(|x| x.powi(2)).sum();
        1.0 - (ss_res / ss_tot)
    }

    /// Get current model information
    pub fn get_current_model_info(&self) -> Option<String> {
        self.current_model.as_ref().map(|model| model.model_version.clone())
    }

    /// Check if a model is trained and ready
    pub fn is_model_ready(&self) -> bool {
        self.current_model.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sqlx::PgPool;
    use std::env;
    use uuid::Uuid;

    async fn setup_test_db() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/ai_coach_test".to_string());

        sqlx::PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    fn create_test_service(db: PgPool) -> MLModelService {
        MLModelService::new(db)
    }

    fn create_test_training_data() -> Vec<TrainingDataPoint> {
        let user_id = Uuid::new_v4();
        let base_date = Utc::now();

        (0..25).map(|i| {
            let ctl = 80.0 + (i as f32 * 2.0);
            let atl = 40.0 + (i as f32 * 1.5);
            let tsb = ctl - atl;
            let tss = 100.0 + (i as f32 * 5.0) + (tsb * 0.5); // TSS correlates with TSB

            TrainingDataPoint {
                features: TrainingFeatures {
                    current_ctl: ctl,
                    current_atl: atl,
                    current_tsb: tsb,
                    days_since_last_workout: (i % 7) as i32,
                    avg_weekly_tss_4weeks: 300.0 + (i as f32 * 10.0),
                    recent_performance_trend: (i as f32 / 25.0) - 0.5, // -0.5 to 0.5
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

    #[test]
    fn test_feature_scaler() {
        // Create test data
        let data = Array2::from_shape_vec((3, 2), vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();

        // Fit scaler
        let scaler = FeatureScaler::fit(&data);

        // Check means and stds
        assert!((scaler.means[0] - 3.0).abs() < 0.001); // Mean of [1, 3, 5] = 3
        assert!((scaler.means[1] - 4.0).abs() < 0.001); // Mean of [2, 4, 6] = 4

        // Transform data
        let transformed = scaler.transform(&data);

        // Check that means are approximately zero after transformation
        let new_means = transformed.mean_axis(ndarray::Axis(0)).unwrap();
        assert!(new_means[0].abs() < 0.001);
        assert!(new_means[1].abs() < 0.001);
    }

    #[test]
    fn test_prepare_training_data() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);
        let training_data = create_test_training_data();

        let result = service.prepare_training_data(&training_data);
        assert!(result.is_ok());

        let (features, targets) = result.unwrap();

        // Check dimensions
        assert_eq!(features.nrows(), training_data.len());
        assert_eq!(features.ncols(), TrainingFeatures::feature_names().len());
        assert_eq!(targets.len(), training_data.len());

        // Check first row corresponds to first training point
        let first_features = training_data[0].features.to_ndarray();
        for (i, &expected) in first_features.iter().enumerate() {
            assert!((features[(0, i)] - expected).abs() < 0.001);
        }

        // Check first target
        assert!((targets[0] - training_data[0].actual_tss as f64).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_train_linear_regression_model_insufficient_data() {
        let db = setup_test_db().await;
        let mut service = create_test_service(db);
        let user_id = Uuid::new_v4();

        // Create insufficient training data (< 10 samples)
        let training_data = create_test_training_data()[0..5].to_vec();

        let result = service.train_linear_regression_model(user_id, &training_data).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Insufficient training data"));
    }

    #[tokio::test]
    async fn test_train_random_forest_model_insufficient_data() {
        let db = setup_test_db().await;
        let mut service = create_test_service(db);
        let user_id = Uuid::new_v4();

        // Create insufficient training data for Random Forest (< 20 samples)
        let training_data = create_test_training_data()[0..15].to_vec();

        let result = service.train_random_forest_model(user_id, &training_data).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Insufficient training data for Random Forest"));
    }

    #[test]
    fn test_discretize_undiscretize_tss() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);

        // Test discretization
        assert_eq!(service.discretize_tss(25.0), 0);   // Recovery
        assert_eq!(service.discretize_tss(75.0), 1);   // Easy
        assert_eq!(service.discretize_tss(150.0), 2);  // Moderate
        assert_eq!(service.discretize_tss(250.0), 3);  // Hard
        assert_eq!(service.discretize_tss(350.0), 4);  // Very Hard
        assert_eq!(service.discretize_tss(500.0), 5);  // Extreme

        // Test undiscretization
        assert_eq!(service.undiscretize_tss(0), 25.0);   // Recovery
        assert_eq!(service.undiscretize_tss(1), 75.0);   // Easy
        assert_eq!(service.undiscretize_tss(2), 150.0);  // Moderate
        assert_eq!(service.undiscretize_tss(3), 250.0);  // Hard
        assert_eq!(service.undiscretize_tss(4), 350.0);  // Very Hard
        assert_eq!(service.undiscretize_tss(5), 450.0);  // Extreme
    }

    #[test]
    fn test_calculate_metrics() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);

        let predictions = Array1::from(vec![100.0, 150.0, 200.0]);
        let targets = Array1::from(vec![110.0, 140.0, 190.0]);

        // Test MAE
        let mae = service.calculate_mae(&predictions, &targets);
        let expected_mae = (10.0 + 10.0 + 10.0) / 3.0;
        assert!((mae - expected_mae).abs() < 0.001);

        // Test RMSE
        let rmse = service.calculate_rmse(&predictions, &targets);
        let expected_rmse = ((100.0 + 100.0 + 100.0) / 3.0).sqrt();
        assert!((rmse - expected_rmse).abs() < 0.001);

        // Test R-squared
        let r_squared = service.calculate_r_squared(&predictions, &targets);
        assert!(r_squared >= 0.0);
        assert!(r_squared <= 1.0);
    }

    #[test]
    fn test_calculate_confidence() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);

        // Test linear confidence
        let reasonable_prediction = 200.0;
        let confidence = service.calculate_linear_confidence(reasonable_prediction);
        assert_eq!(confidence, 0.8); // High confidence for reasonable prediction

        let extreme_prediction = 1000.0;
        let confidence = service.calculate_linear_confidence(extreme_prediction);
        assert_eq!(confidence, 0.5); // Lower confidence for extreme prediction
    }

    #[test]
    fn test_recommend_workout_type() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);

        // Test high fatigue (negative TSB)
        let high_fatigue_features = TrainingFeatures {
            current_tsb: -25.0,
            ..TrainingFeatures::default()
        };
        let workout_type = service.recommend_workout_type(&high_fatigue_features, 100.0);
        assert_eq!(workout_type, "recovery");

        // Test normal state with low TSS
        let normal_features = TrainingFeatures {
            current_tsb: 0.0,
            ..TrainingFeatures::default()
        };
        let workout_type = service.recommend_workout_type(&normal_features, 80.0);
        assert_eq!(workout_type, "endurance");

        // Test normal state with high TSS
        let workout_type = service.recommend_workout_type(&normal_features, 250.0);
        assert_eq!(workout_type, "vo2max");
    }

    #[test]
    fn test_model_ready_state() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);

        // Initially no model should be ready
        assert!(!service.is_model_ready());
        assert!(service.get_current_model_info().is_none());
    }

    #[tokio::test]
    async fn test_predict_tss_no_model() {
        let db = setup_test_db().await;
        let service = create_test_service(db);
        let features = TrainingFeatures::default();

        let result = service.predict_tss(&features).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No trained model available"));
    }

    #[test]
    fn test_training_features_methods() {
        // Test TrainingFeatures creation and conversion methods
        let features = TrainingFeatures {
            current_ctl: 100.0,
            current_atl: 60.0,
            current_tsb: 40.0,
            days_since_last_workout: 2,
            avg_weekly_tss_4weeks: 350.0,
            recent_performance_trend: 0.2,
            days_until_goal_event: Some(45),
            preferred_workout_types: vec!["endurance".to_string(), "vo2max".to_string()],
            seasonal_factors: 0.9,
        };

        // Test conversion to ndarray
        let array = features.to_ndarray();
        assert_eq!(array.len(), 13); // 8 numeric + 5 workout type features

        // Verify numeric features
        assert_eq!(array[0], 100.0);  // current_ctl
        assert_eq!(array[1], 60.0);   // current_atl
        assert_eq!(array[2], 40.0);   // current_tsb
        assert_eq!(array[3], 2.0);    // days_since_last_workout
        assert_eq!(array[4], 350.0);  // avg_weekly_tss_4weeks
        assert_eq!(array[5], 0.2);    // recent_performance_trend
        assert_eq!(array[6], 45.0);   // days_until_goal_event
        assert_eq!(array[7], 0.9);    // seasonal_factors

        // Verify one-hot encoding for workout types
        assert_eq!(array[8], 1.0);    // prefers_endurance
        assert_eq!(array[9], 0.0);    // prefers_threshold
        assert_eq!(array[10], 1.0);   // prefers_vo2max
        assert_eq!(array[11], 0.0);   // prefers_recovery
        assert_eq!(array[12], 0.0);   // prefers_strength
    }

    #[test]
    fn test_edge_cases() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);

        // Test empty training data
        let empty_data: Vec<TrainingDataPoint> = vec![];
        let result = service.prepare_training_data(&empty_data);
        assert!(result.is_ok());
        let (features, targets) = result.unwrap();
        assert_eq!(features.nrows(), 0);
        assert_eq!(targets.len(), 0);

        // Test extreme TSS values
        assert_eq!(service.discretize_tss(-10.0), 0);  // Negative TSS
        assert_eq!(service.discretize_tss(f64::MAX), 5); // Very large TSS

        // Test features with None values
        let features_with_none = TrainingFeatures {
            days_until_goal_event: None,
            ..TrainingFeatures::default()
        };
        let array = features_with_none.to_ndarray();
        assert_eq!(array[6], -1.0); // None should be converted to -1
    }
}