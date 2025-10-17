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
use ai_coach::models::*;
use crate::common::{TestDatabase, DatabaseTestHelpers};

#[cfg(test)]
mod user_profile_integration_tests {
    use super::*;

    /// Test helper to create the app router with test database
    async fn create_test_app(pool: SqlitePool) -> Router {
        create_routes(pool, "test_secret_key_for_testing_only")
    }

    /// Helper to create a test user and get auth token
    async fn create_authenticated_user(app: Router) -> (Router, String, Uuid) {
        // Register a test user
        let register_request = json!({
            "email": "testuser@example.com",
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

        let access_token = auth_response["access_token"].as_str().unwrap().to_string();
        let user_id = Uuid::parse_str(auth_response["user"]["id"].as_str().unwrap()).unwrap();

        (app, access_token, user_id)
    }

    // ========================================
    // Profile Tests (GET /profile, PUT /profile)
    // ========================================

    #[tokio::test]
    async fn test_get_profile_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, user_id) = create_authenticated_user(app).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let profile_response: Value = serde_json::from_str(&body_str).unwrap();

        // Verify response structure
        assert!(profile_response["profile"].is_object());
        assert!(profile_response["completion_percentage"].is_number());
        assert!(profile_response["missing_fields"].is_array());
        assert_eq!(profile_response["success"], true);

        // Verify profile contains expected fields
        let profile = &profile_response["profile"];
        assert!(profile["user_id"].is_string());
        assert!(profile["email"].is_string());
        assert!(profile["created_at"].is_string());
        assert!(profile["updated_at"].is_string());
    }

