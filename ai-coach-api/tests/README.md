# AI Coach Vision Analysis Test Suite

Comprehensive test suite for the vision analysis system, including unit tests, integration tests, and load tests.

## Test Structure

```
tests/
├── unit/                              # Unit tests for business logic
│   ├── keypoint_processor_test.rs    # Keypoint calculations and form scoring
│   ├── goal_service_test.rs          # Goal management
│   ├── user_service_test.rs          # User services
│   └── ...
├── integration/                       # Integration tests
│   ├── vision_pipeline_test.rs       # End-to-end video processing
│   ├── api_endpoints_test.rs         # API integration
│   └── ...
├── pose_estimation_service_test.rs   # Pose estimation service tests
├── vision_load_test.rs               # Load and performance tests
└── common/                            # Shared test utilities
    └── mod.rs
```

## Running Tests

### Quick Start

```bash
# Set database URL
export DATABASE_URL=postgresql://postgres:password@localhost:5432/ai_coach

# Run all tests
cargo test

# Run with output
cargo test -- --nocapture
```

### Unit Tests

```bash
# Run all unit tests
cargo test --lib

# Run specific unit test file
cargo test keypoint_processor_test

# Run specific test
cargo test test_angle_calculation_90_degrees -- --exact
```

### Integration Tests

```bash
# Run all integration tests
cargo test --test integration

# Run vision pipeline tests
cargo test --test vision_pipeline_test

# Run serially (for database tests)
cargo test -- --test-threads=1
```

### Load Tests

Load tests are ignored by default. Run with `--ignored` flag:

```bash
# Run all load tests
cargo test --test vision_load_test -- --ignored

# Run specific load test
cargo test test_concurrent_video_uploads -- --ignored --nocapture

# Run sustained load test
cargo test test_sustained_load -- --ignored --nocapture
```

### Performance Tests

```bash
# Run pose estimation benchmarks
cargo test test_performance_benchmark -- --ignored --nocapture

# Run CPU load test
cargo test test_cpu_intensive_processing_load -- --ignored --nocapture
```

## Test Coverage

### Generate Coverage Report

```bash
# Using the provided script
./scripts/generate_coverage.sh

# Or manually with tarpaulin
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --output-dir target/coverage
open target/coverage/index.html
```

### Coverage Targets

| Component | Target | Files |
|-----------|--------|-------|
| Keypoint Processor | >80% | `keypoint_processor_test.rs` |
| Pose Estimation | >80% | `pose_estimation_service_test.rs` |
| Video Pipeline | >80% | `vision_pipeline_test.rs` |
| Overall | >80% | All tests |

## Test Categories

### Unit Tests

**Purpose**: Test individual functions and algorithms in isolation

**Examples**:
- Angle calculations (90°, 180°, 45°, edge cases)
- Distance calculations
- Alignment detection
- Form scoring logic
- Issue detection algorithms

**Key Test Pattern**:
```rust
#[test]
fn test_angle_calculation_90_degrees() {
    let processor = KeypointProcessor::new();
    let p1 = create_keypoint(0.0, 0.0, 1.0, "p1");
    let p2 = create_keypoint(1.0, 0.0, 1.0, "p2");
    let p3 = create_keypoint(1.0, 1.0, 1.0, "p3");

    let angle = processor.calculate_angle(&p1, &p2, &p3).unwrap();
    assert!((angle - 90.0).abs() < 1.0);
}
```

### Integration Tests

**Purpose**: Test complete workflows with database integration

**Examples**:
- Video upload and storage
- End-to-end analysis workflow
- Concurrent video processing
- Analysis results persistence
- Pagination and filtering

**Key Test Pattern**:
```rust
#[sqlx::test]
async fn test_video_upload_and_storage(pool: PgPool) -> sqlx::Result<()> {
    let storage_service = VideoStorageService::new(pool.clone());

    let video_id = storage_service
        .upload_video(user_id, "test.mp4", video_content)
        .await?;

    // Verify storage
    let stored = sqlx::query!("SELECT * FROM videos WHERE id = $1", video_id)
        .fetch_one(&pool)
        .await?;

    assert_eq!(stored.user_id, user_id);

    Ok(())
}
```

### Load Tests

**Purpose**: Validate performance under concurrent load

**Examples**:
- Concurrent video uploads
- Sustained load (30s, 10 RPS)
- Database connection pooling
- Memory leak detection
- Scalability limits

**Key Test Pattern**:
```rust
#[sqlx::test]
#[ignore]
async fn test_concurrent_video_uploads(pool: PgPool) -> sqlx::Result<()> {
    let config = LoadTestConfig::default();
    let service = Arc::new(VideoStorageService::new(pool));

    // Spawn concurrent users
    for _ in 0..config.concurrent_users {
        // Upload videos concurrently
    }

    // Collect and analyze results
    assert!(success_rate >= 0.95);
    Ok(())
}
```

