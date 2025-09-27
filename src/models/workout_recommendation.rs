use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Core workout recommendation enum as specified in issue #6
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WorkoutRecommendation {
    Endurance {
        duration_minutes: u32,
        target_zones: Vec<u8>
    },
    Intervals {
        warmup: u32,
        intervals: Vec<Interval>,
        cooldown: u32
    },
    Tempo {
        duration: u32,
        target_power_pct: f32
    },
    Recovery {
        duration: u32,
        max_intensity: u8
    },
    Test {
        test_type: TestType,
        instructions: String
    },
}

/// Individual interval structure for interval workouts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interval {
    pub duration_seconds: u32,
    pub target_power_pct: Option<f32>,
    pub target_zone: Option<u8>,
    pub target_heart_rate_pct: Option<f32>,
    pub rest_duration_seconds: Option<u32>,
    pub repetitions: u32,
    pub description: Option<String>,
}

/// Test types for structured testing protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestType {
    #[serde(rename = "ftp")]
    FTP,
    #[serde(rename = "vo2max")]
    VO2Max,
    #[serde(rename = "lactate_threshold")]
    LactateThreshold,
    #[serde(rename = "ramp")]
    Ramp,
    #[serde(rename = "time_trial")]
    TimeTrial { distance_meters: Option<f64> },
}

/// Training zones with associated power/heart rate ranges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingZone {
    pub zone: u8,
    pub name: String,
    pub description: String,
    pub power_pct_min: f32,
    pub power_pct_max: f32,
    pub heart_rate_pct_min: Option<f32>,
    pub heart_rate_pct_max: Option<f32>,
    pub rpe_min: Option<u8>,
    pub rpe_max: Option<u8>,
}

/// Sport-specific workout configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SportType {
    #[serde(rename = "cycling")]
    Cycling,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "swimming")]
    Swimming,
    #[serde(rename = "triathlon")]
    Triathlon,
}

/// Workout difficulty scoring factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutDifficulty {
    pub score: f32,          // 1.0 - 10.0 scale
    pub intensity_factor: f32,
    pub duration_factor: f32,
    pub complexity_factor: f32,
    pub recovery_demand: f32,
}

/// Periodization phase for seasonal training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeriodizationPhase {
    #[serde(rename = "base")]
    Base,
    #[serde(rename = "build")]
    Build,
    #[serde(rename = "peak")]
    Peak,
    #[serde(rename = "recovery")]
    Recovery,
    #[serde(rename = "transition")]
    Transition,
}

/// Complete structured workout recommendation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredWorkoutRecommendation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub sport_type: SportType,
    pub workout: WorkoutRecommendation,
    pub difficulty: WorkoutDifficulty,
    pub estimated_tss: f32,
    pub estimated_duration_minutes: u32,
    pub training_zones: Vec<TrainingZone>,
    pub periodization_phase: PeriodizationPhase,
    pub explanation: WorkoutExplanation,
    pub alternatives: Vec<AlternativeWorkout>,
    pub created_at: DateTime<Utc>,
}

/// Detailed explanation for workout recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutExplanation {
    pub primary_purpose: String,
    pub physiological_benefits: Vec<String>,
    pub timing_rationale: String,
    pub progression_notes: String,
    pub safety_considerations: Vec<String>,
}

/// Alternative workout options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeWorkout {
    pub workout: WorkoutRecommendation,
    pub difficulty: WorkoutDifficulty,
    pub estimated_tss: f32,
    pub reason: String,
}

/// Workout template for different training focuses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutTemplate {
    pub id: String,
    pub name: String,
    pub sport_type: SportType,
    pub workout_type: WorkoutRecommendation,
    pub primary_energy_system: EnergySystem,
    pub secondary_energy_systems: Vec<EnergySystem>,
    pub minimum_fitness_level: u8, // 1-10 scale
    pub equipment_required: Vec<String>,
    pub seasonal_preference: Vec<PeriodizationPhase>,
}

/// Energy systems targeted by workouts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnergySystem {
    #[serde(rename = "aerobic")]
    Aerobic,
    #[serde(rename = "anaerobic_alactic")]
    AnaerobicAlactic,
    #[serde(rename = "anaerobic_lactic")]
    AnaerobicLactic,
    #[serde(rename = "neuromuscular")]
    Neuromuscular,
}

