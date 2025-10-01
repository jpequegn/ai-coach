use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Comprehensive performance insights for an athlete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceInsights {
    pub user_id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,

    // Trend Analysis
    pub fitness_trends: FitnessTrends,
    pub performance_trends: PerformanceTrends,
    pub training_consistency: TrainingConsistency,

    // Weakness Identification
    pub power_curve_analysis: PowerCurveAnalysis,
    pub zone_distribution_analysis: ZoneDistributionAnalysis,
    pub recovery_analysis: RecoveryAnalysis,

    // Goal Progress
    pub goal_progress: Vec<GoalProgress>,
    pub predicted_race_times: Vec<RaceTimePrediction>,
    pub training_plan_adherence: TrainingPlanAdherence,

    // AI-Generated Insights
    pub key_insights: Vec<InsightMessage>,
    pub recommendations: Vec<RecommendationMessage>,
    pub warnings: Vec<WarningMessage>,
    pub achievements: Vec<AchievementMessage>,

    // Comparative Analysis
    pub peer_comparison: Option<PeerComparison>,
    pub age_group_benchmarks: Option<AgeGroupBenchmarks>,
    pub historical_comparison: Option<HistoricalComparison>,
}

/// Fitness progression analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessTrends {
    pub current_ctl: f64,
    pub ctl_trend_6weeks: f64, // Percentage change over 6 weeks
    pub ctl_trend_3months: f64, // Percentage change over 3 months
    pub ctl_stability: f64, // Coefficient of variation (0-1, lower = more stable)

    pub current_atl: f64,
    pub current_tsb: f64,
    pub tsb_trend: TsbTrend,

    pub peak_fitness_date: Option<NaiveDate>,
    pub peak_fitness_value: Option<f64>,
    pub fitness_trajectory: FitnessTrajectory,
}

/// Performance improvement analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrends {
    pub power_trend_30days: f64, // % change in average power
    pub endurance_trend_30days: f64, // % change in long-duration performance
    pub sprint_trend_30days: f64, // % change in short-duration performance

    pub seasonal_performance_pattern: SeasonalPattern,
    pub best_performances: Vec<BestPerformance>,
    pub performance_volatility: f64, // Day-to-day performance consistency
}

/// Training consistency metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConsistency {
    pub weekly_consistency_score: f64, // 0-100, how consistent week-to-week
    pub sessions_per_week_avg: f64,
    pub missed_sessions_rate: f64, // Percentage of planned sessions missed
    pub longest_consistent_streak: u32, // Days with training
    pub current_streak: u32,
    pub training_load_consistency: f64, // CV of weekly TSS
}

/// Power curve analysis to identify strengths/weaknesses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerCurveAnalysis {
    pub duration_strengths: Vec<DurationStrength>, // Durations where athlete excels
    pub duration_weaknesses: Vec<DurationWeakness>, // Durations needing work
    pub power_profile_type: PowerProfileType,
    pub critical_power_estimates: CriticalPowerEstimates,
}

/// Zone distribution analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneDistributionAnalysis {
    pub current_distribution: ZoneDistribution,
    pub recommended_distribution: ZoneDistribution,
    pub polarization_index: f64, // Measure of training polarization
    pub zone_imbalances: Vec<ZoneImbalance>,
}

/// Recovery pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAnalysis {
    pub average_recovery_time: f64, // Hours between hard sessions
    pub recovery_consistency: f64, // How consistent recovery patterns are
    pub overreaching_risk: RiskLevel,
    pub recovery_recommendations: Vec<String>,
    pub hrv_trends: Option<HrvTrends>, // If HRV data available
}

/// Goal progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalProgress {
    pub goal_id: Uuid,
    pub goal_name: String,
    pub goal_type: GoalType,
    pub target_value: f64,
    pub current_value: f64,
    pub progress_percentage: f64,
    pub on_track: bool,
    pub projected_completion_date: Option<NaiveDate>,
    pub required_improvement_rate: f64, // Units per week needed
}

/// Race time predictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceTimePrediction {
    pub distance: String, // "5K", "10K", "Half Marathon", etc.
    pub predicted_time: String, // "HH:MM:SS"
    pub confidence_level: f64, // 0-1
    pub improvement_potential: String, // "Low", "Medium", "High"
    pub key_limiters: Vec<String>, // What's holding back performance
}

/// Training plan adherence analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPlanAdherence {
    pub adherence_percentage: f64, // % of planned sessions completed
    pub intensity_adherence: f64, // How well actual matches planned intensity
    pub volume_adherence: f64, // How well actual matches planned volume
    pub common_deviations: Vec<String>, // Common ways athlete deviates from plan
}

