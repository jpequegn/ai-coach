use anyhow::{Result, anyhow};
use chrono::{Utc, Duration};
use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashMap;
use tracing::{info, warn, error};
use serde::{Serialize, Deserialize};

use crate::models::{TrainingFeatures, TrainingLoadPrediction, ModelPrediction, CreateModelPrediction};
use crate::services::{FeatureEngineeringService, MLModelService, ModelPredictionService};

/// Recommendation request with user preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationRequest {
    pub user_id: Uuid,
    pub target_date: Option<chrono::NaiveDate>,
    pub preferred_workout_type: Option<String>,
    pub max_duration_minutes: Option<i32>,
    pub user_feedback: Option<UserFeedback>,
}

/// User feedback for improving recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    pub perceived_difficulty: i32, // 1-10 scale
    pub energy_level: i32,         // 1-10 scale
    pub motivation: i32,           // 1-10 scale
    pub available_time_minutes: Option<i32>,
    pub preferred_intensity: Option<String>, // "easy", "moderate", "hard"
}

/// Complete training recommendation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRecommendation {
    pub user_id: Uuid,
    pub prediction: TrainingLoadPrediction,
    pub alternative_options: Vec<TrainingLoadPrediction>,
    pub reasoning: String,
    pub warnings: Vec<String>,
    pub cached: bool,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Edge case handling configuration
#[derive(Debug, Clone)]
pub struct EdgeCaseConfig {
    pub min_data_points: usize,
    pub new_user_threshold_days: i32,
    pub fallback_tss_easy: f32,
    pub fallback_tss_moderate: f32,
    pub fallback_tss_hard: f32,
    pub max_tsb_for_hard_workout: f32,
    pub min_tsb_for_recovery: f32,
}

impl Default for EdgeCaseConfig {
    fn default() -> Self {
        Self {
            min_data_points: 5,
            new_user_threshold_days: 14,
            fallback_tss_easy: 75.0,
            fallback_tss_moderate: 150.0,
            fallback_tss_hard: 250.0,
            max_tsb_for_hard_workout: -15.0,
            min_tsb_for_recovery: -25.0,
        }
    }
}

/// Real-time training recommendation inference service
#[derive(Clone)]
pub struct TrainingRecommendationService {
    db: PgPool,
    feature_service: FeatureEngineeringService,
    prediction_service: ModelPredictionService,
    edge_case_config: EdgeCaseConfig,
    // In production, this would be a proper cache like Redis
    cache: std::sync::Arc<tokio::sync::RwLock<HashMap<String, (TrainingRecommendation, chrono::DateTime<chrono::Utc>)>>>,
}

