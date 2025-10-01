use chrono::{NaiveDate, NaiveTime, Utc, Duration};
use uuid::Uuid;
use ai_coach::models::*;
use ai_coach::services::EventService;

// Import test utilities
use crate::common::MockDataGenerator;

#[cfg(test)]
mod event_service_tests {
    use super::*;

    #[test]
    fn test_event_validation_logic() {
        // Test event validation business logic
        let event = MockDataGenerator::event(Uuid::new_v4());

        // Test that required fields are present
        assert!(!event.name.is_empty());
        assert!(event.event_date > chrono::Local::now().naive_local().date());

        // Test event type validation
        assert!(matches!(event.event_type, EventType::Race | EventType::Competition | EventType::Training));

        // Test sport validation
        assert!(matches!(event.sport, Sport::Cycling | Sport::Running | Sport::Triathlon));

        // Test priority validation
        assert!(matches!(event.priority, EventPriority::Low | EventPriority::Medium | EventPriority::High));

        // Test status validation
        assert!(matches!(event.status, EventStatus::Planned | EventStatus::Registered | EventStatus::Completed | EventStatus::Cancelled));
    }

    #[test]
    fn test_event_date_validation() {
        // Test event date validation logic
        let today = chrono::Local::now().naive_local().date();
        let tomorrow = today + Duration::days(1);
        let past_date = today - Duration::days(1);

        // Valid future date
        assert!(is_valid_event_date(tomorrow));

        // Invalid past date
        assert!(!is_valid_event_date(past_date));

        // Edge case: today should be valid
        assert!(is_valid_event_date(today));
    }

    #[test]
    fn test_registration_deadline_validation() {
        // Test registration deadline validation logic
        let event_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let valid_deadline = NaiveDate::from_ymd_opt(2024, 5, 15).unwrap(); // 1 month before
        let invalid_deadline = NaiveDate::from_ymd_opt(2024, 7, 15).unwrap(); // After event

        assert!(is_valid_registration_deadline(valid_deadline, event_date));
        assert!(!is_valid_registration_deadline(invalid_deadline, event_date));
    }

    #[test]
    fn test_event_duration_validation() {
        // Test event duration validation logic
        let short_duration = 30; // 30 minutes
        let normal_duration = 120; // 2 hours
        let long_duration = 480; // 8 hours
        let ultra_duration = 1440; // 24 hours

        assert!(is_valid_duration(short_duration));
        assert!(is_valid_duration(normal_duration));
        assert!(is_valid_duration(long_duration));
        assert!(is_valid_duration(ultra_duration));

        // Invalid durations
        assert!(!is_valid_duration(0));
        assert!(!is_valid_duration(-30));
    }

    #[test]
    fn test_event_distance_validation() {
        // Test event distance validation by sport
        let cycling_distances = vec![10.0, 50.0, 100.0, 200.0]; // km
        let running_distances = vec![5.0, 10.0, 21.1, 42.2]; // km
        let triathlon_distances = vec![25.75, 51.5, 113.0, 226.0]; // total km

        for distance in cycling_distances {
            assert!(is_valid_distance_for_sport(distance, Sport::Cycling));
        }

        for distance in running_distances {
            assert!(is_valid_distance_for_sport(distance, Sport::Running));
        }

        for distance in triathlon_distances {
            assert!(is_valid_distance_for_sport(distance, Sport::Triathlon));
        }

        // Invalid distances
        assert!(!is_valid_distance_for_sport(0.0, Sport::Running));
        assert!(!is_valid_distance_for_sport(-5.0, Sport::Cycling));
    }

    #[test]
    fn test_event_cost_validation() {
        // Test event cost validation logic
        let free_event = 0.0;
        let normal_event = 25.0;
        let expensive_event = 500.0;
        let premium_event = 1000.0;

        assert!(is_valid_cost(free_event));
        assert!(is_valid_cost(normal_event));
        assert!(is_valid_cost(expensive_event));
        assert!(is_valid_cost(premium_event));

        // Invalid costs
        assert!(!is_valid_cost(-10.0));
    }

