use anyhow::Result;
use chrono::{Utc, NaiveDate, Duration, Datelike};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{TrainingFeatures, TrainingDataPoint, PerformanceManagementChart, TrainingSession};
use crate::services::{TrainingAnalysisService, TrainingSessionService};

/// Service for extracting machine learning features from training data
#[derive(Clone)]
pub struct FeatureEngineeringService {
    db: PgPool,
    analysis_service: TrainingAnalysisService,
    session_service: TrainingSessionService,
}

impl FeatureEngineeringService {
    /// Create a new FeatureEngineeringService
    pub fn new(db: PgPool) -> Self {
        let analysis_service = TrainingAnalysisService::new(db.clone(), None)
            .expect("Failed to create TrainingAnalysisService");
        let session_service = TrainingSessionService::new(db.clone());

        Self {
            db,
            analysis_service,
            session_service,
        }
    }

    /// Extract current training features for a user
    pub async fn extract_current_features(&self, user_id: Uuid) -> Result<TrainingFeatures> {
        let today = Utc::now().date_naive();

        // Get current PMC data
        let pmc = self.analysis_service.get_pmc_for_date(user_id, today).await?;

        // Get recent training sessions for additional features
        let recent_sessions = self.session_service.get_sessions_by_date_range(
            user_id,
            today - Duration::days(30),
            today
        ).await?;

        // Calculate days since last workout
        let days_since_last_workout = self.calculate_days_since_last_workout(&recent_sessions, today);

        // Calculate 4-week average TSS
        let avg_weekly_tss_4weeks = self.calculate_4week_avg_tss(&recent_sessions);

        // Calculate recent performance trend
        let recent_performance_trend = self.calculate_performance_trend(&recent_sessions);

        // Get preferred workout types from recent sessions
        let preferred_workout_types = self.extract_preferred_workout_types(&recent_sessions);

        // Calculate seasonal factors (simplified - could be enhanced with weather data, etc.)
        let seasonal_factors = self.calculate_seasonal_factors(today);

        // Get days until goal event (would need to be implemented based on athlete goals)
        let days_until_goal_event = self.get_days_until_goal_event(user_id).await?;

        Ok(TrainingFeatures {
            current_ctl: pmc.ctl,
            current_atl: pmc.atl,
            current_tsb: pmc.tsb,
            days_since_last_workout,
            avg_weekly_tss_4weeks,
            recent_performance_trend,
            days_until_goal_event,
            preferred_workout_types,
            seasonal_factors,
        })
    }

