use crate::models::recommendation::{
    CurrentRecommendationsResponse, RecommendationCategory, RecommendationContext,
    RecommendationOutput, RecommendationPriority,
    RecommendationTemplate, RecoveryIssues, ScoredRecommendation,
    UserRecommendation, UserRecommendationHistory, UserRecommendationStatus,
};
use crate::models::RecoveryScore;
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

pub struct RecommendationEngine {
    pub db: PgPool,
}

impl RecommendationEngine {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Generate personalized recommendations based on recovery state
    pub async fn generate_recommendations(
        &self,
        user_id: Uuid,
        recovery_score: &RecoveryScore,
        context: RecommendationContext,
        limit: Option<i32>,
        category_filter: Option<RecommendationCategory>,
    ) -> Result<CurrentRecommendationsResponse> {
        let limit = limit.unwrap_or(5).min(10) as usize;

        // Step 1: Identify recovery issues
        let issues = self.identify_issues(recovery_score).await?;

        // Step 2: Get user recommendation history
        let history = self.get_user_history(user_id).await?;

        // Step 3: Get applicable recommendation templates
        let templates = self
            .get_applicable_templates(&issues, &context, category_filter)
            .await?;

        // Step 4: Score each recommendation
        let mut scored: Vec<ScoredRecommendation> = templates
            .into_iter()
            .map(|template| self.score_recommendation(&template, &issues, &history, &context))
            .collect();

        // Step 5: Sort by score and apply diversity filtering
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        let filtered = self.apply_diversity_filtering(scored, limit);

        // Step 6: Format output
        let recommendations: Vec<RecommendationOutput> = filtered
            .into_iter()
            .map(|scored| self.format_recommendation(scored))
            .collect();

        info!(
            "Generated {} recommendations for user {} with recovery score {}",
            recommendations.len(),
            user_id,
            recovery_score.readiness_score
        );

        Ok(CurrentRecommendationsResponse { recommendations })
    }

    /// Identify recovery issues from recovery score
    async fn identify_issues(&self, score: &RecoveryScore) -> Result<RecoveryIssues> {
        let mut issues = RecoveryIssues::default();

        // Poor sleep quality (< 70 score)
        if let Some(sleep_score) = score.sleep_quality_score {
            if sleep_score < 70.0 {
                issues.poor_sleep = true;
            }
        }

        // Low HRV (deviation < -10%)
        if let Some(hrv_dev) = score.hrv_deviation {
            if hrv_dev < -10.0 {
                issues.low_hrv = true;
            }
        }

        // High resting heart rate (deviation > 5%)
        if let Some(rhr_dev) = score.rhr_deviation {
            if rhr_dev > 5.0 {
                issues.high_rhr = true;
            }
        }

        // Accumulated fatigue (readiness < 60)
        if score.readiness_score < 60.0 {
            issues.accumulated_fatigue = true;
        }

        // High stress (declining HRV trend)
        if score.hrv_trend == "declining" || score.hrv_trend == "sharply_declining" {
            issues.high_stress = true;
        }

        Ok(issues)
    }

    /// Get user recommendation history for scoring
    async fn get_user_history(&self, user_id: Uuid) -> Result<UserRecommendationHistory> {
        // Get recent recommendations (last 30 days)
        let thirty_days_ago = Utc::now() - Duration::days(30);

        let recommendations = sqlx::query_as::<_, UserRecommendation>(
            r#"
            SELECT * FROM user_recommendations
            WHERE user_id = $1 AND shown_at >= $2
            ORDER BY shown_at DESC
            "#,
        )
        .bind(user_id)
        .bind(thirty_days_ago)
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch user recommendation history")?;

        let mut completed_templates = Vec::new();
        let mut skipped_templates = Vec::new();
        let mut recent_categories: Vec<RecommendationCategory> = Vec::new();
        let mut last_shown_dates = HashMap::new();
        let mut completion_counts: HashMap<Uuid, (i32, i32)> = HashMap::new();

        for rec in recommendations {
            last_shown_dates.insert(rec.recommendation_template_id, rec.shown_at);

            // Track completion counts (completed, total)
            let entry = completion_counts
                .entry(rec.recommendation_template_id)
                .or_insert((0, 0));
            entry.1 += 1;

            match rec.status {
                UserRecommendationStatus::Completed => {
                    completed_templates.push(rec.recommendation_template_id);
                    entry.0 += 1;
                }
                UserRecommendationStatus::Skipped => {
                    skipped_templates.push(rec.recommendation_template_id);
                }
                _ => {}
            }
        }

        // Calculate completion rates
        let completion_rate_by_template = completion_counts
            .into_iter()
            .map(|(template_id, (completed, total))| {
                (template_id, completed as f64 / total as f64)
            })
            .collect();

        Ok(UserRecommendationHistory {
            completed_templates,
            skipped_templates,
            recent_categories,
            last_shown_dates,
            completion_rate_by_template,
        })
    }