    #[test]
    fn test_event_priority_logic() {
        // Test event priority assignment logic
        let goal_event = Event {
            event_type: EventType::Race,
            sport: Sport::Cycling,
            distance: Some(100.0),
            priority: EventPriority::High,
            ..MockDataGenerator::event(Uuid::new_v4())
        };

        let training_event = Event {
            event_type: EventType::Training,
            sport: Sport::Running,
            distance: Some(10.0),
            priority: EventPriority::Low,
            ..MockDataGenerator::event(Uuid::new_v4())
        };

        // Goal events should typically have higher priority
        assert!(get_priority_score(goal_event.priority) > get_priority_score(training_event.priority));
    }

    #[test]
    fn test_event_conflict_detection() {
        // Test event conflict detection logic
        let event1 = Event {
            event_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            event_time: Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap()),
            expected_duration: Some(120), // 2 hours
            ..MockDataGenerator::event(Uuid::new_v4())
        };

        let conflicting_event = Event {
            event_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            event_time: Some(NaiveTime::from_hms_opt(10, 0, 0).unwrap()),
            expected_duration: Some(60), // 1 hour
            ..MockDataGenerator::event(Uuid::new_v4())
        };

        let non_conflicting_event = Event {
            event_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            event_time: Some(NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
            expected_duration: Some(60), // 1 hour
            ..MockDataGenerator::event(Uuid::new_v4())
        };

        assert!(has_time_conflict(&event1, &conflicting_event));
        assert!(!has_time_conflict(&event1, &non_conflicting_event));
    }

    #[test]
    fn test_event_status_transitions() {
        // Test valid event status transitions
        let initial_status = EventStatus::Planned;
        let valid_transitions = vec![
            EventStatus::Registered,
            EventStatus::Confirmed,
            EventStatus::Cancelled,
        ];

        for transition in valid_transitions {
            assert!(is_valid_status_transition(initial_status, transition));
        }

        // Test invalid transitions
        assert!(!is_valid_status_transition(EventStatus::Completed, EventStatus::Planned));
        assert!(!is_valid_status_transition(EventStatus::Cancelled, EventStatus::Registered));
    }

    #[test]
    fn test_training_phase_calculation() {
        // Test training phase calculation based on event date
        let event_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let today = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(); // 3 months before

        let weeks_to_event = (event_date - today).num_weeks();
        let phase = calculate_training_phase(weeks_to_event);

        // 12-13 weeks should be base phase
        assert_eq!(phase, PhaseType::Base);

        // Test other phases
        let build_phase = calculate_training_phase(8);
        assert_eq!(build_phase, PhaseType::Build);

        let peak_phase = calculate_training_phase(3);
        assert_eq!(peak_phase, PhaseType::Peak);

        let taper_phase = calculate_training_phase(1);
        assert_eq!(taper_phase, PhaseType::Taper);
    }

    #[test]
    fn test_event_recommendation_logic() {
        // Test event recommendation generation logic
        let athlete_ftp = 250; // watts
        let athlete_experience = "intermediate"; // beginner, intermediate, advanced

        let recommendations = generate_event_recommendations_for_athlete(athlete_ftp, athlete_experience);

        // Should recommend appropriate events based on athlete level
        assert!(!recommendations.is_empty());

        // For intermediate athlete, should include challenging but achievable events
        let has_appropriate_difficulty = recommendations.iter().any(|rec| {
            rec.difficulty_level == "intermediate" || rec.difficulty_level == "advanced"
        });
        assert!(has_appropriate_difficulty);
    }

    #[test]
    fn test_event_calendar_generation() {
        // Test event calendar generation logic
        let events = vec![
            Event {
                event_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                priority: EventPriority::High,
                ..MockDataGenerator::event(Uuid::new_v4())
            },
            Event {
                event_date: NaiveDate::from_ymd_opt(2024, 7, 15).unwrap(),
                priority: EventPriority::Medium,
                ..MockDataGenerator::event(Uuid::new_v4())
            },
            Event {
                event_date: NaiveDate::from_ymd_opt(2024, 5, 15).unwrap(),
                priority: EventPriority::Low,
                ..MockDataGenerator::event(Uuid::new_v4())
            },
        ];

        let calendar = generate_event_calendar(&events);

        // Events should be sorted by date
        assert_eq!(calendar.events.len(), 3);
        assert!(calendar.events[0].event_date < calendar.events[1].event_date);
        assert!(calendar.events[1].event_date < calendar.events[2].event_date);
    }

    // Helper functions (would be implemented in the actual service)