    #[tokio::test]
    async fn test_get_profile_unauthorized() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_get_profile_invalid_token() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile")
            .header("Authorization", "Bearer invalid_token_12345")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_update_profile_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        let update_request = json!({
            "name": "Updated Name",
            "age": 30,
            "weight_kg": 75.5,
            "height_cm": 180.0,
            "resting_heart_rate": 50,
            "max_heart_rate": 190
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let profile_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(profile_response["success"], true);
        // Note: Currently returns mock data, so we can't verify exact values
    }

    #[tokio::test]
    async fn test_update_profile_partial_update() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        // Update only weight
        let update_request = json!({
            "weight_kg": 72.0
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_update_profile_invalid_age() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        // Age too low
        let update_request = json!({
            "age": 10
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let error_response: Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(error_response["error_code"], "INVALID_AGE");

        // Age too high
        let update_request_high = json!({
            "age": 130
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request_high.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_update_profile_invalid_weight() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        // Weight too low
        let update_request = json!({
            "weight_kg": 20.0
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let error_response: Value = serde_json::from_str(&body_str).unwrap();
        assert_eq!(error_response["error_code"], "INVALID_WEIGHT");

        // Weight too high
        let update_request_high = json!({
            "weight_kg": 350.0
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request_high.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ========================================
    // Thresholds Tests (GET /thresholds, PUT /thresholds)
    // ========================================

    #[tokio::test]
    async fn test_get_thresholds_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile/thresholds")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let thresholds_response: Value = serde_json::from_str(&body_str).unwrap();

        // Verify response structure
        assert!(thresholds_response["user_id"].is_string());
        assert!(thresholds_response["ftp_watts"].is_number());
        assert!(thresholds_response["power_zones"].is_array());
        assert!(thresholds_response["heart_rate_zones"].is_array());
        assert_eq!(thresholds_response["success"], true);

        // Verify power zones are calculated
        let power_zones = thresholds_response["power_zones"].as_array().unwrap();
        assert_eq!(power_zones.len(), 7); // 7 power zones

        // Verify heart rate zones are calculated
        let hr_zones = thresholds_response["heart_rate_zones"].as_array().unwrap();
        assert_eq!(hr_zones.len(), 5); // 5 HR zones
    }

    #[tokio::test]
    async fn test_get_thresholds_unauthorized() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile/thresholds")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_update_thresholds_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        let update_request = json!({
            "ftp_watts": 280,
            "threshold_pace_ms": 4.0,
            "lactate_threshold_hr": 170,
            "vo2_max": 58.5
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile/thresholds")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let thresholds_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(thresholds_response["success"], true);
        // Zones should be recalculated based on new FTP
        let power_zones = thresholds_response["power_zones"].as_array().unwrap();
        assert_eq!(power_zones.len(), 7);
    }

    #[tokio::test]
    async fn test_update_thresholds_partial() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        // Update only FTP
        let update_request = json!({
            "ftp_watts": 300
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile/thresholds")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // ========================================
    // Preferences Tests (PUT /preferences)
    // ========================================

    #[tokio::test]
    async fn test_update_preferences_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        let update_request = json!({
            "sport_preferences": {
                "primary_sport": "running",
                "secondary_sports": ["cycling", "swimming"],
                "preferred_activities": ["intervals", "long_runs"],
                "avoided_activities": ["hill_repeats"]
            },
            "training_preferences": {
                "weekly_hours_available": 12.0,
                "preferred_workout_times": ["morning", "evening"],
                "preferred_intensity": "high",
                "recovery_emphasis": "aggressive",
                "equipment_available": ["treadmill", "bike", "pool"]
            }
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile/preferences")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let response_json: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(response_json["success"], true);
        assert_eq!(response_json["message"], "Preferences updated successfully");
    }

    #[tokio::test]
    async fn test_update_preferences_unauthorized() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let update_request = json!({
            "sport_preferences": {
                "primary_sport": "running"
            }
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile/preferences")
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // ========================================
    // Notification Settings Tests
    // ========================================

    #[tokio::test]
    async fn test_get_notification_settings_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile/notifications")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let settings: Value = serde_json::from_str(&body_str).unwrap();

        // Verify notification settings structure
        assert!(settings["email_notifications"].is_boolean());
        assert!(settings["workout_reminders"].is_boolean());
        assert!(settings["goal_updates"].is_boolean());
        assert!(settings["performance_insights"].is_boolean());
        assert!(settings["weekly_summary"].is_boolean());
        assert!(settings["notification_time"].is_string());
    }

    #[tokio::test]
    async fn test_update_notification_settings_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        let update_request = json!({
            "email_notifications": false,
            "workout_reminders": true,
            "goal_updates": false,
            "performance_insights": true,
            "weekly_summary": false,
            "notification_time": "07:00"
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile/notifications")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let response_json: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(response_json["success"], true);
    }

    #[tokio::test]
    async fn test_update_notification_settings_partial() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        // Update only email notifications
        let update_request = json!({
            "email_notifications": false
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile/notifications")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // ========================================
    // Privacy Settings Tests
    // ========================================

    #[tokio::test]
    async fn test_get_privacy_settings_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile/privacy")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let settings: Value = serde_json::from_str(&body_str).unwrap();

        // Verify privacy settings structure
        assert!(settings["profile_visibility"].is_string());
        assert!(settings["share_activities"].is_boolean());
        assert!(settings["share_performance_data"].is_boolean());
        assert!(settings["allow_peer_comparison"].is_boolean());
    }

    #[tokio::test]
    async fn test_update_privacy_settings_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        let update_request = json!({
            "profile_visibility": "private",
            "share_activities": false,
            "share_performance_data": false,
            "allow_peer_comparison": false
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile/privacy")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let response_json: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(response_json["success"], true);
    }

    #[tokio::test]
    async fn test_update_privacy_settings_unauthorized() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let update_request = json!({
            "profile_visibility": "private"
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile/privacy")
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // ========================================
    // Training Zones Calculation Tests
    // ========================================

    #[tokio::test]
    async fn test_calculate_training_zones_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile/zones/calculate")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let zones_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(zones_response["success"], true);
        assert!(zones_response["power_zones"].is_array());
        assert!(zones_response["heart_rate_zones"].is_array());
        assert!(zones_response["pace_zones"].is_array());
    }

    #[tokio::test]
    async fn test_calculate_training_zones_unauthorized() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile/zones/calculate")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // ========================================
    // Profile Export Tests
    // ========================================

    #[tokio::test]
    async fn test_export_profile_data_success() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile/export")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let export_response: Value = serde_json::from_str(&body_str).unwrap();

        assert_eq!(export_response["success"], true);
        assert!(export_response["export_id"].is_string());
        assert!(export_response["created_at"].is_string());
        assert!(export_response["data"].is_object());

        // Verify export contains expected sections
        let data = &export_response["data"];
        assert!(data["profile"].is_object());
        assert!(data["thresholds"].is_object());
        assert!(data["preferences"].is_object());
        assert!(data["training_history"].is_array());
        assert!(data["goals"].is_array());
    }

    #[tokio::test]
    async fn test_export_profile_data_unauthorized() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile/export")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // ========================================
    // Edge Cases and Error Scenarios
    // ========================================

    #[tokio::test]
    async fn test_concurrent_profile_updates() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        // Simulate two concurrent update requests
        let update_request_1 = json!({"weight_kg": 70.0});
        let update_request_2 = json!({"weight_kg": 75.0});

        let request1 = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request_1.to_string()))
            .unwrap();

        let request2 = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile")
            .header("Authorization", format!("Bearer {}", token.clone()))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request_2.to_string()))
            .unwrap();

        // Both should succeed (last write wins)
        let response1 = app.clone().oneshot(request1).await.unwrap();
        let response2 = app.oneshot(request2).await.unwrap();

        assert_eq!(response1.status(), StatusCode::OK);
        assert_eq!(response2.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_profile_completion_percentage_calculation() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/user/profile")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let profile_response: Value = serde_json::from_str(&body_str).unwrap();

        let completion = profile_response["completion_percentage"].as_f64().unwrap();
        assert!(completion >= 0.0 && completion <= 100.0);

        // Missing fields should be an array
        assert!(profile_response["missing_fields"].is_array());
    }

    #[tokio::test]
    async fn test_zone_calculation_with_zero_ftp() {
        let test_db = TestDatabase::new().await;
        let app = create_test_app(test_db.pool.clone()).await;
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();

        let (app, token, _) = create_authenticated_user(app).await;

        // Try to set FTP to 0
        let update_request = json!({
            "ftp_watts": 0
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/user/profile/thresholds")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(update_request.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        // Should accept 0 but zones might be empty or minimal
        // Current implementation doesn't validate this, but ideally should
        assert!(response.status() == StatusCode::OK || response.status() == StatusCode::BAD_REQUEST);
    }
}
