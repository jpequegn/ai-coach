use anyhow::Result;
use chrono::{NaiveDate, Utc, Duration};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    Event, EventPlan, CreateEventRequest, UpdateEventRequest, CreateEventPlanRequest,
    EventCalendar, EventConflict, EventRecommendation, EventStatus, EventPriority,
    EventType, Sport, PhaseType, TrainingPhase, IntensityDistribution, ConflictType,
    ConflictSeverity, EventRecommendationType
};

#[derive(Clone)]
pub struct EventService {
    db: PgPool,
}

impl EventService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    // Event CRUD operations
    pub async fn create_event(&self, user_id: Uuid, request: CreateEventRequest) -> Result<Event> {
        let event = sqlx::query_as!(
            Event,
            r#"
            INSERT INTO events (
                user_id, name, description, event_type, sport, event_date, event_time,
                location, distance, distance_unit, elevation_gain, expected_duration,
                registration_deadline, cost, website_url, notes, priority,
                status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, 'planned', $18, $18)
            RETURNING
                id, user_id, name, description,
                event_type as "event_type: EventType",
                sport as "sport: Sport",
                event_date, event_time, location, distance, distance_unit,
                elevation_gain, expected_duration, registration_deadline,
                cost, website_url, notes,
                status as "status: EventStatus",
                priority as "priority: EventPriority",
                created_at, updated_at
            "#,
            user_id,
            request.name,
            request.description,
            request.event_type as EventType,
            request.sport as Sport,
            request.event_date,
            request.event_time,
            request.location,
            request.distance,
            request.distance_unit,
            request.elevation_gain,
            request.expected_duration,
            request.registration_deadline,
            request.cost,
            request.website_url,
            request.notes,
            request.priority as EventPriority,
            Utc::now()
        )
        .fetch_one(&self.db)
        .await?;

        Ok(event)
    }

    pub async fn get_event_by_id(&self, event_id: Uuid, user_id: Uuid) -> Result<Option<Event>> {
        let event = sqlx::query_as!(
            Event,
            r#"
            SELECT
                id, user_id, name, description,
                event_type as "event_type: EventType",
                sport as "sport: Sport",
                event_date, event_time, location, distance, distance_unit,
                elevation_gain, expected_duration, registration_deadline,
                cost, website_url, notes,
                status as "status: EventStatus",
                priority as "priority: EventPriority",
                created_at, updated_at
            FROM events
            WHERE id = $1 AND user_id = $2
            "#,
            event_id,
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(event)
    }

    pub async fn get_events_by_user(&self, user_id: Uuid, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<Event>> {
        let limit = limit.unwrap_or(50).min(100);
        let offset = offset.unwrap_or(0);

        let events = sqlx::query_as!(
            Event,
            r#"
            SELECT
                id, user_id, name, description,
                event_type as "event_type: EventType",
                sport as "sport: Sport",
                event_date, event_time, location, distance, distance_unit,
                elevation_gain, expected_duration, registration_deadline,
                cost, website_url, notes,
                status as "status: EventStatus",
                priority as "priority: EventPriority",
                created_at, updated_at
            FROM events
            WHERE user_id = $1
            ORDER BY event_date ASC, priority DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            limit,
            offset
        )
        .fetch_all(&self.db)
        .await?;

        Ok(events)
    }

    pub async fn update_event(&self, event_id: Uuid, user_id: Uuid, request: UpdateEventRequest) -> Result<Option<Event>> {
        let event = sqlx::query_as!(
            Event,
            r#"
            UPDATE events
            SET
                name = COALESCE($3, name),
                description = COALESCE($4, description),
                event_date = COALESCE($5, event_date),
                event_time = COALESCE($6, event_time),
                location = COALESCE($7, location),
                distance = COALESCE($8, distance),
                distance_unit = COALESCE($9, distance_unit),
                elevation_gain = COALESCE($10, elevation_gain),
                expected_duration = COALESCE($11, expected_duration),
                registration_deadline = COALESCE($12, registration_deadline),
                cost = COALESCE($13, cost),
                website_url = COALESCE($14, website_url),
                notes = COALESCE($15, notes),
                status = COALESCE($16, status),
                priority = COALESCE($17, priority),
                updated_at = $18
            WHERE id = $1 AND user_id = $2
            RETURNING
                id, user_id, name, description,
                event_type as "event_type: EventType",
                sport as "sport: Sport",
                event_date, event_time, location, distance, distance_unit,
                elevation_gain, expected_duration, registration_deadline,
                cost, website_url, notes,
                status as "status: EventStatus",
                priority as "priority: EventPriority",
                created_at, updated_at
            "#,
            event_id,
            user_id,
            request.name,
            request.description,
            request.event_date,
            request.event_time,
            request.location,
            request.distance,
            request.distance_unit,
            request.elevation_gain,
            request.expected_duration,
            request.registration_deadline,
            request.cost,
            request.website_url,
            request.notes,
            request.status.map(|s| s as EventStatus),
            request.priority.map(|p| p as EventPriority),
            Utc::now()
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(event)
    }

    pub async fn delete_event(&self, event_id: Uuid, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM events WHERE id = $1 AND user_id = $2",
            event_id,
            user_id
        )
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // Event planning and periodization
    pub async fn create_event_plan(&self, event_id: Uuid, user_id: Uuid, request: CreateEventPlanRequest) -> Result<EventPlan> {
        // Verify the event belongs to the user
        let event = self.get_event_by_id(event_id, user_id).await?;
        if event.is_none() {
            return Err(anyhow::anyhow!("Event not found or access denied"));
        }

        // Generate training phases based on the periodization parameters
        let training_phases = self.generate_training_phases(&request).await?;

        let event_plan = sqlx::query_as!(
            EventPlan,
            r#"
            INSERT INTO event_plans (
                event_id, user_id, training_phases, peak_date, taper_start_date,
                base_training_weeks, build_training_weeks, peak_training_weeks,
                taper_weeks, recovery_weeks, travel_considerations, logistics_notes,
                equipment_checklist, nutrition_plan, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $15)
            RETURNING
                id, event_id, user_id, training_phases, peak_date, taper_start_date,
                base_training_weeks, build_training_weeks, peak_training_weeks,
                taper_weeks, recovery_weeks, travel_considerations, logistics_notes,
                equipment_checklist, nutrition_plan, created_at, updated_at
            "#,
            event_id,
            user_id,
            serde_json::to_value(&training_phases)?,
            request.peak_date,
            request.peak_date - Duration::weeks(request.taper_weeks as i64),
            request.base_training_weeks,
            request.build_training_weeks,
            request.peak_training_weeks,
            request.taper_weeks,
            request.recovery_weeks,
            request.travel_considerations,
            request.logistics_notes,
            request.equipment_checklist,
            request.nutrition_plan,
            Utc::now()
        )
        .fetch_one(&self.db)
        .await?;

        Ok(event_plan)
    }

    pub async fn get_event_plan(&self, event_id: Uuid, user_id: Uuid) -> Result<Option<EventPlan>> {
        let event_plan = sqlx::query_as!(
            EventPlan,
            r#"
            SELECT
                id, event_id, user_id, training_phases, peak_date, taper_start_date,
                base_training_weeks, build_training_weeks, peak_training_weeks,
                taper_weeks, recovery_weeks, travel_considerations, logistics_notes,
                equipment_checklist, nutrition_plan, created_at, updated_at
            FROM event_plans
            WHERE event_id = $1 AND user_id = $2
            "#,
            event_id,
            user_id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(event_plan)
    }

    // Event calendar and conflict detection
    pub async fn get_event_calendar(&self, user_id: Uuid, start_date: NaiveDate, end_date: NaiveDate) -> Result<EventCalendar> {
        let events = sqlx::query_as!(
            Event,
            r#"
            SELECT
                id, user_id, name, description,
                event_type as "event_type: EventType",
                sport as "sport: Sport",
                event_date, event_time, location, distance, distance_unit,
                elevation_gain, expected_duration, registration_deadline,
                cost, website_url, notes,
                status as "status: EventStatus",
                priority as "priority: EventPriority",
                created_at, updated_at
            FROM events
            WHERE user_id = $1 AND event_date BETWEEN $2 AND $3
            ORDER BY event_date ASC
            "#,
            user_id,
            start_date,
            end_date
        )
        .fetch_all(&self.db)
        .await?;

        let event_plans = sqlx::query_as!(
            EventPlan,
            r#"
            SELECT
                ep.id, ep.event_id, ep.user_id, ep.training_phases, ep.peak_date,
                ep.taper_start_date, ep.base_training_weeks, ep.build_training_weeks,
                ep.peak_training_weeks, ep.taper_weeks, ep.recovery_weeks,
                ep.travel_considerations, ep.logistics_notes, ep.equipment_checklist,
                ep.nutrition_plan, ep.created_at, ep.updated_at
            FROM event_plans ep
            INNER JOIN events e ON ep.event_id = e.id
            WHERE ep.user_id = $1 AND e.event_date BETWEEN $2 AND $3
            "#,
            user_id,
            start_date,
            end_date
        )
        .fetch_all(&self.db)
        .await?;

        // Detect conflicts
        let conflicts = self.detect_event_conflicts(&events).await?;

        // Generate recommendations
        let recommendations = self.generate_event_recommendations(&events, &event_plans).await?;

        Ok(EventCalendar {
            events,
            event_plans,
            conflicts,
            recommendations,
        })
    }

    // Conflict detection and resolution
    async fn detect_event_conflicts(&self, events: &[Event]) -> Result<Vec<EventConflict>> {
        let mut conflicts = Vec::new();

        for (i, event1) in events.iter().enumerate() {
            for event2 in events.iter().skip(i + 1) {
                // Check for date overlaps
                if event1.event_date == event2.event_date {
                    conflicts.push(EventConflict {
                        event1_id: event1.id,
                        event2_id: event2.id,
                        conflict_type: ConflictType::DateOverlap,
                        severity: ConflictSeverity::High,
                        description: "Events scheduled on the same date".to_string(),
                        suggested_resolution: "Reschedule one of the events or choose priority".to_string(),
                    });
                }

                // Check for events too close together
                let days_apart = (event2.event_date - event1.event_date).num_days().abs();
                if days_apart > 0 && days_apart < 7 {
                    let severity = match (event1.priority, event2.priority) {
                        (EventPriority::Critical, EventPriority::Critical) => ConflictSeverity::Critical,
                        (EventPriority::Critical, _) | (_, EventPriority::Critical) => ConflictSeverity::High,
                        (EventPriority::High, EventPriority::High) => ConflictSeverity::Medium,
                        _ => ConflictSeverity::Low,
                    };

                    conflicts.push(EventConflict {
                        event1_id: event1.id,
                        event2_id: event2.id,
                        conflict_type: ConflictType::TooClose,
                        severity,
                        description: format!("Events only {} days apart - insufficient recovery time", days_apart),
                        suggested_resolution: "Allow more recovery time between events".to_string(),
                    });
                }
            }
        }

        Ok(conflicts)
    }

    async fn generate_event_recommendations(&self, events: &[Event], _event_plans: &[EventPlan]) -> Result<Vec<EventRecommendation>> {
        let mut recommendations = Vec::new();
        let today = chrono::Local::now().naive_local().date();

        for event in events {
            let days_until_event = (event.event_date - today).num_days();

            // Registration deadline approaching
            if let Some(reg_deadline) = event.registration_deadline {
                let days_until_deadline = (reg_deadline - today).num_days();
                if days_until_deadline <= 7 && days_until_deadline > 0 && matches!(event.status, EventStatus::Planned) {
                    recommendations.push(EventRecommendation {
                        event_id: event.id,
                        recommendation_type: EventRecommendationType::RegisterSoon,
                        title: "Registration Deadline Approaching".to_string(),
                        description: format!("Register for '{}' - deadline in {} days", event.name, days_until_deadline),
                        priority: EventPriority::High,
                        action_required: true,
                        deadline: Some(reg_deadline),
                    });
                }
            }

            // Event approaching - preparation recommendations
            if days_until_event <= 30 && days_until_event > 0 {
                match days_until_event {
                    15..=30 => {
                        recommendations.push(EventRecommendation {
                            event_id: event.id,
                            recommendation_type: EventRecommendationType::CheckEquipment,
                            title: "Equipment Check".to_string(),
                            description: format!("Check and prepare equipment for '{}'", event.name),
                            priority: EventPriority::Medium,
                            action_required: false,
                            deadline: Some(event.event_date - Duration::days(7)),
                        });
                    }
                    7..=14 => {
                        recommendations.push(EventRecommendation {
                            event_id: event.id,
                            recommendation_type: EventRecommendationType::TaperStart,
                            title: "Begin Taper".to_string(),
                            description: format!("Start tapering training for '{}'", event.name),
                            priority: EventPriority::High,
                            action_required: true,
                            deadline: Some(event.event_date),
                        });
                    }
                    1..=6 => {
                        recommendations.push(EventRecommendation {
                            event_id: event.id,
                            recommendation_type: EventRecommendationType::NutritionPlan,
                            title: "Finalize Nutrition".to_string(),
                            description: format!("Review nutrition strategy for '{}'", event.name),
                            priority: EventPriority::Medium,
                            action_required: false,
                            deadline: Some(event.event_date),
                        });
                    }
                    _ => {}
                }
            }

            // Post-event recovery
            if days_until_event < 0 && days_until_event > -7 && matches!(event.status, EventStatus::Completed) {
                recommendations.push(EventRecommendation {
                    event_id: event.id,
                    recommendation_type: EventRecommendationType::RecoveryPlan,
                    title: "Recovery Phase".to_string(),
                    description: format!("Focus on recovery after '{}'", event.name),
                    priority: EventPriority::Medium,
                    action_required: false,
                    deadline: None,
                });
            }
        }

        Ok(recommendations)
    }

    async fn generate_training_phases(&self, request: &CreateEventPlanRequest) -> Result<Vec<TrainingPhase>> {
        let mut phases = Vec::new();
        let mut current_date = request.peak_date
            - Duration::weeks((request.taper_weeks + request.peak_training_weeks + request.build_training_weeks + request.base_training_weeks) as i64);

        // Base phase
        if request.base_training_weeks > 0 {
            phases.push(TrainingPhase {
                phase_name: "Base Building".to_string(),
                phase_type: PhaseType::Base,
                start_date: current_date,
                end_date: current_date + Duration::weeks(request.base_training_weeks as i64) - Duration::days(1),
                weeks: request.base_training_weeks,
                weekly_volume_range: (6.0, 12.0), // hours
                intensity_distribution: IntensityDistribution {
                    zone1_percentage: 20.0,
                    zone2_percentage: 65.0,
                    zone3_percentage: 10.0,
                    zone4_percentage: 5.0,
                    zone5_percentage: 0.0,
                    zone6_percentage: 0.0,
                },
                focus_areas: vec!["Aerobic base".to_string(), "Consistency".to_string(), "Volume".to_string()],
                key_workouts: vec!["Long endurance rides".to_string(), "Base tempo work".to_string()],
            });
            current_date += Duration::weeks(request.base_training_weeks as i64);
        }

        // Build phase
        if request.build_training_weeks > 0 {
            phases.push(TrainingPhase {
                phase_name: "Build".to_string(),
                phase_type: PhaseType::Build,
                start_date: current_date,
                end_date: current_date + Duration::weeks(request.build_training_weeks as i64) - Duration::days(1),
                weeks: request.build_training_weeks,
                weekly_volume_range: (8.0, 15.0),
                intensity_distribution: IntensityDistribution {
                    zone1_percentage: 15.0,
                    zone2_percentage: 50.0,
                    zone3_percentage: 20.0,
                    zone4_percentage: 12.0,
                    zone5_percentage: 3.0,
                    zone6_percentage: 0.0,
                },
                focus_areas: vec!["Threshold power".to_string(), "Lactate tolerance".to_string()],
                key_workouts: vec!["Threshold intervals".to_string(), "Sweet spot work".to_string()],
            });
            current_date += Duration::weeks(request.build_training_weeks as i64);
        }

        // Peak phase
        if request.peak_training_weeks > 0 {
            phases.push(TrainingPhase {
                phase_name: "Peak".to_string(),
                phase_type: PhaseType::Peak,
                start_date: current_date,
                end_date: current_date + Duration::weeks(request.peak_training_weeks as i64) - Duration::days(1),
                weeks: request.peak_training_weeks,
                weekly_volume_range: (10.0, 18.0),
                intensity_distribution: IntensityDistribution {
                    zone1_percentage: 10.0,
                    zone2_percentage: 40.0,
                    zone3_percentage: 20.0,
                    zone4_percentage: 15.0,
                    zone5_percentage: 10.0,
                    zone6_percentage: 5.0,
                },
                focus_areas: vec!["Race-specific power".to_string(), "Neuromuscular power".to_string()],
                key_workouts: vec!["VO2 max intervals".to_string(), "Race simulation".to_string()],
            });
            current_date += Duration::weeks(request.peak_training_weeks as i64);
        }

        // Taper phase
        if request.taper_weeks > 0 {
            phases.push(TrainingPhase {
                phase_name: "Taper".to_string(),
                phase_type: PhaseType::Taper,
                start_date: current_date,
                end_date: request.peak_date,
                weeks: request.taper_weeks,
                weekly_volume_range: (3.0, 8.0),
                intensity_distribution: IntensityDistribution {
                    zone1_percentage: 40.0,
                    zone2_percentage: 35.0,
                    zone3_percentage: 15.0,
                    zone4_percentage: 8.0,
                    zone5_percentage: 2.0,
                    zone6_percentage: 0.0,
                },
                focus_areas: vec!["Recovery".to_string(), "Maintenance".to_string(), "Readiness".to_string()],
                key_workouts: vec!["Short openers".to_string(), "Easy recovery rides".to_string()],
            });
        }

        Ok(phases)
    }
}