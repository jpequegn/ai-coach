use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Utc, NaiveDate, Duration};

use ai_coach::models::*;
use crate::common::{TestDatabase, MockDataGenerator, DatabaseTestHelpers};

#[cfg(test)]
mod database_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_database_connection_and_migrations() {
        let test_db = TestDatabase::new().await;

        // Test that we can execute a simple query
        let result = sqlx::query!("SELECT 1 as test_value")
            .fetch_one(&test_db.pool)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().test_value, Some(1));
    }

    #[tokio::test]
    async fn test_user_table_operations() {
        let test_db = TestDatabase::new().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let user = MockDataGenerator::user();

        // Test INSERT
        let insert_result = sqlx::query!(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            user.id,
            user.email,
            user.password_hash,
            user.created_at,
            user.updated_at
        )
        .execute(&test_db.pool)
        .await;

        assert!(insert_result.is_ok());
        assert_eq!(insert_result.unwrap().rows_affected(), 1);

        // Test SELECT
        let fetched_user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, created_at, updated_at FROM users WHERE id = $1",
            user.id
        )
        .fetch_one(&test_db.pool)
        .await;

        assert!(fetched_user.is_ok());
        let fetched_user = fetched_user.unwrap();
        assert_eq!(fetched_user.id, user.id);
        assert_eq!(fetched_user.email, user.email);
        assert_eq!(fetched_user.password_hash, user.password_hash);

        // Test UPDATE
        let new_email = "updated@example.com";
        let update_result = sqlx::query!(
            "UPDATE users SET email = $1, updated_at = $2 WHERE id = $3",
            new_email,
            Utc::now(),
            user.id
        )
        .execute(&test_db.pool)
        .await;

        assert!(update_result.is_ok());
        assert_eq!(update_result.unwrap().rows_affected(), 1);

        // Verify update
        let updated_user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, created_at, updated_at FROM users WHERE id = $1",
            user.id
        )
        .fetch_one(&test_db.pool)
        .await
        .unwrap();

        assert_eq!(updated_user.email, new_email);

        // Test DELETE
        let delete_result = sqlx::query!(
            "DELETE FROM users WHERE id = $1",
            user.id
        )
        .execute(&test_db.pool)
        .await;

        assert!(delete_result.is_ok());
        assert_eq!(delete_result.unwrap().rows_affected(), 1);

        // Verify deletion
        let deleted_user = sqlx::query_as!(
            User,
            "SELECT id, email, password_hash, created_at, updated_at FROM users WHERE id = $1",
            user.id
        )
        .fetch_optional(&test_db.pool)
        .await
        .unwrap();

        assert!(deleted_user.is_none());
    }

    #[tokio::test]
    async fn test_training_session_table_operations() {
        let test_db = TestDatabase::new().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Create a user first
        let user = MockDataGenerator::user();
        sqlx::query!(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            user.id,
            user.email,
            user.password_hash,
            user.created_at,
            user.updated_at
        )
        .execute(&test_db.pool)
        .await
        .unwrap();

        let session = MockDataGenerator::training_session(user.id);

        // Test INSERT training session
        let insert_result = sqlx::query!(
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
        .execute(&test_db.pool)
        .await;

        assert!(insert_result.is_ok());
        assert_eq!(insert_result.unwrap().rows_affected(), 1);

        // Test SELECT training session
        let fetched_session = sqlx::query_as!(
            TrainingSession,
            r#"
            SELECT id, user_id, session_type, duration_minutes, distance_meters, avg_heart_rate,
                   max_heart_rate, avg_power, normalized_power, tss, if_, notes, perceived_exertion,
                   session_date, created_at, updated_at
            FROM training_sessions WHERE id = $1
            "#,
            session.id
        )
        .fetch_one(&test_db.pool)
        .await;

        assert!(fetched_session.is_ok());
        let fetched_session = fetched_session.unwrap();
        assert_eq!(fetched_session.id, session.id);
        assert_eq!(fetched_session.user_id, session.user_id);
        assert_eq!(fetched_session.duration_minutes, session.duration_minutes);
        assert_eq!(fetched_session.avg_heart_rate, session.avg_heart_rate);
    }

    #[tokio::test]
    async fn test_goal_table_operations() {
        let test_db = TestDatabase::new().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Create a user first
        let user = MockDataGenerator::user();
        sqlx::query!(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            user.id,
            user.email,
            user.password_hash,
            user.created_at,
            user.updated_at
        )
        .execute(&test_db.pool)
        .await
        .unwrap();

        let goal = MockDataGenerator::goal(user.id);

        // Test INSERT goal
        let insert_result = sqlx::query!(
            r#"
            INSERT INTO goals
            (id, user_id, title, description, goal_type, goal_category,
             target_value, current_value, unit, target_date, status, priority,
             event_id, parent_goal_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            "#,
            goal.id,
            goal.user_id,
            goal.title,
            goal.description,
            goal.goal_type as GoalType,
            goal.goal_category as GoalCategory,
            goal.target_value,
            goal.current_value,
            goal.unit,
            goal.target_date,
            goal.status as GoalStatus,
            goal.priority as GoalPriority,
            goal.event_id,
            goal.parent_goal_id,
            goal.created_at,
            goal.updated_at
        )
        .execute(&test_db.pool)
        .await;

        assert!(insert_result.is_ok());
        assert_eq!(insert_result.unwrap().rows_affected(), 1);

        // Test SELECT goal
        let fetched_goal = sqlx::query_as!(
            Goal,
            r#"
            SELECT id, user_id, title, description,
                   goal_type as "goal_type: GoalType",
                   goal_category as "goal_category: GoalCategory",
                   target_value, current_value, unit, target_date,
                   status as "status: GoalStatus",
                   priority as "priority: GoalPriority",
                   event_id, parent_goal_id, created_at, updated_at
            FROM goals WHERE id = $1
            "#,
            goal.id
        )
        .fetch_one(&test_db.pool)
        .await;

        assert!(fetched_goal.is_ok());
        let fetched_goal = fetched_goal.unwrap();
        assert_eq!(fetched_goal.id, goal.id);
        assert_eq!(fetched_goal.user_id, goal.user_id);
        assert_eq!(fetched_goal.title, goal.title);
        assert_eq!(fetched_goal.goal_type, goal.goal_type);
    }

    #[tokio::test]
    async fn test_foreign_key_constraints() {
        let test_db = TestDatabase::new().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let user_id = Uuid::new_v4();
        let session = MockDataGenerator::training_session(user_id);

        // Attempt to insert training session without user (should fail)
        let insert_result = sqlx::query!(
            r#"
            INSERT INTO training_sessions
            (id, user_id, session_type, duration_minutes, session_date, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            session.id,
            user_id,
            session.session_type,
            session.duration_minutes,
            session.session_date,
            session.created_at,
            session.updated_at
        )
        .execute(&test_db.pool)
        .await;

        // Should fail due to foreign key constraint
        assert!(insert_result.is_err());

        // Create user first
        let user = MockDataGenerator::user();
        sqlx::query!(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            user.id,
            user.email,
            user.password_hash,
            user.created_at,
            user.updated_at
        )
        .execute(&test_db.pool)
        .await
        .unwrap();

        let valid_session = MockDataGenerator::training_session(user.id);

        // Now insert should succeed
        let insert_result = sqlx::query!(
            r#"
            INSERT INTO training_sessions
            (id, user_id, session_type, duration_minutes, session_date, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            valid_session.id,
            valid_session.user_id,
            valid_session.session_type,
            valid_session.duration_minutes,
            valid_session.session_date,
            valid_session.created_at,
            valid_session.updated_at
        )
        .execute(&test_db.pool)
        .await;

        assert!(insert_result.is_ok());
        assert_eq!(insert_result.unwrap().rows_affected(), 1);
    }

    #[tokio::test]
    async fn test_unique_constraints() {
        let test_db = TestDatabase::new().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let user1 = MockDataGenerator::user();

        // Insert first user
        sqlx::query!(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            user1.id,
            user1.email,
            user1.password_hash,
            user1.created_at,
            user1.updated_at
        )
        .execute(&test_db.pool)
        .await
        .unwrap();

        let user2 = User {
            id: Uuid::new_v4(),
            email: user1.email.clone(), // Same email
            ..user1
        };

        // Attempt to insert second user with same email (should fail)
        let insert_result = sqlx::query!(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            user2.id,
            user2.email,
            user2.password_hash,
            user2.created_at,
            user2.updated_at
        )
        .execute(&test_db.pool)
        .await;

        // Should fail due to unique constraint on email
        assert!(insert_result.is_err());
    }

    #[tokio::test]
    async fn test_data_types_and_constraints() {
        let test_db = TestDatabase::new().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Create user
        let user = MockDataGenerator::user();
        sqlx::query!(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            user.id,
            user.email,
            user.password_hash,
            user.created_at,
            user.updated_at
        )
        .execute(&test_db.pool)
        .await
        .unwrap();

        // Test edge cases for training session data
        let session = TrainingSession {
            id: Uuid::new_v4(),
            user_id: user.id,
            session_type: Some("endurance".to_string()),
            duration_minutes: 1, // Minimum duration
            distance_meters: Some(0.1), // Very small distance
            avg_heart_rate: Some(40), // Minimum reasonable HR
            max_heart_rate: Some(220), // Maximum reasonable HR
            avg_power: Some(1), // Minimum power
            normalized_power: Some(1),
            tss: Some(0.1), // Very low TSS
            if_: Some(0.0), // Minimum IF
            notes: Some("Test with extreme values".to_string()),
            perceived_exertion: Some(1), // Minimum RPE
            session_date: NaiveDate::from_ymd_opt(1900, 1, 1).unwrap(), // Very old date
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let insert_result = sqlx::query!(
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
        .execute(&test_db.pool)
        .await;

        assert!(insert_result.is_ok());
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let test_db = TestDatabase::new().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let user1 = MockDataGenerator::user();
        let user2 = MockDataGenerator::user();

        // Start a transaction
        let mut tx = test_db.pool.begin().await.unwrap();

        // Insert first user successfully
        let insert1_result = sqlx::query!(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            user1.id,
            user1.email,
            user1.password_hash,
            user1.created_at,
            user1.updated_at
        )
        .execute(&mut *tx)
        .await;

        assert!(insert1_result.is_ok());

        // Attempt to insert second user with same email (will fail)
        let user2_with_duplicate_email = User {
            email: user1.email.clone(),
            ..user2
        };

        let insert2_result = sqlx::query!(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            user2_with_duplicate_email.id,
            user2_with_duplicate_email.email,
            user2_with_duplicate_email.password_hash,
            user2_with_duplicate_email.created_at,
            user2_with_duplicate_email.updated_at
        )
        .execute(&mut *tx)
        .await;

        assert!(insert2_result.is_err());

        // Rollback the transaction
        tx.rollback().await.unwrap();

        // Verify that neither user exists in the database
        let user_count = sqlx::query!("SELECT COUNT(*) as count FROM users")
            .fetch_one(&test_db.pool)
            .await
            .unwrap();

        assert_eq!(user_count.count, Some(0));
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        let test_db = TestDatabase::new().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let user = MockDataGenerator::user();
        let session = MockDataGenerator::training_session(user.id);

        // Start a transaction
        let mut tx = test_db.pool.begin().await.unwrap();

        // Insert user
        sqlx::query!(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            user.id,
            user.email,
            user.password_hash,
            user.created_at,
            user.updated_at
        )
        .execute(&mut *tx)
        .await
        .unwrap();

        // Insert training session
        sqlx::query!(
            r#"
            INSERT INTO training_sessions
            (id, user_id, session_type, duration_minutes, session_date, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            session.id,
            session.user_id,
            session.session_type,
            session.duration_minutes,
            session.session_date,
            session.created_at,
            session.updated_at
        )
        .execute(&mut *tx)
        .await
        .unwrap();

        // Commit the transaction
        tx.commit().await.unwrap();

        // Verify both records exist
        let user_count = sqlx::query!("SELECT COUNT(*) as count FROM users")
            .fetch_one(&test_db.pool)
            .await
            .unwrap();

        let session_count = sqlx::query!("SELECT COUNT(*) as count FROM training_sessions")
            .fetch_one(&test_db.pool)
            .await
            .unwrap();

        assert_eq!(user_count.count, Some(1));
        assert_eq!(session_count.count, Some(1));
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let test_db = TestDatabase::new().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Create users
        let user1 = MockDataGenerator::user();
        let user2 = MockDataGenerator::user();

        // Insert users
        for user in [&user1, &user2] {
            sqlx::query!(
                "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
                user.id,
                user.email,
                user.password_hash,
                user.created_at,
                user.updated_at
            )
            .execute(&test_db.pool)
            .await
            .unwrap();
        }

        // Simulate concurrent operations
        let pool1 = test_db.pool.clone();
        let pool2 = test_db.pool.clone();

        let task1 = tokio::spawn(async move {
            let session = MockDataGenerator::training_session(user1.id);
            sqlx::query!(
                r#"
                INSERT INTO training_sessions
                (id, user_id, session_type, duration_minutes, session_date, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
                session.id,
                session.user_id,
                session.session_type,
                session.duration_minutes,
                session.session_date,
                session.created_at,
                session.updated_at
            )
            .execute(&pool1)
            .await
        });

        let task2 = tokio::spawn(async move {
            let session = MockDataGenerator::training_session(user2.id);
            sqlx::query!(
                r#"
                INSERT INTO training_sessions
                (id, user_id, session_type, duration_minutes, session_date, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
                session.id,
                session.user_id,
                session.session_type,
                session.duration_minutes,
                session.session_date,
                session.created_at,
                session.updated_at
            )
            .execute(&pool2)
            .await
        });

        let (result1, result2) = tokio::join!(task1, task2);

        assert!(result1.is_ok() && result1.unwrap().is_ok());
        assert!(result2.is_ok() && result2.unwrap().is_ok());

        // Verify both sessions were inserted
        let session_count = sqlx::query!("SELECT COUNT(*) as count FROM training_sessions")
            .fetch_one(&test_db.pool)
            .await
            .unwrap();

        assert_eq!(session_count.count, Some(2));
    }

    #[tokio::test]
    async fn test_database_performance() {
        let test_db = TestDatabase::new().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Create a user
        let user = MockDataGenerator::user();
        sqlx::query!(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
            user.id,
            user.email,
            user.password_hash,
            user.created_at,
            user.updated_at
        )
        .execute(&test_db.pool)
        .await
        .unwrap();

        let start_time = std::time::Instant::now();

        // Insert 100 training sessions
        for i in 0..100 {
            let session = TrainingSession {
                id: Uuid::new_v4(),
                user_id: user.id,
                session_type: Some(format!("session_{}", i)),
                duration_minutes: 60 + i,
                distance_meters: Some(30000.0),
                avg_heart_rate: Some(150),
                max_heart_rate: Some(180),
                avg_power: Some(250),
                normalized_power: Some(260),
                tss: Some(100.0),
                if_: Some(0.8),
                notes: Some(format!("Session {}", i)),
                perceived_exertion: Some(7),
                session_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap() + Duration::days(i as i64),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

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
            .execute(&test_db.pool)
            .await
            .unwrap();
        }

        let insert_duration = start_time.elapsed();

        // Test query performance
        let query_start = std::time::Instant::now();

        let sessions = sqlx::query!(
            "SELECT id, session_type, duration_minutes FROM training_sessions WHERE user_id = $1 ORDER BY session_date DESC LIMIT 10",
            user.id
        )
        .fetch_all(&test_db.pool)
        .await
        .unwrap();

        let query_duration = query_start.elapsed();

        assert_eq!(sessions.len(), 10);

        // Performance assertions (these thresholds are reasonable for testing)
        assert!(insert_duration.as_millis() < 5000, "Insert operation took too long: {:?}", insert_duration);
        assert!(query_duration.as_millis() < 100, "Query operation took too long: {:?}", query_duration);

        println!("Performance test results:");
        println!("  Insert 100 sessions: {:?}", insert_duration);
        println!("  Query 10 sessions: {:?}", query_duration);
    }
}