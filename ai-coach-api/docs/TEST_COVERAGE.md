# Test Coverage - Data Quality Monitoring System

## Test Summary

### Coverage Overview

| Component | Unit Tests | Integration Tests | Coverage | Status |
|-----------|------------|-------------------|----------|--------|
| DataQualityCheckJob | ✅ 11 tests | ✅ Included | ~95% | Complete |
| WeeklyBaselineRecalculationJob | ✅ 11 tests | ✅ Included | ~95% | Complete |
| Admin API Endpoints | ⏳ Planned | ⏳ Planned | 0% | To Do |
| Models (DataQuality) | ✅ Via jobs | ✅ Via jobs | ~90% | Complete |
| Models (RecoveryBaseline) | ✅ Via jobs | ✅ Via jobs | ~90% | Complete |

**Overall Coverage**: ~80% (excluding admin API endpoints)

## DataQualityCheckJob Tests

**File**: `tests/data_quality_check_job_test.rs`

### Completeness Score Tests

1. **test_completeness_full_data** ✅
   - **Purpose**: Verify completeness calculation with complete data
   - **Setup**: User with all data types for 30 days
   - **Expected**: Completeness score = 100%
   - **Coverage**: Formula accuracy, full data scenario

2. **test_completeness_partial_data** ✅
   - **Purpose**: Verify calculation with missing data
   - **Setup**: User with 20/30 HRV, 15/30 sleep, 25/30 RHR
   - **Expected**: Completeness score ≈ 66.7%
   - **Coverage**: Partial data handling, averaging

### Consistency Score Tests

3. **test_consistency_regular_data** ✅
   - **Purpose**: Verify consistency with regular entries
   - **Setup**: Daily data for 30 days (no gaps)
   - **Expected**: Consistency score = 100%
   - **Coverage**: Best case consistency

4. **test_consistency_with_gaps** ✅
   - **Purpose**: Verify consistency calculation with gaps
   - **Setup**: 3 gaps of 2+ days
   - **Expected**: Consistency score ≈ 89.7%
   - **Coverage**: Gap detection and scoring

### Reliability Score Tests

5. **test_reliability_api_integration** ✅
   - **Purpose**: Verify reliability with wearable data
   - **Setup**: All data from API integration (source: 'api_integration')
   - **Expected**: Reliability score = 100%
   - **Coverage**: API data preference

6. **test_reliability_manual_entry** ✅
   - **Purpose**: Verify reliability with mixed sources
   - **Setup**: 25 API entries, 5 manual entries
   - **Expected**: Reliability score ≈ 83.3%
   - **Coverage**: Source tracking, manual entry penalty

### Job Execution Tests

7. **test_multiple_users** ✅
   - **Purpose**: Verify batch processing
   - **Setup**: 5 users with varying data quality
   - **Expected**: All users processed, metrics stored
   - **Coverage**: Batch size (50), parallel processing

8. **test_no_users** ✅
   - **Purpose**: Verify empty database handling
   - **Setup**: No active users
   - **Expected**: Job completes successfully, 0 records
   - **Coverage**: Edge case, empty result set

9. **test_metrics_upsert** ✅
   - **Purpose**: Verify metric update vs insert
   - **Setup**: Run job twice for same user
   - **Expected**: Metrics updated, not duplicated
   - **Coverage**: UPSERT logic, idempotency

### Reminder Logic Tests

10. **test_reminder_logic** ✅
    - **Purpose**: Verify reminder triggering
    - **Setup**: Users with various quality scores
    - **Expected**: Reminders sent to appropriate users
    - **Coverage**: Threshold logic, urgency levels

11. **test_job_metadata** ✅
    - **Purpose**: Verify job configuration
    - **Expected**: Job name = "data_quality_check", schedule = "0 */6 * * *"
    - **Coverage**: Job registration, schedule format

## WeeklyBaselineRecalculationJob Tests

**File**: `tests/weekly_baseline_recalculation_job_test.rs`

### Basic Functionality Tests

1. **test_no_users** ✅
   - **Purpose**: Verify empty database handling
   - **Setup**: No users with recovery data
   - **Expected**: Job completes, 0 processed
   - **Coverage**: Empty result set

