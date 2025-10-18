use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tower::ServiceExt;
use uuid::Uuid;

use ai_coach::api::routes::create_routes;
use crate::common::{TestDatabase, DatabaseTestHelpers};

#[cfg(test)]
mod admin_integration_tests {
    use super::*;

    /// Test helper to create the app router with test database
    async fn create_test_app(pool: SqlitePool) -> Router {
        create_routes(pool, "test_secret_key_for_testing_only")
    }

    /// Helper to create authenticated user with specific role
    async fn create_authenticated_user_with_role(
        app: Router,
        role: &str,
    ) -> (Router, String, Uuid) {
        let register_request = json!({
            "email": format!("test_{}@example.com", Uuid::new_v4()),
            "password": "SecurePass123!",
            "role": role
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header("Content-Type", "application/json")
            .body(Body::from(register_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let auth_response: Value = serde_json::from_str(&body_str).unwrap();

        let access_token = auth_response["access_token"].as_str().unwrap().to_string();
        let user_id = Uuid::parse_str(auth_response["user"]["id"].as_str().unwrap()).unwrap();

        (app, access_token, user_id)
    }

    // ========================================
    // List Users Tests (GET /admin/users)
    // ========================================

    #[tokio::test]
    async fn test_list_users_success_as_admin() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app, "admin").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users")
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let users: Value = serde_json::from_str(&body_str).unwrap();

        assert!(users.is_array());
    }

    #[tokio::test]
    async fn test_list_users_forbidden_as_athlete() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, athlete_token, _) = create_authenticated_user_with_role(app, "athlete").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users")
            .header("Authorization", format!("Bearer {}", athlete_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_list_users_forbidden_as_coach() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, coach_token, _) = create_authenticated_user_with_role(app, "coach").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users")
            .header("Authorization", format!("Bearer {}", coach_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_list_users_unauthorized() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // ========================================
    // Update User Role Tests (PUT /admin/users/:id/role)
    // ========================================

    #[tokio::test]
    async fn test_update_user_role_success_as_admin() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app.clone(), "admin").await;
        let (app, _, target_user_id) = create_authenticated_user_with_role(app, "athlete").await;

        let update_request = json!({
            "role": "coach"
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri(format!("/api/v1/admin/users/{}/role", target_user_id))
            .header("Authorization", format!("Bearer {}", admin_token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let message: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(message["message"].as_str().unwrap(), "User role updated successfully");
    }

    #[tokio::test]
    async fn test_update_user_role_forbidden_as_athlete() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, athlete_token, _) = create_authenticated_user_with_role(app.clone(), "athlete").await;
        let (app, _, target_user_id) = create_authenticated_user_with_role(app, "athlete").await;

        let update_request = json!({
            "role": "coach"
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri(format!("/api/v1/admin/users/{}/role", target_user_id))
            .header("Authorization", format!("Bearer {}", athlete_token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_update_user_role_forbidden_as_coach() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, coach_token, _) = create_authenticated_user_with_role(app.clone(), "coach").await;
        let (app, _, target_user_id) = create_authenticated_user_with_role(app, "athlete").await;

        let update_request = json!({
            "role": "admin"
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri(format!("/api/v1/admin/users/{}/role", target_user_id))
            .header("Authorization", format!("Bearer {}", coach_token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_update_user_role_unauthorized() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let random_user_id = Uuid::new_v4();
        let update_request = json!({
            "role": "coach"
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri(format!("/api/v1/admin/users/{}/role", random_user_id))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_update_user_role_invalid_role() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app.clone(), "admin").await;
        let (app, _, target_user_id) = create_authenticated_user_with_role(app, "athlete").await;

        let update_request = json!({
            "role": "superadmin"
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri(format!("/api/v1/admin/users/{}/role", target_user_id))
            .header("Authorization", format!("Bearer {}", admin_token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        // Should return 422 Unprocessable Entity or 400 Bad Request for invalid enum value
        assert!(
            response.status() == StatusCode::UNPROCESSABLE_ENTITY
                || response.status() == StatusCode::BAD_REQUEST
        );
    }

    // ========================================
    // Count Users Tests (GET /admin/users/count)
    // ========================================

    #[tokio::test]
    async fn test_count_users_success_as_admin() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app, "admin").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users/count")
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let count_response: Value = serde_json::from_str(&body_str).unwrap();

        assert!(count_response["count"].is_number());
        assert!(count_response["count"].as_i64().unwrap() >= 1); // At least the admin user
    }

    #[tokio::test]
    async fn test_count_users_with_role_filter() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app.clone(), "admin").await;
        let (app, _, _) = create_authenticated_user_with_role(app.clone(), "coach").await;
        let (app, _, _) = create_authenticated_user_with_role(app.clone(), "athlete").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users/count?role=coach")
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let count_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(count_response["count"].as_i64().unwrap(), 1);
        assert_eq!(count_response["role_filter"].as_str().unwrap(), "coach");
    }

    #[tokio::test]
    async fn test_count_users_invalid_role() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app, "admin").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users/count?role=invalid")
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_count_users_forbidden_as_athlete() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, athlete_token, _) = create_authenticated_user_with_role(app, "athlete").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users/count")
            .header("Authorization", format!("Bearer {}", athlete_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_count_users_unauthorized() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users/count")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // ========================================
    // Search Users Tests (GET /admin/users/search)
    // ========================================

    #[tokio::test]
    async fn test_search_users_by_email_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app, "admin").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users/search?email=test")
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let users: Value = serde_json::from_str(&body_str).unwrap();

        assert!(users.is_array());
        assert!(users.as_array().unwrap().len() >= 1);
    }

    #[tokio::test]
    async fn test_search_users_by_role_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app.clone(), "admin").await;
        let (app, _, _) = create_authenticated_user_with_role(app.clone(), "coach").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users/search?role=coach")
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let users: Value = serde_json::from_str(&body_str).unwrap();

        assert!(users.is_array());
        let users_array = users.as_array().unwrap();
        assert_eq!(users_array.len(), 1);
        assert_eq!(users_array[0]["role"].as_str().unwrap(), "coach");
    }

    #[tokio::test]
    async fn test_search_users_combined_filters() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app, "admin").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users/search?email=test&role=admin")
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let users: Value = serde_json::from_str(&body_str).unwrap();

        assert!(users.is_array());
    }

    #[tokio::test]
    async fn test_search_users_pagination() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app, "admin").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users/search?page=1&limit=10")
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let users: Value = serde_json::from_str(&body_str).unwrap();

        assert!(users.is_array());
        assert!(users.as_array().unwrap().len() <= 10);
    }

    #[tokio::test]
    async fn test_search_users_forbidden_as_coach() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, coach_token, _) = create_authenticated_user_with_role(app, "coach").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users/search?email=test")
            .header("Authorization", format!("Bearer {}", coach_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_search_users_unauthorized() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/users/search?email=test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // ========================================
    // Query Audit Logs Tests (GET /admin/audit-logs)
    // ========================================

    #[tokio::test]
    async fn test_query_audit_logs_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Create admin and perform an action that creates audit log
        let (app, admin_token, _) = create_authenticated_user_with_role(app.clone(), "admin").await;
        let (app, _, target_user_id) = create_authenticated_user_with_role(app.clone(), "athlete").await;

        // Update role to create audit log entry
        let update_request = json!({"role": "coach"});
        let update_req = Request::builder()
            .method(Method::PUT)
            .uri(format!("/api/v1/admin/users/{}/role", target_user_id))
            .header("Authorization", format!("Bearer {}", admin_token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();
        let _ = app.clone().oneshot(update_req).await.unwrap();

        // Now query audit logs
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/audit-logs")
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let logs: Value = serde_json::from_str(&body_str).unwrap();

        assert!(logs.is_array());
        let logs_array = logs.as_array().unwrap();
        assert!(logs_array.len() >= 1);

        // Verify audit log structure
        let first_log = &logs_array[0];
        assert!(first_log["id"].is_string());
        assert!(first_log["action"].is_string());
        assert!(first_log["created_at"].is_string());
    }

    #[tokio::test]
    async fn test_query_audit_logs_filter_by_action() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app.clone(), "admin").await;
        let (app, _, target_user_id) = create_authenticated_user_with_role(app.clone(), "athlete").await;

        // Create audit log entry
        let update_request = json!({"role": "coach"});
        let update_req = Request::builder()
            .method(Method::PUT)
            .uri(format!("/api/v1/admin/users/{}/role", target_user_id))
            .header("Authorization", format!("Bearer {}", admin_token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();
        let _ = app.clone().oneshot(update_req).await.unwrap();

        // Query with action filter
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/audit-logs?action=update_role")
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let logs: Value = serde_json::from_str(&body_str).unwrap();

        assert!(logs.is_array());
        let logs_array = logs.as_array().unwrap();
        for log in logs_array {
            assert_eq!(log["action"].as_str().unwrap(), "update_role");
        }
    }

    #[tokio::test]
    async fn test_query_audit_logs_pagination() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app, "admin").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/audit-logs?page=1&limit=50")
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let logs: Value = serde_json::from_str(&body_str).unwrap();

        assert!(logs.is_array());
        assert!(logs.as_array().unwrap().len() <= 50);
    }

    #[tokio::test]
    async fn test_query_audit_logs_forbidden_as_athlete() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, athlete_token, _) = create_authenticated_user_with_role(app, "athlete").await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/audit-logs")
            .header("Authorization", format!("Bearer {}", athlete_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_query_audit_logs_unauthorized() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/audit-logs")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // ========================================
    // Delete User Tests (DELETE /admin/users/:id)
    // ========================================

    #[tokio::test]
    async fn test_delete_user_success_as_admin() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app.clone(), "admin").await;
        let (app, _, target_user_id) = create_authenticated_user_with_role(app.clone(), "athlete").await;

        let request = Request::builder()
            .method(Method::DELETE)
            .uri(format!("/api/v1/admin/users/{}", target_user_id))
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let message: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(message["message"].as_str().unwrap(), "User deleted successfully");
    }

    #[tokio::test]
    async fn test_delete_user_self_deletion_prevented() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, admin_id) = create_authenticated_user_with_role(app, "admin").await;

        let request = Request::builder()
            .method(Method::DELETE)
            .uri(format!("/api/v1/admin/users/{}", admin_id))
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_delete_user_not_found() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app, "admin").await;
        let non_existent_id = Uuid::new_v4();

        let request = Request::builder()
            .method(Method::DELETE)
            .uri(format!("/api/v1/admin/users/{}", non_existent_id))
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_user_forbidden_as_coach() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, coach_token, _) = create_authenticated_user_with_role(app.clone(), "coach").await;
        let (app, _, target_user_id) = create_authenticated_user_with_role(app, "athlete").await;

