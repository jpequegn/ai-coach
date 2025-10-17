use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::Value;
use sqlx::SqlitePool;
use tower::ServiceExt;

use ai_coach::api::routes::create_routes;
use crate::common::{TestDatabase, DatabaseTestHelpers};

#[cfg(test)]
mod health_check_integration_tests {
    use super::*;

    /// Test helper to create the app router with test database
    async fn create_test_app(pool: SqlitePool) -> Router {
        create_routes(pool, "test_secret_key_for_testing_only")
    }

    // ========================================
    // Basic Health Check Tests (GET /health)
    // ========================================

    #[tokio::test]
    async fn test_health_check_basic_success() {
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
        let health_response: Value = serde_json::from_str(&body_str).unwrap();

        // Verify response structure
        assert_eq!(health_response["status"], "healthy");
        assert_eq!(health_response["service"], "ai-coach");
        assert!(health_response["version"].is_string());
        assert!(health_response["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_health_check_basic_always_returns_ok() {
        // Basic health check should always return 200 OK
        // even if there are underlying issues (it's a simple liveness check)
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Make multiple requests to ensure consistency
        for _ in 0..3 {
            let request = Request::builder()
                .method(Method::GET)
                .uri("/health")
                .body(Body::empty())
                .unwrap();

            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    // ========================================
    // Detailed Health Check Tests (GET /health/detailed)
    // ========================================

    #[tokio::test]
    async fn test_health_check_detailed_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health/detailed")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let health_response: Value = serde_json::from_str(&body_str).unwrap();

        // Verify overall response structure
        assert!(health_response["status"].is_string());
        assert_eq!(health_response["service"], "ai-coach");
        assert!(health_response["version"].is_string());
        assert!(health_response["timestamp"].is_string());

        // Verify components section exists
        assert!(health_response["components"].is_object());

        // Verify database component
        let db_health = &health_response["components"]["database"];
        assert!(db_health["status"].is_string());
        assert!(db_health["latency_ms"].is_number() || db_health["latency_ms"].is_null());

        // Verify Redis component
        let redis_health = &health_response["components"]["redis"];
        assert!(redis_health["status"].is_string());
    }

    #[tokio::test]
    async fn test_health_check_detailed_database_component() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health/detailed")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let health_response: Value = serde_json::from_str(&body_str).unwrap();

        let db_health = &health_response["components"]["database"];

        // Database should be healthy with in-memory SQLite
        assert_eq!(db_health["status"], "healthy");

        // Should have latency measurement
        assert!(db_health["latency_ms"].is_number());
        let latency = db_health["latency_ms"].as_i64().unwrap();
        assert!(latency >= 0);

        // Should have connection pool details
        if let Some(details) = db_health["details"].as_object() {
            assert!(details.contains_key("pool_size"));
            assert!(details.contains_key("idle_connections"));
        }
    }

    #[tokio::test]
    async fn test_health_check_detailed_redis_not_configured() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Ensure REDIS_URL is not set for this test
        std::env::remove_var("REDIS_URL");

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health/detailed")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let health_response: Value = serde_json::from_str(&body_str).unwrap();

        let redis_health = &health_response["components"]["redis"];

        // Redis should be healthy when not configured (optional component)
        assert_eq!(redis_health["status"], "healthy");

        // Should have message indicating it's not configured
        assert!(redis_health["message"].is_string());
        let message = redis_health["message"].as_str().unwrap();
        assert!(message.contains("not configured") || message.contains("optional"));
    }

    #[tokio::test]
    async fn test_health_check_detailed_response_format() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health/detailed")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let health_response: Value = serde_json::from_str(&body_str).unwrap();

        // Verify required top-level fields
        assert!(health_response.get("status").is_some());
        assert!(health_response.get("service").is_some());
        assert!(health_response.get("version").is_some());
        assert!(health_response.get("timestamp").is_some());
        assert!(health_response.get("components").is_some());

        // Verify timestamp format (should be RFC3339)
        let timestamp = health_response["timestamp"].as_str().unwrap();
        assert!(timestamp.contains("T")); // ISO 8601 format
        assert!(timestamp.contains("Z") || timestamp.contains("+")); // Timezone indicator
    }

