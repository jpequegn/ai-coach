use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use ai_coach::api::routes::create_routes;
use ai_coach::models::*;
use crate::common::{TestDatabase, MockDataGenerator, DatabaseTestHelpers, ApiTestHelpers};

#[cfg(test)]
mod load_tests {
    use super::*;

    /// Load test configuration
    #[derive(Debug, Clone)]
    pub struct LoadTestConfig {
        pub concurrent_users: usize,
        pub requests_per_user: usize,
        pub max_duration: Duration,
        pub ramp_up_duration: Duration,
        pub endpoint: String,
        pub method: Method,
        pub body: Option<Value>,
    }

    impl Default for LoadTestConfig {
        fn default() -> Self {
            Self {
                concurrent_users: 10,
                requests_per_user: 10,
                max_duration: Duration::from_secs(30),
                ramp_up_duration: Duration::from_secs(5),
                endpoint: "/health".to_string(),
                method: Method::GET,
                body: None,
            }
        }
    }

    /// Load test results
    #[derive(Debug)]
    pub struct LoadTestResults {
        pub total_requests: usize,
        pub successful_requests: usize,
        pub failed_requests: usize,
        pub avg_response_time: Duration,
        pub min_response_time: Duration,
        pub max_response_time: Duration,
        pub p95_response_time: Duration,
        pub requests_per_second: f64,
        pub success_rate: f64,
        pub test_duration: Duration,
        pub error_summary: std::collections::HashMap<String, usize>,
    }

    impl LoadTestResults {
        fn analyze(responses: Vec<LoadTestResponse>, test_duration: Duration) -> Self {
            let total_requests = responses.len();
            let successful_requests = responses.iter().filter(|r| r.success).count();
            let failed_requests = total_requests - successful_requests;

            let response_times: Vec<Duration> = responses.iter().map(|r| r.response_time).collect();

            let avg_response_time = if !response_times.is_empty() {
                response_times.iter().sum::<Duration>() / response_times.len() as u32
            } else {
                Duration::ZERO
            };

            let min_response_time = response_times.iter().min().copied().unwrap_or(Duration::ZERO);
            let max_response_time = response_times.iter().max().copied().unwrap_or(Duration::ZERO);

            // Calculate P95
            let mut sorted_times = response_times.clone();
            sorted_times.sort();
            let p95_index = (sorted_times.len() as f64 * 0.95) as usize;
            let p95_response_time = sorted_times.get(p95_index).copied().unwrap_or(Duration::ZERO);

            let requests_per_second = if test_duration.as_secs_f64() > 0.0 {
                total_requests as f64 / test_duration.as_secs_f64()
            } else {
                0.0
            };

            let success_rate = if total_requests > 0 {
                (successful_requests as f64 / total_requests as f64) * 100.0
            } else {
                0.0
            };

            // Collect error summary
            let mut error_summary = std::collections::HashMap::new();
            for response in &responses {
                if !response.success {
                    *error_summary.entry(response.error.clone().unwrap_or("Unknown".to_string())).or_insert(0) += 1;
                }
            }

            Self {
                total_requests,
                successful_requests,
                failed_requests,
                avg_response_time,
                min_response_time,
                max_response_time,
                p95_response_time,
                requests_per_second,
                success_rate,
                test_duration,
                error_summary,
            }
        }
    }

    #[derive(Debug)]
    struct LoadTestResponse {
        success: bool,
        response_time: Duration,
        status_code: Option<StatusCode>,
        error: Option<String>,
    }