## Test Data

### Test Video Library

Create test videos using the setup script:

```bash
./scripts/setup_test_videos.sh
```

This creates:

**Standard Videos** (`test_data/videos/standard/`):
- `test_squat.mp4` - Perfect form squat
- `test_deadlift.mp4` - Deadlift exercise
- `test_bench_press.mp4` - Bench press

**Performance Videos** (`test_data/videos/performance/`):
- `test_480p.mp4` - Low resolution (fast)
- `test_1080p.mp4` - Full HD
- `test_4k.mp4` - 4K stress test
- `test_60fps.mp4` - High frame rate
- `test_long.mp4` - 5 minute video

**Edge Cases** (`test_data/videos/edge_cases/`):
- `test_1second.mp4` - Very short video
- `test_144p.mp4` - Very low resolution
- `corrupted.mp4` - Corrupted file
- `wrong_format.mp4` - Invalid format
- `empty.mp4` - Empty file

### Test Metadata

Test video metadata is in `test_data/test_videos.json`:

```json
{
  "standard": [
    {
      "name": "test_squat.mp4",
      "resolution": "1280x720",
      "fps": 30,
      "duration": 10,
      "exercise_type": "squat"
    }
  ],
  "performance": [...],
  "edge_cases": [...]
}
```

## Performance Benchmarks

### Targets

- **Processing Time**: <2x video duration
- **Memory Usage**: <2GB peak
- **Throughput**: >=10 FPS
- **API Latency**: <2000ms P95
- **Error Rate**: <5% under load

### Running Benchmarks

```bash
# Pose estimation benchmark
cargo test test_performance_benchmark -- --ignored --nocapture

# Load test with metrics
cargo test test_concurrent_video_uploads -- --ignored --nocapture

# CPU intensive processing
cargo test test_cpu_intensive_processing_load -- --ignored --nocapture
```

### Expected Output

```
=== Load Test Results ===
Total Requests: 50
Successful: 48
Failed: 2
Success Rate: 96.00%

=== Performance Metrics ===
Total Duration: 12.34s
Average Latency: 247.12ms
P95 Latency: 523.45ms
P99 Latency: 891.23ms
Throughput: 4.05 req/s
```

## CI/CD Integration

### GitHub Actions

The test suite integrates with CI/CD via GitHub Actions:

```yaml
name: Vision Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
    steps:
      - uses: actions/checkout@v3
      - name: Run tests
        run: cargo test
      - name: Generate coverage
        run: ./scripts/generate_coverage.sh
```

### Pre-commit Hooks

Install pre-commit hooks:

```bash
cp scripts/pre-commit .git/hooks/
chmod +x .git/hooks/pre-commit
```

The hook runs:
1. Unit tests
2. Quick integration tests
3. Test compilation check

## Best Practices

### Writing Tests

1. **Arrange-Act-Assert Pattern**
   ```rust
   // Arrange
   let processor = KeypointProcessor::new();

   // Act
   let result = processor.analyze_form(&person, "squat");

   // Assert
   assert!(result.is_ok());
   ```

2. **Descriptive Names**
   - ✅ `test_angle_calculation_with_90_degree_bend`
   - ❌ `test_angle`

3. **Test Isolation**
   - Use `#[sqlx::test]` for database tests
   - Create fresh test data
   - Don't rely on execution order

4. **Error Testing**
   ```rust
   #[test]
   fn test_invalid_input() {
       let result = processor.calculate_angle(&invalid, &p2, &p3);
       assert!(result.is_none());
   }
   ```

### Debugging Failed Tests

```bash
# Run with backtrace
RUST_BACKTRACE=1 cargo test test_name

# Run with logging
RUST_LOG=debug cargo test test_name -- --nocapture

# Run single test
cargo test test_name -- --exact --nocapture
```

## Troubleshooting

### Common Issues

**Database Connection Errors**:
```bash
# Ensure PostgreSQL is running
docker-compose up -d db

# Set correct database URL
export DATABASE_URL=postgresql://postgres:password@localhost:5432/ai_coach
```

**Model Not Found**:
```bash
# Pose estimation tests need ONNX model
# Download or create models/pose_v1.onnx
```

**Compilation Errors**:
```bash
# Clean and rebuild
cargo clean
cargo build --tests
```

**Test Timeouts**:
```bash
# Increase timeout for slow tests
cargo test -- --test-threads=1
```

## Documentation

- **Full Strategy**: `docs/testing-strategy.md`
- **Performance Optimization**: `docs/performance-optimization.md`
- **Validation Framework**: `docs/validation-framework.md`

## Contributing

When adding new features:

1. Write unit tests for core logic
2. Add integration tests for workflows
3. Update test documentation
4. Ensure >80% coverage
5. Run all tests before PR

## Support

For issues or questions:
- Check `docs/testing-strategy.md`
- Review test examples in this directory
- Ask in #engineering-tests channel
