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
use crate::common::{TestDatabase, DatabaseTestHelpers};

#[cfg(test)]
mod auth_integration_tests {
    use super::*;

    /// Test helper to create the app router with test database
    async fn create_test_app(pool: PgPool) -> Router {
        create_routes(pool, "test_secret_key_for_testing_only")
    }

    #[tokio::test]
    async fn test_user_registration() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let register_request = json!({
            "email": "newuser@example.com",
            "password": "SecurePassword123!"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header("Content-Type", "application/json")
            .body(Body::from(register_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let response_json: Value = serde_json::from_str(&body_str).unwrap();

        assert!(response_json["access_token"].is_string());
        assert!(response_json["user"]["id"].is_string());
        assert_eq!(response_json["user"]["email"], "newuser@example.com");
        assert!(response_json["user"]["password_hash"].is_null()); // Should not include password
    }

    #[tokio::test]
    async fn test_user_registration_validation() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Test invalid email
        let invalid_email_request = json!({
            "email": "invalid-email",
            "password": "SecurePassword123!"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header("Content-Type", "application/json")
            .body(Body::from(invalid_email_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test weak password
        let weak_password_request = json!({
            "email": "user@example.com",
            "password": "weak"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header("Content-Type", "application/json")
            .body(Body::from(weak_password_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_user_login() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // First register a user
        let register_request = json!({
            "email": "logintest@example.com",
            "password": "SecurePassword123!"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header("Content-Type", "application/json")
            .body(Body::from(register_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // Now test login
        let login_request = json!({
            "email": "logintest@example.com",
            "password": "SecurePassword123!"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header("Content-Type", "application/json")
            .body(Body::from(login_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let response_json: Value = serde_json::from_str(&body_str).unwrap();

        assert!(response_json["access_token"].is_string());
        assert!(response_json["refresh_token"].is_string());
        assert_eq!(response_json["user"]["email"], "logintest@example.com");
    }

    #[tokio::test]
    async fn test_login_invalid_credentials() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Test login with non-existent user
        let login_request = json!({
            "email": "nonexistent@example.com",
            "password": "AnyPassword123!"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header("Content-Type", "application/json")
            .body(Body::from(login_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let error_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(error_response["error_code"], "INVALID_CREDENTIALS");
    }

    #[tokio::test]
    async fn test_duplicate_email_registration() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let register_request = json!({
            "email": "duplicate@example.com",
            "password": "SecurePassword123!"
        });

        // First registration should succeed
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header("Content-Type", "application/json")
            .body(Body::from(register_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // Second registration with same email should fail
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header("Content-Type", "application/json")
            .body(Body::from(register_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let error_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(error_response["error_code"], "EMAIL_ALREADY_EXISTS");
    }

    #[tokio::test]
    async fn test_token_refresh() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Register and login to get tokens
        let register_request = json!({
            "email": "refreshtest@example.com",
            "password": "SecurePassword123!"
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

        let access_token = auth_response["access_token"].as_str().unwrap();
        let refresh_token = auth_response["refresh_token"].as_str().unwrap();

        // Test token refresh
        let refresh_request = json!({
            "refresh_token": refresh_token
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/refresh")
            .header("Content-Type", "application/json")
            .body(Body::from(refresh_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let refresh_response: Value = serde_json::from_str(&body_str).unwrap();

        assert!(refresh_response["access_token"].is_string());
        assert_ne!(refresh_response["access_token"], access_token); // Should be a new token
    }

    #[tokio::test]
    async fn test_invalid_refresh_token() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let refresh_request = json!({
            "refresh_token": "invalid_refresh_token"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/refresh")
            .header("Content-Type", "application/json")
            .body(Body::from(refresh_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let error_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(error_response["error_code"], "INVALID_REFRESH_TOKEN");
    }

    #[tokio::test]
    async fn test_logout() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Register and login to get tokens
        let register_request = json!({
            "email": "logouttest@example.com",
            "password": "SecurePassword123!"
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

        let access_token = auth_response["access_token"].as_str().unwrap();

        // Test logout
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/logout")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let logout_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(logout_response["message"], "Logged out successfully");
    }

    #[tokio::test]
    async fn test_password_reset_flow() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Register a user first
        let register_request = json!({
            "email": "resettest@example.com",
            "password": "OriginalPassword123!"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header("Content-Type", "application/json")
            .body(Body::from(register_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // Request password reset
        let reset_request = json!({
            "email": "resettest@example.com"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/forgot-password")
            .header("Content-Type", "application/json")
            .body(Body::from(reset_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let reset_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(reset_response["message"], "Password reset email sent");
    }

    #[tokio::test]
    async fn test_password_reset_invalid_email() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Request password reset for non-existent email
        let reset_request = json!({
            "email": "nonexistent@example.com"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/forgot-password")
            .header("Content-Type", "application/json")
            .body(Body::from(reset_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        // Should still return OK for security reasons (don't leak whether email exists)
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let reset_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(reset_response["message"], "Password reset email sent");
    }

    #[tokio::test]
    async fn test_user_profile_access() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Register a user
        let register_request = json!({
            "email": "profiletest@example.com",
            "password": "SecurePassword123!"
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

        let access_token = auth_response["access_token"].as_str().unwrap();

        // Access user profile with valid token
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let profile_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(profile_response["email"], "profiletest@example.com");
        assert!(profile_response["id"].is_string());
    }

    #[tokio::test]
    async fn test_email_case_insensitive() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Register with uppercase email
        let register_request = json!({
            "email": "CASETEST@EXAMPLE.COM",
            "password": "SecurePassword123!"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header("Content-Type", "application/json")
            .body(Body::from(register_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // Login with lowercase email
        let login_request = json!({
            "email": "casetest@example.com",
            "password": "SecurePassword123!"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header("Content-Type", "application/json")
            .body(Body::from(login_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let login_response: Value = serde_json::from_str(&body_str).unwrap();

        assert!(login_response["access_token"].is_string());
    }
}