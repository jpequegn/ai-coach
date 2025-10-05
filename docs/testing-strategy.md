# Vision Analysis Testing Strategy

## Overview

Comprehensive test suite for the AI Coach vision analysis system, ensuring reliability, performance, and correctness across all components.

**Issue**: #89 - Phase 7.3: Comprehensive test suite (unit, integration, load)

## Test Coverage Goals

- **Unit Tests**: >80% code coverage
- **Integration Tests**: All critical workflows
- **Load Tests**: Scalability validation
- **Performance Tests**: Sub-2x video duration processing

## Test Organization

### Directory Structure

```
ai-coach-api/tests/
├── unit/                          # Unit tests for business logic
│   ├── keypoint_processor_test.rs # Angle/distance calculations, form scoring
│   ├── goal_service_test.rs       # Goal management logic
│   ├── user_service_test.rs       # User service logic
│   └── ...
├── integration/                    # Integration tests
│   ├── vision_pipeline_test.rs    # End-to-end video processing
│   ├── api_endpoints_test.rs      # API endpoint integration
│   └── ...
├── pose_estimation_service_test.rs # Service-level pose tests
├── vision_load_test.rs            # Load and scalability tests
└── common/                        # Shared test utilities
    └── mod.rs
```

## Unit Tests

### Keypoint Processor Tests (`keypoint_processor_test.rs`)

**Purpose**: Validate core mathematical operations and form analysis algorithms

**Test Coverage**:
- ✅ Angle calculations (90°, 180°, 45°, edge cases)
- ✅ Distance calculations (horizontal, vertical, diagonal)
- ✅ Alignment detection (vertical/horizontal with thresholds)
- ✅ Form scoring for perfect and poor posture
- ✅ Issue detection (knee alignment, depth, symmetry)
- ✅ Missing keypoint handling
- ✅ Confidence threshold enforcement
- ✅ Edge cases (zero-length vectors, invalid inputs)

**Key Test Patterns**:
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

### Pose Estimation Tests (`pose_estimation_service_test.rs`)

**Purpose**: Validate ML model inference and preprocessing

**Test Coverage**:
- ✅ Service initialization and model loading
- ✅ Image preprocessing pipeline
- ✅ Inference with various image sizes (320x240 to 1920x1080)
- ✅ Keypoint structure validation (17 COCO keypoints)
- ✅ Coordinate normalization
- ✅ Performance benchmarking (<100ms target)
- ✅ Error handling (invalid model path)

**Performance Targets**:
- Inference time: <100ms per frame
- Throughput: >=10 FPS
- Multiple consecutive inferences stable

## Integration Tests

### Vision Pipeline Tests (`vision_pipeline_test.rs`)

**Purpose**: Validate end-to-end video processing workflows

**Test Coverage**:
- ✅ Video upload and storage
- ✅ Video retrieval and status updates
- ✅ End-to-end analysis workflow
- ✅ Concurrent video uploads
- ✅ Analysis results persistence
- ✅ Error handling (not found, invalid format)
- ✅ User video listing with pagination

**Database Integration**:
- Uses `#[sqlx::test]` for automatic transaction rollback
- Tests database operations with real PostgreSQL
- Validates data persistence and retrieval

**Example Test**:
```rust
#[sqlx::test]
async fn test_end_to_end_analysis_workflow(pool: PgPool) -> sqlx::Result<()> {
    let vision_service = VisionAnalysisService::new(pool.clone());
    let video_content = fs::read("test.mp4").await.unwrap();

    // Upload and analyze
    let response = vision_service
        .upload_and_analyze(user_id, "test.mp4", video_content, "squat")
        .await?;

    assert_eq!(response.status, "processing" | "completed");
    Ok(())
}
```

## Load Tests

### Vision Load Tests (`vision_load_test.rs`)

**Purpose**: Validate system performance under concurrent load