/// AI-generated insight message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightMessage {
    pub category: InsightCategory,
    pub message: String,
    pub confidence: f64, // AI confidence in this insight
    pub supporting_data: Vec<String>, // Key metrics supporting this insight
}

/// AI-generated recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationMessage {
    pub priority: RecommendationPriority,
    pub action: String,
    pub reasoning: String,
    pub expected_benefit: String,
    pub time_frame: String, // "This week", "Next 2-4 weeks", etc.
}

/// AI-generated warning message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarningMessage {
    pub severity: WarningSeverity,
    pub title: String,
    pub description: String,
    pub recommended_action: String,
}

/// Achievement recognition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementMessage {
    pub achievement_type: AchievementType,
    pub title: String,
    pub description: String,
    pub date_achieved: NaiveDate,
}

/// Comparison with similar athletes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerComparison {
    pub fitness_percentile: f64, // Where athlete ranks among peers (0-100)
    pub volume_percentile: f64,
    pub consistency_percentile: f64,
    pub peer_group_size: u32,
    pub peer_criteria: String, // How peers were selected
}

/// Age group benchmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgeGroupBenchmarks {
    pub age_group: String, // "35-39", "40-44", etc.
    pub power_percentile: Option<f64>,
    pub endurance_percentile: Option<f64>,
    pub ranking_estimates: Vec<RankingEstimate>,
}

/// Historical comparison with athlete's past performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalComparison {
    pub vs_last_year: PerformanceComparison,
    pub vs_best_year: PerformanceComparison,
    pub long_term_trend: TrendDirection,
    pub career_highlights: Vec<CareerHighlight>,
}

// Supporting enums and types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TsbTrend {
    Improving,
    Declining,
    Stable,
    Overreaching,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FitnessTrajectory {
    Building,
    Peaking,
    Declining,
    Maintaining,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeasonalPattern {
    PeaksInSummer,
    PeaksInWinter,
    NoSeasonalPattern,
    BuildsThroughYear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestPerformance {
    pub metric: String, // "20min Power", "5K Time", etc.
    pub value: f64,
    pub date: NaiveDate,
    pub improvement_from_previous: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationStrength {
    pub duration: String, // "5 seconds", "20 minutes", etc.
    pub percentile: f64, // How good they are at this duration
    pub strength_level: StrengthLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationWeakness {
    pub duration: String,
    pub percentile: f64,
    pub improvement_potential: f64, // How much improvement is realistic
    pub training_focus: String, // What type of training to improve
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PowerProfileType {
    Sprinter,
    Pursuer,
    AllRounder,
    TtSpecialist,
    Climber,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalPowerEstimates {
    pub cp_watts: Option<f64>, // Critical Power
    pub w_prime_kj: Option<f64>, // W' (anaerobic capacity)
    pub ftp_estimate: Option<f64>, // Functional Threshold Power
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneDistribution {
    pub zone_1_percent: f64,
    pub zone_2_percent: f64,
    pub zone_3_percent: f64,
    pub zone_4_percent: f64,
    pub zone_5_percent: f64,
    pub zone_6_percent: f64,
    pub zone_7_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneImbalance {
    pub zone: u8,
    pub current_percent: f64,
    pub recommended_percent: f64,
    pub imbalance_severity: ImbalanceSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Moderate,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrvTrends {
    pub avg_hrv_7day: f64,
    pub hrv_trend_30day: f64, // % change
    pub recovery_score: f64, // 0-100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalType {
    PowerGoal,
    WeightGoal,
    DistanceGoal,
    TimeGoal,
    RaceGoal,
    ConsistencyGoal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightCategory {
    Fitness,
    Performance,
    Training,
    Recovery,
    Goals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AchievementType {
    PersonalBest,
    ConsistencyMilestone,
    VolumeTarget,
    PowerImprovement,
    RaceGoal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingEstimate {
    pub event: String, // "Local 40K TT", "Age Group Triathlon", etc.
    pub estimated_placing: String, // "Top 10%", "Podium Potential", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceComparison {
    pub fitness_change: f64, // % change in CTL
    pub power_change: f64, // % change in average power
    pub volume_change: f64, // % change in training volume
    pub consistency_change: f64, // % change in consistency score
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Declining,
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerHighlight {
    pub achievement: String,
    pub date: NaiveDate,
    pub metric_value: f64,
    pub context: String, // Additional context about the achievement
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrengthLevel {
    Exceptional,
    Strong,
    Average,
    Weak,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImbalanceSeverity {
    Minor,
    Moderate,
    Significant,
}

/// Request for generating performance insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceInsightsRequest {
    pub user_id: Uuid,
    pub period_days: Option<u32>, // Analysis period in days (default: 90)
    pub include_peer_comparison: bool,
    pub include_predictions: bool,
    pub focus_areas: Vec<String>, // "fitness", "performance", "goals", etc.
}