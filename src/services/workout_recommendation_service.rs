use anyhow::Result;
use chrono::{Utc, NaiveDate, Duration, Datelike};
use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashMap;
use tracing::{info, warn};
use serde::{Serialize, Deserialize};
use rand::Rng;

use crate::models::{
    WorkoutRecommendation, StructuredWorkoutRecommendation, WorkoutDifficulty, TrainingZone,
    SportType, PeriodizationPhase, WorkoutExplanation, AlternativeWorkout, Interval, TestType,
    EnergySystem, WorkoutTemplate, TrainingFeatures
};
use crate::services::{FeatureEngineeringService, TrainingRecommendationService};

/// Request for structured workout recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutRecommendationRequest {
    pub user_id: Uuid,
    pub sport_type: SportType,
    pub target_date: Option<NaiveDate>,
    pub max_duration_minutes: Option<u32>,
    pub preferred_intensity: Option<String>, // "easy", "moderate", "hard"
    pub available_equipment: Vec<String>,
    pub goals: Vec<String>,
    pub recent_workouts: Option<Vec<String>>, // For variety
}

/// Workout recommendation engine service
#[derive(Clone)]
pub struct WorkoutRecommendationService {
    db: PgPool,
    feature_service: FeatureEngineeringService,
    training_rec_service: TrainingRecommendationService,
    workout_templates: HashMap<String, WorkoutTemplate>,
}

impl WorkoutRecommendationService {
    /// Create a new WorkoutRecommendationService
    pub fn new(db: PgPool) -> Self {
        let feature_service = FeatureEngineeringService::new(db.clone());
        let training_rec_service = TrainingRecommendationService::new(db.clone());
        let workout_templates = Self::load_workout_templates();

        Self {
            db,
            feature_service,
            training_rec_service,
            workout_templates,
        }
    }

    /// Get structured workout recommendation
    pub async fn get_structured_workout_recommendation(
        &self,
        request: WorkoutRecommendationRequest,
    ) -> Result<StructuredWorkoutRecommendation> {
        info!("Generating structured workout recommendation for user {}", request.user_id);

        // Get user's current training features
        let features = self.feature_service.extract_current_features(request.user_id).await?;

        // Determine periodization phase
        let periodization_phase = self.determine_periodization_phase(&features, request.target_date).await?;

        // Get base TSS recommendation from ML service
        let base_rec_request = crate::services::training_recommendation_service::RecommendationRequest {
            user_id: request.user_id,
            target_date: request.target_date,
            preferred_workout_type: request.preferred_intensity.clone(),
            max_duration_minutes: request.max_duration_minutes.map(|d| d as i32),
            user_feedback: None,
        };
        let base_recommendation = self.training_rec_service.get_recommendation(base_rec_request).await?;

        // Generate structured workout based on recommendations
        let workout = self.generate_structured_workout(
            &features,
            &base_recommendation,
            &request,
            &periodization_phase,
        ).await?;

        // Calculate difficulty
        let difficulty = self.calculate_workout_difficulty(&workout, &request.sport_type);

        // Get training zones for sport
        let training_zones = match request.sport_type {
            SportType::Cycling => TrainingZone::cycling_zones(),
            SportType::Running => TrainingZone::running_zones(),
            SportType::Swimming => TrainingZone::running_zones(), // Use running as baseline for now
            SportType::Triathlon => TrainingZone::cycling_zones(), // Use cycling as baseline
        };

        // Generate explanation
        let explanation = self.generate_workout_explanation(
            &workout,
            &features,
            &periodization_phase,
            base_recommendation.prediction.recommended_tss,
        );

        // Generate alternatives
        let alternatives = self.generate_workout_alternatives(
            &workout,
            &features,
            &request,
            &difficulty,
        ).await?;

        let recommendation = StructuredWorkoutRecommendation {
            id: Uuid::new_v4(),
            user_id: request.user_id,
            sport_type: request.sport_type,
            workout,
            difficulty,
            estimated_tss: base_recommendation.prediction.recommended_tss,
            estimated_duration_minutes: request.max_duration_minutes.unwrap_or(60),
            training_zones,
            periodization_phase,
            explanation,
            alternatives,
            created_at: Utc::now(),
        };

        Ok(recommendation)
    }