    #[tokio::test]
    async fn test_health_check_detailed_status_values() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health/detailed")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let health_response: Value = serde_json::from_str(&body_str).unwrap();

        // Overall status should be one of: healthy, degraded, unhealthy
        let status = health_response["status"].as_str().unwrap();
        assert!(
            status == "healthy" || status == "degraded" || status == "unhealthy",
            "Invalid status: {}",
            status
        );

        // Component statuses should also be valid
        let db_status = health_response["components"]["database"]["status"]
            .as_str()
            .unwrap();
        assert!(
            db_status == "healthy" || db_status == "degraded" || db_status == "unhealthy",
            "Invalid database status: {}",
            db_status
        );

        let redis_status = health_response["components"]["redis"]["status"]
            .as_str()
            .unwrap();
        assert!(
            redis_status == "healthy" || redis_status == "degraded" || redis_status == "unhealthy",
            "Invalid redis status: {}",
            redis_status
        );
    }

    #[tokio::test]
    async fn test_health_check_detailed_concurrent_requests() {
        // Test that multiple concurrent health check requests work correctly
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let mut handles = vec![];

        for _ in 0..5 {
            let app_clone = app.clone();
            let handle = tokio::spawn(async move {
                let request = Request::builder()
                    .method(Method::GET)
                    .uri("/health/detailed")
                    .body(Body::empty())
                    .unwrap();

                app_clone.oneshot(request).await.unwrap()
            });
            handles.push(handle);
        }

        // Wait for all requests to complete
        for handle in handles {
            let response = handle.await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    // ========================================
    // Edge Cases and Error Scenarios
    // ========================================

    #[tokio::test]
    async fn test_health_check_with_database_cleaned() {
        // Test health check after database operations
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // Clean database (should not affect health check)
        DatabaseTestHelpers::clean_database(&test_db.pool)
            .await
            .unwrap();

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health/detailed")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let health_response: Value = serde_json::from_str(&body_str).unwrap();

        // Database should still be healthy
        assert_eq!(health_response["components"]["database"]["status"], "healthy");
    }

    #[tokio::test]
    async fn test_health_check_detailed_http_methods() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // GET should work
        let request = Request::builder()
            .method(Method::GET)
            .uri("/health/detailed")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // POST should not be allowed
        let request = Request::builder()
            .method(Method::POST)
            .uri("/health/detailed")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

        // PUT should not be allowed
        let request = Request::builder()
            .method(Method::PUT)
            .uri("/health/detailed")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

        // DELETE should not be allowed
        let request = Request::builder()
            .method(Method::DELETE)
            .uri("/health/detailed")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_health_check_basic_http_methods() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        // GET should work
        let request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // POST should not be allowed
        let request = Request::builder()
            .method(Method::POST)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_health_check_detailed_database_pool_stats() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health/detailed")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let health_response: Value = serde_json::from_str(&body_str).unwrap();

        let db_health = &health_response["components"]["database"];

        // Check that pool stats are present in details
        if let Some(details) = db_health["details"].as_object() {
            // Pool size should be a positive number
            if let Some(pool_size) = details.get("pool_size") {
                assert!(pool_size.is_number());
                if let Some(size) = pool_size.as_u64() {
                    assert!(size > 0, "Pool size should be greater than 0");
                }
            }

            // Idle connections should be a non-negative number
            if let Some(idle) = details.get("idle_connections") {
                assert!(idle.is_number());
                if let Some(idle_count) = idle.as_u64() {
                    assert!(idle_count >= 0, "Idle connections should be >= 0");
                }
            }
        }
    }
}
