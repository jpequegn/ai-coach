/// Load testing for vision analysis system
///
/// Tests cover:
/// - Concurrent video processing
/// - API rate limiting
/// - Database connection pooling under load
/// - Memory leak detection
/// - Performance degradation under stress
/// - Scalability validation

use ai_coach_api::services::video_storage_service::VideoStorageService;
use ai_coach_api::services::vision_analysis_service::VisionAnalysisService;
use ai_coach_api::services::performance_profiler::PipelineProfiler;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::task;

/// Load test configuration
struct LoadTestConfig {
    concurrent_users: usize,
    requests_per_user: usize,
    video_size_kb: usize,
    target_throughput_rps: f64,
    max_latency_ms: u64,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            concurrent_users: 10,
            requests_per_user: 5,
            video_size_kb: 100,
            target_throughput_rps: 50.0,
            max_latency_ms: 2000,
        }
    }
}

/// Load test results
#[derive(Debug)]
struct LoadTestResults {
    total_requests: usize,
    successful_requests: usize,
    failed_requests: usize,
    total_duration: Duration,
    avg_latency_ms: f64,
    p95_latency_ms: f64,
    p99_latency_ms: f64,
    throughput_rps: f64,
    errors: Vec<String>,
}

impl LoadTestResults {
    fn print_summary(&self) {
        println!("\n=== Load Test Results ===");
        println!("Total Requests: {}", self.total_requests);
        println!("Successful: {}", self.successful_requests);
        println!("Failed: {}", self.failed_requests);
        println!("Success Rate: {:.2}%",
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0);
        println!("\n=== Performance Metrics ===");
        println!("Total Duration: {:.2}s", self.total_duration.as_secs_f64());
        println!("Average Latency: {:.2}ms", self.avg_latency_ms);
        println!("P95 Latency: {:.2}ms", self.p95_latency_ms);
        println!("P99 Latency: {:.2}ms", self.p99_latency_ms);
        println!("Throughput: {:.2} req/s", self.throughput_rps);

        if !self.errors.is_empty() {
            println!("\n=== Errors (first 10) ===");
            for error in self.errors.iter().take(10) {
                println!("  - {}", error);
            }
        }
    }
}

/// Helper to generate test video data
fn generate_test_video_data(size_kb: usize) -> Vec<u8> {
    vec![0u8; size_kb * 1024]
}