    /// Generate structured workout based on TSS and context
    async fn generate_structured_workout(
        &self,
        features: &TrainingFeatures,
        base_recommendation: &crate::services::training_recommendation_service::TrainingRecommendation,
        request: &WorkoutRecommendationRequest,
        phase: &PeriodizationPhase,
    ) -> Result<WorkoutRecommendation> {
        let target_tss = base_recommendation.prediction.recommended_tss;
        let duration = request.max_duration_minutes.unwrap_or(Self::estimate_duration_from_tss(target_tss));

        // Choose workout type based on periodization, TSB, and preferences
        let workout_type = self.select_workout_type(features, phase, &request.preferred_intensity, target_tss);

        let workout = match workout_type.as_str() {
            "recovery" => self.create_recovery_workout(duration),
            "endurance" => self.create_endurance_workout(duration, target_tss, &request.sport_type),
            "tempo" => self.create_tempo_workout(duration, target_tss),
            "threshold" => self.create_threshold_workout(duration, target_tss),
            "vo2max" => self.create_vo2max_workout(duration, target_tss),
            "test" => self.create_test_workout(features, &request.sport_type),
            _ => self.create_endurance_workout(duration, target_tss, &request.sport_type),
        };

        Ok(workout)
    }

    /// Determine current periodization phase
    async fn determine_periodization_phase(
        &self,
        features: &TrainingFeatures,
        target_date: Option<NaiveDate>,
    ) -> Result<PeriodizationPhase> {
        // Simplified periodization logic based on TSB and seasonal factors
        if features.current_tsb < -20.0 {
            return Ok(PeriodizationPhase::Recovery);
        }

        // Use seasonal factors and target date
        let current_date = Utc::now().date_naive();
        let phase = match target_date {
            Some(target) => {
                let days_to_target = (target - current_date).num_days();
                if days_to_target <= 14 {
                    PeriodizationPhase::Peak
                } else if days_to_target <= 42 {
                    PeriodizationPhase::Build
                } else {
                    PeriodizationPhase::Base
                }
            }
            None => {
                // Use seasonal periodization
                let month = current_date.month();
                match month {
                    12 | 1 | 2 => PeriodizationPhase::Base,     // Winter: Base building
                    3 | 4 | 5 => PeriodizationPhase::Build,     // Spring: Build phase
                    6 | 7 | 8 => PeriodizationPhase::Peak,      // Summer: Peak/Race season
                    9 | 10 | 11 => PeriodizationPhase::Transition, // Fall: Transition
                    _ => PeriodizationPhase::Base,
                }
            }
        };

        Ok(phase)
    }

    /// Select appropriate workout type based on context
    fn select_workout_type(
        &self,
        features: &TrainingFeatures,
        phase: &PeriodizationPhase,
        preferred_intensity: &Option<String>,
        target_tss: f32,
    ) -> String {
        // Override for user preference
        if let Some(intensity) = preferred_intensity {
            match intensity.as_str() {
                "easy" => return "recovery".to_string(),
                "moderate" => return "endurance".to_string(),
                "hard" => return "threshold".to_string(),
                _ => {}
            }
        }

        // TSB-based selection
        if features.current_tsb < -15.0 {
            return "recovery".to_string();
        }

        // Periodization-based selection
        match phase {
            PeriodizationPhase::Base => {
                if target_tss < 100.0 { "endurance" } else { "tempo" }
            }
            PeriodizationPhase::Build => {
                if target_tss < 150.0 { "tempo" } else { "threshold" }
            }
            PeriodizationPhase::Peak => {
                if target_tss < 200.0 { "threshold" } else { "vo2max" }
            }
            PeriodizationPhase::Recovery => "recovery",
            PeriodizationPhase::Transition => "endurance",
        }.to_string()
    }

    /// Create recovery workout
    fn create_recovery_workout(&self, duration: u32) -> WorkoutRecommendation {
        WorkoutRecommendation::Recovery {
            duration,
            max_intensity: 2, // Zone 1-2 only
        }
    }

    /// Create endurance workout
    fn create_endurance_workout(&self, duration: u32, target_tss: f32, sport: &SportType) -> WorkoutRecommendation {
        let zones = match sport {
            SportType::Cycling => vec![2, 3], // Zones 2-3 for cycling
            SportType::Running => vec![1, 2], // Zones 1-2 for running
            _ => vec![2], // Default to zone 2
        };

        WorkoutRecommendation::Endurance {
            duration_minutes: duration,
            target_zones: zones,
        }
    }

    /// Create tempo workout
    fn create_tempo_workout(&self, duration: u32, target_tss: f32) -> WorkoutRecommendation {
        let target_power_pct = if target_tss > 200.0 { 85.0 } else { 80.0 };

        WorkoutRecommendation::Tempo {
            duration,
            target_power_pct,
        }
    }

