use anyhow::{Result, anyhow};
use chrono::{Utc, Duration};
use sqlx::PgPool;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tracing::{info, warn, error};

use crate::models::{ModelMetrics, TrainingFeatures, TrainingLoadPrediction};
use crate::services::{MLModelService, ModelPredictionService};

/// Model version metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersion {
    pub id: Uuid,
    pub version: String,
    pub model_type: String,
    pub status: ModelStatus,
    pub metrics: ModelMetrics,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub deployed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub retired_at: Option<chrono::DateTime<chrono::Utc>>,
    pub description: String,
}

/// Model deployment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelStatus {
    Training,     // Model is being trained
    Validation,   // Model is being validated
    Staging,      // Model is in staging environment
    Production,   // Model is serving production traffic
    Champion,     // Model is the current best performer
    Challenger,   // Model is being A/B tested against champion
    Retired,      // Model is no longer in use
}

/// A/B test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestConfig {
    pub test_id: Uuid,
    pub name: String,
    pub champion_version: String,
    pub challenger_version: String,
    pub traffic_split: f32,        // 0.0 to 1.0, percentage to challenger
    pub start_date: chrono::DateTime<chrono::Utc>,
    pub end_date: chrono::DateTime<chrono::Utc>,
    pub status: ABTestStatus,
    pub target_metric: String,     // "rmse", "user_satisfaction", etc.
    pub min_sample_size: usize,
    pub significance_threshold: f32, // Statistical significance threshold
}

/// A/B test status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ABTestStatus {
    Planning,
    Running,
    Analyzing,
    Completed,
    Cancelled,
}

/// A/B test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestResults {
    pub test_id: Uuid,
    pub champion_performance: TestMetrics,
    pub challenger_performance: TestMetrics,
    pub statistical_significance: f32,
    pub winner: Option<String>, // version string of winning model
    pub recommendation: TestRecommendation,
    pub analyzed_at: chrono::DateTime<chrono::Utc>,
}

/// Performance metrics for A/B testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMetrics {
    pub version: String,
    pub sample_size: usize,
    pub avg_rmse: f32,
    pub avg_confidence: f32,
    pub user_satisfaction: Option<f32>, // Would come from user feedback
    pub prediction_latency_ms: f32,
    pub error_rate: f32,
}

/// A/B test recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestRecommendation {
    PromoteChallenger,
    KeepChampion,
    ExtendTest,
    RequireMoreData,
}

/// Model versioning and A/B testing service
#[derive(Clone)]
pub struct ModelVersioningService {
    db: PgPool,
    prediction_service: ModelPredictionService,
    // In production, this would be persisted storage
    model_registry: std::sync::Arc<tokio::sync::RwLock<HashMap<String, ModelVersion>>>,
    ab_tests: std::sync::Arc<tokio::sync::RwLock<HashMap<Uuid, ABTestConfig>>>,
}