    /// Get applicable recommendation templates
    async fn get_applicable_templates(
        &self,
        issues: &RecoveryIssues,
        context: &RecommendationContext,
        category_filter: Option<RecommendationCategory>,
    ) -> Result<Vec<RecommendationTemplate>> {
        let mut query = String::from(
            r#"
            SELECT * FROM recommendation_templates
            WHERE is_active = true
            "#,
        );

        if let Some(category) = category_filter {
            query.push_str(&format!(" AND category = '{}'", category));
        }

        // Filter by time constraints if specified
        if let Some(time_available) = context.time_constraints {
            query.push_str(&format!(
                " AND (time_required_minutes IS NULL OR time_required_minutes <= {})",
                time_available
            ));
        }

        let templates = sqlx::query_as::<_, RecommendationTemplate>(&query)
            .fetch_all(&self.db)
            .await
            .context("Failed to fetch recommendation templates")?;

        // Filter templates based on trigger conditions matching issues
        let filtered: Vec<RecommendationTemplate> = templates
            .into_iter()
            .filter(|t| self.matches_issues(t, issues))
            .collect();

        Ok(filtered)
    }

    /// Check if template trigger conditions match current issues
    fn matches_issues(&self, template: &RecommendationTemplate, issues: &RecoveryIssues) -> bool {
        let conditions = &template.trigger_conditions;

        // Check if any issue triggers this recommendation
        if issues.poor_sleep && conditions.get("poor_sleep").and_then(|v| v.as_bool()).unwrap_or(false) {
            return true;
        }
        if issues.low_hrv && conditions.get("low_hrv").and_then(|v| v.as_bool()).unwrap_or(false) {
            return true;
        }
        if issues.high_rhr && conditions.get("high_rhr").and_then(|v| v.as_bool()).unwrap_or(false) {
            return true;
        }
        if issues.accumulated_fatigue && conditions.get("accumulated_fatigue").and_then(|v| v.as_bool()).unwrap_or(false) {
            return true;
        }
        if issues.high_stress && conditions.get("high_stress").and_then(|v| v.as_bool()).unwrap_or(false) {
            return true;
        }

        false
    }

    /// Score a recommendation using multi-factor algorithm
    fn score_recommendation(
        &self,
        template: &RecommendationTemplate,
        issues: &RecoveryIssues,
        history: &UserRecommendationHistory,
        context: &RecommendationContext,
    ) -> ScoredRecommendation {
        // Multi-factor scoring weights
        const ISSUE_MATCH_WEIGHT: f64 = 0.40;
        const EFFECTIVENESS_WEIGHT: f64 = 0.30;
        const USER_PREFERENCE_WEIGHT: f64 = 0.20;
        const NOVELTY_WEIGHT: f64 = 0.10;

        let issue_score = self.issue_match_score(template, issues);
        let effectiveness_score = template.effectiveness_score;
        let preference_score = self.user_preference_score(template, history);
        let novelty_score = self.novelty_score(template, history);

        let total_score = (issue_score * ISSUE_MATCH_WEIGHT)
            + (effectiveness_score * EFFECTIVENESS_WEIGHT)
            + (preference_score * USER_PREFERENCE_WEIGHT)
            + (novelty_score * NOVELTY_WEIGHT);

        // Determine priority based on score and issues
        let priority = self.determine_priority(total_score, issues, template);

        // Generate why this matters message
        let why_this_matters = self.generate_why_message(template, issues);

        ScoredRecommendation {
            template: template.clone(),
            score: total_score,
            priority,
            why_this_matters,
            recovery_score_id: None,
        }
    }

    /// Calculate issue match score (0.0 - 1.0)
    fn issue_match_score(&self, template: &RecommendationTemplate, issues: &RecoveryIssues) -> f64 {
        let conditions = &template.trigger_conditions;
        let mut matches = 0;
        let mut total_issues = 0;

        if issues.poor_sleep {
            total_issues += 1;
            if conditions.get("poor_sleep").and_then(|v| v.as_bool()).unwrap_or(false) {
                matches += 1;
            }
        }
        if issues.low_hrv {
            total_issues += 1;
            if conditions.get("low_hrv").and_then(|v| v.as_bool()).unwrap_or(false) {
                matches += 1;
            }
        }
        if issues.high_rhr {
            total_issues += 1;
            if conditions.get("high_rhr").and_then(|v| v.as_bool()).unwrap_or(false) {
                matches += 1;
            }
        }
        if issues.accumulated_fatigue {
            total_issues += 1;
            if conditions.get("accumulated_fatigue").and_then(|v| v.as_bool()).unwrap_or(false) {
                matches += 1;
            }
        }
        if issues.high_stress {
            total_issues += 1;
            if conditions.get("high_stress").and_then(|v| v.as_bool()).unwrap_or(false) {
                matches += 1;
            }
        }

        if total_issues == 0 {
            return 0.5; // No issues, medium relevance
        }

        matches as f64 / total_issues as f64
    }

