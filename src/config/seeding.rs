use anyhow::Result;
use chrono::{NaiveDate, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::*;
use crate::services::*;

pub struct DatabaseSeeder {
    pool: PgPool,
}

impl DatabaseSeeder {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn seed_all(&self) -> Result<()> {
        tracing::info!("Starting database seeding...");

        self.seed_users().await?;
        self.seed_athlete_profiles().await?;
        self.seed_training_sessions().await?;
        self.seed_recommendations().await?;
        self.seed_training_plans().await?;
        self.seed_predictions().await?;

        tracing::info!("Database seeding completed!");
        Ok(())
    }

    async fn seed_users(&self) -> Result<()> {
        let user_service = UserService::new(self.pool.clone());

        let demo_users = vec![
            CreateUser {
                email: "john.doe@example.com".to_string(),
                password: "password123".to_string(),
            },
            CreateUser {
                email: "jane.smith@example.com".to_string(),
                password: "password123".to_string(),
            },
            CreateUser {
                email: "mike.cyclist@example.com".to_string(),
                password: "password123".to_string(),
            },
        ];

        for user_data in demo_users {
            if user_service.get_user_by_email(&user_data.email).await?.is_none() {
                user_service.create_user(user_data).await?;
                tracing::info!("Created demo user");
            }
        }

        Ok(())
    }

    async fn seed_athlete_profiles(&self) -> Result<()> {
        let profile_service = AthleteProfileService::new(self.pool.clone());
        let user_service = UserService::new(self.pool.clone());

        // Get the first user for demo profile
        let users = user_service.list_users(Some(1), Some(0)).await?;
        if let Some(user) = users.first() {
            if profile_service.get_profile_by_user_id(user.id).await?.is_none() {
                let profile_data = CreateAthleteProfile {
                    user_id: user.id,
                    sport: "cycling".to_string(),
                    ftp: Some(250),
                    lthr: Some(165),
                    max_heart_rate: Some(185),
                    threshold_pace: Some(4.5),
                    zones: Some(json!({
                        "zone_1_min": 80,
                        "zone_1_max": 120,
                        "zone_2_min": 120,
                        "zone_2_max": 145,
                        "zone_3_min": 145,
                        "zone_3_max": 165,
                        "zone_4_min": 165,
                        "zone_4_max": 180,
                        "zone_5_min": 180,
                        "zone_5_max": 200
                    })),
                };

                profile_service.create_profile(profile_data).await?;
                tracing::info!("Created demo athlete profile");
            }
        }

        Ok(())
    }

    async fn seed_training_sessions(&self) -> Result<()> {
        let session_service = TrainingSessionService::new(self.pool.clone());
        let user_service = UserService::new(self.pool.clone());

        let users = user_service.list_users(Some(1), Some(0)).await?;
        if let Some(user) = users.first() {
            let existing_sessions = session_service.get_sessions_by_user_id(user.id, Some(1), Some(0)).await?;

            if existing_sessions.is_empty() {
                let session_data = CreateTrainingSession {
                    user_id: user.id,
                    date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
                    trainrs_data: Some(json!({
                        "workout_type": "endurance",
                        "power_data": {
                            "avg_power": 220,
                            "max_power": 350,
                            "normalized_power": 235
                        },
                        "heart_rate": {
                            "avg_hr": 150,
                            "max_hr": 172
                        }
                    })),
                    uploaded_file_path: Some("/uploads/training_2024_01_15.fit".to_string()),
                    session_type: Some("endurance".to_string()),
                    duration_seconds: Some(3600),
                    distance_meters: Some(45000.0),
                };

                session_service.create_session(session_data).await?;
                tracing::info!("Created demo training session");
            }
        }

        Ok(())
    }

    async fn seed_recommendations(&self) -> Result<()> {
        let rec_service = CoachingRecommendationService::new(self.pool.clone());
        let user_service = UserService::new(self.pool.clone());

        let users = user_service.list_users(Some(1), Some(0)).await?;
        if let Some(user) = users.first() {
            let existing_recs = rec_service.get_recommendations_by_user_id(user.id, Some(1)).await?;

            if existing_recs.is_empty() {
                let rec_data = CreateCoachingRecommendation {
                    user_id: user.id,
                    recommendation_type: "training_adjustment".to_string(),
                    content: "Consider increasing your endurance training volume by 10% this week based on your recovery metrics.".to_string(),
                    confidence: Some(0.85),
                    metadata: Some(json!({
                        "basis": "recovery_analysis",
                        "metrics_used": ["hrv", "sleep_quality", "subjective_feeling"],
                        "time_frame": "next_week"
                    })),
                };

                rec_service.create_recommendation(rec_data).await?;
                tracing::info!("Created demo coaching recommendation");
            }
        }

        Ok(())
    }

    async fn seed_training_plans(&self) -> Result<()> {
        let plan_service = TrainingPlanService::new(self.pool.clone());
        let user_service = UserService::new(self.pool.clone());

        let users = user_service.list_users(Some(1), Some(0)).await?;
        if let Some(user) = users.first() {
            let existing_plans = plan_service.get_plans_by_user_id(user.id).await?;

            if existing_plans.is_empty() {
                let plan_data = CreateTrainingPlan {
                    user_id: user.id,
                    goal: "Prepare for century ride".to_string(),
                    start_date: NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
                    end_date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
                    plan_data: json!({
                        "weeks": [
                            {
                                "week_number": 1,
                                "weekly_volume": 8,
                                "workouts": [
                                    {"day": "Monday", "type": "rest"},
                                    {"day": "Tuesday", "type": "interval", "duration": 60},
                                    {"day": "Wednesday", "type": "endurance", "duration": 90},
                                    {"day": "Thursday", "type": "recovery", "duration": 45},
                                    {"day": "Friday", "type": "rest"},
                                    {"day": "Saturday", "type": "long_ride", "duration": 120},
                                    {"day": "Sunday", "type": "recovery", "duration": 60}
                                ]
                            }
                        ]
                    }),
                };

                plan_service.create_plan(plan_data).await?;
                tracing::info!("Created demo training plan");
            }
        }

        Ok(())
    }

    async fn seed_predictions(&self) -> Result<()> {
        let pred_service = ModelPredictionService::new(self.pool.clone());
        let user_service = UserService::new(self.pool.clone());

        let users = user_service.list_users(Some(1), Some(0)).await?;
        if let Some(user) = users.first() {
            let existing_preds = pred_service.get_predictions_by_user_id(user.id, None, Some(1)).await?;

            if existing_preds.is_empty() {
                let pred_data = CreateModelPrediction {
                    user_id: user.id,
                    prediction_type: "performance".to_string(),
                    data: json!({
                        "predicted_ftp": 265,
                        "confidence_interval": {
                            "lower": 250,
                            "upper": 280
                        },
                        "time_horizon": "4_weeks",
                        "factors": ["training_load", "recovery", "consistency"]
                    }),
                    confidence: Some(0.78),
                    model_version: Some("v1.2.3".to_string()),
                };

                pred_service.create_prediction(pred_data).await?;
                tracing::info!("Created demo model prediction");
            }
        }

        Ok(())
    }
}