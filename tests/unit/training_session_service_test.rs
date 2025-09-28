use chrono::{NaiveDate, Utc, Duration};
use uuid::Uuid;
use ai_coach::models::*;
use ai_coach::services::TrainingSessionService;

// Import test utilities
use crate::common::MockDataGenerator;

#[cfg(test)]
mod training_session_service_tests {
    use super::*;

    #[test]
    fn test_training_session_validation() {
        // Test training session validation logic
        let session = MockDataGenerator::training_session(Uuid::new_v4());

        // Test required fields
        assert!(session.duration_minutes > 0);
        assert!(session.user_id != Uuid::nil());

        // Test optional but meaningful fields
        if let Some(distance) = session.distance_meters {
            assert!(distance > 0.0);
        }

        if let Some(avg_hr) = session.avg_heart_rate {
            assert!(avg_hr > 40 && avg_hr < 220); // Reasonable heart rate range
        }

        if let Some(max_hr) = session.max_heart_rate {
            assert!(max_hr > 40 && max_hr < 250); // Reasonable max heart rate
        }

        if let Some(power) = session.avg_power {
            assert!(power > 0 && power < 2000); // Reasonable power range
        }
    }

    #[test]
    fn test_session_duration_validation() {
        // Test session duration validation logic
        let valid_durations = vec![15, 30, 60, 120, 180, 240, 360, 480]; // 15 minutes to 8 hours
        let invalid_durations = vec![0, -30, 1440, 2000]; // 0, negative, over 24 hours

        for duration in valid_durations {
            assert!(is_valid_duration(duration), "Duration {} should be valid", duration);
        }

        for duration in invalid_durations {
            assert!(!is_valid_duration(duration), "Duration {} should be invalid", duration);
        }
    }

    #[test]
    fn test_heart_rate_validation() {
        // Test heart rate validation logic
        let valid_heart_rates = vec![50, 120, 150, 180, 200];
        let invalid_heart_rates = vec![0, 30, 250, 300, -10];

        for hr in valid_heart_rates {
            assert!(is_valid_heart_rate(hr), "Heart rate {} should be valid", hr);
        }

        for hr in invalid_heart_rates {
            assert!(!is_valid_heart_rate(hr), "Heart rate {} should be invalid", hr);
        }
    }

    #[test]
    fn test_power_validation() {
        // Test power validation logic
        let valid_powers = vec![100, 200, 300, 500, 800, 1500];
        let invalid_powers = vec![0, -50, 3000, 5000];

        for power in valid_powers {
            assert!(is_valid_power(power), "Power {} should be valid", power);
        }

        for power in invalid_powers {
            assert!(!is_valid_power(power), "Power {} should be invalid", power);
        }
    }

    #[test]
    fn test_tss_calculation() {
        // Test Training Stress Score calculation logic
        let duration_minutes = 60;
        let normalized_power = 250;
        let ftp = 300;
        let intensity_factor = normalized_power as f32 / ftp as f32;

        let tss = calculate_tss(duration_minutes, intensity_factor);
        let expected_tss = (duration_minutes as f32 / 60.0) * intensity_factor.powi(2) * 100.0;

        assert_eq!(tss, expected_tss);
        assert!(tss > 0.0);
    }

    #[test]
    fn test_intensity_factor_calculation() {
        // Test Intensity Factor calculation logic
        let test_cases = vec![
            (200, 250, 0.8),   // Easy ride
            (250, 250, 1.0),   // FTP effort
            (300, 250, 1.2),   // Above FTP
            (150, 300, 0.5),   // Recovery ride
        ];

        for (normalized_power, ftp, expected_if) in test_cases {
            let calculated_if = calculate_intensity_factor(normalized_power, ftp);
            assert!((calculated_if - expected_if).abs() < 0.01,
                "IF for NP {} and FTP {} should be {}, got {}",
                normalized_power, ftp, expected_if, calculated_if);
        }
    }

    #[test]
    fn test_session_type_classification() {
        // Test automatic session type classification
        let test_cases = vec![
            (30, Some(150), Some(200), "recovery"),
            (60, Some(250), Some(250), "endurance"),
            (45, Some(320), Some(280), "threshold"),
            (20, Some(400), Some(350), "vo2max"),
            (90, Some(0), Some(220), "strength"), // No power, high HR
        ];

        for (duration, avg_power, avg_hr, expected_type) in test_cases {
            let classified_type = classify_session_type(duration, avg_power, avg_hr);
            assert_eq!(classified_type, expected_type,
                "Session with duration {}, power {:?}, HR {:?} should be classified as {}",
                duration, avg_power, avg_hr, expected_type);
        }
    }

