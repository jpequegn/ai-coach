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
use crate::common::{TestDatabase, MockDataGenerator, ApiTestHelpers, DatabaseTestHelpers};

#[cfg(test)]
mod api_integration_tests {
    use super::*;

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

    /// Test helper to create the app router with test database
    async fn create_test_app(pool: PgPool) -> Router {
        create_routes(pool, "test_secret_key_for_testing_only")
    }

    #[tokio::test]
    async fn test_health_check_endpoint() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        let json_response: Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(json_response["status"], "healthy");
    }

    #[tokio::test]
    async fn test_goals_crud_endpoints() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Create test user
        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            email: "test@example.com".to_string(),
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
        .execute(&test_db.pool)
        .await
        .unwrap();

        // Test CREATE goal
        let create_goal_request = json!({
            "title": "Test Goal",
            "description": "A test goal for integration testing",
            "goal_type": "power",
            "goal_category": "performance",
            "target_value": 350.0,
            "unit": "watts",
            "target_date": "2024-12-31",
            "priority": "high"
        });

        let request = create_authenticated_request(
            Method::POST,
            "/api/v1/goals",
            Some(create_goal_request),
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let goal_response: Value = serde_json::from_str(&body_str).unwrap();

        let goal_id = goal_response["goal"]["id"].as_str().unwrap();
        let goal_uuid = Uuid::parse_str(goal_id).unwrap();

        // Test GET goals list
        let request = create_authenticated_request(
            Method::GET,
            "/api/v1/goals",
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let goals_list: Value = serde_json::from_str(&body_str).unwrap();

        assert!(goals_list.is_array());
        assert_eq!(goals_list.as_array().unwrap().len(), 1);

        // Test GET specific goal
        let request = create_authenticated_request(
            Method::GET,
            &format!("/api/v1/goals/{}", goal_id),
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let goal_detail: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(goal_detail["goal"]["title"], "Test Goal");
        assert_eq!(goal_detail["success"], true);

        // Test UPDATE goal
        let update_goal_request = json!({
            "title": "Updated Test Goal",
            "current_value": 100.0
        });

        let request = create_authenticated_request(
            Method::PUT,
            &format!("/api/v1/goals/{}", goal_id),
            Some(update_goal_request),
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test DELETE goal
        let request = create_authenticated_request(
            Method::DELETE,
            &format!("/api/v1/goals/{}", goal_id),
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify goal is deleted
        let request = create_authenticated_request(
            Method::GET,
            &format!("/api/v1/goals/{}", goal_id),
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_authentication_required() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Test unauthenticated request
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/goals")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_invalid_authentication_token() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Test with invalid token
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/goals")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer invalid_token")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_user_isolation() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Create two test users
        let user1_id = Uuid::new_v4();
        let user2_id = Uuid::new_v4();

        for user_id in [user1_id, user2_id] {
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
            .execute(&test_db.pool)
            .await
            .unwrap();
        }

        // User 1 creates a goal
        let create_goal_request = json!({
            "title": "User 1 Goal",
            "description": "Goal for user 1",
            "goal_type": "power",
            "goal_category": "performance",
            "target_value": 300.0,
            "unit": "watts",
            "priority": "medium"
        });

        let request = create_authenticated_request(
            Method::POST,
            "/api/v1/goals",
            Some(create_goal_request),
            user1_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // User 2 should not see User 1's goals
        let request = create_authenticated_request(
            Method::GET,
            "/api/v1/goals",
            None,
            user2_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let goals_list: Value = serde_json::from_str(&body_str).unwrap();

        assert!(goals_list.is_array());
        assert_eq!(goals_list.as_array().unwrap().len(), 0); // User 2 should see no goals
    }

    #[tokio::test]
    async fn test_input_validation() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database and create test user
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();
        let user_id = Uuid::new_v4();

        let user = User {
            id: user_id,
            email: "test@example.com".to_string(),
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
        .execute(&test_db.pool)
        .await
        .unwrap();

        // Test empty title validation
        let invalid_goal_request = json!({
            "title": "",
            "description": "Goal with empty title",
            "goal_type": "power",
            "goal_category": "performance",
            "priority": "medium"
        });

        let request = create_authenticated_request(
            Method::POST,
            "/api/v1/goals",
            Some(invalid_goal_request),
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let error_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(error_response["error_code"], "INVALID_TITLE");
    }

    #[tokio::test]
    async fn test_error_handling() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let user_id = Uuid::new_v4();

        // Test accessing non-existent goal
        let request = create_authenticated_request(
            Method::GET,
            &format!("/api/v1/goals/{}", Uuid::new_v4()),
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let error_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(error_response["error_code"], "GOAL_NOT_FOUND");
    }

    #[tokio::test]
    async fn test_goal_progress_endpoints() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database and create test user with goal
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();
        let user_id = Uuid::new_v4();

        let user = User {
            id: user_id,
            email: "test@example.com".to_string(),
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
        .execute(&test_db.pool)
        .await
        .unwrap();

        // Create a goal first
        let create_goal_request = json!({
            "title": "Progress Test Goal",
            "description": "Goal for testing progress tracking",
            "goal_type": "power",
            "goal_category": "performance",
            "target_value": 400.0,
            "unit": "watts",
            "priority": "high"
        });

        let request = create_authenticated_request(
            Method::POST,
            "/api/v1/goals",
            Some(create_goal_request),
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let goal_response: Value = serde_json::from_str(&body_str).unwrap();

        let goal_id = goal_response["goal"]["id"].as_str().unwrap();

        // Add progress to the goal
        let add_progress_request = json!({
            "value": 250.0,
            "note": "Good progress this week",
            "milestone_achieved": "Milestone 1"
        });

        let request = create_authenticated_request(
            Method::POST,
            &format!("/api/v1/goals/{}/progress", goal_id),
            Some(add_progress_request),
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Get progress summary
        let request = create_authenticated_request(
            Method::GET,
            &format!("/api/v1/goals/{}/progress", goal_id),
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let progress_response: Value = serde_json::from_str(&body_str).unwrap();

        assert!(progress_response["progress_percentage"].is_number());
        assert!(progress_response["recent_entries"].is_array());
    }

    #[tokio::test]
    async fn test_pagination() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database and create test user
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();
        let user_id = Uuid::new_v4();

        let user = User {
            id: user_id,
            email: "test@example.com".to_string(),
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
        .execute(&test_db.pool)
        .await
        .unwrap();

        // Create multiple goals
        for i in 1..=5 {
            let create_goal_request = json!({
                "title": format!("Goal {}", i),
                "description": format!("Description for goal {}", i),
                "goal_type": "power",
                "goal_category": "performance",
                "target_value": 300.0 + (i as f64 * 10.0),
                "unit": "watts",
                "priority": "medium"
            });

            let request = create_authenticated_request(
                Method::POST,
                "/api/v1/goals",
                Some(create_goal_request),
                user_id,
            ).await;

            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        // Test pagination with limit
        let request = create_authenticated_request(
            Method::GET,
            "/api/v1/goals?limit=3",
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let goals_list: Value = serde_json::from_str(&body_str).unwrap();

        assert!(goals_list.is_array());
        assert_eq!(goals_list.as_array().unwrap().len(), 3);

        // Test pagination with offset
        let request = create_authenticated_request(
            Method::GET,
            "/api/v1/goals?limit=3&offset=3",
            None,
            user_id,
        ).await;

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let goals_list: Value = serde_json::from_str(&body_str).unwrap();

        assert!(goals_list.is_array());
        assert_eq!(goals_list.as_array().unwrap().len(), 2); // Should have 2 remaining goals
    }
}