impl TrainingRecommendationService {
    /// Create a new TrainingRecommendationService
    pub fn new(db: PgPool) -> Self {
        let feature_service = FeatureEngineeringService::new(db.clone());
        let prediction_service = ModelPredictionService::new(db.clone());

        Self {
            db,
            feature_service,
            prediction_service,
            edge_case_config: EdgeCaseConfig::default(),
            cache: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Get training recommendation for a user
    pub async fn get_recommendation(&self, request: RecommendationRequest) -> Result<TrainingRecommendation> {
        let cache_key = format!("rec_{}_{}", request.user_id,
            request.target_date.unwrap_or_else(|| Utc::now().date_naive()));

        // Check cache first
        if let Some(cached) = self.get_from_cache(&cache_key).await {
            info!("Returning cached recommendation for user {}", request.user_id);
            return Ok(cached);
        }

        // Extract current features
        let features = self.feature_service.extract_current_features(request.user_id).await?;

        // Check for edge cases and handle them
        let recommendation = match self.detect_edge_case(&features, request.user_id).await? {
            Some(edge_case_type) => {
                info!("Detected edge case '{}' for user {}", edge_case_type, request.user_id);
                self.handle_edge_case(edge_case_type, &features, &request).await?
            }
            None => {
                // Normal prediction flow
                self.generate_ml_recommendation(&features, &request).await?
            }
        };

        // Cache the recommendation
        self.cache_recommendation(&cache_key, &recommendation).await;

        // Store prediction in database for analytics
        if let Err(e) = self.store_recommendation_analytics(&recommendation).await {
            warn!("Failed to store recommendation analytics: {}", e);
        }

        Ok(recommendation)
    }

    /// Generate ML-based recommendation
    async fn generate_ml_recommendation(
        &self,
        features: &TrainingFeatures,
        request: &RecommendationRequest,
    ) -> Result<TrainingRecommendation> {
        let mut ml_service = MLModelService::new(self.db.clone());

        // Get base prediction
        let base_prediction = ml_service.predict_tss(features).await?;

        // Apply user preferences and constraints
        let adjusted_prediction = self.apply_user_preferences(&base_prediction, request)?;

        // Generate alternative options
        let alternatives = self.generate_alternatives(&adjusted_prediction, features).await?;

        // Generate reasoning
        let reasoning = self.generate_reasoning(features, &adjusted_prediction);

        // Check for warnings
        let warnings = self.generate_warnings(features, &adjusted_prediction);

        Ok(TrainingRecommendation {
            user_id: request.user_id,
            prediction: adjusted_prediction,
            alternative_options: alternatives,
            reasoning,
            warnings,
            cached: false,
            generated_at: Utc::now(),
        })
    }

    /// Detect edge cases that require special handling
    async fn detect_edge_case(&self, features: &TrainingFeatures, user_id: Uuid) -> Result<Option<String>> {
        // Check if user is new (insufficient data)
        if features.avg_weekly_tss_4weeks == 0.0 || features.days_since_last_workout > self.edge_case_config.new_user_threshold_days {
            return Ok(Some("new_user".to_string()));
        }

        // Check for extremely fatigued state
        if features.current_tsb < self.edge_case_config.min_tsb_for_recovery {
            return Ok(Some("overtrained".to_string()));
        }

        // Check for insufficient recent data
        let quality_report = self.feature_service
            .get_training_load_stats(user_id, 30)
            .await?;

        if quality_report.session_count < self.edge_case_config.min_data_points {
            return Ok(Some("insufficient_data".to_string()));
        }

        // Check for extreme TSB values (very high or very low)
        if features.current_tsb > 50.0 {
            return Ok(Some("detraining".to_string()));
        }

        Ok(None)
    }

    /// Handle edge cases with fallback recommendations
    async fn handle_edge_case(
        &self,
        edge_case_type: String,
        features: &TrainingFeatures,
        request: &RecommendationRequest,
    ) -> Result<TrainingRecommendation> {
        let (recommended_tss, workout_type, reasoning) = match edge_case_type.as_str() {
            "new_user" => {
                (self.edge_case_config.fallback_tss_easy,
                 "endurance".to_string(),
                 "New user detected - starting with conservative endurance workout".to_string())
            }
            "overtrained" => {
                (self.edge_case_config.fallback_tss_easy * 0.5,
                 "recovery".to_string(),
                 "High fatigue detected (TSB < -25) - recommending recovery workout".to_string())
            }
            "insufficient_data" => {
                let base_tss = features.avg_weekly_tss_4weeks.max(self.edge_case_config.fallback_tss_easy);
                (base_tss,
                 "endurance".to_string(),
                 "Limited recent data - using conservative recommendation".to_string())
            }
            "detraining" => {
                (self.edge_case_config.fallback_tss_moderate,
                 "endurance".to_string(),
                 "Extended break detected - gradual return to training recommended".to_string())
            }
            _ => {
                (self.edge_case_config.fallback_tss_moderate,
                 "endurance".to_string(),
                 "Default fallback recommendation".to_string())
            }
        };

        let prediction = TrainingLoadPrediction {
            recommended_tss,
            confidence: 0.6, // Lower confidence for edge cases
            confidence_lower: recommended_tss * 0.8,
            confidence_upper: recommended_tss * 1.2,
            model_version: "edge_case_handler_v1".to_string(),
            recommended_workout_type: workout_type,
            predicted_at: Utc::now(),
        };

        let alternatives = self.generate_alternatives(&prediction, features).await?;
        let warnings = vec![format!("Edge case detected: {}", edge_case_type)];

        Ok(TrainingRecommendation {
            user_id: request.user_id,
            prediction,
            alternative_options: alternatives,
            reasoning,
            warnings,
            cached: false,
            generated_at: Utc::now(),
        })
    }

    /// Apply user preferences to adjust predictions
    fn apply_user_preferences(
        &self,
        base_prediction: &TrainingLoadPrediction,
        request: &RecommendationRequest,
    ) -> Result<TrainingLoadPrediction> {
        let mut adjusted = base_prediction.clone();

        // Apply workout type preference
        if let Some(preferred_type) = &request.preferred_workout_type {
            adjusted.recommended_workout_type = preferred_type.clone();
        }

        // Apply user feedback adjustments
        if let Some(feedback) = &request.user_feedback {
            // Adjust TSS based on energy level and motivation
            let energy_factor = (feedback.energy_level as f32 - 5.0) / 10.0; // -0.5 to 0.5
            let motivation_factor = (feedback.motivation as f32 - 5.0) / 10.0;
            let combined_factor = (energy_factor + motivation_factor) / 2.0;

            adjusted.recommended_tss *= 1.0 + combined_factor * 0.3; // ±30% adjustment

            // Adjust workout type based on preferred intensity
            if let Some(intensity) = &feedback.preferred_intensity {
                adjusted.recommended_workout_type = match intensity.as_str() {
                    "easy" => "recovery".to_string(),
                    "moderate" => "endurance".to_string(),
                    "hard" => "threshold".to_string(),
                    _ => adjusted.recommended_workout_type,
                };
            }
        }

        // Apply duration constraints
        if let Some(max_duration) = request.max_duration_minutes {
            let max_tss_for_duration = (max_duration as f32 / 60.0) * 100.0; // Rough estimate
            if adjusted.recommended_tss > max_tss_for_duration {
                adjusted.recommended_tss = max_tss_for_duration;
                adjusted.recommended_workout_type = "endurance".to_string(); // Lower intensity for shorter duration
            }
        }

        // Ensure reasonable bounds
        adjusted.recommended_tss = adjusted.recommended_tss.clamp(10.0, 500.0);

        Ok(adjusted)
    }

    /// Generate alternative workout options
    async fn generate_alternatives(
        &self,
        primary: &TrainingLoadPrediction,
        features: &TrainingFeatures,
    ) -> Result<Vec<TrainingLoadPrediction>> {
        let mut alternatives = Vec::new();

        // Easy alternative (75% of primary TSS)
        let easy_tss = primary.recommended_tss * 0.75;
        alternatives.push(TrainingLoadPrediction {
            recommended_tss: easy_tss,
            confidence: primary.confidence * 0.9,
            confidence_lower: easy_tss * 0.9,
            confidence_upper: easy_tss * 1.1,
            model_version: primary.model_version.clone(),
            recommended_workout_type: "endurance".to_string(),
            predicted_at: primary.predicted_at,
        });

        // Hard alternative (125% of primary TSS, if TSB allows)
        if features.current_tsb > self.edge_case_config.max_tsb_for_hard_workout {
            let hard_tss = primary.recommended_tss * 1.25;
            alternatives.push(TrainingLoadPrediction {
                recommended_tss: hard_tss,
                confidence: primary.confidence * 0.8,
                confidence_lower: hard_tss * 0.85,
                confidence_upper: hard_tss * 1.15,
                model_version: primary.model_version.clone(),
                recommended_workout_type: "threshold".to_string(),
                predicted_at: primary.predicted_at,
            });
        }

        Ok(alternatives)
    }

    /// Generate human-readable reasoning for the recommendation
    fn generate_reasoning(&self, features: &TrainingFeatures, _prediction: &TrainingLoadPrediction) -> String {
        let mut reasoning = Vec::new();

        // TSB-based reasoning
        if features.current_tsb < -15.0 {
            reasoning.push("High fatigue levels suggest a recovery or easy workout".to_string());
        } else if features.current_tsb > 10.0 {
            reasoning.push("Low fatigue levels allow for more intensive training".to_string());
        } else {
            reasoning.push("Balanced training stress suggests moderate intensity training".to_string());
        }

        // Recent training pattern
        if features.days_since_last_workout >= 3 {
            reasoning.push(format!("It's been {} days since your last workout", features.days_since_last_workout));
        }

        // CTL progression
        if features.current_ctl > features.avg_weekly_tss_4weeks * 7.0 / 4.0 {
            reasoning.push("Fitness levels are trending upward".to_string());
        }

        reasoning.join(". ")
    }

    /// Generate warnings based on current state
    fn generate_warnings(&self, features: &TrainingFeatures, prediction: &TrainingLoadPrediction) -> Vec<String> {
        let mut warnings = Vec::new();

        if features.current_tsb < -20.0 {
            warnings.push("Consider taking a rest day - high fatigue detected".to_string());
        }

        if features.days_since_last_workout >= 7 {
            warnings.push("Long break from training - start gradually".to_string());
        }

        if prediction.recommended_tss > features.avg_weekly_tss_4weeks * 1.5 {
            warnings.push("Recommended TSS is significantly higher than recent average".to_string());
        }

        if prediction.confidence < 0.6 {
            warnings.push("Low confidence prediction - consider user feedback".to_string());
        }

        warnings
    }

    /// Cache recommendation with TTL
    async fn cache_recommendation(&self, key: &str, recommendation: &TrainingRecommendation) {
        let mut cache = self.cache.write().await;
        let expiry = Utc::now() + Duration::hours(1); // 1 hour TTL
        cache.insert(key.to_string(), (recommendation.clone(), expiry));
    }

    /// Get recommendation from cache if not expired
    async fn get_from_cache(&self, key: &str) -> Option<TrainingRecommendation> {
        let cache = self.cache.read().await;
        if let Some((recommendation, expiry)) = cache.get(key) {
            if *expiry > Utc::now() {
                let mut cached_rec = recommendation.clone();
                cached_rec.cached = true;
                return Some(cached_rec);
            }
        }
        None
    }

    /// Store recommendation for analytics
    async fn store_recommendation_analytics(&self, recommendation: &TrainingRecommendation) -> Result<()> {
        let analytics_data = serde_json::json!({
            "recommended_tss": recommendation.prediction.recommended_tss,
            "confidence": recommendation.prediction.confidence,
            "workout_type": recommendation.prediction.recommended_workout_type,
            "alternatives_count": recommendation.alternative_options.len(),
            "warnings_count": recommendation.warnings.len(),
            "cached": recommendation.cached,
            "model_version": recommendation.prediction.model_version
        });

        let prediction = CreateModelPrediction {
            user_id: recommendation.user_id,
            prediction_type: "TrainingRecommendation".to_string(),
            data: analytics_data,
            confidence: Some(recommendation.prediction.confidence),
            model_version: Some(recommendation.prediction.model_version.clone()),
        };

        self.prediction_service.create_prediction(prediction).await?;
        Ok(())
    }

    /// Clear expired cache entries
    pub async fn cleanup_cache(&self) {
        let mut cache = self.cache.write().await;
        let now = Utc::now();
        cache.retain(|_, (_, expiry)| *expiry > now);
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total = cache.len();
        let expired = cache.values().filter(|(_, expiry)| *expiry <= Utc::now()).count();
        (total, expired)
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

    fn create_test_service(db: PgPool) -> TrainingRecommendationService {
        TrainingRecommendationService::new(db)
    }

    fn create_test_user_id() -> Uuid {
        Uuid::new_v4()
    }

    fn create_test_features(tsb: f32, days_since_workout: i32, avg_tss: f32) -> TrainingFeatures {
        TrainingFeatures {
            current_ctl: 100.0,
            current_atl: 100.0 - tsb,
            current_tsb: tsb,
            days_since_last_workout: days_since_workout,
            avg_weekly_tss_4weeks: avg_tss,
            recent_performance_trend: 0.0,
            days_until_goal_event: None,
            preferred_workout_types: vec!["endurance".to_string()],
            seasonal_factors: 1.0,
        }
    }

    fn create_test_request(user_id: Uuid) -> RecommendationRequest {
        RecommendationRequest {
            user_id,
            target_date: None,
            preferred_workout_type: None,
            max_duration_minutes: None,
            user_feedback: None,
        }
    }

    fn create_test_request_with_feedback(user_id: Uuid, energy: i32, motivation: i32) -> RecommendationRequest {
        RecommendationRequest {
            user_id,
            target_date: None,
            preferred_workout_type: None,
            max_duration_minutes: None,
            user_feedback: Some(UserFeedback {
                perceived_difficulty: 5,
                energy_level: energy,
                motivation,
                available_time_minutes: None,
                preferred_intensity: None,
            }),
        }
    }

    #[tokio::test]
    async fn test_detect_edge_case_new_user() {
        let db = setup_test_db().await;
        let service = create_test_service(db);
        let user_id = create_test_user_id();

        // Create features indicating new user (no recent training data)
        let features = TrainingFeatures {
            avg_weekly_tss_4weeks: 0.0,
            days_since_last_workout: 20, // Long time since last workout
            ..TrainingFeatures::default()
        };

        let edge_case = service.detect_edge_case(&features, user_id).await;
        assert!(edge_case.is_ok());
        assert_eq!(edge_case.unwrap(), Some("new_user".to_string()));
    }

    #[tokio::test]
    async fn test_detect_edge_case_overtrained() {
        let db = setup_test_db().await;
        let service = create_test_service(db);
        let user_id = create_test_user_id();

        // Create features indicating overtraining (very negative TSB)
        let features = TrainingFeatures {
            current_tsb: -30.0, // Very negative TSB
            avg_weekly_tss_4weeks: 400.0, // Some training data
            days_since_last_workout: 1,
            ..TrainingFeatures::default()
        };

        let edge_case = service.detect_edge_case(&features, user_id).await;
        assert!(edge_case.is_ok());
        assert_eq!(edge_case.unwrap(), Some("overtrained".to_string()));
    }

    #[tokio::test]
    async fn test_detect_edge_case_detraining() {
        let db = setup_test_db().await;
        let service = create_test_service(db);
        let user_id = create_test_user_id();

        // Create features indicating detraining (very high TSB)
        let features = TrainingFeatures {
            current_tsb: 60.0, // Very high TSB indicates detraining
            avg_weekly_tss_4weeks: 200.0,
            days_since_last_workout: 2,
            ..TrainingFeatures::default()
        };

        let edge_case = service.detect_edge_case(&features, user_id).await;
        assert!(edge_case.is_ok());
        assert_eq!(edge_case.unwrap(), Some("detraining".to_string()));
    }

    #[tokio::test]
    async fn test_handle_edge_case_new_user() {
        let db = setup_test_db().await;
        let service = create_test_service(db);
        let user_id = create_test_user_id();
        let features = TrainingFeatures::default();
        let request = create_test_request(user_id);

        let recommendation = service
            .handle_edge_case("new_user".to_string(), &features, &request)
            .await;

        assert!(recommendation.is_ok());
        let rec = recommendation.unwrap();

        // Should recommend easy TSS and endurance workout
        assert_eq!(rec.prediction.recommended_tss, service.edge_case_config.fallback_tss_easy);
        assert_eq!(rec.prediction.recommended_workout_type, "endurance");
        assert_eq!(rec.prediction.confidence, 0.6); // Lower confidence for edge cases
        assert!(!rec.warnings.is_empty());
        assert!(rec.reasoning.contains("New user detected"));
    }

    #[tokio::test]
    async fn test_handle_edge_case_overtrained() {
        let db = setup_test_db().await;
        let service = create_test_service(db);
        let user_id = create_test_user_id();
        let features = TrainingFeatures::default();
        let request = create_test_request(user_id);

        let recommendation = service
            .handle_edge_case("overtrained".to_string(), &features, &request)
            .await;

        assert!(recommendation.is_ok());
        let rec = recommendation.unwrap();

        // Should recommend very low TSS and recovery workout
        assert_eq!(rec.prediction.recommended_tss, service.edge_case_config.fallback_tss_easy * 0.5);
        assert_eq!(rec.prediction.recommended_workout_type, "recovery");
        assert!(rec.reasoning.contains("High fatigue detected"));
    }

    #[test]
    fn test_apply_user_preferences_energy_motivation() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);

        let base_prediction = TrainingLoadPrediction {
            recommended_tss: 200.0,
            confidence: 0.8,
            confidence_lower: 180.0,
            confidence_upper: 220.0,
            model_version: "test_v1".to_string(),
            recommended_workout_type: "endurance".to_string(),
            predicted_at: Utc::now(),
        };

        // Test high energy and motivation (should increase TSS)
        let high_energy_request = create_test_request_with_feedback(create_test_user_id(), 9, 9);
        let adjusted = service.apply_user_preferences(&base_prediction, &high_energy_request);
        assert!(adjusted.is_ok());
        let high_energy_rec = adjusted.unwrap();
        assert!(high_energy_rec.recommended_tss > base_prediction.recommended_tss);

        // Test low energy and motivation (should decrease TSS)
        let low_energy_request = create_test_request_with_feedback(create_test_user_id(), 2, 2);
        let adjusted = service.apply_user_preferences(&base_prediction, &low_energy_request);
        assert!(adjusted.is_ok());
        let low_energy_rec = adjusted.unwrap();
        assert!(low_energy_rec.recommended_tss < base_prediction.recommended_tss);
    }

    #[test]
    fn test_apply_user_preferences_duration_constraint() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);