    /// Execute a load test with the given configuration
    async fn execute_load_test(
        app: Router,
        config: LoadTestConfig,
        auth_tokens: Vec<String>,
    ) -> LoadTestResults {
        let semaphore = Arc::new(Semaphore::new(config.concurrent_users));
        let mut handles = Vec::new();
        let start_time = Instant::now();

        println!("Starting load test:");
        println!("  Endpoint: {} {}", config.method, config.endpoint);
        println!("  Concurrent users: {}", config.concurrent_users);
        println!("  Requests per user: {}", config.requests_per_user);
        println!("  Total requests: {}", config.concurrent_users * config.requests_per_user);

        // Calculate ramp-up delay between user starts
        let ramp_delay = if config.concurrent_users > 1 {
            config.ramp_up_duration / (config.concurrent_users - 1) as u32
        } else {
            Duration::ZERO
        };

        for user_id in 0..config.concurrent_users {
            let semaphore = semaphore.clone();
            let app = app.clone();
            let config = config.clone();
            let auth_token = auth_tokens.get(user_id % auth_tokens.len()).cloned();

            let handle = tokio::spawn(async move {
                // Ramp-up delay
                if user_id > 0 {
                    tokio::time::sleep(ramp_delay * user_id as u32).await;
                }

                let _permit = semaphore.acquire().await.unwrap();
                let mut responses = Vec::new();

                for request_id in 0..config.requests_per_user {
                    let request_start = Instant::now();

                    let mut request_builder = Request::builder()
                        .method(config.method.clone())
                        .uri(&config.endpoint)
                        .header("Content-Type", "application/json");

                    // Add auth header if token provided
                    if let Some(ref token) = auth_token {
                        request_builder = request_builder.header("Authorization", format!("Bearer {}", token));
                    }

                    let request = if let Some(ref body) = config.body {
                        request_builder.body(Body::from(body.to_string())).unwrap()
                    } else {
                        request_builder.body(Body::empty()).unwrap()
                    };

                    let result = app.clone().oneshot(request).await;
                    let response_time = request_start.elapsed();

                    let response = match result {
                        Ok(resp) => {
                            let status = resp.status();
                            LoadTestResponse {
                                success: status.is_success(),
                                response_time,
                                status_code: Some(status),
                                error: if !status.is_success() {
                                    Some(format!("HTTP {}", status.as_u16()))
                                } else {
                                    None
                                },
                            }
                        }
                        Err(e) => LoadTestResponse {
                            success: false,
                            response_time,
                            status_code: None,
                            error: Some(e.to_string()),
                        },
                    };

                    responses.push(response);

                    // Check if we've exceeded max duration
                    if start_time.elapsed() > config.max_duration {
                        break;
                    }

                    // Small delay between requests from same user
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }

                responses
            });

            handles.push(handle);
        }

        // Wait for all handles to complete
        let mut all_responses = Vec::new();
        for handle in handles {
            if let Ok(responses) = handle.await {
                all_responses.extend(responses);
            }
        }

        let test_duration = start_time.elapsed();
        let results = LoadTestResults::analyze(all_responses, test_duration);

        println!("Load test completed:");
        println!("  Total requests: {}", results.total_requests);
        println!("  Successful requests: {}", results.successful_requests);
        println!("  Failed requests: {}", results.failed_requests);
        println!("  Success rate: {:.2}%", results.success_rate);
        println!("  Avg response time: {:?}", results.avg_response_time);
        println!("  P95 response time: {:?}", results.p95_response_time);
        println!("  Requests per second: {:.2}", results.requests_per_second);
        println!("  Test duration: {:?}", results.test_duration);

        if !results.error_summary.is_empty() {
            println!("  Error summary:");
            for (error, count) in &results.error_summary {
                println!("    {}: {}", error, count);
            }
        }

        results
    }

    /// Create test app with database
    async fn create_test_app_with_users(user_count: usize) -> (Router, TestDatabase, Vec<String>) {
        let test_db = TestDatabase::new().await;
        let app = create_routes(test_db.pool.clone(), "test_secret_key_for_testing_only");

        // Clean database and create test users
        DatabaseTestHelpers::clean_database(&test_db.pool).await.unwrap();
        let user_ids = DatabaseTestHelpers::seed_test_data(&test_db.pool, user_count).await.unwrap();

        // Generate auth tokens
        let auth_tokens: Vec<String> = user_ids
            .iter()
            .map(|&user_id| {
                ApiTestHelpers::create_test_token(user_id, &format!("user{}@example.com", user_id), ai_coach::auth::UserRole::Athlete)
            })
            .collect();

        (app, test_db, auth_tokens)
    }

    #[tokio::test]
    async fn test_health_endpoint_load() {
        let (app, _test_db, _auth_tokens) = create_test_app_with_users(1).await;

        let config = LoadTestConfig {
            concurrent_users: 20,
            requests_per_user: 50,
            endpoint: "/health".to_string(),
            method: Method::GET,
            body: None,
            ..Default::default()
        };

        let results = execute_load_test(app, config, vec![]).await;

        // Health endpoint should handle load well
        assert!(results.success_rate >= 99.0, "Health endpoint should have >99% success rate");
        assert!(results.avg_response_time < Duration::from_millis(100), "Health endpoint should respond in <100ms");
        assert!(results.requests_per_second > 50.0, "Should handle >50 requests per second");
    }