    /// Create threshold workout
    fn create_threshold_workout(&self, duration: u32, target_tss: f32) -> WorkoutRecommendation {
        let warmup = (duration as f32 * 0.2) as u32;
        let cooldown = (duration as f32 * 0.15) as u32;
        let work_time = duration - warmup - cooldown;

        // Create threshold intervals
        let intervals = if work_time >= 40 {
            // Long threshold intervals
            vec![
                Interval {
                    duration_seconds: (work_time / 2) * 60,
                    target_power_pct: Some(95.0),
                    target_zone: Some(4),
                    target_heart_rate_pct: Some(90.0),
                    rest_duration_seconds: Some(300), // 5 min rest
                    repetitions: 2,
                    description: Some("Threshold interval".to_string()),
                }
            ]
        } else {
            // Short threshold intervals
            vec![
                Interval {
                    duration_seconds: 480, // 8 minutes
                    target_power_pct: Some(100.0),
                    target_zone: Some(4),
                    target_heart_rate_pct: Some(92.0),
                    rest_duration_seconds: Some(180), // 3 min rest
                    repetitions: (work_time / 11).max(2), // 8min work + 3min rest = 11min blocks
                    description: Some("Short threshold interval".to_string()),
                }
            ]
        };

        WorkoutRecommendation::Intervals {
            warmup,
            intervals,
            cooldown,
        }
    }

    /// Create VO2max workout
    fn create_vo2max_workout(&self, duration: u32, target_tss: f32) -> WorkoutRecommendation {
        let warmup = (duration as f32 * 0.25) as u32;
        let cooldown = (duration as f32 * 0.2) as u32;

        let intervals = vec![
            Interval {
                duration_seconds: 300, // 5 minutes
                target_power_pct: Some(115.0),
                target_zone: Some(5),
                target_heart_rate_pct: Some(95.0),
                rest_duration_seconds: Some(300), // Equal rest
                repetitions: ((duration - warmup - cooldown) / 10).max(3), // 5min work + 5min rest = 10min
                description: Some("VO2max interval".to_string()),
            }
        ];

        WorkoutRecommendation::Intervals {
            warmup,
            intervals,
            cooldown,
        }
    }

    /// Create test workout
    fn create_test_workout(&self, features: &TrainingFeatures, sport: &SportType) -> WorkoutRecommendation {
        // Determine appropriate test based on recent training
        let test_type = if features.days_since_last_workout > 30 {
            TestType::Ramp // Start with ramp test after long break
        } else if features.current_ctl > 80.0 {
            TestType::FTP // Standard FTP test for trained athletes
        } else {
            TestType::TimeTrial { distance_meters: Some(5000.0) } // 5k TT for newer athletes
        };

        WorkoutRecommendation::Test {
            test_type,
            instructions: "Follow warmup protocol. Execute test at steady effort. Cool down properly.".to_string(),
        }
    }

    /// Calculate workout difficulty
    fn calculate_workout_difficulty(&self, workout: &WorkoutRecommendation, sport: &SportType) -> WorkoutDifficulty {
        let (intensity_factor, duration_minutes, complexity_factor) = match workout {
            WorkoutRecommendation::Recovery { duration, .. } => {
                (2.0, *duration, 1.0)
            }
            WorkoutRecommendation::Endurance { duration_minutes, target_zones } => {
                let avg_zone = target_zones.iter().sum::<u8>() as f32 / target_zones.len() as f32;
                (avg_zone * 1.2, *duration_minutes, 1.5)
            }
            WorkoutRecommendation::Tempo { duration, target_power_pct } => {
                let intensity = target_power_pct / 100.0 * 6.0; // Convert to zone equivalent
                (intensity, *duration, 2.0)
            }
            WorkoutRecommendation::Intervals { warmup, intervals, cooldown } => {
                let total_duration = warmup + cooldown +
                    intervals.iter().map(|i| (i.duration_seconds / 60) * i.repetitions).sum::<u32>();
                let avg_intensity = intervals.iter()
                    .map(|i| i.target_power_pct.unwrap_or(80.0) / 100.0 * 7.0)
                    .sum::<f32>() / intervals.len() as f32;
                let complexity = 3.0 + intervals.len() as f32 * 0.5;
                (avg_intensity, total_duration, complexity)
            }
            WorkoutRecommendation::Test { .. } => {
                (8.0, 60, 4.0) // Tests are always high intensity and complex
            }
        };

        WorkoutDifficulty::calculate(intensity_factor, duration_minutes, complexity_factor)
    }

