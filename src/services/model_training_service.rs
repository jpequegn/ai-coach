use anyhow::{Result, anyhow};
use chrono::{Utc, NaiveDate, Duration};
use sqlx::PgPool;
use uuid::Uuid;
use tracing::{info, warn, error};

use crate::models::{TrainingDataPoint, ModelMetrics, TrainingFeatures};
use crate::services::{FeatureEngineeringService, MLModelService, ModelPredictionService};

/// Configuration for model training
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    /// Minimum number of training samples required
    pub min_training_samples: usize,
    /// Days of historical data to use for training
    pub training_window_days: i32,
    /// Validation split ratio (0.0 to 1.0)
    pub validation_split: f64,
    /// Whether to retrain existing models
    pub force_retrain: bool,
    /// Target model accuracy (RMSE threshold)
    pub target_rmse_threshold: f32,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            min_training_samples: 20,
            training_window_days: 365, // 1 year of data
            validation_split: 0.2,
            force_retrain: false,
            target_rmse_threshold: 50.0, // 50 TSS RMSE threshold
        }
    }
}

/// Model training pipeline service
#[derive(Clone)]
pub struct ModelTrainingService {
    db: PgPool,
    feature_service: FeatureEngineeringService,
    prediction_service: ModelPredictionService,
}

impl ModelTrainingService {
    /// Create a new ModelTrainingService
    pub fn new(db: PgPool) -> Self {
        let feature_service = FeatureEngineeringService::new(db.clone());
        let prediction_service = ModelPredictionService::new(db.clone());

        Self {
            db,
            feature_service,
            prediction_service,
        }
    }

    /// Train models for a specific user using their historical data
    pub async fn train_user_models(
        &self,
        user_id: Uuid,
        config: Option<TrainingConfig>,
    ) -> Result<Vec<ModelMetrics>> {
        let config = config.unwrap_or_default();
        info!("Starting model training for user {}", user_id);

        // Extract historical training data
        let end_date = Utc::now().date_naive();
        let start_date = end_date - Duration::days(config.training_window_days as i64);

        let training_data = self.feature_service
            .extract_historical_data_points(user_id, start_date, end_date)
            .await?;

        if training_data.len() < config.min_training_samples {
            return Err(anyhow!(
                "Insufficient training data for user {}: {} samples (need {})",
                user_id,
                training_data.len(),
                config.min_training_samples
            ));
        }

        info!("Extracted {} training samples for user {}", training_data.len(), user_id);

        // Train multiple models and compare performance
        let mut model_metrics = Vec::new();
        let mut ml_service = MLModelService::new(self.db.clone());

        // Train Linear Regression model
        match ml_service.train_linear_regression_model(user_id, &training_data).await {
            Ok(metrics) => {
                info!("Linear regression model trained. RMSE: {:.2}, MAE: {:.2}, R²: {:.3}",
                    metrics.rmse_tss, metrics.mae_tss, metrics.r_squared);

                // Store model prediction record
                if let Err(e) = self.store_model_metrics(user_id, &metrics).await {
                    warn!("Failed to store linear regression metrics: {}", e);
                }

                model_metrics.push(metrics);
            }
            Err(e) => {
                error!("Failed to train linear regression model for user {}: {}", user_id, e);
            }
        }

        // Train Random Forest model if enough data
        if training_data.len() >= 50 {
            match ml_service.train_random_forest_model(user_id, &training_data).await {
                Ok(metrics) => {
                    info!("Random Forest model trained. RMSE: {:.2}, MAE: {:.2}, R²: {:.3}",
                        metrics.rmse_tss, metrics.mae_tss, metrics.r_squared);

                    // Store model prediction record
                    if let Err(e) = self.store_model_metrics(user_id, &metrics).await {
                        warn!("Failed to store Random Forest metrics: {}", e);
                    }

                    model_metrics.push(metrics);
                }
                Err(e) => {
                    error!("Failed to train Random Forest model for user {}: {}", user_id, e);
                }
            }
        } else {
            warn!("Skipping Random Forest training for user {} - insufficient data ({} samples, need 50+)",
                user_id, training_data.len());
        }

        if model_metrics.is_empty() {
            return Err(anyhow!("No models were successfully trained for user {}", user_id));
        }

        // Select the best performing model
        let best_model = model_metrics.iter()
            .min_by(|a, b| a.rmse_tss.partial_cmp(&b.rmse_tss).unwrap())
            .unwrap();

        info!("Best model for user {}: {} (RMSE: {:.2})",
            user_id, best_model.model_version, best_model.rmse_tss);

        Ok(model_metrics)
    }

    /// Batch train models for multiple users
    pub async fn batch_train_models(
        &self,
        user_ids: &[Uuid],
        config: Option<TrainingConfig>,
    ) -> Result<Vec<(Uuid, Result<Vec<ModelMetrics>>)>> {
        let mut results = Vec::new();

        for &user_id in user_ids {
            let result = self.train_user_models(user_id, config.clone()).await;
            results.push((user_id, result));
        }

        Ok(results)
    }