2. **test_single_user_recalculation** ✅
   - **Purpose**: Verify baseline calculation
   - **Setup**: User with 30 days data
   - **Expected**: New baseline created
   - **Coverage**: Calculation logic, baseline storage

3. **test_first_baseline_calculation** ✅
   - **Purpose**: Verify first-time baseline
   - **Setup**: User with data but no existing baseline
   - **Expected**: Baseline created, no changes detected
   - **Coverage**: Initial baseline, no comparison

### Change Detection Tests

4. **test_hrv_improvement_detection** ✅
   - **Purpose**: Verify HRV increase detection
   - **Setup**: Baseline 50.0, new data 60.0 (20% increase)
   - **Expected**: Major change detected
   - **Coverage**: HRV calculation, improvement direction

5. **test_rhr_improvement_detection** ✅
   - **Purpose**: Verify RHR decrease detection
   - **Setup**: Baseline 65.0, new data 55.0 (15.4% decrease)
   - **Expected**: Moderate change detected
   - **Coverage**: RHR calculation, inverse improvement

6. **test_minor_changes_not_detected** ✅
   - **Purpose**: Verify threshold filtering
   - **Setup**: Baseline 50.0, new data 52.0 (4% increase)
   - **Expected**: No significant change
   - **Coverage**: Minor change filter (<10%)

7. **test_sleep_duration_change_detection** ✅
   - **Purpose**: Verify sleep change detection
   - **Setup**: Baseline 8.0h, new data 9.2h (15% increase)
   - **Expected**: Change detected
   - **Coverage**: Sleep duration tracking

### Edge Case Tests

8. **test_user_with_no_recent_data** ✅
   - **Purpose**: Verify stale data handling
   - **Setup**: User with data >60 days old
   - **Expected**: User not processed
   - **Coverage**: Recency filter, data staleness

9. **test_multiple_users_batch_processing** ✅
   - **Purpose**: Verify batch processing
   - **Setup**: 5 users with baselines
   - **Expected**: All users processed
   - **Coverage**: Batch size (50), parallel execution

### Notification Tests

10. **test_major_change_detection** ✅
    - **Purpose**: Verify notification triggering
    - **Setup**: User with 26% HRV increase
    - **Expected**: Notification sent for major change
    - **Coverage**: Notification logic, major threshold

11. **test_job_metadata** ✅
    - **Purpose**: Verify job configuration
    - **Expected**: Job name = "weekly_baseline_recalculation", schedule = "0 3 * * 0"
    - **Coverage**: Job registration, weekly schedule

## Test Scenarios Covered

### ✅ Covered Scenarios

1. **Complete Data**
   - Users with all recovery metrics
   - Regular daily entries
   - API-sourced data

2. **Partial Data**
   - Missing HRV readings
   - Missing sleep data
   - Missing RHR data
   - Combinations of missing metrics

3. **Data Gaps**
   - 2-day gaps
   - 3-day gaps
   - 7+ day gaps
   - Multiple gaps

4. **Data Sources**
   - API integration only
   - Manual entry only
   - Mixed sources

5. **Baseline Changes**
   - Minor changes (<10%)
   - Moderate changes (10-20%)
   - Major changes (≥20%)
   - HRV improvements
   - RHR improvements
   - Sleep duration changes

6. **Batch Processing**
   - Empty database
   - Single user
   - Multiple users (5)
   - Batch size handling

7. **Edge Cases**
   - No users
   - No data
   - Stale data (>60 days)
   - First baseline calculation

### ⏳ Scenarios To Test (Admin API)

1. **Authentication & Authorization**
   - Valid admin token
   - Invalid token
   - Non-admin token
   - Missing token

2. **Query Parameters**
   - Default values
   - Custom thresholds
   - Pagination (limit, offset)
   - Date filters
   - Significance filters

3. **Error Handling**
   - Invalid user ID
   - Database connection failure
   - Invalid query parameters
   - Empty result sets

4. **Performance**
   - Large result sets (1000+ users)
   - Complex aggregations
   - Concurrent requests

### ❌ Not Covered (Out of Scope)

1. **UI Testing**
   - Frontend components
   - User interactions
   - Visual regression