        let request = Request::builder()
            .method(Method::DELETE)
            .uri(format!("/api/v1/admin/users/{}", target_user_id))
            .header("Authorization", format!("Bearer {}", coach_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_delete_user_unauthorized() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let random_user_id = Uuid::new_v4();
        let request = Request::builder()
            .method(Method::DELETE)
            .uri(format!("/api/v1/admin/users/{}", random_user_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_delete_user_creates_audit_log() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, admin_token, _) = create_authenticated_user_with_role(app.clone(), "admin").await;
        let (app, _, target_user_id) = create_authenticated_user_with_role(app.clone(), "athlete").await;

        // Delete user
        let delete_req = Request::builder()
            .method(Method::DELETE)
            .uri(format!("/api/v1/admin/users/{}", target_user_id))
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();
        let _ = app.clone().oneshot(delete_req).await.unwrap();

        // Check audit log
        let audit_req = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/admin/audit-logs?action=delete_user")
            .header("Authorization", format!("Bearer {}", admin_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(audit_req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let logs: Value = serde_json::from_str(&body_str).unwrap();

        assert!(logs.is_array());
        let logs_array = logs.as_array().unwrap();
        assert!(logs_array.len() >= 1);

        let deletion_log = &logs_array[0];
        assert_eq!(deletion_log["action"].as_str().unwrap(), "delete_user");
        assert!(deletion_log["old_value"].is_string());
    }
}