    /// Validate model performance using cross-validation
    pub async fn cross_validate_model(
        &self,
        user_id: Uuid,
        k_folds: usize,
        config: Option<TrainingConfig>,
    ) -> Result<Vec<ModelMetrics>> {
        let config = config.unwrap_or_default();

        // Extract historical data
        let end_date = Utc::now().date_naive();
        let start_date = end_date - Duration::days(config.training_window_days as i64);
        let training_data = self.feature_service
            .extract_historical_data_points(user_id, start_date, end_date)
            .await?;

        if training_data.len() < config.min_training_samples {
            return Err(anyhow!("Insufficient data for cross-validation"));
        }

        let fold_size = training_data.len() / k_folds;
        let mut cv_results = Vec::new();

        for fold in 0..k_folds {
            let start_idx = fold * fold_size;
            let end_idx = if fold == k_folds - 1 { training_data.len() } else { (fold + 1) * fold_size };

            // Split data into train and validation
            let mut train_data = Vec::new();
            let mut val_data = Vec::new();

            for (i, data_point) in training_data.iter().enumerate() {
                if i >= start_idx && i < end_idx {
                    val_data.push(data_point.clone());
                } else {
                    train_data.push(data_point.clone());
                }
            }

            // Train model on fold
            let mut ml_service = MLModelService::new(self.db.clone());
            if let Ok(metrics) = ml_service.train_linear_regression_model(user_id, &train_data).await {
                cv_results.push(metrics);
            }
        }

        Ok(cv_results)
    }

    /// Get data quality assessment for training
    pub async fn assess_data_quality(&self, user_id: Uuid, days: i32) -> Result<DataQualityReport> {
        let end_date = Utc::now().date_naive();
        let start_date = end_date - Duration::days(days as i64);

        let training_data = self.feature_service
            .extract_historical_data_points(user_id, start_date, end_date)
            .await?;

        let total_samples = training_data.len();
        let mut valid_samples = 0;
        let mut missing_tss = 0;
        let mut zero_tss = 0;
        let mut extreme_tss = 0;

        for point in &training_data {
            if point.actual_tss > 0.0 && point.actual_tss < 1000.0 {
                valid_samples += 1;
            }
            if point.actual_tss == 0.0 {
                zero_tss += 1;
            }
            if point.actual_tss > 500.0 {
                extreme_tss += 1;
            }
        }

        let data_completeness = if total_samples > 0 {
            valid_samples as f32 / total_samples as f32
        } else {
            0.0
        };

        Ok(DataQualityReport {
            total_samples,
            valid_samples,
            data_completeness,
            missing_tss,
            zero_tss,
            extreme_tss,
            is_sufficient: total_samples >= 20 && data_completeness >= 0.7,
        })
    }

    /// Store model metrics in the database
    async fn store_model_metrics(&self, user_id: Uuid, metrics: &ModelMetrics) -> Result<()> {
        let prediction_data = serde_json::json!({
            "mae_tss": metrics.mae_tss,
            "rmse_tss": metrics.rmse_tss,
            "r_squared": metrics.r_squared,
            "sample_count": metrics.sample_count,
            "evaluated_at": metrics.evaluated_at
        });

        let prediction = crate::models::CreateModelPrediction {
            user_id,
            prediction_type: "ModelMetrics".to_string(),
            data: prediction_data,
            confidence: Some(metrics.r_squared),
            model_version: Some(metrics.model_version.clone()),
        };

        self.prediction_service.create_prediction(prediction).await?;
        Ok(())
    }

    /// Schedule periodic model retraining
    pub async fn schedule_retraining(&self, user_ids: &[Uuid], interval_days: i32) -> Result<()> {
        // This would integrate with the background job service
        // For now, it's a placeholder for future implementation
        info!("Scheduling model retraining for {} users every {} days", user_ids.len(), interval_days);
        Ok(())
    }

    /// Get training recommendations for improving model performance
    pub fn get_training_recommendations(&self, quality_report: &DataQualityReport) -> Vec<String> {
        let mut recommendations = Vec::new();

        if quality_report.total_samples < 50 {
            recommendations.push("Collect more training data - aim for at least 50 workout sessions".to_string());
        }

        if quality_report.data_completeness < 0.8 {
            recommendations.push("Improve data quality - ensure TSS values are recorded for all workouts".to_string());
        }

        if quality_report.zero_tss > quality_report.total_samples / 4 {
            recommendations.push("Review zero TSS sessions - many workouts have no training stress recorded".to_string());
        }

        if quality_report.extreme_tss > quality_report.total_samples / 10 {
            recommendations.push("Review high TSS sessions - some workouts may have unrealistic training stress values".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Data quality is good - models should train effectively".to_string());
        }

        recommendations
    }
}

/// Data quality assessment report
#[derive(Debug, Clone)]
pub struct DataQualityReport {
    pub total_samples: usize,
    pub valid_samples: usize,
    pub data_completeness: f32,
    pub missing_tss: usize,
    pub zero_tss: usize,
    pub extreme_tss: usize,
    pub is_sufficient: bool,
}