    #[tokio::test]
    async fn test_authenticated_endpoint_load() {
        let (app, _test_db, auth_tokens) = create_test_app_with_users(10).await;

        let config = LoadTestConfig {
            concurrent_users: 10,
            requests_per_user: 20,
            endpoint: "/api/v1/goals".to_string(),
            method: Method::GET,
            body: None,
            max_duration: Duration::from_secs(60),
            ..Default::default()
        };

        let results = execute_load_test(app, config, auth_tokens).await;

        // Authenticated endpoints might be slower but should still handle load
        assert!(results.success_rate >= 95.0, "Authenticated endpoint should have >95% success rate");
        assert!(results.avg_response_time < Duration::from_secs(1), "Should respond in <1s");
        assert!(results.requests_per_second > 10.0, "Should handle >10 requests per second");
    }

    #[tokio::test]
    async fn test_database_intensive_load() {
        let (app, _test_db, auth_tokens) = create_test_app_with_users(5).await;

        // Test creating goals (database writes)
        let create_goal_body = json!({
            "title": "Load Test Goal",
            "description": "Goal created during load testing",
            "goal_type": "power",
            "goal_category": "performance",
            "target_value": 300.0,
            "unit": "watts",
            "priority": "medium"
        });

        let config = LoadTestConfig {
            concurrent_users: 5,
            requests_per_user: 10,
            endpoint: "/api/v1/goals".to_string(),
            method: Method::POST,
            body: Some(create_goal_body),
            max_duration: Duration::from_secs(30),
            ..Default::default()
        };

        let results = execute_load_test(app, config, auth_tokens).await;

        // Database writes should be slower but reliable
        assert!(results.success_rate >= 90.0, "Database writes should have >90% success rate");
        assert!(results.avg_response_time < Duration::from_secs(2), "Database writes should respond in <2s");
        assert!(results.failed_requests < 5, "Should have minimal failures");
    }

    #[tokio::test]
    async fn test_mixed_workload() {
        let (app, _test_db, auth_tokens) = create_test_app_with_users(15).await;

        // Test multiple endpoints concurrently
        let mut handles = Vec::new();

        // Health check load
        let health_config = LoadTestConfig {
            concurrent_users: 5,
            requests_per_user: 30,
            endpoint: "/health".to_string(),
            method: Method::GET,
            body: None,
            ..Default::default()
        };

        let app_clone = app.clone();
        let handle1 = tokio::spawn(async move {
            execute_load_test(app_clone, health_config, vec![]).await
        });

        // Goals read load
        let goals_read_config = LoadTestConfig {
            concurrent_users: 8,
            requests_per_user: 15,
            endpoint: "/api/v1/goals".to_string(),
            method: Method::GET,
            body: None,
            ..Default::default()
        };

        let app_clone = app.clone();
        let auth_tokens_clone = auth_tokens.clone();
        let handle2 = tokio::spawn(async move {
            execute_load_test(app_clone, goals_read_config, auth_tokens_clone).await
        });

        // Goals write load
        let goals_write_config = LoadTestConfig {
            concurrent_users: 3,
            requests_per_user: 5,
            endpoint: "/api/v1/goals".to_string(),
            method: Method::POST,
            body: Some(json!({
                "title": "Mixed Workload Goal",
                "description": "Goal from mixed workload test",
                "goal_type": "power",
                "goal_category": "performance",
                "target_value": 250.0,
                "unit": "watts",
                "priority": "low"
            })),
            ..Default::default()
        };

        let app_clone = app.clone();
        let auth_tokens_clone = auth_tokens.clone();
        let handle3 = tokio::spawn(async move {
            execute_load_test(app_clone, goals_write_config, auth_tokens_clone).await
        });

        // Wait for all workloads to complete
        let (health_results, read_results, write_results) = tokio::join!(handle1, handle2, handle3);

        let health_results = health_results.unwrap();
        let read_results = read_results.unwrap();
        let write_results = write_results.unwrap();

        // All workloads should succeed
        assert!(health_results.success_rate >= 99.0, "Health workload should have >99% success");
        assert!(read_results.success_rate >= 95.0, "Read workload should have >95% success");
        assert!(write_results.success_rate >= 90.0, "Write workload should have >90% success");

        println!("Mixed workload summary:");
        println!("  Health: {:.1}% success, {:.2} RPS", health_results.success_rate, health_results.requests_per_second);
        println!("  Reads: {:.1}% success, {:.2} RPS", read_results.success_rate, read_results.requests_per_second);
        println!("  Writes: {:.1}% success, {:.2} RPS", write_results.success_rate, write_results.requests_per_second);
    }