    /// Calculate user preference score based on history
    fn user_preference_score(
        &self,
        template: &RecommendationTemplate,
        history: &UserRecommendationHistory,
    ) -> f64 {
        // Check completion rate for this specific template
        if let Some(&rate) = history
            .completion_rate_by_template
            .get(&template.id)
        {
            return rate;
        }

        // Default to template's global effectiveness if no user history
        template.effectiveness_score
    }

    /// Calculate novelty score (avoid repetition)
    fn novelty_score(
        &self,
        template: &RecommendationTemplate,
        history: &UserRecommendationHistory,
    ) -> f64 {
        // Check when this was last shown
        if let Some(&last_shown) = history.last_shown_dates.get(&template.id) {
            let days_since = (Utc::now() - last_shown).num_days();

            // Higher score for recommendations not shown recently
            match days_since {
                0..=2 => 0.2,   // Very recently shown
                3..=7 => 0.5,   // Shown this week
                8..=14 => 0.8,  // Shown last week
                _ => 1.0,       // Not shown for 2+ weeks
            }
        } else {
            1.0 // Never shown before
        }
    }

    /// Determine priority level based on score and issues
    fn determine_priority(
        &self,
        score: f64,
        issues: &RecoveryIssues,
        template: &RecommendationTemplate,
    ) -> RecommendationPriority {
        // Critical if accumulated fatigue and high score
        if issues.accumulated_fatigue && score > 0.75 {
            return RecommendationPriority::Critical;
        }

        // Use template default, potentially adjusted by score
        if score > 0.8 {
            match template.priority_default {
                RecommendationPriority::Low => RecommendationPriority::Medium,
                RecommendationPriority::Medium => RecommendationPriority::High,
                other => other,
            }
        } else {
            template.priority_default
        }
    }

    /// Generate contextual "why this matters" message
    fn generate_why_message(&self, template: &RecommendationTemplate, issues: &RecoveryIssues) -> String {
        let mut reasons = Vec::new();

        if issues.poor_sleep {
            reasons.push("your sleep quality has been suboptimal");
        }
        if issues.low_hrv {
            reasons.push("your HRV is below baseline");
        }
        if issues.high_rhr {
            reasons.push("your resting heart rate is elevated");
        }
        if issues.accumulated_fatigue {
            reasons.push("you're showing signs of accumulated fatigue");
        }
        if issues.high_stress {
            reasons.push("stress indicators are elevated");
        }

        if reasons.is_empty() {
            return template
                .expected_impact
                .clone()
                .unwrap_or_else(|| "This recommendation supports overall recovery".to_string());
        }

        format!(
            "{}. {}",
            reasons.join(", and "),
            template
                .expected_impact
                .clone()
                .unwrap_or_else(|| "This action can help improve your recovery".to_string())
        )
    }

    /// Apply diversity filtering to ensure variety
    fn apply_diversity_filtering(
        &self,
        mut scored: Vec<ScoredRecommendation>,
        limit: usize,
    ) -> Vec<ScoredRecommendation> {
        let mut selected = Vec::new();
        let mut category_counts: HashMap<String, usize> = HashMap::new();
        let mut difficulty_counts: HashMap<String, usize> = HashMap::new();

        for rec in scored.drain(..) {
            let category_key = rec.template.category.to_string();
            let difficulty_key = rec.template.difficulty.to_string();

            let category_count = category_counts.get(&category_key).unwrap_or(&0);
            let difficulty_count = difficulty_counts.get(&difficulty_key).unwrap_or(&0);

            // Max 2 per category, max 3 of same difficulty
            if *category_count < 2 && *difficulty_count < 3 && selected.len() < limit {
                *category_counts.entry(category_key).or_insert(0) += 1;
                *difficulty_counts.entry(difficulty_key).or_insert(0) += 1;
                selected.push(rec);
            }

            if selected.len() >= limit {
                break;
            }
        }

        selected
    }

    /// Format recommendation for API output
    fn format_recommendation(&self, scored: ScoredRecommendation) -> RecommendationOutput {
        let time_required = scored
            .template
            .time_required_minutes
            .map(|mins| format!("{} minutes", mins));

        RecommendationOutput {
            id: scored.template.id,
            category: scored.template.category,
            priority: scored.priority,
            title: scored.template.title,
            description: scored.template.description,
            action: scored.template.action,
            why_this_matters: scored.why_this_matters,
            expected_impact: scored.template.expected_impact,
            time_required,
            difficulty: scored.template.difficulty,
            score: scored.score,
        }
    }
}