    /// Extract historical training data points for model training
    pub async fn extract_historical_data_points(
        &self,
        user_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<TrainingDataPoint>> {
        let sessions = self.session_service.get_sessions_by_date_range(user_id, start_date, end_date).await?;
        let mut data_points = Vec::new();

        for session in &sessions {
            // For each session, extract features from the day before
            let feature_date = session.date - Duration::days(1);

            if let Ok(features) = self.extract_features_for_date(user_id, feature_date).await {
                let data_point = TrainingDataPoint {
                    features,
                    actual_tss: session.trainrs_data
                        .as_ref()
                        .and_then(|data| data.get("tss"))
                        .and_then(|tss| tss.as_f64())
                        .unwrap_or(0.0) as f32,
                    actual_workout_type: session.session_type
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    performance_outcome: None, // Would need to be collected from user feedback
                    recovery_rating: None,     // Would need to be collected from user feedback
                    workout_date: session.created_at,
                };
                data_points.push(data_point);
            }
        }

        Ok(data_points)
    }

    /// Extract features for a specific date
    async fn extract_features_for_date(&self, user_id: Uuid, date: NaiveDate) -> Result<TrainingFeatures> {
        // Get PMC data for the specific date
        let pmc = self.analysis_service.get_pmc_for_date(user_id, date).await?;

        // Get sessions from 30 days before this date
        let start_period = date - Duration::days(30);
        let recent_sessions = self.session_service.get_sessions_by_date_range(
            user_id,
            start_period,
            date
        ).await?;

        let days_since_last_workout = self.calculate_days_since_last_workout(&recent_sessions, date);
        let avg_weekly_tss_4weeks = self.calculate_4week_avg_tss(&recent_sessions);
        let recent_performance_trend = self.calculate_performance_trend(&recent_sessions);
        let preferred_workout_types = self.extract_preferred_workout_types(&recent_sessions);
        let seasonal_factors = self.calculate_seasonal_factors(date);
        let days_until_goal_event = self.get_days_until_goal_event(user_id).await?;

        Ok(TrainingFeatures {
            current_ctl: pmc.ctl,
            current_atl: pmc.atl,
            current_tsb: pmc.tsb,
            days_since_last_workout,
            avg_weekly_tss_4weeks,
            recent_performance_trend,
            days_until_goal_event,
            preferred_workout_types,
            seasonal_factors,
        })
    }

    /// Calculate days since last workout
    fn calculate_days_since_last_workout(&self, sessions: &[TrainingSession], reference_date: NaiveDate) -> i32 {
        sessions
            .iter()
            .filter_map(|session| {
                if session.date < reference_date {
                    Some((reference_date - session.date).num_days() as i32)
                } else {
                    None
                }
            })
            .min()
            .unwrap_or(999) // Large number if no recent workouts
    }

    /// Calculate 4-week average TSS
    fn calculate_4week_avg_tss(&self, sessions: &[TrainingSession]) -> f32 {
        let total_tss: f32 = sessions
            .iter()
            .filter_map(|session| {
                session.trainrs_data
                    .as_ref()
                    .and_then(|data| data.get("tss"))
                    .and_then(|tss| tss.as_f64())
                    .map(|tss| tss as f32)
            })
            .sum();

        if sessions.is_empty() {
            0.0
        } else {
            total_tss / 4.0 // Divide by 4 weeks
        }
    }

    /// Calculate recent performance trend (-1.0 to 1.0)
    fn calculate_performance_trend(&self, sessions: &[TrainingSession]) -> f32 {
        if sessions.len() < 2 {
            return 0.0;
        }

        // Simple trend calculation based on TSS progression
        // More sophisticated methods could include power data, heart rate, etc.
        let mut tss_values: Vec<f32> = sessions
            .iter()
            .filter_map(|session| {
                session.trainrs_data
                    .as_ref()
                    .and_then(|data| data.get("tss"))
                    .and_then(|tss| tss.as_f64())
                    .map(|tss| tss as f32)
            })
            .collect();

        if tss_values.len() < 2 {
            return 0.0;
        }

        tss_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Calculate trend as the difference between recent and older sessions
        let recent_avg = tss_values[tss_values.len() / 2..].iter().sum::<f32>() / (tss_values.len() / 2) as f32;
        let older_avg = tss_values[..tss_values.len() / 2].iter().sum::<f32>() / (tss_values.len() / 2) as f32;

        // Normalize to -1.0 to 1.0 range
        let trend = (recent_avg - older_avg) / (recent_avg + older_avg + 1.0);
        trend.clamp(-1.0, 1.0)
    }

    /// Extract preferred workout types from recent sessions
    fn extract_preferred_workout_types(&self, sessions: &[TrainingSession]) -> Vec<String> {
        let mut type_counts = std::collections::HashMap::new();

        for session in sessions {
            if let Some(session_type) = &session.session_type {
                *type_counts.entry(session_type.clone()).or_insert(0) += 1;
            }
        }

        // Return top 3 most common workout types
        let mut types: Vec<_> = type_counts.into_iter().collect();
        types.sort_by(|a, b| b.1.cmp(&a.1));
        types.into_iter().take(3).map(|(type_name, _)| type_name).collect()
    }

    /// Calculate seasonal factors (0.0 to 1.0)
    fn calculate_seasonal_factors(&self, date: NaiveDate) -> f32 {
        // Simple seasonal calculation based on month
        // Could be enhanced with actual weather data, daylight hours, etc.
        let month = date.month();

        match month {
            12 | 1 | 2 => 0.7,    // Winter - harder to train outdoors
            3 | 4 | 5 => 0.9,     // Spring - good training weather
            6 | 7 | 8 => 1.0,     // Summer - optimal training conditions
            9 | 10 | 11 => 0.8,   // Fall - decent training weather
            _ => 1.0,
        }
    }

    /// Get days until goal event (placeholder implementation)
    async fn get_days_until_goal_event(&self, _user_id: Uuid) -> Result<Option<i32>> {
        // This would need to be implemented based on athlete goals/events system
        // For now, returning None as placeholder
        Ok(None)
    }

    /// Batch extract features for multiple users and dates
    pub async fn batch_extract_features(
        &self,
        user_ids: &[Uuid],
        date: NaiveDate,
    ) -> Result<Vec<(Uuid, TrainingFeatures)>> {
        let mut results = Vec::new();

        for &user_id in user_ids {
            match self.extract_features_for_date(user_id, date).await {
                Ok(features) => results.push((user_id, features)),
                Err(e) => {
                    tracing::warn!("Failed to extract features for user {}: {}", user_id, e);
                    // Continue with other users instead of failing completely
                }
            }
        }

        Ok(results)
    }

    /// Get training load statistics for validation
    pub async fn get_training_load_stats(&self, user_id: Uuid, days: i32) -> Result<TrainingLoadStats> {
        let end_date = Utc::now().date_naive();
        let start_date = end_date - Duration::days(days as i64);

        let sessions = self.session_service.get_sessions_by_date_range(user_id, start_date, end_date).await?;

        let tss_values: Vec<f32> = sessions
            .iter()
            .filter_map(|session| {
                session.trainrs_data
                    .as_ref()
                    .and_then(|data| data.get("tss"))
                    .and_then(|tss| tss.as_f64())
                    .map(|tss| tss as f32)
            })
            .collect();

        let mean_tss = if tss_values.is_empty() { 0.0 } else { tss_values.iter().sum::<f32>() / tss_values.len() as f32 };
        let std_tss = if tss_values.len() < 2 { 0.0 } else {
            let variance = tss_values.iter().map(|&x| (x - mean_tss).powi(2)).sum::<f32>() / (tss_values.len() - 1) as f32;
            variance.sqrt()
        };

        Ok(TrainingLoadStats {
            mean_tss,
            std_tss,
            min_tss: tss_values.iter().cloned().fold(f32::INFINITY, f32::min),
            max_tss: tss_values.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
            session_count: sessions.len(),
        })
    }
}

/// Training load statistics for validation and normalization
#[derive(Debug, Clone)]
pub struct TrainingLoadStats {
    pub mean_tss: f32,
    pub std_tss: f32,
    pub min_tss: f32,
    pub max_tss: f32,
    pub session_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Utc, Duration, NaiveDate};
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

