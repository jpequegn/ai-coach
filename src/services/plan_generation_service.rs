use anyhow::Result;
use chrono::{NaiveDate, Utc, Duration, Weekday};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    GeneratedPlan, PlanGenerationRequest, UserTrainingPreferences, TrainingConstraints,
    PlanAdaptation, PlanAlternative, CoachingInsight, CreateEventPlanRequest,
    PlanWeekStructure, WorkoutDay, PlanType, AdaptationType, InsightType,
    WorkoutType, IntensityZone, Equipment, PowerTargets, HeartRateTargets,
    ImportanceLevel, Goal, Event, GoalType, GoalCategory, EventPriority,
    ExperienceLevel, IntensityPreference
};

use super::goal_service::GoalService;
use super::event_service::EventService;

#[derive(Clone)]
pub struct PlanGenerationService {
    db: PgPool,
    goal_service: GoalService,
    event_service: EventService,
}

impl PlanGenerationService {
    pub fn new(db: PgPool) -> Self {
        let goal_service = GoalService::new(db.clone());
        let event_service = EventService::new(db.clone());

        Self {
            db,
            goal_service,
            event_service,
        }
    }

    // Plan generation based on goals and events
    pub async fn generate_plan(&self, user_id: Uuid, request: PlanGenerationRequest) -> Result<GeneratedPlan> {
        // Get user preferences and constraints
        let preferences = self.get_user_preferences(user_id).await?;
        let constraints = self.get_user_constraints(user_id).await?;

        // Analyze goals and events
        let goals = self.analyze_goals(&request.goals, user_id).await?;
        let events = self.analyze_events(&request.events, user_id).await?;

        // Determine plan type and duration
        let plan_type = self.determine_plan_type(&goals, &events).await?;
        let plan_duration_weeks = request.plan_duration_weeks.unwrap_or_else(|| self.calculate_optimal_duration(&goals, &events));

        // Generate plan structure
        let plan_structure = self.generate_plan_structure(
            user_id,
            &goals,
            &events,
            &preferences,
            &constraints,
            plan_duration_weeks,
            request.start_date,
        ).await?;

        // Calculate confidence and success prediction
        let confidence_score = self.calculate_confidence_score(&goals, &events, &preferences, &constraints).await?;
        let success_prediction = self.calculate_success_prediction(&goals, &plan_structure).await?;

        // Store the generated plan
        let plan_name = self.generate_plan_name(&goals, &events).await?;
        let generation_parameters = serde_json::to_value(&request)?;

        let generated_plan = sqlx::query_as!(
            GeneratedPlan,
            r#"
            INSERT INTO generated_plans (
                user_id, goal_id, event_id, plan_name, plan_type, start_date, end_date,
                total_weeks, plan_structure, generation_parameters, adaptation_history,
                status, confidence_score, success_prediction, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'draft', $12, $13, $14, $14)
            RETURNING
                id, user_id, goal_id, event_id, plan_name,
                plan_type as "plan_type: PlanType",
                start_date, end_date, total_weeks, plan_structure,
                generation_parameters, adaptation_history, status,
                confidence_score, success_prediction, created_at, updated_at
            "#,
            user_id,
            goals.get(0).map(|g| g.id),
            events.get(0).map(|e| e.id),
            plan_name,
            plan_type as PlanType,
            request.start_date,
            request.start_date + Duration::weeks(plan_duration_weeks as i64),
            plan_duration_weeks,
            serde_json::to_value(&plan_structure)?,
            generation_parameters,
            serde_json::json!([]),
            confidence_score,
            success_prediction,
            Utc::now()
        )
        .fetch_one(&self.db)
        .await?;

        Ok(generated_plan)
    }