**Test Coverage**:
- ✅ Concurrent video uploads (configurable users and requests)
- ✅ Sustained load testing (30s duration, 10 RPS)
- ✅ Database connection pooling under load
- ✅ Memory leak detection (100 iterations)
- ✅ Error rate testing with mixed valid/invalid requests
- ✅ Scalability limits (10, 25, 50, 100 concurrent)
- ✅ CPU-intensive processing load

**Load Test Configuration**:
```rust
struct LoadTestConfig {
    concurrent_users: 10,
    requests_per_user: 5,
    video_size_kb: 100,
    target_throughput_rps: 50.0,
    max_latency_ms: 2000,
}
```

**Performance Metrics**:
- Total requests and success rate
- Average, P95, P99 latency
- Throughput (requests/second)
- Error rate and types

**Scalability Validation**:
- Tests increasing load levels: 10 → 25 → 50 → 100
- Measures throughput degradation
- Identifies breaking points

## Running Tests

### Unit Tests

```bash
# Run all unit tests
cargo test --test unit

# Run specific unit test file
cargo test --test keypoint_processor_test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_angle_calculation_90_degrees -- --exact
```

### Integration Tests

```bash
# Run all integration tests (requires database)
export DATABASE_URL=postgresql://postgres:password@localhost:5432/ai_coach
cargo test --test integration

# Run vision pipeline tests
cargo test --test vision_pipeline_test

# Run serially (for database tests)
cargo test -- --test-threads=1
```

### Load Tests

```bash
# Run all load tests (ignored by default)
cargo test --test vision_load_test -- --ignored

# Run specific load test
cargo test test_concurrent_video_uploads -- --ignored

# Run with verbose output
cargo test --test vision_load_test -- --ignored --nocapture
```

### Performance Tests

```bash
# Run pose estimation benchmarks
cargo test test_performance_benchmark -- --ignored --nocapture

# Run CPU load test
cargo test test_cpu_intensive_processing_load -- --ignored
```

## Test Coverage

### Generating Coverage Report

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir target/coverage

# View report
open target/coverage/index.html
```

### Coverage Targets

| Component | Target | Status |
|-----------|--------|--------|
| Keypoint Processor | >80% | ✅ |
| Pose Estimation | >80% | ✅ |
| Video Storage | >80% | ✅ |
| Vision Analysis | >80% | ✅ |
| Overall | >80% | 🎯 |

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: Vision Analysis Tests

on:
  push:
    branches: [main, develop]
    paths:
      - 'ai-coach-api/src/services/vision_*.rs'
      - 'ai-coach-api/src/services/pose_*.rs'
      - 'ai-coach-api/src/services/keypoint_*.rs'
  pull_request:
    branches: [main, develop]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: password
          POSTGRES_DB: ai_coach_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run unit tests
        run: cargo test --lib

      - name: Run integration tests
        env:
          DATABASE_URL: postgresql://postgres:password@localhost:5432/ai_coach_test
        run: cargo test --test integration

      - name: Run vision tests
        run: cargo test --test pose_estimation_service_test

      - name: Generate coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Lcov --output-dir ./coverage

      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: ./coverage/lcov.info
```

### Pre-commit Hooks

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Run unit tests
cargo test --lib || exit 1

# Run quick integration tests
cargo test --test keypoint_processor_test || exit 1

# Check test compilation
cargo test --no-run || exit 1

echo "✓ All pre-commit tests passed"
```

## Test Data

### Test Video Library

Located in `test_data/` directory:

**Standard Test Videos**:
- `test_squat.mp4` - Perfect form squat (10s, 720p, 30fps)
- `test_squat_poor_form.mp4` - Poor form with common issues
- `test_deadlift.mp4` - Deadlift exercise
- `test_bench_press.mp4` - Bench press exercise

**Edge Case Videos**:
- `corrupted.mp4` - Corrupted file for error handling
- `wrong_format.avi` - Unsupported format
- `4k_video.mp4` - 4K resolution for performance testing
- `long_video.mp4` - 5 minute video for memory testing
- `low_quality.mp4` - 240p low quality video

**Performance Test Videos**:
- `30fps_video.mp4` - Standard frame rate
- `60fps_video.mp4` - High frame rate
- `variable_fps.mp4` - Variable frame rate

### Creating Test Videos

```bash
# Create test video with FFmpeg
ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=30 \
  -pix_fmt yuv420p test_squat.mp4

