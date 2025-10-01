use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use ai_coach::api::routes::create_routes;
use ai_coach::models::*;
use crate::common::{TestDatabase, DatabaseTestHelpers, ApiTestHelpers};

#[cfg(test)]
mod notification_integration_tests {
    use super::*;

    /// Test helper to create the app router with test database
    async fn create_test_app(pool: PgPool) -> Router {
        create_routes(pool, "test_secret_key_for_testing_only")
    }

    /// Test helper to create authenticated requests
    async fn create_authenticated_request(
        method: Method,
        uri: &str,
        body: Option<Value>,
        user_id: Uuid,
    ) -> Request<Body> {
        let (header_name, header_value) = ApiTestHelpers::auth_header(user_id, ai_coach::auth::UserRole::Athlete);

        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("Content-Type", "application/json")
            .header(header_name, header_value);

        if let Some(body_data) = body {
            builder.body(Body::from(body_data.to_string())).unwrap()
        } else {
            builder.body(Body::empty()).unwrap()
        }
    }

    /// Helper to create test user
    async fn create_test_user(pool: &PgPool) -> Uuid {
        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            email: format!("test{}@example.com", user_id),
            password_hash: "$2b$12$dummy_hash".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
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
        .await
        .unwrap();

        user_id
    }

    #[tokio::test]
    async fn test_notification_preferences_endpoints() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database and create test user
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();
        let user_id = create_test_user(&test_db.pool).await;