        let base_prediction = TrainingLoadPrediction {
            recommended_tss: 300.0, // High TSS
            confidence: 0.8,
            confidence_lower: 270.0,
            confidence_upper: 330.0,
            model_version: "test_v1".to_string(),
            recommended_workout_type: "threshold".to_string(),
            predicted_at: Utc::now(),
        };

        let mut request = create_test_request(create_test_user_id());
        request.max_duration_minutes = Some(30); // Short duration

        let adjusted = service.apply_user_preferences(&base_prediction, &request);
        assert!(adjusted.is_ok());
        let adjusted_rec = adjusted.unwrap();

        // Should reduce TSS and switch to endurance
        assert!(adjusted_rec.recommended_tss < base_prediction.recommended_tss);
        assert_eq!(adjusted_rec.recommended_workout_type, "endurance");
    }

    #[test]
    fn test_apply_user_preferences_workout_type() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);

        let base_prediction = TrainingLoadPrediction {
            recommended_tss: 200.0,
            confidence: 0.8,
            confidence_lower: 180.0,
            confidence_upper: 220.0,
            model_version: "test_v1".to_string(),
            recommended_workout_type: "endurance".to_string(),
            predicted_at: Utc::now(),
        };

        let mut request = create_test_request(create_test_user_id());
        request.preferred_workout_type = Some("threshold".to_string());

        let adjusted = service.apply_user_preferences(&base_prediction, &request);
        assert!(adjusted.is_ok());
        let adjusted_rec = adjusted.unwrap();

        // Should use preferred workout type
        assert_eq!(adjusted_rec.recommended_workout_type, "threshold");
    }

    #[tokio::test]
    async fn test_generate_alternatives() {
        let db = setup_test_db().await;
        let service = create_test_service(db);

        let primary_prediction = TrainingLoadPrediction {
            recommended_tss: 200.0,
            confidence: 0.8,
            confidence_lower: 180.0,
            confidence_upper: 220.0,
            model_version: "test_v1".to_string(),
            recommended_workout_type: "endurance".to_string(),
            predicted_at: Utc::now(),
        };

        // Test with good TSB (should allow hard alternative)
        let good_features = TrainingFeatures {
            current_tsb: 10.0, // Good TSB
            ..TrainingFeatures::default()
        };

        let alternatives = service.generate_alternatives(&primary_prediction, &good_features).await;
        assert!(alternatives.is_ok());
        let alts = alternatives.unwrap();

        // Should have easy and hard alternatives
        assert_eq!(alts.len(), 2);
        assert_eq!(alts[0].recommended_workout_type, "endurance"); // Easy alternative
        assert_eq!(alts[1].recommended_workout_type, "threshold"); // Hard alternative
        assert!(alts[0].recommended_tss < primary_prediction.recommended_tss); // Easy is lower TSS
        assert!(alts[1].recommended_tss > primary_prediction.recommended_tss); // Hard is higher TSS

        // Test with poor TSB (should not allow hard alternative)
        let poor_features = TrainingFeatures {
            current_tsb: -20.0, // Poor TSB
            ..TrainingFeatures::default()
        };

        let alternatives = service.generate_alternatives(&primary_prediction, &poor_features).await;
        assert!(alternatives.is_ok());
        let alts = alternatives.unwrap();

        // Should only have easy alternative
        assert_eq!(alts.len(), 1);
        assert_eq!(alts[0].recommended_workout_type, "endurance");
    }

    #[test]
    fn test_generate_reasoning() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);

        let prediction = TrainingLoadPrediction {
            recommended_tss: 200.0,
            confidence: 0.8,
            confidence_lower: 180.0,
            confidence_upper: 220.0,
            model_version: "test_v1".to_string(),
            recommended_workout_type: "endurance".to_string(),
            predicted_at: Utc::now(),
        };

        // Test high fatigue
        let high_fatigue_features = create_test_features(-20.0, 2, 300.0);
        let reasoning = service.generate_reasoning(&high_fatigue_features, &prediction);
        assert!(reasoning.contains("High fatigue"));

        // Test low fatigue
        let low_fatigue_features = create_test_features(15.0, 1, 300.0);
        let reasoning = service.generate_reasoning(&low_fatigue_features, &prediction);
        assert!(reasoning.contains("Low fatigue"));

        // Test long break
        let long_break_features = create_test_features(0.0, 5, 300.0);
        let reasoning = service.generate_reasoning(&long_break_features, &prediction);
        assert!(reasoning.contains("5 days since"));
    }

    #[test]
    fn test_generate_warnings() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);

        let high_tss_prediction = TrainingLoadPrediction {
            recommended_tss: 500.0, // Very high TSS
            confidence: 0.4, // Low confidence
            confidence_lower: 450.0,
            confidence_upper: 550.0,
            model_version: "test_v1".to_string(),
            recommended_workout_type: "vo2max".to_string(),
            predicted_at: Utc::now(),
        };

        let features = create_test_features(-25.0, 10, 200.0); // High fatigue, long break, low avg TSS
        let warnings = service.generate_warnings(&features, &high_tss_prediction);

        // Should generate multiple warnings
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("rest day")));      // High fatigue warning
        assert!(warnings.iter().any(|w| w.contains("Long break")));    // Long break warning
        assert!(warnings.iter().any(|w| w.contains("significantly higher"))); // High TSS warning
        assert!(warnings.iter().any(|w| w.contains("Low confidence"))); // Low confidence warning
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let db = setup_test_db().await;
        let service = create_test_service(db);

        let test_recommendation = TrainingRecommendation {
            user_id: create_test_user_id(),
            prediction: TrainingLoadPrediction {
                recommended_tss: 200.0,
                confidence: 0.8,
                confidence_lower: 180.0,
                confidence_upper: 220.0,
                model_version: "test_v1".to_string(),
                recommended_workout_type: "endurance".to_string(),
                predicted_at: Utc::now(),
            },
            alternative_options: vec![],
            reasoning: "Test reasoning".to_string(),
            warnings: vec![],
            cached: false,
            generated_at: Utc::now(),
        };

        // Test caching
        let cache_key = "test_key";
        service.cache_recommendation(cache_key, &test_recommendation).await;

        // Test cache retrieval
        let cached = service.get_from_cache(cache_key).await;
        assert!(cached.is_some());
        let cached_rec = cached.unwrap();
        assert!(cached_rec.cached);
        assert_eq!(cached_rec.user_id, test_recommendation.user_id);

        // Test cache stats
        let (total, expired) = service.get_cache_stats().await;
        assert_eq!(total, 1);
        assert_eq!(expired, 0);

        // Test cache cleanup
        service.cleanup_cache().await;
        let (total_after_cleanup, _) = service.get_cache_stats().await;
        assert_eq!(total_after_cleanup, 1); // Should still be there as not expired
    }

    #[test]
    fn test_edge_case_config_defaults() {
        let config = EdgeCaseConfig::default();

        assert_eq!(config.min_data_points, 5);
        assert_eq!(config.new_user_threshold_days, 14);
        assert_eq!(config.fallback_tss_easy, 75.0);
        assert_eq!(config.fallback_tss_moderate, 150.0);
        assert_eq!(config.fallback_tss_hard, 250.0);
        assert_eq!(config.max_tsb_for_hard_workout, -15.0);
        assert_eq!(config.min_tsb_for_recovery, -25.0);
    }

    #[test]
    fn test_user_feedback_struct() {
        let feedback = UserFeedback {
            perceived_difficulty: 7,
            energy_level: 8,
            motivation: 6,
            available_time_minutes: Some(90),
            preferred_intensity: Some("moderate".to_string()),
        };

        assert_eq!(feedback.perceived_difficulty, 7);
        assert_eq!(feedback.energy_level, 8);
        assert_eq!(feedback.motivation, 6);
        assert_eq!(feedback.available_time_minutes, Some(90));
        assert_eq!(feedback.preferred_intensity, Some("moderate".to_string()));
    }

    #[test]
    fn test_recommendation_request_struct() {
        let user_id = create_test_user_id();
        let target_date = Some(chrono::Utc::now().date_naive());

        let request = RecommendationRequest {
            user_id,
            target_date,
            preferred_workout_type: Some("threshold".to_string()),
            max_duration_minutes: Some(60),
            user_feedback: None,
        };

        assert_eq!(request.user_id, user_id);
        assert_eq!(request.target_date, target_date);
        assert_eq!(request.preferred_workout_type, Some("threshold".to_string()));
        assert_eq!(request.max_duration_minutes, Some(60));
        assert!(request.user_feedback.is_none());
    }
}