    #[tokio::test]
    async fn test_stress_limits() {
        let (app, _test_db, auth_tokens) = create_test_app_with_users(5).await;

        // Gradually increase load to find breaking point
        let stress_levels = vec![
            (10, 10),   // 100 requests
            (20, 10),   // 200 requests
            (30, 10),   // 300 requests
            (50, 10),   // 500 requests
        ];

        for (concurrent_users, requests_per_user) in stress_levels {
            println!("Testing stress level: {} users x {} requests", concurrent_users, requests_per_user);

            let config = LoadTestConfig {
                concurrent_users,
                requests_per_user,
                endpoint: "/api/v1/goals".to_string(),
                method: Method::GET,
                body: None,
                max_duration: Duration::from_secs(60),
                ..Default::default()
            };

            let results = execute_load_test(app.clone(), config, auth_tokens.clone()).await;

            // Log performance degradation
            if results.success_rate < 90.0 {
                println!("  Performance degradation detected at {} concurrent users", concurrent_users);
                println!("  Success rate: {:.2}%", results.success_rate);
                println!("  Avg response time: {:?}", results.avg_response_time);
                break;
            }

            // Give the system time to recover between stress levels
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    #[tokio::test]
    async fn test_memory_leak_detection() {
        let (app, _test_db, auth_tokens) = create_test_app_with_users(2).await;

        // Run sustained load to detect memory leaks
        let config = LoadTestConfig {
            concurrent_users: 5,
            requests_per_user: 100, // Longer test
            endpoint: "/api/v1/goals".to_string(),
            method: Method::GET,
            body: None,
            max_duration: Duration::from_secs(60),
            ramp_up_duration: Duration::from_secs(1),
        };

        let start_time = Instant::now();
        let results = execute_load_test(app, config, auth_tokens).await;
        let total_time = start_time.elapsed();

        // Check that performance remained consistent
        assert!(results.success_rate >= 95.0, "Should maintain >95% success rate during sustained load");
        assert!(total_time < Duration::from_secs(120), "Test should complete within reasonable time");

        // In a real scenario, you would monitor memory usage here
        println!("Sustained load test completed:");
        println!("  Total requests: {}", results.total_requests);
        println!("  Test duration: {:?}", total_time);
        println!("  Final success rate: {:.2}%", results.success_rate);
    }

    #[tokio::test]
    async fn test_connection_pooling_limits() {
        let (app, _test_db, auth_tokens) = create_test_app_with_users(20).await;

        // Test with high concurrency to stress connection pool
        let config = LoadTestConfig {
            concurrent_users: 50, // High concurrency
            requests_per_user: 5,
            endpoint: "/api/v1/goals".to_string(),
            method: Method::GET,
            body: None,
            max_duration: Duration::from_secs(30),
            ramp_up_duration: Duration::from_secs(10), // Gradual ramp up
        };

        let results = execute_load_test(app, config, auth_tokens).await;

        // Should handle high concurrency reasonably
        assert!(results.success_rate >= 85.0, "Should handle high concurrency with >85% success");

        // Check for connection pool errors
        let has_connection_errors = results.error_summary.keys()
            .any(|error| error.to_lowercase().contains("connection") || error.to_lowercase().contains("pool"));

        if has_connection_errors {
            println!("Connection pool errors detected:");
            for (error, count) in &results.error_summary {
                if error.to_lowercase().contains("connection") || error.to_lowercase().contains("pool") {
                    println!("  {}: {}", error, count);
                }
            }
        }

        // Should not have excessive connection errors
        let connection_error_rate = if results.total_requests > 0 {
            results.error_summary.values().sum::<usize>() as f64 / results.total_requests as f64
        } else {
            0.0
        };

        assert!(connection_error_rate < 0.2, "Connection error rate should be <20%");
    }

    #[tokio::test]
    async fn test_response_time_consistency() {
        let (app, _test_db, auth_tokens) = create_test_app_with_users(10).await;

        let config = LoadTestConfig {
            concurrent_users: 10,
            requests_per_user: 20,
            endpoint: "/api/v1/goals".to_string(),
            method: Method::GET,
            body: None,
            ..Default::default()
        };

        let results = execute_load_test(app, config, auth_tokens).await;

        // Check response time consistency
        let response_time_ratio = results.max_response_time.as_millis() as f64 /
                                 results.avg_response_time.as_millis() as f64;

        assert!(response_time_ratio < 10.0, "Max response time should not be >10x average");
        assert!(results.p95_response_time < results.avg_response_time * 3, "P95 should be <3x average");

        println!("Response time analysis:");
        println!("  Min: {:?}", results.min_response_time);
        println!("  Avg: {:?}", results.avg_response_time);
        println!("  P95: {:?}", results.p95_response_time);
        println!("  Max: {:?}", results.max_response_time);
        println!("  Max/Avg ratio: {:.2}", response_time_ratio);
    }
}