impl TrainingZone {
    /// Get standard training zones for cycling
    pub fn cycling_zones() -> Vec<TrainingZone> {
        vec![
            TrainingZone {
                zone: 1,
                name: "Active Recovery".to_string(),
                description: "Very easy pace for recovery".to_string(),
                power_pct_min: 0.0,
                power_pct_max: 55.0,
                heart_rate_pct_min: Some(0.0),
                heart_rate_pct_max: Some(68.0),
                rpe_min: Some(1),
                rpe_max: Some(3),
            },
            TrainingZone {
                zone: 2,
                name: "Endurance".to_string(),
                description: "Aerobic base building".to_string(),
                power_pct_min: 56.0,
                power_pct_max: 75.0,
                heart_rate_pct_min: Some(69.0),
                heart_rate_pct_max: Some(83.0),
                rpe_min: Some(3),
                rpe_max: Some(5),
            },
            TrainingZone {
                zone: 3,
                name: "Tempo".to_string(),
                description: "Steady sustainable effort".to_string(),
                power_pct_min: 76.0,
                power_pct_max: 90.0,
                heart_rate_pct_min: Some(84.0),
                heart_rate_pct_max: Some(94.0),
                rpe_min: Some(5),
                rpe_max: Some(7),
            },
            TrainingZone {
                zone: 4,
                name: "Lactate Threshold".to_string(),
                description: "Sustainable intensity at threshold".to_string(),
                power_pct_min: 91.0,
                power_pct_max: 105.0,
                heart_rate_pct_min: Some(95.0),
                heart_rate_pct_max: Some(105.0),
                rpe_min: Some(7),
                rpe_max: Some(8),
            },
            TrainingZone {
                zone: 5,
                name: "VO2 Max".to_string(),
                description: "Maximal aerobic intervals".to_string(),
                power_pct_min: 106.0,
                power_pct_max: 120.0,
                heart_rate_pct_min: Some(106.0),
                heart_rate_pct_max: Some(120.0),
                rpe_min: Some(8),
                rpe_max: Some(9),
            },
            TrainingZone {
                zone: 6,
                name: "Anaerobic Capacity".to_string(),
                description: "Short high-intensity efforts".to_string(),
                power_pct_min: 121.0,
                power_pct_max: 150.0,
                heart_rate_pct_min: Some(121.0),
                heart_rate_pct_max: Some(150.0),
                rpe_min: Some(9),
                rpe_max: Some(10),
            },
            TrainingZone {
                zone: 7,
                name: "Neuromuscular Power".to_string(),
                description: "Maximum sprint efforts".to_string(),
                power_pct_min: 151.0,
                power_pct_max: 300.0,
                heart_rate_pct_min: Some(151.0),
                heart_rate_pct_max: Some(200.0),
                rpe_min: Some(10),
                rpe_max: Some(10),
            },
        ]
    }

    /// Get standard training zones for running
    pub fn running_zones() -> Vec<TrainingZone> {
        vec![
            TrainingZone {
                zone: 1,
                name: "Recovery".to_string(),
                description: "Easy recovery pace".to_string(),
                power_pct_min: 0.0,
                power_pct_max: 59.0,
                heart_rate_pct_min: Some(0.0),
                heart_rate_pct_max: Some(72.0),
                rpe_min: Some(1),
                rpe_max: Some(3),
            },
            TrainingZone {
                zone: 2,
                name: "Aerobic Base".to_string(),
                description: "Conversational pace".to_string(),
                power_pct_min: 60.0,
                power_pct_max: 73.0,
                heart_rate_pct_min: Some(73.0),
                heart_rate_pct_max: Some(80.0),
                rpe_min: Some(3),
                rpe_max: Some(4),
            },
            TrainingZone {
                zone: 3,
                name: "Aerobic Threshold".to_string(),
                description: "Comfortably hard pace".to_string(),
                power_pct_min: 74.0,
                power_pct_max: 83.0,
                heart_rate_pct_min: Some(81.0),
                heart_rate_pct_max: Some(87.0),
                rpe_min: Some(4),
                rpe_max: Some(6),
            },
            TrainingZone {
                zone: 4,
                name: "Lactate Threshold".to_string(),
                description: "Comfortably hard to hard".to_string(),
                power_pct_min: 84.0,
                power_pct_max: 94.0,
                heart_rate_pct_min: Some(88.0),
                heart_rate_pct_max: Some(92.0),
                rpe_min: Some(6),
                rpe_max: Some(8),
            },
            TrainingZone {
                zone: 5,
                name: "VO2 Max".to_string(),
                description: "Hard to very hard".to_string(),
                power_pct_min: 95.0,
                power_pct_max: 110.0,
                heart_rate_pct_min: Some(93.0),
                heart_rate_pct_max: Some(100.0),
                rpe_min: Some(8),
                rpe_max: Some(10),
            },
        ]
    }
}

impl WorkoutDifficulty {
    /// Calculate workout difficulty score
    pub fn calculate(
        intensity_factor: f32,
        duration_minutes: u32,
        complexity_factor: f32,
    ) -> Self {
        let duration_factor = (duration_minutes as f32 / 60.0).min(3.0); // Cap at 3 hours
        let recovery_demand = (intensity_factor * duration_factor * complexity_factor).min(10.0);
        let score = (intensity_factor * 0.4 + duration_factor * 0.3 + complexity_factor * 0.3).min(10.0);

        Self {
            score,
            intensity_factor,
            duration_factor,
            complexity_factor,
            recovery_demand,
        }
    }
}

impl TestType {
    /// Get instructions for test protocols
    pub fn instructions(&self) -> String {
        match self {
            TestType::FTP => {
                "20-minute all-out effort after proper warm-up. Target steady power throughout.".to_string()
            }
            TestType::VO2Max => {
                "5-minute all-out effort. Start conservative and build gradually.".to_string()
            }
            TestType::LactateThreshold => {
                "30-minute time trial at highest sustainable effort.".to_string()
            }
            TestType::Ramp => {
                "Incremental test starting easy, increasing every minute until exhaustion.".to_string()
            }
            TestType::TimeTrial { distance_meters } => {
                match distance_meters {
                    Some(distance) => format!("Complete {} meters as fast as possible with even pacing.", distance),
                    None => "Time trial at specified distance with even pacing strategy.".to_string(),
                }
            }
        }
    }
}