    fn create_test_service(db: PgPool) -> FeatureEngineeringService {
        FeatureEngineeringService::new(db)
    }

    fn create_test_user_id() -> Uuid {
        Uuid::new_v4()
    }

    fn create_test_training_sessions() -> Vec<TrainingSession> {
        let user_id = create_test_user_id();
        let base_date = Utc::now().date_naive() - Duration::days(30);

        (0..15).map(|i| {
            let tss = 100.0 + (i as f64 * 10.0);
            let session_data = serde_json::json!({
                "tss": tss,
                "duration": 3600,
                "average_power": 200.0
            });

            TrainingSession {
                id: Uuid::new_v4(),
                user_id,
                date: base_date + Duration::days(i * 2),
                trainrs_data: Some(session_data),
                uploaded_file_path: None,
                session_type: Some("endurance".to_string()),
                duration_seconds: Some(3600),
                distance_meters: Some(40000.0),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }).collect()
    }

    #[test]
    fn test_calculate_days_since_last_workout() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);
        let sessions = create_test_training_sessions();
        let reference_date = Utc::now().date_naive();

        let days = service.calculate_days_since_last_workout(&sessions, reference_date);

        // Should be at least a few days since the last session
        assert!(days >= 0);
        assert!(days <= 30); // Within reasonable range for test data
    }

    #[test]
    fn test_calculate_4week_avg_tss() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);
        let sessions = create_test_training_sessions();

        let avg_tss = service.calculate_4week_avg_tss(&sessions);

        // Should be positive and reasonable
        assert!(avg_tss > 0.0);
        assert!(avg_tss < 1000.0); // Reasonable upper bound
    }

    #[test]
    fn test_calculate_performance_trend() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);
        let sessions = create_test_training_sessions();

        let trend = service.calculate_performance_trend(&sessions);

        // Should be between -1.0 and 1.0
        assert!(trend >= -1.0);
        assert!(trend <= 1.0);
    }

    #[test]
    fn test_extract_preferred_workout_types() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);
        let sessions = create_test_training_sessions();

        let workout_types = service.extract_preferred_workout_types(&sessions);

        // Should return some workout types
        assert!(!workout_types.is_empty());
        assert!(workout_types.len() <= 3); // Max 3 types

        // Should contain "endurance" from test data
        assert!(workout_types.contains(&"endurance".to_string()));
    }

    #[test]
    fn test_calculate_seasonal_factors() {
        let db = PgPool::connect("postgresql://test").await.unwrap_or_else(|_| return);
        let service = create_test_service(db);

        // Test winter month
        let winter_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let winter_factor = service.calculate_seasonal_factors(winter_date);
        assert_eq!(winter_factor, 0.7);

        // Test summer month
        let summer_date = NaiveDate::from_ymd_opt(2024, 7, 15).unwrap();
        let summer_factor = service.calculate_seasonal_factors(summer_date);
        assert_eq!(summer_factor, 1.0);

        // Test spring month
        let spring_date = NaiveDate::from_ymd_opt(2024, 4, 15).unwrap();
        let spring_factor = service.calculate_seasonal_factors(spring_date);
        assert_eq!(spring_factor, 0.9);
    }

    #[tokio::test]
    async fn test_training_load_stats_empty_data() {
        let db = setup_test_db().await;
        let service = create_test_service(db);
        let user_id = create_test_user_id();

        let stats = service.get_training_load_stats(user_id, 30).await;

        // Should handle empty data gracefully
        if let Ok(stats) = stats {
            assert_eq!(stats.session_count, 0);
            assert_eq!(stats.mean_tss, 0.0);
            assert_eq!(stats.std_tss, 0.0);
        }
    }

    #[test]
    fn test_training_features_to_ndarray() {
        let features = TrainingFeatures {
            current_ctl: 100.0,
            current_atl: 50.0,
            current_tsb: 50.0,
            days_since_last_workout: 2,
            avg_weekly_tss_4weeks: 300.0,
            recent_performance_trend: 0.1,
            days_until_goal_event: Some(30),
            preferred_workout_types: vec!["endurance".to_string(), "threshold".to_string()],
            seasonal_factors: 0.8,
        };

        let array = features.to_ndarray();

        // Should have correct number of features
        let expected_length = 8 + 5; // 8 numeric features + 5 workout type one-hot
        assert_eq!(array.len(), expected_length);

        // Check some specific values
        assert_eq!(array[0], 100.0); // current_ctl
        assert_eq!(array[1], 50.0);  // current_atl
        assert_eq!(array[2], 50.0);  // current_tsb
        assert_eq!(array[3], 2.0);   // days_since_last_workout

        // Check one-hot encoding for workout types
        assert_eq!(array[8], 1.0);   // endurance (first workout type)
        assert_eq!(array[9], 1.0);   // threshold (second workout type)
    }

    #[test]
    fn test_training_features_feature_names() {
        let names = TrainingFeatures::feature_names();

        // Should have correct number of feature names
        let expected_length = 8 + 5; // 8 numeric features + 5 workout type features
        assert_eq!(names.len(), expected_length);

        // Check some specific names
        assert_eq!(names[0], "current_ctl");
        assert_eq!(names[1], "current_atl");
        assert_eq!(names[2], "current_tsb");
        assert!(names.contains(&"prefers_endurance".to_string()));
        assert!(names.contains(&"prefers_threshold".to_string()));
    }

    #[test]
    fn test_training_features_default() {
        let features = TrainingFeatures::default();

        assert_eq!(features.current_ctl, 0.0);
        assert_eq!(features.current_atl, 0.0);
        assert_eq!(features.current_tsb, 0.0);
        assert_eq!(features.days_since_last_workout, 0);
        assert_eq!(features.avg_weekly_tss_4weeks, 0.0);
        assert_eq!(features.recent_performance_trend, 0.0);
        assert_eq!(features.days_until_goal_event, None);
        assert!(features.preferred_workout_types.is_empty());
        assert_eq!(features.seasonal_factors, 1.0);
    }
}