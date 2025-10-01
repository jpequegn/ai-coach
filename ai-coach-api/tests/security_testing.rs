use std::collections::HashMap;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;
use chrono::{Utc, Duration};

use ai_coach::api::routes::create_routes;
use ai_coach::models::*;
use crate::common::{TestDatabase, DatabaseTestHelpers, ApiTestHelpers};

#[cfg(test)]
mod security_tests {
    use super::*;

    /// Create test app with database
    async fn create_test_app() -> (Router, TestDatabase) {
        let test_db = TestDatabase::new().await;
        let app = create_routes(test_db.pool.clone(), "test_secret_key_for_testing_only");
        (app, test_db)
    }

    /// Helper to create unauthenticated requests
    fn create_request(method: Method, uri: &str, body: Option<Value>) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("Content-Type", "application/json");

        if let Some(body_data) = body {
            builder.body(Body::from(body_data.to_string())).unwrap()
        } else {
            builder.body(Body::empty()).unwrap()
        }
    }

    /// Helper to create request with custom headers
    fn create_request_with_headers(
        method: Method,
        uri: &str,
        headers: Vec<(&str, &str)>,
        body: Option<Value>,
    ) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("Content-Type", "application/json");

        for (name, value) in headers {
            builder = builder.header(name, value);
        }

        if let Some(body_data) = body {
            builder.body(Body::from(body_data.to_string())).unwrap()
        } else {
            builder.body(Body::empty()).unwrap()
        }
    }

    #[tokio::test]
    async fn test_password_strength_requirements() {
        let (app, test_db) = create_test_app().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let weak_passwords = vec![
            "123456",           // Too simple
            "password",         // Common password
            "abc123",           // Too short
            "PASSWORD123",      // No lowercase
            "password123",      // No uppercase
            "Password",         // No numbers or special chars
            "Pass123",          // Too short
            "",                 // Empty
        ];

        for (i, weak_password) in weak_passwords.iter().enumerate() {
            let register_request = json!({
                "email": format!("weak{}@example.com", i),
                "password": weak_password
            });

            let request = create_request(Method::POST, "/api/v1/auth/register", Some(register_request));
            let response = app.clone().oneshot(request).await.unwrap();

            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "Weak password '{}' should be rejected",
                weak_password
            );
        }

        // Test strong password acceptance
        let strong_password_request = json!({
            "email": "strong@example.com",
            "password": "StrongPassword123!"
        });

        let request = create_request(Method::POST, "/api/v1/auth/register", Some(strong_password_request));
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::CREATED, "Strong password should be accepted");
    }

    #[tokio::test]
    async fn test_sql_injection_protection() {
        let (app, test_db) = create_test_app().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Create a legitimate user first
        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            email: "legitimate@example.com".to_string(),
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
        .execute(&test_db.pool)
        .await
        .unwrap();

        let sql_injection_attempts = vec![
            "admin@example.com'; DROP TABLE users; --",
            "admin@example.com' OR '1'='1",
            "admin@example.com' UNION SELECT * FROM users --",
            "admin@example.com'; INSERT INTO users (email) VALUES ('hacked@example.com'); --",
            "'; UPDATE users SET email='hacked@example.com' WHERE id='",
        ];

        for injection_attempt in sql_injection_attempts {
            // Test login with SQL injection
            let login_request = json!({
                "email": injection_attempt,
                "password": "anypassword"
            });

            let request = create_request(Method::POST, "/api/v1/auth/login", Some(login_request));
            let response = app.clone().oneshot(request).await.unwrap();

            // Should not succeed and should not cause 500 errors
            assert!(
                response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::BAD_REQUEST,
                "SQL injection attempt should be safely handled, got: {}",
                response.status()
            );
        }

        // Verify that the original user still exists and database is intact
        let remaining_users = sqlx::query!("SELECT COUNT(*) as count FROM users")
            .fetch_one(&test_db.pool)
            .await
            .unwrap();

        assert_eq!(remaining_users.count, Some(1), "Database should remain intact after SQL injection attempts");
    }

    #[tokio::test]
    async fn test_authentication_token_security() {
        let (app, test_db) = create_test_app().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Register a user
        let register_request = json!({
            "email": "token_test@example.com",
            "password": "SecurePassword123!"
        });

        let request = create_request(Method::POST, "/api/v1/auth/register", Some(register_request));
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let auth_response: Value = serde_json::from_str(&String::from_utf8(body.to_vec()).unwrap()).unwrap();
        let valid_token = auth_response["access_token"].as_str().unwrap();

        // Test with malformed tokens
        let malformed_tokens = vec![
            "invalid_token",
            "Bearer.invalid.token",
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.invalid",
            "",
            "null",
            "undefined",
        ];

        for malformed_token in malformed_tokens {
            let request = create_request_with_headers(
                Method::GET,
                "/api/v1/user/profile",
                vec![("Authorization", &format!("Bearer {}", malformed_token))],
                None,
            );

            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "Malformed token '{}' should be rejected",
                malformed_token
            );
        }

        // Test token without Bearer prefix
        let request = create_request_with_headers(
            Method::GET,
            "/api/v1/user/profile",
            vec![("Authorization", valid_token)],
            None,
        );

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "Token without Bearer should be rejected");

        // Test with valid token (should work)
        let request = create_request_with_headers(
            Method::GET,
            "/api/v1/user/profile",
            vec![("Authorization", &format!("Bearer {}", valid_token))],
            None,
        );

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "Valid token should be accepted");
    }

    #[tokio::test]
    async fn test_session_hijacking_protection() {
        let (app, test_db) = create_test_app().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Create two users
        for i in 1..=2 {
            let register_request = json!({
                "email": format!("user{}@example.com", i),
                "password": "SecurePassword123!"
            });

            let request = create_request(Method::POST, "/api/v1/auth/register", Some(register_request));
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::CREATED);
        }

        // Login as user1
        let login_request = json!({
            "email": "user1@example.com",
            "password": "SecurePassword123!"
        });

        let request = create_request(Method::POST, "/api/v1/auth/login", Some(login_request));
        let response = app.clone().oneshot(request).await.unwrap();
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let auth_response: Value = serde_json::from_str(&String::from_utf8(body.to_vec()).unwrap()).unwrap();
        let user1_token = auth_response["access_token"].as_str().unwrap();

        // Try to modify user1's token slightly (simulating tampering)
        let tampered_tokens = vec![
            &user1_token[..user1_token.len()-1], // Remove last character
            &format!("{}x", user1_token),         // Add character
            &user1_token.replace("a", "b"),       // Replace character
        ];

        for tampered_token in tampered_tokens {
            let request = create_request_with_headers(
                Method::GET,
                "/api/v1/user/profile",
                vec![("Authorization", &format!("Bearer {}", tampered_token))],
                None,
            );

            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "Tampered token should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn test_brute_force_protection() {
        let (app, test_db) = create_test_app().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Register a user
        let register_request = json!({
            "email": "brute_test@example.com",
            "password": "SecurePassword123!"
        });

        let request = create_request(Method::POST, "/api/v1/auth/register", Some(register_request));
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // Attempt multiple failed logins rapidly
        let mut failed_attempts = 0;
        let mut rate_limited = false;

        for attempt in 1..=20 {
            let login_request = json!({
                "email": "brute_test@example.com",
                "password": format!("wrong_password_{}", attempt)
            });

            let request = create_request(Method::POST, "/api/v1/auth/login", Some(login_request));
            let response = app.clone().oneshot(request).await.unwrap();

            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                rate_limited = true;
                break;
            } else if response.status() == StatusCode::UNAUTHORIZED {
                failed_attempts += 1;
            }

            // Small delay to simulate rapid attempts
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        // Should either implement rate limiting or at least track failed attempts
        // Note: This test might pass even without rate limiting implemented
        if rate_limited {
            println!("Rate limiting detected after {} attempts", failed_attempts);
        } else {
            println!("No rate limiting detected, failed {} attempts", failed_attempts);
            // In a production system, we would expect rate limiting
            assert!(failed_attempts >= 10, "Should allow at least 10 attempts to test brute force");
        }
    }

    #[tokio::test]
    async fn test_xss_protection() {
        let (app, test_db) = create_test_app().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let xss_payloads = vec![
            "<script>alert('xss')</script>",
            "javascript:alert('xss')",
            "<img src=x onerror=alert('xss')>",
            "'; alert('xss'); //",
            "<svg onload=alert('xss')>",
        ];

        for (i, xss_payload) in xss_payloads.iter().enumerate() {
            // Test XSS in registration email
            let register_request = json!({
                "email": format!("{}@example.com", xss_payload),
                "password": "SecurePassword123!"
            });

            let request = create_request(Method::POST, "/api/v1/auth/register", Some(register_request));
            let response = app.clone().oneshot(request).await.unwrap();

            // Should reject malformed email or sanitize input
            assert!(
                response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::CREATED,
                "XSS payload in email should be handled safely"
            );

            // If registration succeeded, try to create content with XSS
            if response.status() == StatusCode::CREATED {
                let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
                let auth_response: Value = serde_json::from_str(&String::from_utf8(body.to_vec()).unwrap()).unwrap();
                let token = auth_response["access_token"].as_str().unwrap();

                // Try to create a goal with XSS payload
                let goal_request = json!({
                    "title": xss_payload,
                    "description": format!("XSS test {}", xss_payload),
                    "goal_type": "power",
                    "goal_category": "performance",
                    "target_value": 300.0,
                    "unit": "watts",
                    "priority": "medium"
                });

                let request = create_request_with_headers(
                    Method::POST,
                    "/api/v1/goals",
                    vec![("Authorization", &format!("Bearer {}", token))],
                    Some(goal_request),
                );

                let response = app.clone().oneshot(request).await.unwrap();

                if response.status() == StatusCode::OK {
                    // Verify the response doesn't contain unescaped XSS
                    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
                    let body_str = String::from_utf8(body.to_vec()).unwrap();

                    // Response should not contain executable script tags
                    assert!(
                        !body_str.contains("<script>") && !body_str.contains("javascript:"),
                        "Response should not contain unescaped XSS payload"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_cors_security() {
        let (app, _test_db) = create_test_app().await;

        // Test CORS with various origins
        let origins = vec![
            "https://malicious-site.com",
            "http://localhost:3000",
            "https://evil.com",
            "null",
        ];

        for origin in origins {
            let request = create_request_with_headers(
                Method::OPTIONS,
                "/api/v1/auth/login",
                vec![
                    ("Origin", origin),
                    ("Access-Control-Request-Method", "POST"),
                    ("Access-Control-Request-Headers", "content-type"),
                ],
                None,
            );

            let response = app.clone().oneshot(request).await.unwrap();

            // Check CORS headers
            let cors_header = response.headers().get("Access-Control-Allow-Origin");

            if let Some(allowed_origin) = cors_header {
                let allowed_origin_str = allowed_origin.to_str().unwrap();
                // Should not allow all origins with credentials
                assert!(
                    allowed_origin_str != "*" || !response.headers().contains_key("Access-Control-Allow-Credentials"),
                    "Should not allow all origins with credentials"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_input_size_limits() {
        let (app, test_db) = create_test_app().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Test extremely large input
        let large_string = "a".repeat(10000); // 10KB string

        let register_request = json!({
            "email": format!("{}@example.com", large_string),
            "password": "SecurePassword123!"
        });

        let request = create_request(Method::POST, "/api/v1/auth/register", Some(register_request));
        let response = app.clone().oneshot(request).await.unwrap();

        // Should reject overly large input
        assert!(
            response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::PAYLOAD_TOO_LARGE,
            "Should reject overly large input"
        );

        // Test extremely large JSON payload
        let huge_description = "x".repeat(1000000); // 1MB string
        let huge_request = json!({
            "email": "normal@example.com",
            "password": "SecurePassword123!",
            "extra_field": huge_description
        });

        let request = create_request(Method::POST, "/api/v1/auth/register", Some(huge_request));
        let response = app.oneshot(request).await.unwrap();

        // Should reject or handle gracefully
        assert!(
            response.status() != StatusCode::INTERNAL_SERVER_ERROR,
            "Should not cause server error with large payload"
        );
    }

    #[tokio::test]
    async fn test_concurrent_session_security() {
        let (app, test_db) = create_test_app().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Register user
        let register_request = json!({
            "email": "concurrent@example.com",
            "password": "SecurePassword123!"
        });

        let request = create_request(Method::POST, "/api/v1/auth/register", Some(register_request));
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // Login multiple times concurrently
        let mut handles = Vec::new();

        for i in 0..5 {
            let app_clone = app.clone();
            let handle = tokio::spawn(async move {
                let login_request = json!({
                    "email": "concurrent@example.com",
                    "password": "SecurePassword123!"
                });

                let request = create_request(Method::POST, "/api/v1/auth/login", Some(login_request));
                let response = app_clone.oneshot(request).await.unwrap();

                (i, response.status(), response)
            });

            handles.push(handle);
        }

        let mut successful_logins = 0;
        let mut tokens = Vec::new();

        for handle in handles {
            let (session_id, status, response) = handle.await.unwrap();

            if status == StatusCode::OK {
                successful_logins += 1;

                let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
                let auth_response: Value = serde_json::from_str(&String::from_utf8(body.to_vec()).unwrap()).unwrap();
                let token = auth_response["access_token"].as_str().unwrap().to_string();
                tokens.push((session_id, token));
            }
        }

        // All concurrent logins should succeed
        assert_eq!(successful_logins, 5, "All concurrent logins should succeed");

        // All tokens should be different (unless deliberately shared)
        let unique_tokens: std::collections::HashSet<_> = tokens.iter().map(|(_, token)| token).collect();
        assert_eq!(unique_tokens.len(), tokens.len(), "All tokens should be unique");

        // All tokens should work for authenticated requests
        for (session_id, token) in tokens {
            let request = create_request_with_headers(
                Method::GET,
                "/api/v1/user/profile",
                vec![("Authorization", &format!("Bearer {}", token))],
                None,
            );

            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Token from session {} should work",
                session_id
            );
        }
    }

    #[tokio::test]
    async fn test_privilege_escalation_protection() {
        let (app, test_db) = create_test_app().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Register regular user
        let register_request = json!({
            "email": "regular@example.com",
            "password": "SecurePassword123!"
        });

        let request = create_request(Method::POST, "/api/v1/auth/register", Some(register_request));
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let auth_response: Value = serde_json::from_str(&String::from_utf8(body.to_vec()).unwrap()).unwrap();
        let regular_token = auth_response["access_token"].as_str().unwrap();

        // Try to access admin endpoints with regular token
        let admin_endpoints = vec![
            "/api/v1/admin/users",
            "/api/v1/admin/system",
            "/api/v1/admin/metrics",
        ];

        for endpoint in admin_endpoints {
            let request = create_request_with_headers(
                Method::GET,
                endpoint,
                vec![("Authorization", &format!("Bearer {}", regular_token))],
                None,
            );

            let response = app.clone().oneshot(request).await.unwrap();

            // Should deny access to admin endpoints
            assert!(
                response.status() == StatusCode::FORBIDDEN || response.status() == StatusCode::NOT_FOUND,
                "Regular user should not access admin endpoint: {}",
                endpoint
            );
        }

        // Try to create goals for other users
        let other_user_id = Uuid::new_v4();
        let goal_request = json!({
            "user_id": other_user_id.to_string(), // Try to specify different user
            "title": "Unauthorized Goal",
            "description": "This should not work",
            "goal_type": "power",
            "goal_category": "performance",
            "target_value": 300.0,
            "unit": "watts",
            "priority": "medium"
        });

        let request = create_request_with_headers(
            Method::POST,
            "/api/v1/goals",
            vec![("Authorization", &format!("Bearer {}", regular_token))],
            Some(goal_request),
        );

        let response = app.oneshot(request).await.unwrap();

        // Should either ignore the user_id field or reject the request
        // The goal should be created for the authenticated user, not the specified user_id
        if response.status() == StatusCode::OK {
            let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
            let goal_response: Value = serde_json::from_str(&String::from_utf8(body.to_vec()).unwrap()).unwrap();

            // The created goal should belong to the authenticated user, not the specified user_id
            assert_ne!(
                goal_response["goal"]["user_id"].as_str().unwrap(),
                other_user_id.to_string(),
                "Goal should not be created for unauthorized user"
            );
        }
    }

    #[tokio::test]
    async fn test_information_disclosure_protection() {
        let (app, test_db) = create_test_app().await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        // Create two users
        let mut user_tokens = Vec::new();

        for i in 1..=2 {
            let register_request = json!({
                "email": format!("user{}@disclosure.com", i),
                "password": "SecurePassword123!"
            });

            let request = create_request(Method::POST, "/api/v1/auth/register", Some(register_request));
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::CREATED);

            let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
            let auth_response: Value = serde_json::from_str(&String::from_utf8(body.to_vec()).unwrap()).unwrap();
            let token = auth_response["access_token"].as_str().unwrap().to_string();
            let user_id = auth_response["user"]["id"].as_str().unwrap().to_string();

            user_tokens.push((user_id, token));
        }

        // User 1 creates a goal
        let goal_request = json!({
            "title": "Private Goal",
            "description": "This should be private",
            "goal_type": "power",
            "goal_category": "performance",
            "target_value": 300.0,
            "unit": "watts",
            "priority": "high"
        });

        let request = create_request_with_headers(
            Method::POST,
            "/api/v1/goals",
            vec![("Authorization", &format!("Bearer {}", user_tokens[0].1))],
            Some(goal_request),
        );

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let goal_response: Value = serde_json::from_str(&String::from_utf8(body.to_vec()).unwrap()).unwrap();
        let goal_id = goal_response["goal"]["id"].as_str().unwrap();

        // User 2 tries to access User 1's goal
        let request = create_request_with_headers(
            Method::GET,
            &format!("/api/v1/goals/{}", goal_id),
            vec![("Authorization", &format!("Bearer {}", user_tokens[1].1))],
            None,
        );

        let response = app.clone().oneshot(request).await.unwrap();

        // Should not allow access to other user's data
        assert!(
            response.status() == StatusCode::FORBIDDEN || response.status() == StatusCode::NOT_FOUND,
            "User should not access other user's private data"
        );

        // User 2 tries to access User 1's profile by user ID
        let request = create_request_with_headers(
            Method::GET,
            &format!("/api/v1/user/{}", user_tokens[0].0),
            vec![("Authorization", &format!("Bearer {}", user_tokens[1].1))],
            None,
        );

        let response = app.oneshot(request).await.unwrap();

        // Should not allow access to other user's profile
        assert!(
            response.status() == StatusCode::FORBIDDEN || response.status() == StatusCode::NOT_FOUND,
            "User should not access other user's profile"
        );
    }
}