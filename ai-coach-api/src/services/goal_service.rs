use anyhow::Result;
use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    Goal, GoalProgress, CreateGoalRequest, UpdateGoalRequest, CreateGoalProgressRequest,
    GoalProgressSummary, GoalRecommendation, TrendDirection, GoalStatus, GoalType,
    GoalCategory, GoalPriority, RecommendationType
};

#[derive(Clone)]
pub struct GoalService {
    db: PgPool,
}

impl GoalService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    // Goal CRUD operations
    pub async fn create_goal(&self, user_id: Uuid, request: CreateGoalRequest) -> Result<Goal> {
        let goal = sqlx::query_as!(
            Goal,
            r#"
            INSERT INTO goals (
                user_id, title, description, goal_type, goal_category,
                target_value, unit, target_date, priority, event_id, parent_goal_id,
                status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'active', $12, $12)
            RETURNING
                id, user_id, title, description,
                goal_type as "goal_type: GoalType",
                goal_category as "goal_category: GoalCategory",
                target_value, current_value, unit, target_date,
                status as "status: GoalStatus",
                priority as "priority: GoalPriority",
                event_id, parent_goal_id, created_at, updated_at
            "#,
            user_id,
            request.title,
            request.description,
            request.goal_type as GoalType,
            request.goal_category as GoalCategory,
            request.target_value,
            request.unit,
            request.target_date,
            request.priority as GoalPriority,
            request.event_id,
            request.parent_goal_id,
            Utc::now()
        )
        .fetch_one(&self.db)
        .await?;

        Ok(goal)
    }

    pub async fn get_goal_by_id(&self, goal_id: Uuid, user_id: Uuid) -> Result<Option<Goal>> {
        let goal = sqlx::query_as!(
            Goal,
            r#"
            SELECT
                id, user_id, title, description,
                goal_type as "goal_type: GoalType",
                goal_category as "goal_category: GoalCategory",
                target_value, current_value, unit, target_date,
                status as "status: GoalStatus",
                priority as "priority: GoalPriority",
                event_id, parent_goal_id, created_at, updated_at
            FROM goals
            WHERE id = $1 AND user_id = $2
            "#,
            goal_id,
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(goal)
    }

    pub async fn get_goals_by_user(&self, user_id: Uuid, status_filter: Option<String>, goal_type_filter: Option<String>, priority_filter: Option<String>, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<Goal>> {
        let limit = limit.unwrap_or(50).min(100);
        let offset = offset.unwrap_or(0);

        let mut query = "SELECT id, user_id, title, description, goal_type, goal_category, target_value, current_value, unit, target_date, status, priority, event_id, parent_goal_id, created_at, updated_at FROM goals WHERE user_id = $1".to_string();
        let mut param_count = 2;

        if status_filter.is_some() {
            query.push_str(&format!(" AND status = ${}", param_count));
            param_count += 1;
        }

        if goal_type_filter.is_some() {
            query.push_str(&format!(" AND goal_type = ${}", param_count));
            param_count += 1;
        }

        if priority_filter.is_some() {
            query.push_str(&format!(" AND priority = ${}", param_count));
            param_count += 1;
        }

        query.push_str(" ORDER BY priority DESC, target_date ASC");
        query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let mut query_builder = sqlx::query_as::<_, Goal>(&query).bind(user_id);

        if let Some(status) = status_filter {
            query_builder = query_builder.bind(status);
        }
        if let Some(goal_type) = goal_type_filter {
            query_builder = query_builder.bind(goal_type);
        }
        if let Some(priority) = priority_filter {
            query_builder = query_builder.bind(priority);
        }

        let goals = query_builder.fetch_all(&self.db).await?;
        Ok(goals)
    }

    pub async fn update_goal(&self, goal_id: Uuid, user_id: Uuid, request: UpdateGoalRequest) -> Result<Option<Goal>> {
        let goal = sqlx::query_as!(
            Goal,
            r#"
            UPDATE goals
            SET
                title = COALESCE($3, title),
                description = COALESCE($4, description),
                target_value = COALESCE($5, target_value),
                current_value = COALESCE($6, current_value),
                target_date = COALESCE($7, target_date),
                status = COALESCE($8, status),
                priority = COALESCE($9, priority),
                event_id = COALESCE($10, event_id),
                updated_at = $11
            WHERE id = $1 AND user_id = $2
            RETURNING
                id, user_id, title, description,
                goal_type as "goal_type: GoalType",
                goal_category as "goal_category: GoalCategory",
                target_value, current_value, unit, target_date,
                status as "status: GoalStatus",
                priority as "priority: GoalPriority",
                event_id, parent_goal_id, created_at, updated_at
            "#,
            goal_id,
            user_id,
            request.title,
            request.description,
            request.target_value,
            request.current_value,
            request.target_date,
            request.status.map(|s| s as GoalStatus),
            request.priority.map(|p| p as GoalPriority),
            request.event_id,
            Utc::now()
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(goal)
    }

    pub async fn delete_goal(&self, goal_id: Uuid, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM goals WHERE id = $1 AND user_id = $2",
            goal_id,
            user_id
        )
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // Goal progress tracking
    pub async fn add_progress(&self, goal_id: Uuid, user_id: Uuid, request: CreateGoalProgressRequest) -> Result<GoalProgress> {
        // First verify the goal belongs to the user
        let goal_exists = sqlx::query!(
            "SELECT id FROM goals WHERE id = $1 AND user_id = $2",
            goal_id,
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        if goal_exists.is_none() {
            return Err(anyhow::anyhow!("Goal not found or access denied"));
        }

        let date = request.date.unwrap_or_else(|| chrono::Local::now().naive_local().date());

        let progress = sqlx::query_as!(
            GoalProgress,
            r#"
            INSERT INTO goal_progress (goal_id, value, date, note, milestone_achieved, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, goal_id, value, date, note, milestone_achieved, created_at
            "#,
            goal_id,
            request.value,
            date,
            request.note,
            request.milestone_achieved,
            Utc::now()
        )
        .fetch_one(&self.db)
        .await?;

        // Update the goal's current value with the latest progress
        sqlx::query!(
            "UPDATE goals SET current_value = $1, updated_at = $2 WHERE id = $3",
            request.value,
            Utc::now(),
            goal_id
        )
        .execute(&self.db)
        .await?;

        Ok(progress)
    }

    pub async fn get_goal_progress(&self, goal_id: Uuid, user_id: Uuid) -> Result<GoalProgressSummary> {
        // Verify goal belongs to user
        let goal = self.get_goal_by_id(goal_id, user_id).await?;
        if goal.is_none() {
            return Err(anyhow::anyhow!("Goal not found or access denied"));
        }
        let goal = goal.unwrap();

        // Get recent progress entries
        let recent_entries = sqlx::query_as!(
            GoalProgress,
            r#"
            SELECT id, goal_id, value, date, note, milestone_achieved, created_at
            FROM goal_progress
            WHERE goal_id = $1
            ORDER BY date DESC, created_at DESC
            LIMIT 20
            "#,
            goal_id
        )
        .fetch_all(&self.db)
        .await?;

        // Calculate progress percentage
        let progress_percentage = if let (Some(target), Some(current)) = (goal.target_value, goal.current_value) {
            Some(((current / target) * 100.0).min(100.0))
        } else {
            None
        };

        // Calculate trend direction
        let trend_direction = self.calculate_trend_direction(&recent_entries).await;

        // Get milestones achieved
        let milestones_achieved: Vec<String> = recent_entries
            .iter()
            .filter_map(|entry| entry.milestone_achieved.clone())
            .collect();

        // Calculate success prediction based on current progress and time remaining
        let success_probability = self.calculate_success_probability(&goal, &recent_entries).await;

        // Project completion date
        let projected_completion_date = self.project_completion_date(&goal, &recent_entries).await;

        Ok(GoalProgressSummary {
            goal_id,
            progress_percentage,
            trend_direction,
            projected_completion_date,
            recent_entries,
            milestones_achieved,
            success_probability,
        })
    }

    // Goal recommendations and insights
    pub async fn generate_goal_recommendations(&self, user_id: Uuid) -> Result<Vec<GoalRecommendation>> {
        let goals = self.get_goals_by_user(user_id, Some("active".to_string()), None, None, Some(20), None).await?;
        let mut recommendations = Vec::new();

        for goal in goals {
            let progress_summary = self.get_goal_progress(goal.id, user_id).await?;

            // Generate recommendations based on progress
            if let Some(progress_pct) = progress_summary.progress_percentage {
                if progress_pct < 20.0 && goal.target_date.map_or(false, |date| (date - chrono::Local::now().naive_local().date()).num_days() < 30) {
                    recommendations.push(GoalRecommendation {
                        goal_id: goal.id,
                        recommendation_type: RecommendationType::Warning,
                        title: "Goal at Risk".to_string(),
                        description: format!("Goal '{}' has low progress with deadline approaching", goal.title),
                        priority: GoalPriority::High,
                        suggested_actions: vec![
                            "Increase training frequency".to_string(),
                            "Adjust goal target".to_string(),
                            "Extend deadline".to_string(),
                        ],
                        generated_at: Utc::now(),
                    });
                } else if progress_pct >= 100.0 {
                    recommendations.push(GoalRecommendation {
                        goal_id: goal.id,
                        recommendation_type: RecommendationType::Celebration,
                        title: "Goal Achieved!".to_string(),
                        description: format!("Congratulations! You've achieved your goal '{}'", goal.title),
                        priority: GoalPriority::Medium,
                        suggested_actions: vec![
                            "Set a new challenging goal".to_string(),
                            "Maintain your progress".to_string(),
                        ],
                        generated_at: Utc::now(),
                    });
                }
            }
        }

        Ok(recommendations)
    }

    pub async fn get_goals_summary(&self, user_id: Uuid) -> Result<serde_json::Value> {
        let all_goals = self.get_goals_by_user(user_id, None, None, None, None, None).await?;

        let total_goals = all_goals.len();
        let active_goals = all_goals.iter().filter(|g| matches!(g.status, GoalStatus::Active | GoalStatus::OnTrack)).count();
        let completed_goals = all_goals.iter().filter(|g| matches!(g.status, GoalStatus::Completed)).count();

        let completion_rate = if total_goals > 0 {
            (completed_goals as f64 / total_goals as f64) * 100.0
        } else {
            0.0
        };

        // Get upcoming deadlines
        let upcoming_deadlines: Vec<serde_json::Value> = all_goals
            .iter()
            .filter(|g| g.target_date.is_some() && matches!(g.status, GoalStatus::Active | GoalStatus::OnTrack))
            .filter_map(|g| {
                g.target_date.map(|date| {
                    let days_remaining = (date - chrono::Local::now().naive_local().date()).num_days();
                    if days_remaining <= 30 && days_remaining >= 0 {
                        Some(serde_json::json!({
                            "goal_id": g.id,
                            "title": g.title,
                            "days_remaining": days_remaining
                        }))
                    } else {
                        None
                    }
                }).flatten()
            })
            .collect();

        // Get recent achievements
        let recent_achievements: Vec<serde_json::Value> = all_goals
            .iter()
            .filter(|g| matches!(g.status, GoalStatus::Completed))
            .take(5)
            .map(|g| serde_json::json!({
                "goal_id": g.id,
                "title": g.title,
                "completed_at": g.updated_at
            }))
            .collect();

        Ok(serde_json::json!({
            "total_goals": total_goals,
            "active_goals": active_goals,
            "completed_goals": completed_goals,
            "completion_rate": completion_rate,
            "upcoming_deadlines": upcoming_deadlines,
            "recent_achievements": recent_achievements,
            "success": true
        }))
    }

    // Private helper methods
    async fn calculate_trend_direction(&self, progress_entries: &[GoalProgress]) -> TrendDirection {
        if progress_entries.len() < 2 {
            return TrendDirection::Insufficient;
        }

        let recent_values: Vec<f64> = progress_entries.iter().take(5).map(|p| p.value).collect();
        let earlier_values: Vec<f64> = progress_entries.iter().skip(5).take(5).map(|p| p.value).collect();

        if earlier_values.is_empty() {
            return TrendDirection::Insufficient;
        }

        let recent_avg = recent_values.iter().sum::<f64>() / recent_values.len() as f64;
        let earlier_avg = earlier_values.iter().sum::<f64>() / earlier_values.len() as f64;

        let change_pct = ((recent_avg - earlier_avg) / earlier_avg) * 100.0;

        if change_pct > 5.0 {
            TrendDirection::Improving
        } else if change_pct < -5.0 {
            TrendDirection::Declining
        } else {
            TrendDirection::Stable
        }
    }

    async fn calculate_success_probability(&self, goal: &Goal, progress_entries: &[GoalProgress]) -> Option<f64> {
        if let (Some(target), Some(current), Some(target_date)) = (goal.target_value, goal.current_value, goal.target_date) {
            let days_remaining = (target_date - chrono::Local::now().naive_local().date()).num_days() as f64;

            if days_remaining <= 0.0 {
                return Some(if current >= target { 100.0 } else { 0.0 });
            }

            let progress_rate = if progress_entries.len() >= 2 {
                let days_of_data = (progress_entries[0].date - progress_entries[progress_entries.len() - 1].date).num_days() as f64;
                let value_change = progress_entries[0].value - progress_entries[progress_entries.len() - 1].value;
                if days_of_data > 0.0 { value_change / days_of_data } else { 0.0 }
            } else {
                0.0
            };

            let projected_final_value = current + (progress_rate * days_remaining);
            let success_probability = ((projected_final_value / target) * 100.0).min(100.0).max(0.0);

            Some(success_probability)
        } else {
            None
        }
    }

    async fn project_completion_date(&self, goal: &Goal, progress_entries: &[GoalProgress]) -> Option<NaiveDate> {
        if let (Some(target), Some(current)) = (goal.target_value, goal.current_value) {
            if current >= target {
                return Some(chrono::Local::now().naive_local().date());
            }

            if progress_entries.len() >= 2 {
                let days_of_data = (progress_entries[0].date - progress_entries[progress_entries.len() - 1].date).num_days() as f64;
                let value_change = progress_entries[0].value - progress_entries[progress_entries.len() - 1].value;

                if days_of_data > 0.0 && value_change > 0.0 {
                    let progress_rate = value_change / days_of_data;
                    let remaining_value = target - current;
                    let estimated_days = remaining_value / progress_rate;

                    return Some(chrono::Local::now().naive_local().date() + chrono::Duration::days(estimated_days as i64));
                }
            }
        }

        None
    }
}