    // Plan adaptation based on progress
    pub async fn adapt_plan(&self, plan_id: Uuid, user_id: Uuid, trigger_reason: String, adaptation_type: AdaptationType) -> Result<GeneratedPlan> {
        // Get the current plan
        let current_plan = self.get_plan_by_id(plan_id, user_id).await?;
        if current_plan.is_none() {
            return Err(anyhow::anyhow!("Plan not found or access denied"));
        }
        let mut plan = current_plan.unwrap();

        // Generate adaptation changes based on type
        let changes = self.generate_adaptation_changes(&plan, &adaptation_type).await?;

        // Apply changes to plan structure
        let mut plan_structure: Vec<PlanWeekStructure> = serde_json::from_value(plan.plan_structure)?;
        self.apply_adaptation_changes(&mut plan_structure, &changes, &adaptation_type).await?;

        // Store adaptation record
        let adaptation = sqlx::query_as!(
            PlanAdaptation,
            r#"
            INSERT INTO plan_adaptations (
                plan_id, adaptation_type, trigger_reason, changes_made, applied_date, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $5)
            RETURNING
                id, plan_id,
                adaptation_type as "adaptation_type: AdaptationType",
                trigger_reason, changes_made, effectiveness_score, applied_date, created_at
            "#,
            plan_id,
            adaptation_type as AdaptationType,
            trigger_reason,
            changes,
            Utc::now()
        )
        .fetch_one(&self.db)
        .await?;

        // Update plan with new structure
        let updated_plan = sqlx::query_as!(
            GeneratedPlan,
            r#"
            UPDATE generated_plans
            SET
                plan_structure = $2,
                adaptation_history = adaptation_history || $3::jsonb,
                updated_at = $4
            WHERE id = $1 AND user_id = $5
            RETURNING
                id, user_id, goal_id, event_id, plan_name,
                plan_type as "plan_type: PlanType",
                start_date, end_date, total_weeks, plan_structure,
                generation_parameters, adaptation_history, status,
                confidence_score, success_prediction, created_at, updated_at
            "#,
            plan_id,
            serde_json::to_value(&plan_structure)?,
            serde_json::json!([adaptation]),
            Utc::now(),
            user_id
        )
        .fetch_one(&self.db)
        .await?;

        Ok(updated_plan)
    }