#[sqlx::test]
#[ignore] // Run with --ignored flag for load tests
async fn test_concurrent_video_uploads(pool: PgPool) -> sqlx::Result<()> {
    let config = LoadTestConfig::default();
    let storage_service = Arc::new(VideoStorageService::new(pool.clone()));

    let mut latencies = Vec::new();
    let mut errors = Vec::new();

    let start_time = Instant::now();

    // Spawn concurrent users
    let mut handles = vec![];

    for user_idx in 0..config.concurrent_users {
        let service = Arc::clone(&storage_service);
        let video_data = generate_test_video_data(config.video_size_kb);

        let handle = task::spawn(async move {
            let user_id = uuid::Uuid::new_v4();
            let mut user_latencies = Vec::new();
            let mut user_errors = Vec::new();

            for req_idx in 0..config.requests_per_user {
                let request_start = Instant::now();

                match service.upload_video(
                    user_id,
                    &format!("video_{}_{}.mp4", user_idx, req_idx),
                    video_data.clone(),
                ).await {
                    Ok(_) => {
                        user_latencies.push(request_start.elapsed());
                    }
                    Err(e) => {
                        user_errors.push(format!("Upload failed: {}", e));
                    }
                }
            }

            (user_latencies, user_errors)
        });

        handles.push(handle);
    }

    // Collect results
    let mut successful_requests = 0;

    for handle in handles {
        if let Ok((user_latencies, user_errors)) = handle.await {
            successful_requests += user_latencies.len();
            latencies.extend(user_latencies);
            errors.extend(user_errors);
        }
    }

    let total_duration = start_time.elapsed();
    let total_requests = config.concurrent_users * config.requests_per_user;

    // Calculate metrics
    let avg_latency_ms = latencies.iter()
        .map(|d| d.as_millis() as f64)
        .sum::<f64>() / latencies.len() as f64;

    let mut sorted_latencies = latencies.clone();
    sorted_latencies.sort();

    let p95_index = (latencies.len() as f64 * 0.95) as usize;
    let p99_index = (latencies.len() as f64 * 0.99) as usize;

    let p95_latency_ms = sorted_latencies.get(p95_index)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);

    let p99_latency_ms = sorted_latencies.get(p99_index)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);

    let throughput_rps = successful_requests as f64 / total_duration.as_secs_f64();

    let results = LoadTestResults {
        total_requests,
        successful_requests,
        failed_requests: total_requests - successful_requests,
        total_duration,
        avg_latency_ms,
        p95_latency_ms,
        p99_latency_ms,
        throughput_rps,
        errors,
    };

    results.print_summary();

    // Assertions
    assert!(
        results.successful_requests as f64 / total_requests as f64 >= 0.95,
        "Success rate should be >= 95%"
    );

    assert!(
        results.avg_latency_ms <= config.max_latency_ms as f64,
        "Average latency should be <= {}ms, got {:.2}ms",
        config.max_latency_ms,
        results.avg_latency_ms
    );

    // Cleanup
    sqlx::query!("DELETE FROM videos WHERE created_at > NOW() - INTERVAL '1 hour'")
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
#[ignore]
async fn test_sustained_load(pool: PgPool) -> sqlx::Result<()> {
    let storage_service = Arc::new(VideoStorageService::new(pool.clone()));

    let duration = Duration::from_secs(30); // 30 second sustained load
    let request_rate = 10; // 10 requests per second
    let start_time = Instant::now();

    let mut successful = 0;
    let mut failed = 0;

    while start_time.elapsed() < duration {
        let iteration_start = Instant::now();
        let mut iteration_handles = vec![];

        // Spawn batch of concurrent requests
        for _ in 0..request_rate {
            let service = Arc::clone(&storage_service);
            let video_data = generate_test_video_data(50);

            let handle = task::spawn(async move {
                let user_id = uuid::Uuid::new_v4();
                service.upload_video(user_id, "sustained_test.mp4", video_data).await
            });

            iteration_handles.push(handle);
        }

        // Wait for this batch to complete
        for handle in iteration_handles {
            if let Ok(result) = handle.await {
                if result.is_ok() {
                    successful += 1;
                } else {
                    failed += 1;
                }
            }
        }

        // Sleep to maintain request rate
        let iteration_duration = iteration_start.elapsed();
        if iteration_duration < Duration::from_secs(1) {
            tokio::time::sleep(Duration::from_secs(1) - iteration_duration).await;
        }
    }

    let actual_duration = start_time.elapsed();

    println!("\n=== Sustained Load Test Results ===");
    println!("Duration: {:.2}s", actual_duration.as_secs_f64());
    println!("Successful: {}", successful);
    println!("Failed: {}", failed);
    println!("Average RPS: {:.2}", successful as f64 / actual_duration.as_secs_f64());

    assert!(
        failed as f64 / (successful + failed) as f64 < 0.05,
        "Failure rate should be < 5% under sustained load"
    );

    // Cleanup
    sqlx::query!("DELETE FROM videos WHERE filename = 'sustained_test.mp4'")
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
#[ignore]
async fn test_database_connection_pool_under_load(pool: PgPool) -> sqlx::Result<()> {
    let concurrent_connections = 50;
    let queries_per_connection = 20;

    let pool = Arc::new(pool);
    let mut handles = vec![];

    let start_time = Instant::now();

    for _ in 0..concurrent_connections {
        let pool_clone = Arc::clone(&pool);

        let handle = task::spawn(async move {
            let mut query_times = Vec::new();

            for _ in 0..queries_per_connection {
                let query_start = Instant::now();

                let result = sqlx::query!("SELECT 1 as value")
                    .fetch_one(pool_clone.as_ref())
                    .await;

                if result.is_ok() {
                    query_times.push(query_start.elapsed());
                }
            }

            query_times
        });

        handles.push(handle);
    }

    // Collect results
    let mut all_query_times = Vec::new();

    for handle in handles {
        if let Ok(query_times) = handle.await {
            all_query_times.extend(query_times);
        }
    }

    let total_duration = start_time.elapsed();
    let total_queries = all_query_times.len();

    let avg_query_time_ms = all_query_times.iter()
        .map(|d| d.as_millis() as f64)
        .sum::<f64>() / total_queries as f64;

    let queries_per_second = total_queries as f64 / total_duration.as_secs_f64();

    println!("\n=== Database Connection Pool Test ===");
    println!("Concurrent Connections: {}", concurrent_connections);
    println!("Total Queries: {}", total_queries);
    println!("Average Query Time: {:.2}ms", avg_query_time_ms);
    println!("Queries Per Second: {:.2}", queries_per_second);

    assert!(
        avg_query_time_ms < 100.0,
        "Average query time should be < 100ms under load, got {:.2}ms",
        avg_query_time_ms
    );

    Ok(())
}

#[sqlx::test]
#[ignore]
async fn test_memory_leak_detection(pool: PgPool) -> sqlx::Result<()> {
    let storage_service = VideoStorageService::new(pool.clone());

    // Track memory growth over multiple iterations
    let iterations = 100;
    let user_id = uuid::Uuid::new_v4();
    let video_data = generate_test_video_data(100);

    println!("\n=== Memory Leak Detection Test ===");
    println!("Running {} iterations...", iterations);

    for i in 0..iterations {
        // Upload and immediately delete to check for leaks
        let video_id = storage_service
            .upload_video(user_id, &format!("leak_test_{}.mp4", i), video_data.clone())
            .await?;

        sqlx::query!("DELETE FROM videos WHERE id = $1", video_id)
            .execute(&pool)
            .await?;

        if i % 20 == 0 {
            println!("  Iteration {}/{}", i, iterations);
        }
    }

    println!("✓ Completed {} iterations without crash", iterations);
    println!("  Manual memory inspection recommended");

    Ok(())
}

#[sqlx::test]
#[ignore]
async fn test_error_rate_under_load(pool: PgPool) -> sqlx::Result<()> {
    let storage_service = Arc::new(VideoStorageService::new(pool.clone()));

    let total_requests = 100;
    let mut handles = vec![];

    // Mix valid and invalid requests
    for i in 0..total_requests {
        let service = Arc::clone(&storage_service);

        let handle = task::spawn(async move {
            let user_id = uuid::Uuid::new_v4();

            // Every 10th request is invalid (empty data)
            let video_data = if i % 10 == 0 {
                vec![]
            } else {
                generate_test_video_data(50)
            };

            service.upload_video(user_id, &format!("test_{}.mp4", i), video_data).await
        });

        handles.push(handle);
    }

    let mut successful = 0;
    let mut failed = 0;

    for handle in handles {
        if let Ok(result) = handle.await {
            if result.is_ok() {
                successful += 1;
            } else {
                failed += 1;
            }
        }
    }

    println!("\n=== Error Rate Test ===");
    println!("Total Requests: {}", total_requests);
    println!("Successful: {}", successful);
    println!("Failed: {}", failed);
    println!("Error Rate: {:.2}%", (failed as f64 / total_requests as f64) * 100.0);

    // Should handle errors gracefully without crashing
    assert!(successful > 0, "Some requests should succeed");

    // Cleanup
    sqlx::query!("DELETE FROM videos WHERE filename LIKE 'test_%.mp4'")
        .execute(&pool)
        .await?;

    Ok(())
}

#[sqlx::test]
#[ignore]
async fn test_scalability_limits(pool: PgPool) -> sqlx::Result<()> {
    let storage_service = Arc::new(VideoStorageService::new(pool.clone()));

    // Test increasing load levels
    let load_levels = vec![10, 25, 50, 100];

    println!("\n=== Scalability Test ===");

    for concurrent_requests in load_levels {
        let mut handles = vec![];
        let start_time = Instant::now();

        for i in 0..concurrent_requests {
            let service = Arc::clone(&storage_service);
            let video_data = generate_test_video_data(50);

            let handle = task::spawn(async move {
                let user_id = uuid::Uuid::new_v4();
                service.upload_video(user_id, &format!("scale_test_{}.mp4", i), video_data).await
            });

            handles.push(handle);
        }

        let mut successful = 0;

        for handle in handles {
            if let Ok(Ok(_)) = handle.await {
                successful += 1;
            }
        }

        let duration = start_time.elapsed();
        let throughput = successful as f64 / duration.as_secs_f64();

        println!("Load Level: {} concurrent requests", concurrent_requests);
        println!("  Duration: {:.2}s", duration.as_secs_f64());
        println!("  Success Rate: {:.2}%", (successful as f64 / concurrent_requests as f64) * 100.0);
        println!("  Throughput: {:.2} req/s", throughput);
    }

    // Cleanup
    sqlx::query!("DELETE FROM videos WHERE filename LIKE 'scale_test_%.mp4'")
        .execute(&pool)
        .await?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_cpu_intensive_processing_load() {
    use ai_coach_api::services::pose_estimation_service::PoseEstimationService;
    use image::{DynamicImage, ImageBuffer, Rgb};

    let service = match PoseEstimationService::new("models/pose_v1.onnx") {
        Ok(s) => s,
        Err(_) => {
            println!("ℹ️  Skipping CPU test - model not available");
            return;
        }
    };

    let service = Arc::new(service);
    let concurrent_inferences = 10;
    let inferences_per_task = 20;

    println!("\n=== CPU Load Test ===");

    let start_time = Instant::now();
    let mut handles = vec![];

    for _ in 0..concurrent_inferences {
        let service_clone = Arc::clone(&service);

        let handle = task::spawn(async move {
            let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                ImageBuffer::from_pixel(640, 640, Rgb([128, 128, 128]));
            let img = DynamicImage::ImageRgb8(img);

            let mut inference_times = Vec::new();

            for _ in 0..inferences_per_task {
                let start = Instant::now();
                let _ = service_clone.estimate_pose(&img);
                inference_times.push(start.elapsed());
            }

            inference_times
        });

        handles.push(handle);
    }

    let mut all_times = Vec::new();

    for handle in handles {
        if let Ok(times) = handle.await {
            all_times.extend(times);
        }
    }

    let total_duration = start_time.elapsed();
    let total_inferences = all_times.len();

    let avg_time_ms = all_times.iter()
        .map(|d| d.as_millis() as f64)
        .sum::<f64>() / total_inferences as f64;

    let throughput_fps = 1000.0 / avg_time_ms;

    println!("Total Inferences: {}", total_inferences);
    println!("Total Duration: {:.2}s", total_duration.as_secs_f64());
    println!("Average Inference Time: {:.2}ms", avg_time_ms);
    println!("Throughput: {:.2} FPS", throughput_fps);

    assert!(
        throughput_fps >= 10.0,
        "Should maintain >= 10 FPS under load, got {:.2} FPS",
        throughput_fps
    );
}
