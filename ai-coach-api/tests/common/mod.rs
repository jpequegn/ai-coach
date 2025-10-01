use ai_coach::models::*;
use ai_coach::auth::{UserRole, UserSession};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use fake::{Fake, Faker};
use sqlx::PgPool;
use std::sync::Once;
use testcontainers::{clients, images, Container, Docker};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

static INIT: Once = Once::new();

/// Initialize test logging
pub fn init_test_logging() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("debug")
            .with_test_writer()
            .init();
    });
}

/// Test database setup using testcontainers
pub struct TestDatabase {
    _container: Container<'static, Postgres>,
    pub pool: PgPool,
}

impl TestDatabase {
    pub async fn new() -> Self {
        let docker = clients::Cli::default();
        let postgres_image = images::postgres::Postgres::default()
            .with_db_name("ai_coach_test")
            .with_user("postgres")
            .with_password("password");

        let container = docker.run(postgres_image);
        let connection_string = format!(
            "postgresql://postgres:password@127.0.0.1:{}/ai_coach_test",
            container.get_host_port_ipv4(5432)
        );

        let pool = PgPool::connect(&connection_string)
            .await
            .expect("Failed to connect to test database");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        Self {
            _container: container,
            pool,
        }
    }
}

/// Mock data generators
pub struct MockDataGenerator;

impl MockDataGenerator {
    /// Generate a test user
    pub fn user() -> User {
        User {
            id: Uuid::new_v4(),
            email: format!("test{}@example.com", Faker.fake::<u32>()),
            password_hash: "$2b$12$dummy_hash".to_string(),
            created_at: Utc::now() - Duration::days(Faker.fake::<i64>() % 365),
            updated_at: Utc::now(),
        }
    }

    /// Generate a test athlete profile
    pub fn athlete_profile(user_id: Uuid) -> AthleteProfile {
        AthleteProfile {
            id: Uuid::new_v4(),
            user_id,
            sport: "cycling".to_string(),
            ftp: Some((150..400).fake()),
            lthr: Some((150..200).fake()),
            max_heart_rate: Some((180..220).fake()),
            threshold_pace: Some("4:30".to_string()),
            zones: serde_json::json!({
                "zone1": [0, 120],
                "zone2": [120, 150],
                "zone3": [150, 170],
                "zone4": [170, 190],
                "zone5": [190, 220]
            }),
            created_at: Utc::now() - Duration::days(30),
            updated_at: Utc::now(),
        }
    }