        // Test GET notification preferences (should return defaults)
        let request = create_authenticated_request(
            Method::GET,
            "/api/v1/notifications/preferences",
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let preferences: Value = serde_json::from_str(&body_str).unwrap();

        // Check default values
        assert_eq!(preferences["workout_reminders"], true);
        assert_eq!(preferences["email_enabled"], true);
        assert_eq!(preferences["quiet_hours_start"], "22:00");
        assert_eq!(preferences["quiet_hours_end"], "07:00");

        // Test UPDATE notification preferences
        let update_request = json!({
            "workout_reminders": false,
            "email_enabled": false,
            "quiet_hours_start": "23:00",
            "quiet_hours_end": "06:00"
        });

        let request = create_authenticated_request(
            Method::PUT,
            "/api/v1/notifications/preferences",
            Some(update_request),
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify the updates
        let request = create_authenticated_request(
            Method::GET,
            "/api/v1/notifications/preferences",
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let updated_preferences: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(updated_preferences["workout_reminders"], false);
        assert_eq!(updated_preferences["email_enabled"], false);
        assert_eq!(updated_preferences["quiet_hours_start"], "23:00");
        assert_eq!(updated_preferences["quiet_hours_end"], "06:00");
    }

    #[tokio::test]
    async fn test_notification_creation_and_retrieval() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database and create test user
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();
        let user_id = create_test_user(&test_db.pool).await;

        // Test CREATE notification
        let create_notification_request = json!({
            "notification_type": "workout_reminder",
            "title": "Workout Reminder",
            "message": "Time for your threshold training session",
            "delivery_channels": ["in_app", "email"],
            "scheduled_at": "2024-06-15T14:00:00Z"
        });

        let request = create_authenticated_request(
            Method::POST,
            "/api/v1/notifications",
            Some(create_notification_request),
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let notification: Value = serde_json::from_str(&body_str).unwrap();

        let notification_id = notification["id"].as_str().unwrap();
        assert_eq!(notification["title"], "Workout Reminder");
        assert_eq!(notification["delivery_status"], "scheduled");

        // Test GET notifications list
        let request = create_authenticated_request(
            Method::GET,
            "/api/v1/notifications",
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let notifications_list: Value = serde_json::from_str(&body_str).unwrap();

        assert!(notifications_list.is_array());
        assert_eq!(notifications_list.as_array().unwrap().len(), 1);

        // Test GET specific notification
        let request = create_authenticated_request(
            Method::GET,
            &format!("/api/v1/notifications/{}", notification_id),
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let notification_detail: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(notification_detail["title"], "Workout Reminder");
        assert_eq!(notification_detail["message"], "Time for your threshold training session");
    }

    #[tokio::test]
    async fn test_notification_marking_as_read() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database and create test user
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();
        let user_id = create_test_user(&test_db.pool).await;

        // Create a notification first
        let create_notification_request = json!({
            "notification_type": "goal_achievement",
            "title": "Goal Achieved!",
            "message": "Congratulations on reaching your power goal!",
            "delivery_channels": ["in_app"]
        });

        let request = create_authenticated_request(
            Method::POST,
            "/api/v1/notifications",
            Some(create_notification_request),
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let notification: Value = serde_json::from_str(&body_str).unwrap();

        let notification_id = notification["id"].as_str().unwrap();
        assert!(notification["read_at"].is_null());

        // Mark notification as read
        let request = create_authenticated_request(
            Method::PATCH,
            &format!("/api/v1/notifications/{}/read", notification_id),
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify notification is marked as read
        let request = create_authenticated_request(
            Method::GET,
            &format!("/api/v1/notifications/{}", notification_id),
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let updated_notification: Value = serde_json::from_str(&body_str).unwrap();

        assert!(updated_notification["read_at"].is_string());
    }

    #[tokio::test]
    async fn test_notification_filtering() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database and create test user
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();
        let user_id = create_test_user(&test_db.pool).await;

        // Create multiple notifications with different types
        let notification_types = vec![
            ("workout_reminder", "Workout Reminder", "training"),
            ("goal_achievement", "Goal Achieved", "performance"),
            ("overtraining_risk", "Overtraining Risk", "health"),
        ];

        for (notification_type, title, _category) in notification_types {
            let create_request = json!({
                "notification_type": notification_type,
                "title": title,
                "message": format!("Test message for {}", notification_type),
                "delivery_channels": ["in_app"]
            });

            let request = create_authenticated_request(
                Method::POST,
                "/api/v1/notifications",
                Some(create_request),
                user_id,
            ).await;

            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::CREATED);
        }

        // Test filtering by category
        let request = create_authenticated_request(
            Method::GET,
            "/api/v1/notifications?category=training",
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let filtered_notifications: Value = serde_json::from_str(&body_str).unwrap();

        assert!(filtered_notifications.is_array());
        let notifications = filtered_notifications.as_array().unwrap();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0]["category"], "training");

        // Test filtering by read status
        let request = create_authenticated_request(
            Method::GET,
            "/api/v1/notifications?read=false",
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let unread_notifications: Value = serde_json::from_str(&body_str).unwrap();

        assert!(unread_notifications.is_array());
        assert_eq!(unread_notifications.as_array().unwrap().len(), 3); // All should be unread
    }

    #[tokio::test]
    async fn test_notification_user_isolation() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database and create two test users
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();
        let user1_id = create_test_user(&test_db.pool).await;
        let user2_id = create_test_user(&test_db.pool).await;

        // User 1 creates a notification
        let create_notification_request = json!({
            "notification_type": "workout_reminder",
            "title": "User 1 Notification",
            "message": "This is for user 1 only",
            "delivery_channels": ["in_app"]
        });

        let request = create_authenticated_request(
            Method::POST,
            "/api/v1/notifications",
            Some(create_notification_request),
            user1_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // User 2 should not see User 1's notifications
        let request = create_authenticated_request(
            Method::GET,
            "/api/v1/notifications",
            None,
            user2_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let user2_notifications: Value = serde_json::from_str(&body_str).unwrap();

        assert!(user2_notifications.is_array());
        assert_eq!(user2_notifications.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_notification_delivery_status_updates() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database and create test user
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();
        let user_id = create_test_user(&test_db.pool).await;

        // Create a notification
        let create_notification_request = json!({
            "notification_type": "fitness_improvement",
            "title": "Fitness Improvement",
            "message": "Your fitness has improved by 5%",
            "delivery_channels": ["email", "in_app"]
        });

        let request = create_authenticated_request(
            Method::POST,
            "/api/v1/notifications",
            Some(create_notification_request),
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let notification: Value = serde_json::from_str(&body_str).unwrap();

        let notification_id = notification["id"].as_str().unwrap();
        assert_eq!(notification["delivery_status"], "scheduled");

        // Simulate delivery status update
        let status_update_request = json!({
            "delivery_status": "sent"
        });

        let request = create_authenticated_request(
            Method::PATCH,
            &format!("/api/v1/notifications/{}/status", notification_id),
            Some(status_update_request),
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify status update
        let request = create_authenticated_request(
            Method::GET,
            &format!("/api/v1/notifications/{}", notification_id),
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let updated_notification: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(updated_notification["delivery_status"], "sent");
        assert!(updated_notification["sent_at"].is_string());
    }

    #[tokio::test]
    async fn test_notification_metrics_endpoint() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database and create test user
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();
        let user_id = create_test_user(&test_db.pool).await;

        // Create several notifications with different statuses
        let notification_data = vec![
            ("sent", true),
            ("delivered", true),
            ("delivered", false),  // delivered but not read
            ("failed", false),
        ];

        for (status, read) in notification_data {
            let create_request = json!({
                "notification_type": "workout_reminder",
                "title": "Test Notification",
                "message": "Test message",
                "delivery_channels": ["in_app"]
            });

            let request = create_authenticated_request(
                Method::POST,
                "/api/v1/notifications",
                Some(create_request),
                user_id,
            ).await;

            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::CREATED);

            let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
            let body_str = String::from_utf8(body.to_vec()).unwrap();
            let notification: Value = serde_json::from_str(&body_str).unwrap();

            let notification_id = notification["id"].as_str().unwrap();

            // Update delivery status
            let status_update = json!({
                "delivery_status": status
            });

            let request = create_authenticated_request(
                Method::PATCH,
                &format!("/api/v1/notifications/{}/status", notification_id),
                Some(status_update),
                user_id,
            ).await;

            let _response = app.clone().oneshot(request).await.unwrap();

            // Mark as read if specified
            if read {
                let request = create_authenticated_request(
                    Method::PATCH,
                    &format!("/api/v1/notifications/{}/read", notification_id),
                    None,
                    user_id,
                ).await;

                let _response = app.clone().oneshot(request).await.unwrap();
            }
        }

        // Test metrics endpoint
        let request = create_authenticated_request(
            Method::GET,
            "/api/v1/notifications/metrics",
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let metrics: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(metrics["total_sent"], 4);
        assert_eq!(metrics["total_delivered"], 2);
        assert_eq!(metrics["total_read"], 2);
        assert!(metrics["delivery_rate"].is_number());
        assert!(metrics["read_rate"].is_number());
    }
}