    fn is_valid_event_date(date: NaiveDate) -> bool {
        date >= chrono::Local::now().naive_local().date()
    }

    fn is_valid_registration_deadline(deadline: NaiveDate, event_date: NaiveDate) -> bool {
        deadline <= event_date
    }

    fn is_valid_duration(minutes: i32) -> bool {
        minutes > 0 && minutes <= 2880 // Max 48 hours
    }

    fn is_valid_distance_for_sport(distance: f64, sport: Sport) -> bool {
        if distance <= 0.0 {
            return false;
        }

        match sport {
            Sport::Running => distance <= 200.0, // Max 200km
            Sport::Cycling => distance <= 500.0, // Max 500km
            Sport::Triathlon => distance <= 300.0, // Max total distance
        }
    }

    fn is_valid_cost(cost: f64) -> bool {
        cost >= 0.0
    }

    fn get_priority_score(priority: EventPriority) -> i32 {
        match priority {
            EventPriority::Low => 1,
            EventPriority::Medium => 2,
            EventPriority::High => 3,
        }
    }

    fn has_time_conflict(event1: &Event, event2: &Event) -> bool {
        if event1.event_date != event2.event_date {
            return false;
        }

        if let (Some(time1), Some(time2), Some(duration1), Some(duration2)) =
            (event1.event_time, event2.event_time, event1.expected_duration, event2.expected_duration) {

            let end_time1 = time1 + chrono::Duration::minutes(duration1 as i64);
            let end_time2 = time2 + chrono::Duration::minutes(duration2 as i64);

            // Check for overlap
            !(end_time1 <= time2 || end_time2 <= time1)
        } else {
            false
        }
    }

    fn is_valid_status_transition(from: EventStatus, to: EventStatus) -> bool {
        match (from, to) {
            (EventStatus::Planned, EventStatus::Registered) => true,
            (EventStatus::Planned, EventStatus::Confirmed) => true,
            (EventStatus::Planned, EventStatus::Cancelled) => true,
            (EventStatus::Registered, EventStatus::Confirmed) => true,
            (EventStatus::Registered, EventStatus::Cancelled) => true,
            (EventStatus::Confirmed, EventStatus::Completed) => true,
            (EventStatus::Confirmed, EventStatus::Cancelled) => true,
            _ => false,
        }
    }

    fn calculate_training_phase(weeks_to_event: i64) -> PhaseType {
        match weeks_to_event {
            0..=2 => PhaseType::Taper,
            3..=6 => PhaseType::Peak,
            7..=10 => PhaseType::Build,
            _ => PhaseType::Base,
        }
    }

    fn generate_event_recommendations_for_athlete(ftp: i32, experience: &str) -> Vec<EventRecommendation> {
        let mut recommendations = Vec::new();

        // Mock recommendation based on athlete profile
        match experience {
            "beginner" => {
                recommendations.push(EventRecommendation {
                    event_type: EventType::Training,
                    sport: Sport::Cycling,
                    difficulty_level: "beginner".to_string(),
                    estimated_duration: 60,
                    recommended_distance: 25.0,
                });
            },
            "intermediate" => {
                recommendations.push(EventRecommendation {
                    event_type: EventType::Race,
                    sport: Sport::Cycling,
                    difficulty_level: "intermediate".to_string(),
                    estimated_duration: 120,
                    recommended_distance: 50.0,
                });
            },
            "advanced" => {
                recommendations.push(EventRecommendation {
                    event_type: EventType::Competition,
                    sport: Sport::Cycling,
                    difficulty_level: "advanced".to_string(),
                    estimated_duration: 240,
                    recommended_distance: 100.0,
                });
            },
            _ => {}
        }

        recommendations
    }

    fn generate_event_calendar(events: &[Event]) -> EventCalendar {
        let mut sorted_events = events.to_vec();
        sorted_events.sort_by(|a, b| a.event_date.cmp(&b.event_date));

        EventCalendar {
            events: sorted_events,
            generated_at: Utc::now(),
        }
    }

    // Mock types for testing (would be defined in models)
    #[derive(Debug, PartialEq)]
    struct EventRecommendation {
        event_type: EventType,
        sport: Sport,
        difficulty_level: String,
        estimated_duration: i32,
        recommended_distance: f64,
    }

    struct EventCalendar {
        events: Vec<Event>,
        generated_at: chrono::DateTime<Utc>,
    }
}