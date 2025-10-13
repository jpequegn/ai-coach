use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

/// Recommendation outcome model for tracking effectiveness
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecommendationOutcome {
    pub id: Uuid,
    pub user_recommendation_id: Uuid,
    pub user_id: Uuid,
    pub recommendation_template_id: Uuid,

    // Timing metrics
    pub shown_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub completion_time_hours: f64,

    // Recovery metrics
    pub baseline_recovery_score_id: Option<Uuid>,
    pub baseline_recovery_score: f64,
    pub next_day_recovery_score_id: Option<Uuid>,
    pub next_day_recovery_score: Option<f64>,
    pub recovery_improvement: Option<f64>,

    // User feedback
    pub user_rating: Option<i32>,
    pub user_feedback: Option<String>,

    // Calculated effectiveness
    pub effectiveness_score: Option<f64>,

    // Metadata
    pub metadata: JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create an outcome record (typically automatic)
#[derive(Debug, Deserialize)]
pub struct CreateOutcomeRequest {
    pub user_recommendation_id: Uuid,
    pub baseline_recovery_score_id: Option<Uuid>,
    pub baseline_recovery_score: f64,
    pub user_rating: Option<i32>,
    pub user_feedback: Option<String>,
}

/// Request to update outcome with next-day data
#[derive(Debug, Deserialize)]
pub struct UpdateOutcomeRequest {
    pub next_day_recovery_score_id: Option<Uuid>,
    pub next_day_recovery_score: f64,
}

/// Effectiveness analytics for a template or category
#[derive(Debug, Serialize)]
pub struct EffectivenessAnalytics {
    pub template_id: Option<Uuid>,
    pub category: Option<String>,
    pub total_suggestions: i64,
    pub total_completions: i64,
    pub completion_rate: f64,
    pub avg_user_rating: Option<f64>,
    pub avg_recovery_improvement: Option<f64>,
    pub avg_effectiveness_score: Option<f64>,
    pub effectiveness_trend: String, // improving, stable, declining
    pub last_30_days_completions: i64,
    pub last_30_days_avg_rating: Option<f64>,
}

/// System-wide recommendation analytics
#[derive(Debug, Serialize)]
pub struct SystemAnalytics {
    pub total_recommendations_shown: i64,
    pub total_recommendations_completed: i64,
    pub overall_completion_rate: f64,
    pub overall_avg_rating: Option<f64>,
    pub overall_avg_effectiveness: Option<f64>,
    pub category_analytics: Vec<CategoryAnalytics>,
    pub top_performing_templates: Vec<TemplatePerformance>,
    pub underperforming_templates: Vec<TemplatePerformance>,
}

/// Per-category analytics
#[derive(Debug, Serialize)]
pub struct CategoryAnalytics {
    pub category: String,
    pub completions: i64,
    pub completion_rate: f64,
    pub avg_rating: Option<f64>,
    pub avg_effectiveness: Option<f64>,
}

/// Template performance summary
#[derive(Debug, Serialize)]
pub struct TemplatePerformance {
    pub template_id: Uuid,
    pub title: String,
    pub category: String,
    pub completions: i64,
    pub completion_rate: f64,
    pub avg_rating: Option<f64>,
    pub avg_effectiveness: Option<f64>,
    pub needs_review: bool,
}

/// Query filters for effectiveness analytics
#[derive(Debug, Deserialize)]
pub struct EffectivenessFilter {
    pub template_id: Option<Uuid>,
    pub category: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub min_completions: Option<i64>,
}

/// Batch outcome processing result
#[derive(Debug, Serialize)]
pub struct ProcessOutcomesResult {
    pub processed_count: u64,
    pub updated_templates: Vec<Uuid>,
    pub flagged_for_review: Vec<Uuid>,
    pub errors: Vec<String>,
}

/// Calculate effectiveness score based on multiple factors
///
/// Formula: (recovery_improvement * 0.5 + user_rating * 0.5) * time_factor
/// - Recovery improvement: normalized 0-1 based on score change
/// - User rating: normalized 0-1 (rating / 5)
/// - Time factor: 1.0 if completed within 24h, 0.8 otherwise
pub fn calculate_outcome_effectiveness_score(
    baseline_score: f64,
    next_day_score: f64,
    user_rating: Option<i32>,
    completion_time_hours: f64,
) -> f64 {
    // Recovery improvement component (0-1 scale)
    // Positive improvement is good, negative is bad
    // Normalize to 0-1 range assuming max improvement of 50 points
    let recovery_delta = (next_day_score - baseline_score) / 100.0;
    let recovery_component = ((recovery_delta + 0.5).max(0.0)).min(1.0);

    // User rating component (0-1 scale)
    // If no rating provided, use neutral 0.5
    let rating_component = user_rating
        .map(|r| (r as f64) / 5.0)
        .unwrap_or(0.5);

    // Time factor - penalize slow completion
    // Complete within 24h = 1.0, otherwise 0.8
    let time_factor = if completion_time_hours <= 24.0 {
        1.0
    } else {
        0.8
    };

    // Combined score with equal weighting
    let base_score = (recovery_component * 0.5) + (rating_component * 0.5);
    base_score * time_factor
}

/// Calculate exponential moving average for template effectiveness
///
/// Uses 30% weight for new data, 70% for existing average
pub fn update_effectiveness_with_ema(
    current_effectiveness: f64,
    new_outcome: f64,
) -> f64 {
    const ALPHA: f64 = 0.3; // 30% weight to new data
    (ALPHA * new_outcome) + ((1.0 - ALPHA) * current_effectiveness)
}

/// Determine effectiveness trend based on recent vs historical data
pub fn determine_effectiveness_trend(
    last_30_days_avg: Option<f64>,
    overall_avg: Option<f64>,
) -> String {
    match (last_30_days_avg, overall_avg) {
        (Some(recent), Some(overall)) => {
            let diff = recent - overall;
            if diff > 0.05 {
                "improving".to_string()
            } else if diff < -0.05 {
                "declining".to_string()
            } else {
                "stable".to_string()
            }
        }
        _ => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_effectiveness_score_positive_improvement() {
        // Baseline 60, next day 80, rating 5, completed in 12h
        let score = calculate_outcome_effectiveness_score(60.0, 80.0, Some(5), 12.0);

        // Recovery: (80-60)/100 = 0.2, normalized: (0.2+0.5) = 0.7
        // Rating: 5/5 = 1.0
        // Time factor: 1.0
        // Score: (0.7*0.5 + 1.0*0.5) * 1.0 = 0.85
        assert!((score - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_calculate_effectiveness_score_negative_improvement() {
        // Baseline 70, next day 60, rating 2, completed in 30h
        let score = calculate_outcome_effectiveness_score(70.0, 60.0, Some(2), 30.0);

        // Recovery: (60-70)/100 = -0.1, normalized: (-0.1+0.5) = 0.4
        // Rating: 2/5 = 0.4
        // Time factor: 0.8 (>24h)
        // Score: (0.4*0.5 + 0.4*0.5) * 0.8 = 0.32
        assert!((score - 0.32).abs() < 0.01);
    }

    #[test]
    fn test_calculate_effectiveness_score_no_rating() {
        // Baseline 65, next day 75, no rating, completed in 18h
        let score = calculate_outcome_effectiveness_score(65.0, 75.0, None, 18.0);

        // Recovery: (75-65)/100 = 0.1, normalized: (0.1+0.5) = 0.6
        // Rating: default 0.5
        // Time factor: 1.0
        // Score: (0.6*0.5 + 0.5*0.5) * 1.0 = 0.55
        assert!((score - 0.55).abs() < 0.01);
    }

    #[test]
    fn test_update_effectiveness_with_ema() {
        let current = 0.7;
        let new_outcome = 0.9;

        // EMA: 0.3 * 0.9 + 0.7 * 0.7 = 0.27 + 0.49 = 0.76
        let updated = update_effectiveness_with_ema(current, new_outcome);
        assert!((updated - 0.76).abs() < 0.01);
    }

    #[test]
    fn test_determine_effectiveness_trend_improving() {
        let trend = determine_effectiveness_trend(Some(0.8), Some(0.7));
        assert_eq!(trend, "improving");
    }

    #[test]
    fn test_determine_effectiveness_trend_declining() {
        let trend = determine_effectiveness_trend(Some(0.6), Some(0.75));
        assert_eq!(trend, "declining");
    }

    #[test]
    fn test_determine_effectiveness_trend_stable() {
        let trend = determine_effectiveness_trend(Some(0.72), Some(0.70));
        assert_eq!(trend, "stable");
    }
}