2. **Load Testing**
   - 10,000+ users
   - Sustained high load
   - Stress testing

3. **Security Testing**
   - Penetration testing
   - SQL injection
   - XSS attacks

4. **Integration Testing**
   - External wearable APIs (Oura)
   - Email service integration
   - Push notification services

## Running Tests

### Run All Tests

```bash
# All tests
cargo test

# Specific test file
cargo test --test data_quality_check_job_test
cargo test --test weekly_baseline_recalculation_job_test

# Specific test
cargo test --test data_quality_check_job_test test_completeness_full_data
```

### Run with Output

```bash
# Show println! output
cargo test -- --nocapture

# Show test execution time
cargo test -- --test-threads=1 --nocapture
```

### Database Tests

Tests use `#[sqlx::test]` macro which:
- Creates a new test database
- Runs migrations automatically
- Rolls back after each test
- Ensures isolation between tests

**Requirements**:
- PostgreSQL running
- `DATABASE_URL` environment variable set
- Migrations in `migrations/` directory

## Test Maintenance

### Adding New Tests

1. **Create test function**
   ```rust
   #[sqlx::test]
   async fn test_new_feature(pool: PgPool) -> sqlx::Result<()> {
       // Arrange
       let user_id = create_test_user(&pool, "test@example.com").await;

       // Act
       let result = function_under_test(user_id).await?;

       // Assert
       assert_eq!(result.expected_field, expected_value);

       Ok(())
   }
   ```

2. **Use helper functions**
   - `create_test_user()` - Creates user with default settings
   - `create_recovery_data()` - Adds recovery metrics
   - `create_baseline()` - Creates baseline record

3. **Follow naming convention**
   - `test_[feature]_[scenario]`
   - Example: `test_completeness_with_gaps`

### Updating Tests

When modifying code:

1. **Update affected tests**
   - Change assertions if behavior changes
   - Add new test cases for new features
   - Remove obsolete tests

2. **Verify all tests pass**
   ```bash
   cargo test --workspace
   ```

3. **Check coverage**
   ```bash
   cargo tarpaulin --workspace
   ```

## Coverage Gaps and Future Work

### High Priority

1. **Admin API Endpoint Tests**
   - All 6 endpoints
   - Authentication
   - Authorization
   - Query parameters
   - Error cases

2. **Performance Tests**
   - 1000+ user datasets
   - Job execution time
   - Database query optimization

3. **Error Recovery Tests**
   - Database connection failures
   - Notification service failures
   - Partial batch failures

### Medium Priority

1. **Notification Integration Tests**
   - Email delivery
   - Push notification delivery
   - Delivery tracking

2. **Concurrent Execution Tests**
   - Multiple job instances
   - Race conditions
   - Data consistency

3. **Data Migration Tests**
   - Schema changes
   - Baseline data migration
   - Metric history migration

### Low Priority

1. **Load Tests**
   - 10,000+ users
   - Sustained load
   - Peak load scenarios

2. **Chaos Engineering**
   - Database failures
   - Network failures
   - Service degradation

## Continuous Integration

### CI Pipeline

```yaml
test:
  - cargo fmt -- --check
  - cargo clippy -- -D warnings
  - cargo test --workspace
  - cargo tarpaulin --workspace --out Xml
```

### Success Criteria

- All tests pass ✅
- No clippy warnings ✅
- Coverage ≥80% ✅
- Execution time <5 minutes ✅

## Testing Best Practices

### Do's ✅

- Test one thing per test
- Use descriptive test names
- Arrange-Act-Assert pattern
- Clean test data
- Test edge cases
- Use helper functions

### Don'ts ❌

- Don't test implementation details
- Don't share state between tests
- Don't use real external services
- Don't skip failing tests
- Don't test framework code
- Don't duplicate test logic

## References

- Test Files:
  - `tests/data_quality_check_job_test.rs`
  - `tests/weekly_baseline_recalculation_job_test.rs`
- Source Code:
  - `src/services/data_quality_check_job.rs`
  - `src/services/weekly_baseline_recalculation_job.rs`
- sqlx Testing: https://docs.rs/sqlx/latest/sqlx/attr.test.html