    /// Generate workout explanation
    fn generate_workout_explanation(
        &self,
        workout: &WorkoutRecommendation,
        features: &TrainingFeatures,
        phase: &PeriodizationPhase,
        target_tss: f32,
    ) -> WorkoutExplanation {
        let (primary_purpose, physiological_benefits, timing_rationale) = match workout {
            WorkoutRecommendation::Recovery { .. } => (
                "Active recovery to promote adaptation and prepare for future training".to_string(),
                vec![
                    "Increases blood flow to aid recovery".to_string(),
                    "Maintains movement patterns".to_string(),
                    "Reduces muscle stiffness".to_string(),
                ],
                format!("TSB of {:.1} indicates need for recovery", features.current_tsb),
            ),
            WorkoutRecommendation::Endurance { .. } => (
                "Aerobic base development and metabolic efficiency".to_string(),
                vec![
                    "Improves mitochondrial density".to_string(),
                    "Enhances fat oxidation".to_string(),
                    "Builds aerobic capacity".to_string(),
                    "Improves cardiac output".to_string(),
                ],
                format!("Base phase training aligned with {:?} periodization", phase),
            ),
            WorkoutRecommendation::Tempo { .. } => (
                "Improve sustainable power and metabolic efficiency".to_string(),
                vec![
                    "Enhances lactate clearance".to_string(),
                    "Improves aerobic power".to_string(),
                    "Increases sustainable pace".to_string(),
                ],
                "Moderate intensity for aerobic development".to_string(),
            ),
            WorkoutRecommendation::Intervals { .. } => (
                "High-intensity training for anaerobic and neuromuscular development".to_string(),
                vec![
                    "Improves VO2max".to_string(),
                    "Enhances lactate tolerance".to_string(),
                    "Increases anaerobic capacity".to_string(),
                    "Improves neuromuscular power".to_string(),
                ],
                format!("High TSS target ({:.0}) requires interval training", target_tss),
            ),
            WorkoutRecommendation::Test { test_type, .. } => (
                format!("Performance testing: {:?}", test_type),
                vec![
                    "Establishes current fitness level".to_string(),
                    "Provides data for training zones".to_string(),
                    "Tracks fitness progression".to_string(),
                ],
                "Regular testing ensures accurate training zones".to_string(),
            ),
        };

        let progression_notes = match features.recent_performance_trend {
            x if x > 0.1 => "Fitness is improving - maintain current progression".to_string(),
            x if x < -0.1 => "Performance declining - focus on recovery and consistency".to_string(),
            _ => "Stable fitness - good time for targeted improvements".to_string(),
        };

        let mut safety_considerations = vec![
            "Complete proper warmup before intense efforts".to_string(),
            "Listen to your body and adjust intensity if needed".to_string(),
        ];

        if features.current_tsb < -10.0 {
            safety_considerations.push("High fatigue detected - consider reducing intensity".to_string());
        }

        if features.days_since_last_workout >= 7 {
            safety_considerations.push("Return to training gradually after extended break".to_string());
        }

        WorkoutExplanation {
            primary_purpose,
            physiological_benefits,
            timing_rationale,
            progression_notes,
            safety_considerations,
        }
    }

    /// Generate alternative workout options
    async fn generate_workout_alternatives(
        &self,
        primary_workout: &WorkoutRecommendation,
        features: &TrainingFeatures,
        request: &WorkoutRecommendationRequest,
        primary_difficulty: &WorkoutDifficulty,
    ) -> Result<Vec<AlternativeWorkout>> {
        let mut alternatives = Vec::new();

        // Easy alternative (75% intensity)
        let easy_workout = match primary_workout {
            WorkoutRecommendation::Intervals { warmup, cooldown, .. } => {
                WorkoutRecommendation::Endurance {
                    duration_minutes: warmup + cooldown + 30,
                    target_zones: vec![2],
                }
            }
            _ => WorkoutRecommendation::Recovery {
                duration: request.max_duration_minutes.unwrap_or(45),
                max_intensity: 2,
            }
        };

        let easy_difficulty = WorkoutDifficulty::calculate(
            primary_difficulty.intensity_factor * 0.6,
            request.max_duration_minutes.unwrap_or(45),
            1.0,
        );

        alternatives.push(AlternativeWorkout {
            workout: easy_workout,
            difficulty: easy_difficulty,
            estimated_tss: primary_difficulty.score * 40.0, // Rough TSS estimate
            reason: "Easier option for recovery or low energy days".to_string(),
        });

        // Hard alternative (if TSB allows)
        if features.current_tsb > -10.0 {
            let hard_workout = WorkoutRecommendation::Intervals {
                warmup: 20,
                intervals: vec![
                    Interval {
                        duration_seconds: 240, // 4 minutes
                        target_power_pct: Some(110.0),
                        target_zone: Some(5),
                        target_heart_rate_pct: Some(95.0),
                        rest_duration_seconds: Some(240),
                        repetitions: 4,
                        description: Some("High-intensity interval".to_string()),
                    }
                ],
                cooldown: 15,
            };

            let hard_difficulty = WorkoutDifficulty::calculate(
                primary_difficulty.intensity_factor * 1.3,
                55, // Total duration
                3.5,
            );

            alternatives.push(AlternativeWorkout {
                workout: hard_workout,
                difficulty: hard_difficulty,
                estimated_tss: primary_difficulty.score * 80.0,
                reason: "Higher intensity option for challenging training".to_string(),
            });
        }

        Ok(alternatives)
    }