    // Generate alternative plans
    pub async fn generate_alternatives(&self, original_plan_id: Uuid, user_id: Uuid) -> Result<Vec<PlanAlternative>> {
        let original_plan = self.get_plan_by_id(original_plan_id, user_id).await?;
        if original_plan.is_none() {
            return Err(anyhow::anyhow!("Original plan not found"));
        }
        let plan = original_plan.unwrap();

        let mut alternatives = Vec::new();

        // Generate high-volume alternative
        alternatives.push(self.generate_volume_alternative(&plan).await?);

        // Generate high-intensity alternative
        alternatives.push(self.generate_intensity_alternative(&plan).await?);

        // Generate time-efficient alternative
        alternatives.push(self.generate_time_efficient_alternative(&plan).await?);

        // Store alternatives in database
        for alternative in &alternatives {
            sqlx::query!(
                r#"
                INSERT INTO plan_alternatives (
                    original_plan_id, alternative_name, alternative_description,
                    differences, estimated_effectiveness, suitability_score, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
                original_plan_id,
                alternative.alternative_name,
                alternative.alternative_description,
                alternative.differences,
                alternative.estimated_effectiveness,
                alternative.suitability_score,
                Utc::now()
            )
            .execute(&self.db)
            .await?;
        }

        Ok(alternatives)
    }

    // Generate coaching insights
    pub async fn generate_coaching_insights(&self, plan_id: Uuid, user_id: Uuid) -> Result<Vec<CoachingInsight>> {
        let plan = self.get_plan_by_id(plan_id, user_id).await?;
        if plan.is_none() {
            return Err(anyhow::anyhow!("Plan not found"));
        }
        let plan = plan.unwrap();

        let mut insights = Vec::new();

        // Analyze plan structure for insights
        let plan_structure: Vec<PlanWeekStructure> = serde_json::from_value(plan.plan_structure)?;

        // Check for volume progression
        insights.extend(self.analyze_volume_progression(&plan_structure).await?);

        // Check for intensity distribution
        insights.extend(self.analyze_intensity_distribution(&plan_structure).await?);

        // Check for recovery balance
        insights.extend(self.analyze_recovery_balance(&plan_structure).await?);

        // Store insights in database
        for insight in &insights {
            sqlx::query!(
                r#"
                INSERT INTO coaching_insights (
                    plan_id, insight_type, title, description, recommended_action,
                    importance, generated_at, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
                "#,
                plan_id,
                insight.insight_type as InsightType,
                insight.title,
                insight.description,
                insight.recommended_action,
                insight.importance as ImportanceLevel,
                Utc::now()
            )
            .execute(&self.db)
            .await?;
        }

        Ok(insights)
    }

    // User preferences and constraints management
    pub async fn update_user_preferences(&self, user_id: Uuid, preferences: UserTrainingPreferences) -> Result<UserTrainingPreferences> {
        let updated_preferences = sqlx::query!(
            r#"
            INSERT INTO user_training_preferences (
                user_id, available_days_per_week, preferred_workout_duration,
                max_workout_duration, intensity_preference, preferred_training_times,
                equipment_available, training_location, experience_level,
                injury_history, recovery_needs, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12)
            ON CONFLICT (user_id) DO UPDATE SET
                available_days_per_week = $2,
                preferred_workout_duration = $3,
                max_workout_duration = $4,
                intensity_preference = $5,
                preferred_training_times = $6,
                equipment_available = $7,
                training_location = $8,
                experience_level = $9,
                injury_history = $10,
                recovery_needs = $11,
                updated_at = $12
            "#,
            user_id,
            preferences.available_days_per_week,
            preferences.preferred_workout_duration,
            preferences.max_workout_duration,
            preferences.intensity_preference as IntensityPreference,
            serde_json::to_value(&preferences.preferred_training_times)?,
            serde_json::to_value(&preferences.equipment_available)?,
            preferences.training_location as _,
            preferences.experience_level as ExperienceLevel,
            serde_json::to_value(&preferences.injury_history)?,
            preferences.recovery_needs as _,
            Utc::now()
        )
        .execute(&self.db)
        .await?;

        Ok(preferences)
    }

    pub async fn update_user_constraints(&self, user_id: Uuid, constraints: TrainingConstraints) -> Result<TrainingConstraints> {
        sqlx::query!(
            r#"
            INSERT INTO training_constraints (
                user_id, max_weekly_hours, min_weekly_hours, max_consecutive_hard_days,
                required_rest_days, travel_dates, blackout_dates, priority_dates,
                equipment_limitations, health_considerations, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
            ON CONFLICT (user_id) DO UPDATE SET
                max_weekly_hours = $2,
                min_weekly_hours = $3,
                max_consecutive_hard_days = $4,
                required_rest_days = $5,
                travel_dates = $6,
                blackout_dates = $7,
                priority_dates = $8,
                equipment_limitations = $9,
                health_considerations = $10,
                updated_at = $11
            "#,
            user_id,
            constraints.max_weekly_hours,
            constraints.min_weekly_hours,
            constraints.max_consecutive_hard_days,
            constraints.required_rest_days,
            serde_json::to_value(&constraints.travel_dates)?,
            serde_json::to_value(&constraints.blackout_dates)?,
            serde_json::to_value(&constraints.priority_dates)?,
            serde_json::to_value(&constraints.equipment_limitations)?,
            serde_json::to_value(&constraints.health_considerations)?,
            Utc::now()
        )
        .execute(&self.db)
        .await?;

        Ok(constraints)
    }

    // Private helper methods
    pub async fn get_plan_by_id(&self, plan_id: Uuid, user_id: Uuid) -> Result<Option<GeneratedPlan>> {
        let plan = sqlx::query_as!(
            GeneratedPlan,
            r#"
            SELECT
                id, user_id, goal_id, event_id, plan_name,
                plan_type as "plan_type: PlanType",
                start_date, end_date, total_weeks, plan_structure,
                generation_parameters, adaptation_history, status,
                confidence_score, success_prediction, created_at, updated_at
            FROM generated_plans
            WHERE id = $1 AND user_id = $2
            "#,
            plan_id,
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(plan)
    }

    async fn get_user_preferences(&self, user_id: Uuid) -> Result<UserTrainingPreferences> {
        // Try to get existing preferences, otherwise return defaults
        let preferences = sqlx::query!(
            r#"
            SELECT
                available_days_per_week, preferred_workout_duration, max_workout_duration,
                intensity_preference, preferred_training_times, equipment_available,
                training_location, experience_level, injury_history, recovery_needs
            FROM user_training_preferences
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        if let Some(prefs) = preferences {
            Ok(UserTrainingPreferences {
                available_days_per_week: prefs.available_days_per_week,
                preferred_workout_duration: prefs.preferred_workout_duration,
                max_workout_duration: prefs.max_workout_duration,
                intensity_preference: serde_json::from_str(&prefs.intensity_preference.unwrap_or_else(|| "moderate_intensity_moderate_volume".to_string()))?,
                preferred_training_times: serde_json::from_value(prefs.preferred_training_times)?,
                equipment_available: serde_json::from_value(prefs.equipment_available)?,
                training_location: serde_json::from_str(&prefs.training_location.unwrap_or_else(|| "mixed".to_string()))?,
                experience_level: serde_json::from_str(&prefs.experience_level.unwrap_or_else(|| "intermediate".to_string()))?,
                injury_history: serde_json::from_value(prefs.injury_history)?,
                recovery_needs: serde_json::from_str(&prefs.recovery_needs.unwrap_or_else(|| "normal".to_string()))?,
            })
        } else {
            // Return default preferences
            Ok(UserTrainingPreferences {
                available_days_per_week: 5,
                preferred_workout_duration: 60,
                max_workout_duration: 120,
                intensity_preference: IntensityPreference::ModerateIntensityModerateVolume,
                preferred_training_times: vec!["morning".to_string()],
                equipment_available: vec![Equipment::Road, Equipment::HeartRateMonitor],
                training_location: crate::models::TrainingLocation::Mixed,
                experience_level: ExperienceLevel::Intermediate,
                injury_history: vec![],
                recovery_needs: crate::models::RecoveryLevel::Normal,
            })
        }
    }

    async fn get_user_constraints(&self, user_id: Uuid) -> Result<TrainingConstraints> {
        let constraints = sqlx::query!(
            r#"
            SELECT
                max_weekly_hours, min_weekly_hours, max_consecutive_hard_days,
                required_rest_days, travel_dates, blackout_dates, priority_dates,
                equipment_limitations, health_considerations
            FROM training_constraints
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        if let Some(cons) = constraints {
            Ok(TrainingConstraints {
                max_weekly_hours: cons.max_weekly_hours,
                min_weekly_hours: cons.min_weekly_hours,
                max_consecutive_hard_days: cons.max_consecutive_hard_days,
                required_rest_days: cons.required_rest_days,
                travel_dates: serde_json::from_value(cons.travel_dates)?,
                blackout_dates: serde_json::from_value(cons.blackout_dates)?,
                priority_dates: serde_json::from_value(cons.priority_dates)?,
                equipment_limitations: serde_json::from_value(cons.equipment_limitations)?,
                health_considerations: serde_json::from_value(cons.health_considerations)?,
            })
        } else {
            // Return default constraints
            Ok(TrainingConstraints {
                max_weekly_hours: 10.0,
                min_weekly_hours: 3.0,
                max_consecutive_hard_days: 2,
                required_rest_days: 1,
                travel_dates: vec![],
                blackout_dates: vec![],
                priority_dates: vec![],
                equipment_limitations: vec![],
                health_considerations: vec![],
            })
        }
    }

    async fn analyze_goals(&self, goal_ids: &[Uuid], user_id: Uuid) -> Result<Vec<Goal>> {
        let mut goals = Vec::new();
        for goal_id in goal_ids {
            if let Some(goal) = self.goal_service.get_goal_by_id(*goal_id, user_id).await? {
                goals.push(goal);
            }
        }
        Ok(goals)
    }

    async fn analyze_events(&self, event_ids: &[Uuid], user_id: Uuid) -> Result<Vec<Event>> {
        let mut events = Vec::new();
        for event_id in event_ids {
            if let Some(event) = self.event_service.get_event_by_id(*event_id, user_id).await? {
                events.push(event);
            }
        }
        Ok(events)
    }

    async fn determine_plan_type(&self, goals: &[Goal], events: &[Event]) -> Result<PlanType> {
        if !events.is_empty() {
            Ok(PlanType::EventBased)
        } else if !goals.is_empty() {
            Ok(PlanType::GoalBased)
        } else {
            Ok(PlanType::Progressive)
        }
    }

    fn calculate_optimal_duration(&self, goals: &[Goal], events: &[Event]) -> i32 {
        // Calculate based on goals and events
        let mut max_weeks = 12; // Default

        for goal in goals {
            if let Some(target_date) = goal.target_date {
                let weeks_to_goal = (target_date - chrono::Local::now().naive_local().date()).num_weeks();
                max_weeks = max_weeks.max(weeks_to_goal as i32);
            }
        }

        for event in events {
            let weeks_to_event = (event.event_date - chrono::Local::now().naive_local().date()).num_weeks();
            max_weeks = max_weeks.max(weeks_to_event as i32);
        }

        max_weeks.min(52).max(4) // Between 4 and 52 weeks
    }

    async fn generate_plan_structure(
        &self,
        _user_id: Uuid,
        _goals: &[Goal],
        _events: &[Event],
        preferences: &UserTrainingPreferences,
        constraints: &TrainingConstraints,
        duration_weeks: i32,
        start_date: NaiveDate,
    ) -> Result<Vec<PlanWeekStructure>> {
        let mut weeks = Vec::new();

        for week_num in 1..=duration_weeks {
            let week_start = start_date + Duration::weeks((week_num - 1) as i64);

            // Generate workout days based on user preferences
            let workout_days = self.generate_workout_days(week_num, preferences, constraints).await?;

            // Calculate rest days
            let workout_day_nums: Vec<i32> = workout_days.iter().map(|w| w.day_of_week).collect();
            let rest_days: Vec<i32> = (1..=7).filter(|day| !workout_day_nums.contains(day)).collect();

            weeks.push(PlanWeekStructure {
                week_number: week_num,
                phase_name: self.determine_phase_name(week_num, duration_weeks),
                weekly_volume: self.calculate_weekly_volume(week_num, duration_weeks, preferences),
                weekly_intensity: self.calculate_weekly_intensity(week_num, duration_weeks),
                workout_days,
                rest_days,
                week_goals: self.generate_week_goals(week_num, duration_weeks),
                key_sessions: self.generate_key_sessions(week_num, duration_weeks),
            });
        }

        Ok(weeks)
    }

    async fn generate_workout_days(&self, week_num: i32, preferences: &UserTrainingPreferences, _constraints: &TrainingConstraints) -> Result<Vec<WorkoutDay>> {
        let mut workout_days = Vec::new();
        let days_per_week = preferences.available_days_per_week;

        // Distribute workouts across the week
        let workout_days_pattern = match days_per_week {
            3 => vec![1, 3, 5], // Mon, Wed, Fri
            4 => vec![1, 3, 5, 7], // Mon, Wed, Fri, Sun
            5 => vec![1, 2, 4, 5, 7], // Mon, Tue, Thu, Fri, Sun
            6 => vec![1, 2, 3, 5, 6, 7], // Mon, Tue, Wed, Fri, Sat, Sun
            _ => vec![1, 2, 3, 4, 5, 6, 7], // Daily
        };

        for (i, &day) in workout_days_pattern.iter().enumerate() {
            let workout_type = self.determine_workout_type(i, days_per_week, week_num);
            let intensity_zone = self.determine_intensity_zone(&workout_type);

            workout_days.push(WorkoutDay {
                day_of_week: day,
                workout_type: workout_type.clone(),
                duration_minutes: self.calculate_workout_duration(&workout_type, preferences),
                intensity_zone,
                workout_description: self.generate_workout_description(&workout_type),
                power_targets: self.generate_power_targets(&workout_type),
                heart_rate_targets: self.generate_heart_rate_targets(&workout_type),
                pace_targets: None, // Can be added based on sport
                equipment_needed: self.determine_equipment_needed(&workout_type),
                notes: None,
            });
        }

        Ok(workout_days)
    }

    // Additional helper methods would continue here...
    // For brevity, I'll include a few key ones:

    fn determine_phase_name(&self, week_num: i32, total_weeks: i32) -> String {
        let phase_ratio = week_num as f64 / total_weeks as f64;

        match phase_ratio {
            ratio if ratio <= 0.3 => "Base Building".to_string(),
            ratio if ratio <= 0.6 => "Build".to_string(),
            ratio if ratio <= 0.85 => "Peak".to_string(),
            _ => "Taper".to_string(),
        }
    }

    fn calculate_weekly_volume(&self, week_num: i32, total_weeks: i32, preferences: &UserTrainingPreferences) -> f64 {
        let base_volume = preferences.available_days_per_week as f64 * (preferences.preferred_workout_duration as f64 / 60.0);
        let phase_ratio = week_num as f64 / total_weeks as f64;

        // Volume progression: start lower, peak in middle, taper at end
        let volume_multiplier = if phase_ratio <= 0.3 {
            0.7 + (phase_ratio / 0.3) * 0.3 // 0.7 to 1.0
        } else if phase_ratio <= 0.85 {
            1.0 + ((phase_ratio - 0.3) / 0.55) * 0.3 // 1.0 to 1.3
        } else {
            1.3 - ((phase_ratio - 0.85) / 0.15) * 0.6 // 1.3 to 0.7
        };

        base_volume * volume_multiplier
    }

    fn calculate_weekly_intensity(&self, week_num: i32, total_weeks: i32) -> f64 {
        let phase_ratio = week_num as f64 / total_weeks as f64;

        // Intensity progression: gradual increase, peak before taper
        match phase_ratio {
            ratio if ratio <= 0.3 => 0.6, // Base phase - lower intensity
            ratio if ratio <= 0.6 => 0.6 + ((ratio - 0.3) / 0.3) * 0.25, // Build phase
            ratio if ratio <= 0.85 => 0.85, // Peak phase - highest intensity
            _ => 0.85 - ((ratio - 0.85) / 0.15) * 0.25, // Taper
        }
    }

    fn determine_workout_type(&self, workout_index: usize, total_workouts: i32, week_num: i32) -> WorkoutType {
        let phase_ratio = week_num as f64 / 20.0; // Assume 20 week cycle

        match (workout_index, total_workouts) {
            (0, _) => if week_num % 4 == 0 { WorkoutType::Test } else { WorkoutType::Endurance },
            (1, 3) => if phase_ratio > 0.6 { WorkoutType::Threshold } else { WorkoutType::Tempo },
            (1, _) => WorkoutType::Recovery,
            (2, 3) => WorkoutType::Recovery,
            (2, _) => if phase_ratio > 0.3 { WorkoutType::SweetSpot } else { WorkoutType::Endurance },
            (3, _) => if phase_ratio > 0.6 { WorkoutType::Vo2Max } else { WorkoutType::Tempo },
            _ => WorkoutType::Endurance,
        }
    }

    fn determine_intensity_zone(&self, workout_type: &WorkoutType) -> IntensityZone {
        match workout_type {
            WorkoutType::Recovery => IntensityZone::Zone1,
            WorkoutType::Endurance => IntensityZone::Zone2,
            WorkoutType::Tempo => IntensityZone::Zone3,
            WorkoutType::SweetSpot => IntensityZone::Zone3,
            WorkoutType::Threshold => IntensityZone::Zone4,
            WorkoutType::Vo2Max => IntensityZone::Zone5,
            WorkoutType::Neuromuscular => IntensityZone::Zone6,
            _ => IntensityZone::Zone2,
        }
    }

    fn calculate_workout_duration(&self, workout_type: &WorkoutType, preferences: &UserTrainingPreferences) -> i32 {
        let base_duration = preferences.preferred_workout_duration;

        match workout_type {
            WorkoutType::Recovery => (base_duration as f64 * 0.6) as i32,
            WorkoutType::Endurance => (base_duration as f64 * 1.2) as i32,
            WorkoutType::Vo2Max => (base_duration as f64 * 0.8) as i32,
            WorkoutType::Test => (base_duration as f64 * 1.5) as i32,
            _ => base_duration,
        }
    }

    fn generate_workout_description(&self, workout_type: &WorkoutType) -> String {
        match workout_type {
            WorkoutType::Recovery => "Easy recovery ride, focus on spinning and mobility".to_string(),
            WorkoutType::Endurance => "Steady endurance ride, maintain conversational pace".to_string(),
            WorkoutType::Tempo => "Tempo intervals with moderate effort".to_string(),
            WorkoutType::SweetSpot => "Sweet spot intervals at upper Zone 3".to_string(),
            WorkoutType::Threshold => "Lactate threshold intervals".to_string(),
            WorkoutType::Vo2Max => "VO2 max intervals at high intensity".to_string(),
            WorkoutType::Test => "Fitness test - FTP or time trial".to_string(),
            _ => "Training session".to_string(),
        }
    }

    fn generate_power_targets(&self, workout_type: &WorkoutType) -> Option<PowerTargets> {
        match workout_type {
            WorkoutType::Recovery => Some(PowerTargets {
                ftp_percentage_low: 45.0,
                ftp_percentage_high: 55.0,
                average_watts: None,
                normalized_power: None,
            }),
            WorkoutType::Endurance => Some(PowerTargets {
                ftp_percentage_low: 55.0,
                ftp_percentage_high: 75.0,
                average_watts: None,
                normalized_power: None,
            }),
            WorkoutType::Tempo => Some(PowerTargets {
                ftp_percentage_low: 75.0,
                ftp_percentage_high: 88.0,
                average_watts: None,
                normalized_power: None,
            }),
            WorkoutType::SweetSpot => Some(PowerTargets {
                ftp_percentage_low: 88.0,
                ftp_percentage_high: 95.0,
                average_watts: None,
                normalized_power: None,
            }),
            WorkoutType::Threshold => Some(PowerTargets {
                ftp_percentage_low: 95.0,
                ftp_percentage_high: 105.0,
                average_watts: None,
                normalized_power: None,
            }),
            WorkoutType::Vo2Max => Some(PowerTargets {
                ftp_percentage_low: 105.0,
                ftp_percentage_high: 120.0,
                average_watts: None,
                normalized_power: None,
            }),
            _ => None,
        }
    }

    fn generate_heart_rate_targets(&self, workout_type: &WorkoutType) -> Option<HeartRateTargets> {
        match workout_type {
            WorkoutType::Recovery => Some(HeartRateTargets {
                hr_percentage_low: 60.0,
                hr_percentage_high: 68.0,
                average_hr: None,
            }),
            WorkoutType::Endurance => Some(HeartRateTargets {
                hr_percentage_low: 69.0,
                hr_percentage_high: 83.0,
                average_hr: None,
            }),
            WorkoutType::Tempo => Some(HeartRateTargets {
                hr_percentage_low: 84.0,
                hr_percentage_high: 94.0,
                average_hr: None,
            }),
            WorkoutType::Threshold => Some(HeartRateTargets {
                hr_percentage_low: 95.0,
                hr_percentage_high: 105.0,
                average_hr: None,
            }),
            _ => None,
        }
    }

    fn determine_equipment_needed(&self, workout_type: &WorkoutType) -> Vec<Equipment> {
        match workout_type {
            WorkoutType::Test => vec![Equipment::PowerMeter, Equipment::HeartRateMonitor],
            WorkoutType::Vo2Max | WorkoutType::Threshold => vec![Equipment::PowerMeter],
            _ => vec![Equipment::HeartRateMonitor],
        }
    }

    fn generate_week_goals(&self, week_num: i32, total_weeks: i32) -> Vec<String> {
        let phase_ratio = week_num as f64 / total_weeks as f64;

        if phase_ratio <= 0.3 {
            vec!["Build aerobic base".to_string(), "Establish routine".to_string()]
        } else if phase_ratio <= 0.6 {
            vec!["Increase training load".to_string(), "Build threshold power".to_string()]
        } else if phase_ratio <= 0.85 {
            vec!["Peak performance".to_string(), "Race-specific training".to_string()]
        } else {
            vec!["Maintain fitness".to_string(), "Recover and prepare".to_string()]
        }
    }

    fn generate_key_sessions(&self, week_num: i32, total_weeks: i32) -> Vec<String> {
        let phase_ratio = week_num as f64 / total_weeks as f64;

        if phase_ratio <= 0.3 {
            vec!["Long endurance ride".to_string()]
        } else if phase_ratio <= 0.6 {
            vec!["Threshold intervals".to_string(), "Sweet spot work".to_string()]
        } else if phase_ratio <= 0.85 {
            vec!["VO2 max intervals".to_string(), "Race simulation".to_string()]
        } else {
            vec!["Easy spin".to_string(), "Short opener".to_string()]
        }
    }

    async fn calculate_confidence_score(&self, _goals: &[Goal], _events: &[Event], _preferences: &UserTrainingPreferences, _constraints: &TrainingConstraints) -> Result<f64> {
        // Simplified confidence calculation
        Ok(85.0) // Would implement more sophisticated calculation
    }

    async fn calculate_success_prediction(&self, _goals: &[Goal], _plan_structure: &[PlanWeekStructure]) -> Result<f64> {
        // Simplified success prediction
        Ok(78.0) // Would implement ML-based prediction
    }

    async fn generate_plan_name(&self, goals: &[Goal], events: &[Event]) -> Result<String> {
        if let Some(event) = events.first() {
            Ok(format!("Training Plan for {}", event.name))
        } else if let Some(goal) = goals.first() {
            Ok(format!("Plan for {}", goal.title))
        } else {
            Ok("Progressive Training Plan".to_string())
        }
    }

    // Stub implementations for adaptation methods
    async fn generate_adaptation_changes(&self, _plan: &GeneratedPlan, _adaptation_type: &AdaptationType) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"change": "sample adaptation"}))
    }

    async fn apply_adaptation_changes(&self, _plan_structure: &mut Vec<PlanWeekStructure>, _changes: &serde_json::Value, _adaptation_type: &AdaptationType) -> Result<()> {
        Ok(())
    }

    // Stub implementations for alternative generation
    async fn generate_volume_alternative(&self, _plan: &GeneratedPlan) -> Result<PlanAlternative> {
        Ok(PlanAlternative {
            id: Uuid::new_v4(),
            original_plan_id: _plan.id,
            alternative_name: "High Volume Alternative".to_string(),
            alternative_description: "Increased training volume with longer sessions".to_string(),
            differences: serde_json::json!({"volume_increase": 25}),
            estimated_effectiveness: 88.0,
            suitability_score: 75.0,
            created_at: Utc::now(),
        })
    }

    async fn generate_intensity_alternative(&self, _plan: &GeneratedPlan) -> Result<PlanAlternative> {
        Ok(PlanAlternative {
            id: Uuid::new_v4(),
            original_plan_id: _plan.id,
            alternative_name: "High Intensity Alternative".to_string(),
            alternative_description: "Higher intensity with reduced volume".to_string(),
            differences: serde_json::json!({"intensity_increase": 15, "volume_decrease": 10}),
            estimated_effectiveness: 82.0,
            suitability_score: 80.0,
            created_at: Utc::now(),
        })
    }

    async fn generate_time_efficient_alternative(&self, _plan: &GeneratedPlan) -> Result<PlanAlternative> {
        Ok(PlanAlternative {
            id: Uuid::new_v4(),
            original_plan_id: _plan.id,
            alternative_name: "Time Efficient Alternative".to_string(),
            alternative_description: "Shorter, high-intensity sessions for time-constrained schedules".to_string(),
            differences: serde_json::json!({"duration_decrease": 30, "intensity_increase": 20}),
            estimated_effectiveness: 75.0,
            suitability_score: 90.0,
            created_at: Utc::now(),
        })
    }

    // Stub implementations for coaching insights
    async fn analyze_volume_progression(&self, _plan_structure: &[PlanWeekStructure]) -> Result<Vec<CoachingInsight>> {
        Ok(vec![])
    }

    async fn analyze_intensity_distribution(&self, _plan_structure: &[PlanWeekStructure]) -> Result<Vec<CoachingInsight>> {
        Ok(vec![])
    }

    async fn analyze_recovery_balance(&self, _plan_structure: &[PlanWeekStructure]) -> Result<Vec<CoachingInsight>> {
        Ok(vec![])
    }
}