# Create 4K test video
ffmpeg -f lavfi -i testsrc=duration=5:size=3840x2160:rate=30 \
  -pix_fmt yuv420p 4k_video.mp4

# Create corrupted file
dd if=/dev/urandom of=corrupted.mp4 bs=1024 count=100
```

## Quality Gates

### Automated Quality Checks

1. **Syntax & Type Checking**: `cargo check`
2. **Linting**: `cargo clippy -- -D warnings`
3. **Formatting**: `cargo fmt -- --check`
4. **Unit Tests**: Must pass with >80% coverage
5. **Integration Tests**: All critical paths must pass
6. **Load Tests**: Performance targets must be met
7. **Security Scan**: `cargo audit`

### Performance Validation

**Acceptance Criteria**:
- Processing time: <2x video duration ✅
- Memory usage: <2GB peak ✅
- Throughput: >=10 FPS ✅
- API latency: <2000ms P95 ✅
- Error rate: <5% under load ✅

### Test Failure Response

**Critical Failures** (Block deployment):
- Unit test failures
- Integration test failures
- Security vulnerabilities
- Performance regression >20%

**Warning Failures** (Review required):
- Coverage drop >5%
- Load test degradation >10%
- New linting warnings

## Best Practices

### Writing Tests

1. **Arrange-Act-Assert Pattern**:
   ```rust
   // Arrange
   let processor = KeypointProcessor::new();
   let keypoints = create_test_keypoints();

   // Act
   let result = processor.analyze_form(&keypoints, "squat");

   // Assert
   assert!(result.is_ok());
   assert!(result.unwrap().overall_score > 0.7);
   ```

2. **Test Naming**: Use descriptive names
   - ✅ `test_angle_calculation_with_90_degree_bend`
   - ❌ `test_angle`

3. **Test Isolation**: Each test should be independent
   - Use `#[sqlx::test]` for automatic database cleanup
   - Create fresh test data in each test
   - Don't rely on test execution order

4. **Error Testing**: Test both success and failure paths
   ```rust
   #[test]
   fn test_invalid_input_handling() {
       let result = processor.calculate_angle(&invalid_keypoint, &p2, &p3);
       assert!(result.is_none());
   }
   ```

5. **Performance Testing**: Use `#[ignore]` for slow tests
   ```rust
   #[test]
   #[ignore]
   fn test_performance_benchmark() {
       // Long-running performance test
   }
   ```

### Debugging Failed Tests

```bash
# Run test with backtrace
RUST_BACKTRACE=1 cargo test test_name

# Run test with logging
RUST_LOG=debug cargo test test_name -- --nocapture

# Run single test in isolation
cargo test test_name -- --exact --nocapture
```

## Monitoring & Metrics

### Test Execution Metrics

Track in CI/CD:
- Test execution time trends
- Flaky test detection
- Coverage trends over time
- Performance regression detection

### Production Correlation

Map test results to production metrics:
- Load test results → Production throughput
- Performance tests → User-perceived latency
- Error handling tests → Production error rates

## Future Enhancements

1. **Visual Regression Testing**
   - Screenshot comparison for UI components
   - Form visualization validation

2. **Mutation Testing**
   - Verify test suite effectiveness
   - Identify untested code paths

3. **Chaos Engineering**
   - Network failure simulation
   - Database connection loss
   - Resource exhaustion scenarios

4. **A/B Test Framework**
   - Test different ML models
   - Compare algorithm variations
   - Performance optimization validation

## References

- Issue #89: Phase 7.3: Comprehensive test suite
- Issue #88: Phase 7.2: Performance optimization
- Issue #87: Phase 7.1: Validation Framework
- Rust testing best practices: https://doc.rust-lang.org/book/ch11-00-testing.html
- SQLx testing: https://docs.rs/sqlx/latest/sqlx/attr.test.html