    /// Generate a test training session
    pub fn training_session(user_id: Uuid) -> TrainingSession {
        let session_types = ["endurance", "threshold", "vo2max", "recovery", "strength"];

        TrainingSession {
            id: Uuid::new_v4(),
            user_id,
            session_type: Some(session_types[(0..session_types.len()).fake()].to_string()),
            duration_minutes: (30..180).fake(),
            distance_meters: Some((5000..50000).fake()),
            avg_heart_rate: Some((120..180).fake()),
            max_heart_rate: Some((160..200).fake()),
            avg_power: Some((100..350).fake()),
            normalized_power: Some((110..360).fake()),
            tss: Some((50..400).fake()),
            if_: Some((0.6..1.2).fake()),
            notes: Some("Generated test session".to_string()),
            perceived_exertion: Some((1..10).fake()),
            session_date: Self::random_date_within_days(365),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Generate test goal data
    pub fn goal(user_id: Uuid) -> Goal {
        let goal_types = [GoalType::Power, GoalType::Pace, GoalType::RaceTime, GoalType::Distance];
        let categories = [GoalCategory::Performance, GoalCategory::Process, GoalCategory::Event];
        let priorities = [GoalPriority::Low, GoalPriority::Medium, GoalPriority::High];

        Goal {
            id: Uuid::new_v4(),
            user_id,
            title: format!("Test Goal {}", Faker.fake::<u32>()),
            description: "Auto-generated test goal".to_string(),
            goal_type: goal_types[(0..goal_types.len()).fake()].clone(),
            goal_category: categories[(0..categories.len()).fake()].clone(),
            target_value: Some((100.0..500.0).fake()),
            current_value: Some((0.0..250.0).fake()),
            unit: Some("watts".to_string()),
            target_date: Some(Self::random_future_date()),
            status: GoalStatus::Active,
            priority: priorities[(0..priorities.len()).fake()].clone(),
            event_id: None,
            parent_goal_id: None,
            created_at: Utc::now() - Duration::days((1..90).fake()),
            updated_at: Utc::now(),
        }
    }

    /// Generate test event data
    pub fn event(user_id: Uuid) -> Event {
        let event_types = [EventType::Race, EventType::Competition, EventType::Training];
        let sports = [Sport::Cycling, Sport::Running, Sport::Triathlon];
        let priorities = [EventPriority::Low, EventPriority::Medium, EventPriority::High];

        Event {
            id: Uuid::new_v4(),
            user_id,
            name: format!("Test Event {}", Faker.fake::<u32>()),
            description: Some("Auto-generated test event".to_string()),
            event_type: event_types[(0..event_types.len()).fake()].clone(),
            sport: sports[(0..sports.len()).fake()].clone(),
            event_date: Self::random_future_date(),
            event_time: None,
            location: Some("Test Location".to_string()),
            distance: Some((10.0..200.0).fake()),
            distance_unit: Some("km".to_string()),
            elevation_gain: Some((100.0..3000.0).fake()),
            expected_duration: Some((60..480).fake()),
            registration_deadline: Some(Self::random_future_date() - Duration::days(30)),
            cost: Some((25.0..500.0).fake()),
            website_url: Some("https://example.com".to_string()),
            notes: Some("Test event notes".to_string()),
            status: EventStatus::Planned,
            priority: priorities[(0..priorities.len()).fake()].clone(),
            created_at: Utc::now() - Duration::days((1..30).fake()),
            updated_at: Utc::now(),
        }
    }

    /// Generate test training features
    pub fn training_features() -> TrainingFeatures {
        TrainingFeatures {
            current_ctl: (50.0..150.0).fake(),
            current_atl: (30.0..100.0).fake(),
            current_tsb: (-50.0..50.0).fake(),
            days_since_last_workout: (0..14).fake(),
            avg_weekly_tss_4weeks: (100.0..600.0).fake(),
            recent_performance_trend: (-0.5..0.5).fake(),
            days_until_goal_event: if (0..2).fake() == 1 { Some((1..365).fake()) } else { None },
            preferred_workout_types: vec!["endurance".to_string(), "threshold".to_string()],
            seasonal_factors: (0.6..1.0).fake(),
        }
    }

    /// Generate test training data point
    pub fn training_data_point() -> TrainingDataPoint {
        let features = Self::training_features();
        let actual_tss = features.current_ctl * (0.5..1.5).fake::<f32>();

        TrainingDataPoint {
            features,
            actual_tss,
            actual_workout_type: "endurance".to_string(),
            performance_outcome: Some((1.0..10.0).fake()),
            recovery_rating: Some((1.0..10.0).fake()),
            workout_date: Utc::now() - Duration::days((1..365).fake()),
        }
    }

    /// Generate test notification
    pub fn notification(user_id: Uuid) -> Notification {
        let types = [
            NotificationType::WorkoutReminder,
            NotificationType::FitnessImprovement,
            NotificationType::OvertrainingRisk,
            NotificationType::WeeklyProgressSummary,
        ];
        let categories = [
            NotificationCategory::TrainingReminder,
            NotificationCategory::PerformanceAlert,
            NotificationCategory::HealthSafety,
            NotificationCategory::Motivation,
        ];

        Notification {
            id: Uuid::new_v4(),
            user_id,
            notification_type: types[(0..types.len()).fake()].clone(),
            category: categories[(0..categories.len()).fake()].clone(),
            priority: NotificationPriority::Medium,
            title: "Test Notification".to_string(),
            message: "This is a test notification message".to_string(),
            data: Some(serde_json::json!({"test": true})),
            scheduled_at: Utc::now() + Duration::hours((1..24).fake()),
            sent_at: None,
            read_at: None,
            delivery_channels: vec![DeliveryChannel::Email, DeliveryChannel::InApp],
            delivery_status: DeliveryStatus::Pending,
            expires_at: Some(Utc::now() + Duration::days(7)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Generate a user session for testing
    pub fn user_session(user_id: Uuid) -> UserSession {
        UserSession {
            user_id,
            email: format!("test{}@example.com", Faker.fake::<u32>()),
            role: UserRole::Athlete,
        }
    }

    // Helper methods
    fn random_date_within_days(days: i64) -> DateTime<Utc> {
        Utc::now() - Duration::days((0..days).fake())
    }

    fn random_future_date() -> NaiveDate {
        let future_days: i64 = (1..365).fake();
        (Utc::now() + Duration::days(future_days)).date_naive()
    }
}

/// API test helpers
pub struct ApiTestHelpers;

impl ApiTestHelpers {
    /// Create a test JWT token
    pub fn create_test_token(user_id: Uuid, email: &str, role: UserRole) -> String {
        use ai_coach::auth::JwtService;
        let jwt_service = JwtService::new("test_secret_key_for_testing_only");
        jwt_service.create_access_token(user_id, email, role).unwrap()
    }

    /// Get authorization header with test token
    pub fn auth_header(user_id: Uuid, role: UserRole) -> (String, String) {
        let email = format!("test{}@example.com", user_id);
        let token = Self::create_test_token(user_id, &email, role);
        ("Authorization".to_string(), format!("Bearer {}", token))
    }
}

/// Performance testing utilities
pub struct PerformanceTestHelpers;

impl PerformanceTestHelpers {
    /// Create load test scenario
    pub fn create_load_test_config(endpoint: &str, concurrent_users: u32, duration_seconds: u32) -> String {
        format!(
            r#"
import http from 'k6/http';
import {{ check }} from 'k6';

export let options = {{
    vus: {},
    duration: '{}s',
}};

export default function() {{
    let response = http.get('{}');
    check(response, {{
        'status is 200': (r) => r.status === 200,
        'response time < 500ms': (r) => r.timings.duration < 500,
    }});
}}
"#,
            concurrent_users, duration_seconds, endpoint
        )
    }
}

/// Database test utilities
pub struct DatabaseTestHelpers;

impl DatabaseTestHelpers {
    /// Clean all test data from database
    pub async fn clean_database(pool: &PgPool) -> Result<(), sqlx::Error> {
        // Delete in order to respect foreign key constraints
        sqlx::query("DELETE FROM goal_progress").execute(pool).await?;
        sqlx::query("DELETE FROM goal_recommendations").execute(pool).await?;
        sqlx::query("DELETE FROM goals").execute(pool).await?;
        sqlx::query("DELETE FROM event_conflicts").execute(pool).await?;
        sqlx::query("DELETE FROM event_recommendations").execute(pool).await?;
        sqlx::query("DELETE FROM event_plans").execute(pool).await?;
        sqlx::query("DELETE FROM events").execute(pool).await?;
        sqlx::query("DELETE FROM notifications").execute(pool).await?;
        sqlx::query("DELETE FROM training_sessions").execute(pool).await?;
        sqlx::query("DELETE FROM athlete_profiles").execute(pool).await?;
        sqlx::query("DELETE FROM refresh_tokens").execute(pool).await?;
        sqlx::query("DELETE FROM password_reset_tokens").execute(pool).await?;
        sqlx::query("DELETE FROM token_blacklist").execute(pool).await?;
        sqlx::query("DELETE FROM users").execute(pool).await?;
        Ok(())
    }

    /// Seed database with test data
    pub async fn seed_test_data(pool: &PgPool, user_count: usize) -> Result<Vec<Uuid>, sqlx::Error> {
        let mut user_ids = Vec::new();

        for i in 0..user_count {
            let user = User {
                id: Uuid::new_v4(),
                email: format!("testuser{}@example.com", i),
                password_hash: "$2b$12$dummy_hash".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            sqlx::query!(
                "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
                user.id,
                user.email,
                user.password_hash,
                user.created_at,
                user.updated_at
            )
            .execute(pool)
            .await?;

            // Create athlete profile
            let profile = MockDataGenerator::athlete_profile(user.id);
            sqlx::query!(
                "INSERT INTO athlete_profiles (id, user_id, sport, ftp, lthr, max_heart_rate, threshold_pace, zones, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                profile.id,
                profile.user_id,
                profile.sport,
                profile.ftp,
                profile.lthr,
                profile.max_heart_rate,
                profile.threshold_pace,
                profile.zones,
                profile.created_at,
                profile.updated_at
            )
            .execute(pool)
            .await?;

            // Create some training sessions
            for _ in 0..10 {
                let session = MockDataGenerator::training_session(user.id);
                sqlx::query!(
                    r#"
                    INSERT INTO training_sessions
                    (id, user_id, session_type, duration_minutes, distance_meters, avg_heart_rate,
                     max_heart_rate, avg_power, normalized_power, tss, if_, notes, perceived_exertion,
                     session_date, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
                    "#,
                    session.id,
                    session.user_id,
                    session.session_type,
                    session.duration_minutes,
                    session.distance_meters,
                    session.avg_heart_rate,
                    session.max_heart_rate,
                    session.avg_power,
                    session.normalized_power,
                    session.tss,
                    session.if_,
                    session.notes,
                    session.perceived_exertion,
                    session.session_date,
                    session.created_at,
                    session.updated_at
                )
                .execute(pool)
                .await?;
            }

            user_ids.push(user.id);
        }

        Ok(user_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_data_generation() {
        let user_id = Uuid::new_v4();

        // Test user generation
        let user = MockDataGenerator::user();
        assert!(!user.email.is_empty());
        assert!(!user.password_hash.is_empty());

        // Test athlete profile generation
        let profile = MockDataGenerator::athlete_profile(user_id);
        assert_eq!(profile.user_id, user_id);
        assert!(!profile.sport.is_empty());

        // Test training session generation
        let session = MockDataGenerator::training_session(user_id);
        assert_eq!(session.user_id, user_id);
        assert!(session.duration_minutes > 0);

        // Test goal generation
        let goal = MockDataGenerator::goal(user_id);
        assert_eq!(goal.user_id, user_id);
        assert!(!goal.title.is_empty());

        // Test event generation
        let event = MockDataGenerator::event(user_id);
        assert_eq!(event.user_id, user_id);
        assert!(!event.name.is_empty());
    }

    #[test]
    fn test_api_helpers() {
        let user_id = Uuid::new_v4();
        let token = ApiTestHelpers::create_test_token(user_id, "test@example.com", UserRole::Athlete);
        assert!(!token.is_empty());

        let (header_name, header_value) = ApiTestHelpers::auth_header(user_id, UserRole::Athlete);
        assert_eq!(header_name, "Authorization");
        assert!(header_value.starts_with("Bearer "));
    }
}