    /// Estimate workout duration from TSS
    fn estimate_duration_from_tss(tss: f32) -> u32 {
        // Rough estimation: 1 TSS per minute for moderate intensity
        let base_duration = tss.min(300.0).max(30.0); // Cap between 30-300 minutes
        base_duration as u32
    }

    /// Load workout templates (simplified for MVP)
    fn load_workout_templates() -> HashMap<String, WorkoutTemplate> {
        let mut templates = HashMap::new();

        // Add basic templates - in production these would come from database
        templates.insert("endurance_cycling".to_string(), WorkoutTemplate {
            id: "endurance_cycling".to_string(),
            name: "Endurance Ride".to_string(),
            sport_type: SportType::Cycling,
            workout_type: WorkoutRecommendation::Endurance {
                duration_minutes: 90,
                target_zones: vec![2, 3],
            },
            primary_energy_system: EnergySystem::Aerobic,
            secondary_energy_systems: vec![],
            minimum_fitness_level: 3,
            equipment_required: vec!["bike".to_string()],
            seasonal_preference: vec![PeriodizationPhase::Base, PeriodizationPhase::Transition],
        });

        templates
    }

    /// Prevent workout monotony by considering recent workout history
    pub async fn apply_variety_filter(
        &self,
        workouts: &mut Vec<StructuredWorkoutRecommendation>,
        recent_workouts: &[String],
    ) -> Result<()> {
        // Simple variety logic - avoid repeating same workout type 3+ times
        let recent_count = recent_workouts.len();
        if recent_count >= 2 {
            let last_two_types: Vec<String> = recent_workouts.iter()
                .take(2)
                .map(|w| self.extract_workout_type(w))
                .collect();

            // If last two workouts were the same type, modify recommendation
            if last_two_types.len() == 2 && last_two_types[0] == last_two_types[1] {
                for workout in workouts.iter_mut() {
                    let current_type = self.workout_to_type_string(&workout.workout);
                    if current_type == last_two_types[0] {
                        info!("Applying variety filter - changing workout type to prevent monotony");
                        workout.workout = self.get_variety_alternative(&workout.workout);
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract workout type from workout string representation
    fn extract_workout_type(&self, workout: &str) -> String {
        // Simplified parsing - in production would use proper deserialization
        if workout.contains("Endurance") { "endurance".to_string() }
        else if workout.contains("Intervals") { "intervals".to_string() }
        else if workout.contains("Tempo") { "tempo".to_string() }
        else if workout.contains("Recovery") { "recovery".to_string() }
        else { "unknown".to_string() }
    }

    /// Convert workout to type string
    fn workout_to_type_string(&self, workout: &WorkoutRecommendation) -> String {
        match workout {
            WorkoutRecommendation::Endurance { .. } => "endurance".to_string(),
            WorkoutRecommendation::Intervals { .. } => "intervals".to_string(),
            WorkoutRecommendation::Tempo { .. } => "tempo".to_string(),
            WorkoutRecommendation::Recovery { .. } => "recovery".to_string(),
            WorkoutRecommendation::Test { .. } => "test".to_string(),
        }
    }

    /// Get alternative workout type for variety
    fn get_variety_alternative(&self, original: &WorkoutRecommendation) -> WorkoutRecommendation {
        match original {
            WorkoutRecommendation::Endurance { duration_minutes, .. } => {
                WorkoutRecommendation::Tempo {
                    duration: *duration_minutes,
                    target_power_pct: 80.0,
                }
            }
            WorkoutRecommendation::Tempo { duration, .. } => {
                WorkoutRecommendation::Endurance {
                    duration_minutes: *duration,
                    target_zones: vec![2],
                }
            }
            WorkoutRecommendation::Intervals { warmup, cooldown, .. } => {
                WorkoutRecommendation::Tempo {
                    duration: warmup + cooldown + 30,
                    target_power_pct: 85.0,
                }
            }
            _ => original.clone(), // Don't change recovery or test workouts
        }
    }
}