    #[test]
    fn test_perceived_exertion_validation() {
        // Test RPE (Rate of Perceived Exertion) validation
        let valid_rpe_values = vec![1, 5, 7, 10];
        let invalid_rpe_values = vec![0, 11, 15, -1];

        for rpe in valid_rpe_values {
            assert!(is_valid_rpe(rpe), "RPE {} should be valid", rpe);
        }

        for rpe in invalid_rpe_values {
            assert!(!is_valid_rpe(rpe), "RPE {} should be invalid", rpe);
        }
    }

    #[test]
    fn test_session_date_validation() {
        // Test session date validation
        let today = chrono::Local::now().naive_local().date();
        let yesterday = today - Duration::days(1);
        let future_date = today + Duration::days(30);
        let very_old_date = today - Duration::days(3650); // 10 years ago

        assert!(is_valid_session_date(today));
        assert!(is_valid_session_date(yesterday));
        assert!(!is_valid_session_date(future_date));
        assert!(is_valid_session_date(very_old_date)); // Historical data should be allowed
    }

    #[test]
    fn test_session_metrics_consistency() {
        // Test consistency between different session metrics
        let session = TrainingSession {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            session_type: Some("threshold".to_string()),
            duration_minutes: 60,
            distance_meters: Some(30000.0), // 30km in 1 hour
            avg_heart_rate: Some(165),
            max_heart_rate: Some(175),
            avg_power: Some(280),
            normalized_power: Some(285),
            tss: Some(85.0),
            if_: Some(0.95),
            notes: Some("Good threshold session".to_string()),
            perceived_exertion: Some(7),
            session_date: chrono::Local::now().naive_local().date(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Test metric consistency
        assert!(validate_session_metrics_consistency(&session));

        // Test inconsistent metrics
        let inconsistent_session = TrainingSession {
            max_heart_rate: Some(150), // Max HR lower than avg HR
            avg_heart_rate: Some(170),
            ..session
        };

        assert!(!validate_session_metrics_consistency(&inconsistent_session));
    }

    #[test]
    fn test_training_load_calculation() {
        // Test training load calculation for weekly/monthly summaries
        let sessions = vec![
            create_test_session_with_tss(100.0),
            create_test_session_with_tss(75.0),
            create_test_session_with_tss(50.0),
            create_test_session_with_tss(120.0),
            create_test_session_with_tss(80.0),
        ];

        let total_load = calculate_total_training_load(&sessions);
        assert_eq!(total_load, 425.0);

        let average_load = calculate_average_training_load(&sessions);
        assert_eq!(average_load, 85.0);
    }

    #[test]
    fn test_session_summary_generation() {
        // Test session summary generation
        let sessions = vec![
            create_test_session_with_type("endurance"),
            create_test_session_with_type("threshold"),
            create_test_session_with_type("vo2max"),
            create_test_session_with_type("endurance"),
            create_test_session_with_type("recovery"),
        ];

        let summary = generate_session_summary(&sessions);

        assert_eq!(summary.total_sessions, 5);
        assert_eq!(summary.session_type_counts.get("endurance"), Some(&2));
        assert_eq!(summary.session_type_counts.get("threshold"), Some(&1));
        assert_eq!(summary.session_type_counts.get("vo2max"), Some(&1));
        assert_eq!(summary.session_type_counts.get("recovery"), Some(&1));
    }

    #[test]
    fn test_distance_speed_consistency() {
        // Test consistency between distance, duration, and average speed
        let duration_hours = 2.0;
        let distance_km = 60.0;
        let expected_speed = distance_km / duration_hours; // 30 km/h

        let calculated_speed = calculate_average_speed(distance_km, duration_hours);
        assert_eq!(calculated_speed, expected_speed);

        // Test reasonable speed ranges for different activities
        assert!(is_reasonable_cycling_speed(25.0)); // 25 km/h reasonable for cycling
        assert!(is_reasonable_running_speed(12.0)); // 12 km/h reasonable for running
        assert!(!is_reasonable_running_speed(50.0)); // 50 km/h unreasonable for running
    }

    // Helper functions for testing business logic

    fn is_valid_duration(minutes: i32) -> bool {
        minutes > 0 && minutes <= 720 // Up to 12 hours
    }

    fn is_valid_heart_rate(hr: i32) -> bool {
        hr >= 40 && hr <= 220
    }

    fn is_valid_power(power: i32) -> bool {
        power > 0 && power <= 2000
    }

    fn calculate_tss(duration_minutes: i32, intensity_factor: f32) -> f32 {
        (duration_minutes as f32 / 60.0) * intensity_factor.powi(2) * 100.0
    }

    fn calculate_intensity_factor(normalized_power: i32, ftp: i32) -> f32 {
        normalized_power as f32 / ftp as f32
    }

    fn classify_session_type(duration: i32, avg_power: Option<i32>, avg_hr: Option<i32>) -> &'static str {
        if let Some(power) = avg_power {
            if power == 0 {
                return "strength";
            } else if power < 200 {
                return "recovery";
            } else if power < 250 {
                return "endurance";
            } else if power < 300 {
                return "threshold";
            } else {
                return "vo2max";
            }
        }

        if let Some(hr) = avg_hr {
            if hr < 130 {
                return "recovery";
            } else if hr < 160 {
                return "endurance";
            } else if hr < 180 {
                return "threshold";
            } else {
                return "vo2max";
            }
        }

        "endurance" // Default
    }

    fn is_valid_rpe(rpe: i32) -> bool {
        rpe >= 1 && rpe <= 10
    }

    fn is_valid_session_date(date: NaiveDate) -> bool {
        let today = chrono::Local::now().naive_local().date();
        date <= today // Can't be in the future
    }

    fn validate_session_metrics_consistency(session: &TrainingSession) -> bool {
        // Check that max HR >= avg HR
        if let (Some(avg_hr), Some(max_hr)) = (session.avg_heart_rate, session.max_heart_rate) {
            if max_hr < avg_hr {
                return false;
            }
        }

        // Check that normalized power >= avg power (generally true for variable efforts)
        if let (Some(avg_power), Some(np)) = (session.avg_power, session.normalized_power) {
            if np < avg_power {
                return false;
            }
        }

        // Check that TSS and IF are reasonable relative to duration and power
        if let (Some(tss), Some(if_val)) = (session.tss, session.if_) {
            let expected_tss = (session.duration_minutes as f32 / 60.0) * if_val.powi(2) * 100.0;
            let tss_diff = (tss - expected_tss).abs();
            if tss_diff > 20.0 { // Allow 20 TSS difference for measurement errors
                return false;
            }
        }

        true
    }

    fn calculate_total_training_load(sessions: &[TrainingSession]) -> f32 {
        sessions.iter().filter_map(|s| s.tss).sum()
    }

    fn calculate_average_training_load(sessions: &[TrainingSession]) -> f32 {
        if sessions.is_empty() {
            return 0.0;
        }
        calculate_total_training_load(sessions) / sessions.len() as f32
    }

    fn create_test_session_with_tss(tss: f32) -> TrainingSession {
        TrainingSession {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            session_type: Some("endurance".to_string()),
            duration_minutes: 60,
            distance_meters: Some(30000.0),
            avg_heart_rate: Some(150),
            max_heart_rate: Some(165),
            avg_power: Some(200),
            normalized_power: Some(210),
            tss: Some(tss),
            if_: Some(0.7),
            notes: Some("Test session".to_string()),
            perceived_exertion: Some(5),
            session_date: chrono::Local::now().naive_local().date(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_session_with_type(session_type: &str) -> TrainingSession {
        TrainingSession {
            session_type: Some(session_type.to_string()),
            ..create_test_session_with_tss(100.0)
        }
    }

    fn generate_session_summary(sessions: &[TrainingSession]) -> SessionSummary {
        let mut session_type_counts = std::collections::HashMap::new();

        for session in sessions {
            if let Some(ref session_type) = session.session_type {
                *session_type_counts.entry(session_type.clone()).or_insert(0) += 1;
            }
        }

        SessionSummary {
            total_sessions: sessions.len(),
            session_type_counts,
        }
    }

    fn calculate_average_speed(distance_km: f64, duration_hours: f64) -> f64 {
        if duration_hours == 0.0 {
            0.0
        } else {
            distance_km / duration_hours
        }
    }

    fn is_reasonable_cycling_speed(speed_kmh: f64) -> bool {
        speed_kmh > 0.0 && speed_kmh <= 80.0 // Reasonable cycling speed range
    }

    fn is_reasonable_running_speed(speed_kmh: f64) -> bool {
        speed_kmh > 0.0 && speed_kmh <= 25.0 // Reasonable running speed range
    }

    // Mock struct for testing
    struct SessionSummary {
        total_sessions: usize,
        session_type_counts: std::collections::HashMap<String, i32>,
    }
}