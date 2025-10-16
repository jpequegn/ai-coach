// ============================================================================
// MVP Services - Minimal functionality only
// ============================================================================
pub mod user_service;
pub mod athlete_profile_service;
// pub mod goal_service;  // Blocked by SQLite DateTime TEXT compatibility - see docs/sqlite-compatibility-notes.md

// Disabled for MVP - too many compilation errors
// pub mod crud_service;
// pub mod job_registry;
// pub mod training_session_service;
// pub mod training_analysis_service;
// pub mod background_job_service;
// pub mod coaching_recommendation_service;
// pub mod training_plan_service;
// pub mod model_prediction_service;
// pub mod feature_engineering_service;
// pub mod ml_model_service;
// pub mod model_training_service;
// pub mod training_recommendation_service;
// pub mod model_versioning_service;
// pub mod workout_recommendation_service;
// pub mod performance_insights_service;
// pub mod notification_service;
// pub mod notification_scheduler;
// pub mod email_notification_service;
// pub mod event_service;
// pub mod plan_generation_service;
// pub mod vision_analysis_service;
// pub mod video_storage_service;
// pub mod video_processing_service;
// pub mod pose_estimation_service;
// pub mod keypoint_processor;
// pub mod recovery_data_service;
// pub mod recovery_analysis_service;
// pub mod training_adjustment_service;
// pub mod recovery_alert_service;
// pub mod oura_api_client;
// pub mod oura_integration_service;
// pub mod validation_service;
// pub mod performance_profiler;
// pub mod processing_config;
// pub mod performance_benchmark;
// pub mod recommendation_tracking_service;
// pub mod recommendation_engine_service;
// pub mod user_recovery_profile_service;
// pub mod recommendation_effectiveness_service;
// pub mod recovery_job_scheduler;
// pub mod daily_recovery_calculation_job;
// pub mod alert_delivery_queue_service;
// pub mod alert_delivery_job;
// pub mod email_batching_service;
// pub mod data_quality_check_job;
// pub mod weekly_baseline_recalculation_job;

pub use user_service::UserService;
pub use athlete_profile_service::AthleteProfileService;
// pub use goal_service::GoalService;  // Blocked by SQLite DateTime TEXT compatibility

// Disabled for MVP
// pub use training_session_service::TrainingSessionService;
// pub use training_analysis_service::TrainingAnalysisService;
// pub use background_job_service::BackgroundJobService;
// pub use coaching_recommendation_service::CoachingRecommendationService;
// pub use training_plan_service::TrainingPlanService;
// pub use model_prediction_service::ModelPredictionService;
// pub use feature_engineering_service::FeatureEngineeringService;
// pub use ml_model_service::MLModelService;
// pub use model_training_service::ModelTrainingService;
// pub use training_recommendation_service::TrainingRecommendationService;
// pub use model_versioning_service::ModelVersioningService;
// pub use workout_recommendation_service::WorkoutRecommendationService;
// pub use performance_insights_service::PerformanceInsightsService;
// pub use notification_service::NotificationService;
// pub use notification_scheduler::NotificationScheduler;
// pub use email_notification_service::EmailNotificationService;
// pub use event_service::EventService;
// pub use plan_generation_service::PlanGenerationService;
// pub use vision_analysis_service::VisionAnalysisService;
// pub use video_storage_service::VideoStorageService;
// pub use video_processing_service::VideoProcessingService;
// pub use pose_estimation_service::PoseEstimationService;
// pub use keypoint_processor::KeypointProcessor;
// pub use recovery_data_service::RecoveryDataService;
// pub use recovery_analysis_service::RecoveryAnalysisService;
// pub use training_adjustment_service::TrainingAdjustmentService;
// pub use recovery_alert_service::RecoveryAlertService;
// pub use oura_api_client::OuraApiClient;
// pub use oura_integration_service::OuraIntegrationService;
// pub use validation_service::ValidationService;
// pub use recommendation_tracking_service::RecommendationTrackingService;
// pub use recommendation_engine_service::RecommendationEngine;
// pub use user_recovery_profile_service::UserRecoveryProfileService;
// pub use recommendation_effectiveness_service::RecommendationEffectivenessService;
// pub use recovery_job_scheduler::RecoveryJobScheduler;
// pub use daily_recovery_calculation_job::DailyRecoveryCalculationJob;
// pub use alert_delivery_queue_service::AlertDeliveryQueueService;
// pub use alert_delivery_job::AlertDeliveryJob;
// pub use email_batching_service::{EmailBatchConfig, EmailBatchingService};
// pub use data_quality_check_job::DataQualityCheckJob;
// pub use weekly_baseline_recalculation_job::WeeklyBaselineRecalculationJob;
// pub use job_registry::{Job, JobRegistry};
// pub use crud_service::{CrudService, ListParams, PaginatedResult, SortOrder};