impl ModelVersioningService {
    /// Create a new ModelVersioningService
    pub fn new(db: PgPool) -> Self {
        let prediction_service = ModelPredictionService::new(db.clone());

        Self {
            db,
            prediction_service,
            model_registry: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            ab_tests: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Register a new model version
    pub async fn register_model_version(
        &self,
        version: String,
        model_type: String,
        metrics: ModelMetrics,
        description: String,
    ) -> Result<ModelVersion> {
        let model_version = ModelVersion {
            id: Uuid::new_v4(),
            version: version.clone(),
            model_type,
            status: ModelStatus::Validation,
            metrics,
            created_at: Utc::now(),
            deployed_at: None,
            retired_at: None,
            description,
        };

        // Store in registry
        let mut registry = self.model_registry.write().await;
        registry.insert(version.clone(), model_version.clone());

        // Store in database for persistence
        self.store_model_version(&model_version).await?;

        info!("Registered new model version: {}", version);
        Ok(model_version)
    }

    /// Deploy a model version to production
    pub async fn deploy_model_version(&self, version: &str) -> Result<()> {
        let mut registry = self.model_registry.write().await;

        if let Some(model) = registry.get_mut(version) {
            // Retire current champion
            for (_, existing_model) in registry.iter_mut() {
                if existing_model.status == ModelStatus::Champion {
                    existing_model.status = ModelStatus::Production;
                }
            }

            // Promote this model to champion
            model.status = ModelStatus::Champion;
            model.deployed_at = Some(Utc::now());

            self.store_model_version(model).await?;
            info!("Deployed model version {} as champion", version);
            Ok(())
        } else {
            Err(anyhow!("Model version {} not found", version))
        }
    }

    /// Create a new A/B test
    pub async fn create_ab_test(
        &self,
        name: String,
        champion_version: String,
        challenger_version: String,
        traffic_split: f32,
        duration_days: i32,
        target_metric: String,
    ) -> Result<ABTestConfig> {
        // Validate versions exist
        let registry = self.model_registry.read().await;
        if !registry.contains_key(&champion_version) {
            return Err(anyhow!("Champion version {} not found", champion_version));
        }
        if !registry.contains_key(&challenger_version) {
            return Err(anyhow!("Challenger version {} not found", challenger_version));
        }
        drop(registry);

        let test_config = ABTestConfig {
            test_id: Uuid::new_v4(),
            name,
            champion_version: champion_version.clone(),
            challenger_version: challenger_version.clone(),
            traffic_split: traffic_split.clamp(0.0, 1.0),
            start_date: Utc::now(),
            end_date: Utc::now() + Duration::days(duration_days as i64),
            status: ABTestStatus::Planning,
            target_metric,
            min_sample_size: 100,
            significance_threshold: 0.05,
        };

        // Store test configuration
        let mut tests = self.ab_tests.write().await;
        tests.insert(test_config.test_id, test_config.clone());

        // Update model statuses
        let mut registry = self.model_registry.write().await;
        if let Some(champion) = registry.get_mut(&champion_version) {
            if champion.status != ModelStatus::Champion {
                champion.status = ModelStatus::Champion;
            }
        }
        if let Some(challenger) = registry.get_mut(&challenger_version) {
            challenger.status = ModelStatus::Challenger;
        }

        self.store_ab_test(&test_config).await?;
        info!("Created A/B test: {} vs {}", champion_version, challenger_version);
        Ok(test_config)
    }

    /// Start an A/B test
    pub async fn start_ab_test(&self, test_id: Uuid) -> Result<()> {
        let mut tests = self.ab_tests.write().await;
        if let Some(test) = tests.get_mut(&test_id) {
            test.status = ABTestStatus::Running;
            self.store_ab_test(test).await?;
            info!("Started A/B test: {}", test.name);
            Ok(())
        } else {
            Err(anyhow!("A/B test {} not found", test_id))
        }
    }

    /// Determine which model version to use for a prediction (A/B testing logic)
    pub async fn select_model_for_prediction(&self, user_id: Uuid) -> String {
        // Check if there's an active A/B test
        let tests = self.ab_tests.read().await;
        for test in tests.values() {
            if test.status == ABTestStatus::Running && Utc::now() < test.end_date {
                // Use deterministic hash-based assignment for consistent user experience
                let user_hash = self.hash_user_id(user_id);
                if user_hash < test.traffic_split {
                    return test.challenger_version.clone();
                } else {
                    return test.champion_version.clone();
                }
            }
        }

        // No active test, use champion model
        let registry = self.model_registry.read().await;
        for model in registry.values() {
            if model.status == ModelStatus::Champion {
                return model.version.clone();
            }
        }

        // Fallback to latest production model
        registry.values()
            .filter(|m| m.status == ModelStatus::Production)
            .max_by_key(|m| m.created_at)
            .map(|m| m.version.clone())
            .unwrap_or_else(|| "fallback_v1".to_string())
    }

    /// Analyze A/B test results
    pub async fn analyze_ab_test(&self, test_id: Uuid) -> Result<ABTestResults> {
        let tests = self.ab_tests.read().await;
        let test = tests.get(&test_id)
            .ok_or_else(|| anyhow!("A/B test {} not found", test_id))?;

        // Collect performance metrics for both models
        let champion_metrics = self.collect_model_metrics(&test.champion_version, test.start_date, Utc::now()).await?;
        let challenger_metrics = self.collect_model_metrics(&test.challenger_version, test.start_date, Utc::now()).await?;

        // Calculate statistical significance (simplified t-test)
        let significance = self.calculate_statistical_significance(&champion_metrics, &challenger_metrics);

        // Determine winner based on target metric
        let winner = self.determine_winner(&champion_metrics, &challenger_metrics, &test.target_metric);

        // Generate recommendation
        let recommendation = self.generate_test_recommendation(
            &champion_metrics,
            &challenger_metrics,
            significance,
            test.significance_threshold,
            test.min_sample_size,
        );

        Ok(ABTestResults {
            test_id,
            champion_performance: champion_metrics,
            challenger_performance: challenger_metrics,
            statistical_significance: significance,
            winner,
            recommendation,
            analyzed_at: Utc::now(),
        })
    }

    /// Complete an A/B test and optionally promote the winner
    pub async fn complete_ab_test(&self, test_id: Uuid, promote_winner: bool) -> Result<ABTestResults> {
        let results = self.analyze_ab_test(test_id).await?;

        // Update test status
        let mut tests = self.ab_tests.write().await;
        if let Some(test) = tests.get_mut(&test_id) {
            test.status = ABTestStatus::Completed;
        }

        // Promote winner if requested and recommendation supports it
        if promote_winner && matches!(results.recommendation, TestRecommendation::PromoteChallenger) {
            if let Some(winner) = &results.winner {
                self.deploy_model_version(winner).await?;
                info!("Promoted A/B test winner: {}", winner);
            }
        }

        // Update challenger status
        let mut registry = self.model_registry.write().await;
        for model in registry.values_mut() {
            if model.status == ModelStatus::Challenger {
                model.status = if promote_winner && results.winner.as_ref() == Some(&model.version) {
                    ModelStatus::Champion
                } else {
                    ModelStatus::Production
                };
            }
        }

        Ok(results)
    }

    /// Get current champion model version
    pub async fn get_champion_version(&self) -> Option<String> {
        let registry = self.model_registry.read().await;
        registry.values()
            .find(|m| m.status == ModelStatus::Champion)
            .map(|m| m.version.clone())
    }

    /// List all model versions
    pub async fn list_model_versions(&self) -> Vec<ModelVersion> {
        let registry = self.model_registry.read().await;
        registry.values().cloned().collect()
    }

    /// List active A/B tests
    pub async fn list_active_ab_tests(&self) -> Vec<ABTestConfig> {
        let tests = self.ab_tests.read().await;
        tests.values()
            .filter(|t| matches!(t.status, ABTestStatus::Running | ABTestStatus::Planning))
            .cloned()
            .collect()
    }

    /// Hash user ID for consistent A/B test assignment
    fn hash_user_id(&self, user_id: Uuid) -> f32 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        user_id.hash(&mut hasher);
        let hash = hasher.finish();

        // Convert to 0.0-1.0 range
        (hash % 10000) as f32 / 10000.0
    }

    /// Collect performance metrics for a model version
    async fn collect_model_metrics(
        &self,
        version: &str,
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<TestMetrics> {
        // This would query actual prediction logs from the database
        // For now, returning mock data based on model version
        let base_rmse = if version.contains("linear") { 45.0 } else { 40.0 };
        let base_confidence = if version.contains("forest") { 0.8 } else { 0.75 };

        Ok(TestMetrics {
            version: version.to_string(),
            sample_size: 250, // Would be actual count from logs
            avg_rmse: base_rmse + (rand::random::<f32>() - 0.5) * 10.0,
            avg_confidence: base_confidence + (rand::random::<f32>() - 0.5) * 0.1,
            user_satisfaction: Some(7.5 + (rand::random::<f32>() - 0.5) * 2.0),
            prediction_latency_ms: 50.0 + rand::random::<f32>() * 20.0,
            error_rate: rand::random::<f32>() * 0.05,
        })
    }

    /// Calculate statistical significance between two sets of metrics
    fn calculate_statistical_significance(&self, champion: &TestMetrics, challenger: &TestMetrics) -> f32 {
        // Simplified t-test calculation
        // In production, this would use proper statistical libraries
        let rmse_diff = (champion.avg_rmse - challenger.avg_rmse).abs();
        let pooled_variance = ((champion.sample_size + challenger.sample_size) as f32) / 10.0;

        if pooled_variance == 0.0 {
            return 1.0; // No significance
        }

        let t_stat = rmse_diff / pooled_variance.sqrt();

        // Convert t-statistic to p-value (very simplified)
        (1.0 - (t_stat / 5.0).min(1.0)).max(0.0)
    }

    /// Determine winner based on target metric
    fn determine_winner(&self, champion: &TestMetrics, challenger: &TestMetrics, target_metric: &str) -> Option<String> {
        match target_metric {
            "rmse" => {
                if challenger.avg_rmse < champion.avg_rmse {
                    Some(challenger.version.clone())
                } else {
                    Some(champion.version.clone())
                }
            }
            "confidence" => {
                if challenger.avg_confidence > champion.avg_confidence {
                    Some(challenger.version.clone())
                } else {
                    Some(champion.version.clone())
                }
            }
            "user_satisfaction" => {
                let challenger_satisfaction = challenger.user_satisfaction.unwrap_or(0.0);
                let champion_satisfaction = champion.user_satisfaction.unwrap_or(0.0);
                if challenger_satisfaction > champion_satisfaction {
                    Some(challenger.version.clone())
                } else {
                    Some(champion.version.clone())
                }
            }
            _ => None,
        }
    }

    /// Generate test recommendation
    fn generate_test_recommendation(
        &self,
        champion: &TestMetrics,
        challenger: &TestMetrics,
        significance: f32,
        threshold: f32,
        min_samples: usize,
    ) -> TestRecommendation {
        if champion.sample_size < min_samples || challenger.sample_size < min_samples {
            return TestRecommendation::RequireMoreData;
        }

        if significance > threshold {
            return TestRecommendation::ExtendTest;
        }

        // Check if challenger is meaningfully better
        let rmse_improvement = (champion.avg_rmse - challenger.avg_rmse) / champion.avg_rmse;
        if rmse_improvement > 0.05 { // 5% improvement
            TestRecommendation::PromoteChallenger
        } else {
            TestRecommendation::KeepChampion
        }
    }

    /// Store model version in database
    async fn store_model_version(&self, model: &ModelVersion) -> Result<()> {
        let model_data = serde_json::to_value(model)?;

        let prediction = crate::models::CreateModelPrediction {
            user_id: Uuid::new_v4(), // System user ID
            prediction_type: "ModelVersion".to_string(),
            data: model_data,
            confidence: Some(model.metrics.r_squared),
            model_version: Some(model.version.clone()),
        };

        self.prediction_service.create_prediction(prediction).await?;
        Ok(())
    }

    /// Store A/B test configuration in database
    async fn store_ab_test(&self, test: &ABTestConfig) -> Result<()> {
        let test_data = serde_json::to_value(test)?;

        let prediction = crate::models::CreateModelPrediction {
            user_id: Uuid::new_v4(), // System user ID
            prediction_type: "ABTest".to_string(),
            data: test_data,
            confidence: Some(test.traffic_split),
            model_version: Some(format!("{}_{}", test.champion_version, test.challenger_version)),
        };

        self.prediction_service.create_prediction(prediction).await?;
        Ok(())
    }
}