# Data Quality Monitoring System

## Overview

The Data Quality Monitoring system tracks recovery data completeness and quality for all users, automatically detects issues, and sends targeted reminders to improve data collection rates.

## Goals

- Achieve **≥80% average data completeness** across all users
- Reduce users with poor quality (<50% completeness) to **<10%**
- Detect significant baseline changes within **24 hours**
- Improve data completeness by **20% within 30 days** through automated reminders

## System Components

### 1. DataQualityCheckJob

**Schedule**: Every 6 hours (0 */6 * * *)

**Purpose**: Calculate quality metrics for all active users and send reminders when needed.

**Metrics Calculated**:
- **Completeness Score** (0-100%): Percentage of expected data points collected
- **Consistency Score** (0-100%): Regularity of data collection patterns
- **Reliability Score** (0-100%): Quality of data sources (API integration > manual)

**Reminder Triggers**:
- Days without data ≥ 3
- Completeness score < 50%
- Missing critical recovery metrics (HRV, sleep, RHR)

### 2. WeeklyBaselineRecalculationJob

**Schedule**: Weekly on Sunday at 3 AM UTC (0 3 * * 0)

**Purpose**: Recalculate recovery baselines and detect significant changes.

**Detection Thresholds**:
- **Minor Change**: <10% deviation from baseline
- **Moderate Change**: 10-20% deviation from baseline
- **Major Change**: ≥20% deviation from baseline

**Notifications**:
- Moderate changes: In-app notification
- Major changes: Push notification + email

### 3. Admin Dashboard API

**Base Path**: `/api/v1/admin/data-quality`

**Endpoints**:
- `GET /summary` - Aggregate statistics
- `GET /users/poor` - Users with low completeness
- `GET /users/missing` - Users with data gaps
- `GET /users/{user_id}/history` - Individual user trends
- `GET /baselines/changes` - Recent baseline changes
- `GET /trends` - System-wide trends over time

## Metrics Calculation Formulas

### Completeness Score

```rust
completeness_score = (data_points_collected / expected_data_points) * 100

where:
  expected_data_points = days_in_period * data_types_expected
  data_types_expected = 3 (HRV, Sleep, RHR)
```

**Example**: User has 20 HRV readings, 18 sleep records, and 22 RHR readings over 30 days:
```
collected = 20 + 18 + 22 = 60
expected = 30 days × 3 types = 90
completeness = 60/90 × 100 = 66.7%
```

### Consistency Score

```rust
consistency_score = (1 - (gaps_count / expected_collections)) * 100

where:
  gaps_count = number of 2+ day gaps in data
  expected_collections = days_in_period - 1
```

**Example**: User has 3 gaps of 2+ days over 30 days:
```
consistency = (1 - 3/29) × 100 = 89.7%
```

### Reliability Score

```rust
reliability_score = (api_data_points / total_data_points) * 100

where:
  api_data_points = data from wearable integrations (Oura, etc.)
  total_data_points = api_data_points + manual_entries
```

**Example**: User has 25 API entries and 5 manual entries:
```
reliability = 25/30 × 100 = 83.3%
```

### Days Without Data

```rust
days_without_data = current_date - max(last_hrv_date, last_sleep_date, last_rhr_date)
```

### Baseline Change Detection

```rust
percent_change = ((new_baseline - old_baseline) / old_baseline) * 100

significance =
  if |percent_change| >= 20.0 then Major
  else if |percent_change| >= 10.0 then Moderate
  else Minor
```

## Reminder Logic

### Reminder Urgency Levels

| Urgency | Conditions | Action |
|---------|-----------|--------|
| **High** | Days without data ≥ 7 | Send push notification immediately |
| **Medium** | Days without data ≥ 3 OR completeness < 30% | Queue for batch processing |
| **Low** | Completeness < 50% | Send weekly summary |
| **None** | Completeness ≥ 50% AND recent data | No reminder needed |

### Reminder Frequency

- **Maximum**: 1 reminder per user per 24 hours
- **Cooldown**: 7 days after user adds data
- **Batch Processing**: Reminders sent every 6 hours

## Performance Targets

| Component | Target | Current |
|-----------|--------|---------|
| DataQualityCheckJob | <2 minutes for 1000 users | TBD |
| WeeklyBaselineRecalculationJob | <5 minutes for 1000 users | TBD |
| Admin API response time | <500ms | TBD |
| Database queries | <100ms per query | TBD |

## Monitoring and Alerts

### Key Metrics to Monitor

1. **Average Completeness Score**
   - Target: ≥80%
   - Alert: <70%

2. **Users with Poor Quality**
   - Target: <10%
   - Alert: >20%

3. **Job Execution Time**
   - DataQualityCheckJob: Alert if >3 minutes
   - WeeklyBaselineRecalculationJob: Alert if >7 minutes

4. **Job Success Rate**
   - Target: ≥99%
   - Alert: <95%

5. **Reminder Delivery Rate**
   - Target: ≥98%
   - Alert: <90%

### Prometheus Metrics (Planned)

```promql
# Gauges
data_quality_avg_completeness{period="30d"}
data_quality_poor_users
data_quality_users_with_gaps

# Counters
data_quality_reminders_sent_total{urgency="high|medium|low"}
baseline_changes_detected_total{significance="major|moderate|minor"}

# Histograms
data_quality_job_duration_seconds{job="quality_check|baseline_recalc"}
admin_api_request_duration_seconds{endpoint="/summary|/poor|..."}
```

### Recommended Alerts

```yaml
# Average completeness drops below 70%
- alert: LowDataCompleteness
  expr: data_quality_avg_completeness < 70
  for: 1h
  severity: warning

# More than 20% of users have poor quality
- alert: HighPoorQualityUsers
  expr: (data_quality_poor_users / data_quality_total_users) > 0.20
  for: 30m
  severity: warning

# Job taking too long
- alert: SlowDataQualityJob
  expr: data_quality_job_duration_seconds > 180
  for: 5m
  severity: critical

# Job failing
- alert: DataQualityJobFailure
  expr: rate(data_quality_job_failures_total[5m]) > 0
  for: 1m
  severity: critical
```

## Success Metrics

### Primary KPIs

1. **Data Completeness Improvement**
   - Baseline: Current average completeness
   - Target: +20% within 30 days
   - Measurement: Weekly average completeness trend

2. **Reminder Effectiveness**
   - Metric: % of users improving completeness after reminder
   - Target: ≥40% improvement within 7 days
   - Measurement: Before/after completeness comparison

3. **User Engagement**
   - Metric: % of users responding to reminders
   - Target: ≥60% add data within 48 hours
   - Measurement: Data additions after reminder sent

4. **Baseline Change Detection Accuracy**
   - Metric: % of significant changes detected within 24 hours
   - Target: ≥95%
   - Measurement: Detection latency analysis

### Secondary KPIs

- Average days without data (target: <2 days)
- Wearable integration adoption (target: ≥40%)
- Manual entry frequency (target: ≥3 entries/week)
- Data consistency score (target: ≥85%)

## Troubleshooting Guide

### Issue: Job not running

**Symptoms**:
- No recent execution logs
- Metrics not updating

**Diagnosis**:
```bash
# Check job status
curl -H "Authorization: Bearer $TOKEN" \
  https://api.example.com/api/v1/admin/jobs/status/data_quality_check

# Check scheduler health
curl -H "Authorization: Bearer $TOKEN" \
  https://api.example.com/api/v1/admin/jobs/health
```

**Solutions**:
1. Verify scheduler is running (check main.rs startup logs)
2. Check cron expression is valid
3. Restart application if scheduler stopped

### Issue: Reminders not being sent

**Symptoms**:
- Poor quality users not receiving notifications
- Reminder delivery count = 0

**Diagnosis**:
```bash
# Check notification service
curl -H "Authorization: Bearer $TOKEN" \
  https://api.example.com/api/v1/admin/alerts/delivery/status

# Check user notification preferences
SELECT email_notifications_enabled, push_notifications_enabled
FROM users
WHERE id = 'user_uuid';
```

**Solutions**:
1. Verify notification service is configured (email/push credentials)
2. Check user has enabled notifications in preferences
3. Review alert delivery queue for pending/failed deliveries

### Issue: Inaccurate completeness scores

**Symptoms**:
- Scores don't match expected values
- Users with data showing 0% completeness

**Diagnosis**:
```sql
-- Check recovery data for user
SELECT
  (SELECT COUNT(*) FROM hrv_readings WHERE user_id = 'uuid' AND measurement_date >= CURRENT_DATE - 30) as hrv_count,
  (SELECT COUNT(*) FROM sleep_data WHERE user_id = 'uuid' AND sleep_date >= CURRENT_DATE - 30) as sleep_count,
  (SELECT COUNT(*) FROM resting_hr_data WHERE user_id = 'uuid' AND measurement_date >= CURRENT_DATE - 30) as rhr_count;

-- Check quality metrics
SELECT * FROM data_quality_metrics WHERE user_id = 'uuid' ORDER BY metric_date DESC LIMIT 5;
```

**Solutions**:
1. Verify data exists in recovery tables
2. Check date filters in quality calculation
3. Re-run DataQualityCheckJob manually
4. Review metric calculation logic in data_quality_check_job.rs

### Issue: Slow job execution

**Symptoms**:
- Job takes >5 minutes to complete
- Database connection timeouts

**Diagnosis**:
```sql
-- Check active users count
SELECT COUNT(*) FROM users WHERE active = true;

-- Check data volume
SELECT COUNT(*) FROM data_quality_metrics;

-- Check for missing indexes
SELECT schemaname, tablename, indexname
FROM pg_indexes
WHERE tablename IN ('data_quality_metrics', 'hrv_readings', 'sleep_data', 'resting_hr_data');
```

**Solutions**:
1. Add indexes on user_id and date columns
2. Increase batch size (currently 50 users)
3. Reduce lookback period (currently 30 days)
4. Optimize database queries with EXPLAIN ANALYZE

### Issue: Baseline changes not detected

**Symptoms**:
- Significant changes in data but no notifications
- Baseline values not updating

**Diagnosis**:
```sql
-- Check baseline history
SELECT * FROM recovery_baselines
WHERE user_id = 'uuid'
ORDER BY calculated_at DESC
LIMIT 5;

-- Check recent recovery data
SELECT AVG(rmssd) as avg_hrv, AVG(resting_hr) as avg_rhr
FROM hrv_readings h
JOIN resting_hr_data r ON h.user_id = r.user_id
WHERE h.user_id = 'uuid' AND h.measurement_date >= CURRENT_DATE - 30;
```

**Solutions**:
1. Verify WeeklyBaselineRecalculationJob is running
2. Check if user has sufficient data (≥30 days)
3. Review change detection thresholds (10%, 20%)
4. Check if baseline actually changed significantly

## Database Schema

### data_quality_metrics

```sql
CREATE TABLE data_quality_metrics (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    metric_date DATE NOT NULL,

    completeness_score DOUBLE PRECISION NOT NULL,
    consistency_score DOUBLE PRECISION,
    reliability_score DOUBLE PRECISION,

    last_data_timestamp TIMESTAMPTZ,
    days_without_data INTEGER NOT NULL DEFAULT 0,
    data_sources JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(user_id, metric_date)
);

CREATE INDEX idx_dqm_user_date ON data_quality_metrics(user_id, metric_date DESC);
CREATE INDEX idx_dqm_completeness ON data_quality_metrics(completeness_score);
CREATE INDEX idx_dqm_days_without ON data_quality_metrics(days_without_data);
```

### recovery_baselines

```sql
CREATE TABLE recovery_baselines (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,

    hrv_baseline_rmssd DOUBLE PRECISION,
    rhr_baseline DOUBLE PRECISION,
    typical_sleep_hours DOUBLE PRECISION,

    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    data_points_count INTEGER NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(user_id)
);

CREATE INDEX idx_rb_user ON recovery_baselines(user_id);
CREATE INDEX idx_rb_calculated ON recovery_baselines(calculated_at DESC);
```

## API Usage Examples

### Get System Summary

```bash
curl -X GET "https://api.example.com/api/v1/admin/data-quality/summary" \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

Response:
```json
{
  "total_users": 1250,
  "users_with_good_quality": 950,
  "users_with_poor_quality": 125,
  "users_with_missing_data": 180,
  "avg_completeness_score": 76.5,
  "avg_days_without_data": 1.8
}
```

### Get Poor Quality Users

```bash
curl -X GET "https://api.example.com/api/v1/admin/data-quality/users/poor?threshold=50&limit=10" \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

### Get Quality Trends

```bash
curl -X GET "https://api.example.com/api/v1/admin/data-quality/trends?period=month" \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

## Maintenance

### Daily Tasks

- Monitor average completeness score
- Review reminder delivery success rate
- Check job execution logs for errors

### Weekly Tasks

- Review baseline change notifications
- Analyze quality trend graphs
- Identify users needing manual intervention

### Monthly Tasks

- Evaluate reminder effectiveness
- Optimize slow queries
- Review and update quality thresholds
- Generate executive summary report

## Future Enhancements

1. **Machine Learning Integration**
   - Predict users likely to have data quality issues
   - Personalized reminder timing based on user behavior
   - Anomaly detection for unusual patterns

2. **Advanced Notifications**
   - Contextual reminders based on training schedule
   - Gamification elements (streaks, achievements)
   - Social comparison (anonymized benchmarks)

3. **Wearable Integration Improvements**
   - Auto-reconnect for disconnected wearables
   - Real-time sync status monitoring
   - Multi-wearable support

4. **Enhanced Analytics**
   - Cohort analysis (new vs. established users)
   - Seasonal variation tracking
   - Correlation with training outcomes

## References

- DataQualityCheckJob: `src/services/data_quality_check_job.rs`
- WeeklyBaselineRecalculationJob: `src/services/weekly_baseline_recalculation_job.rs`
- Admin API: `src/api/data_quality_admin.rs`
- Tests: `tests/data_quality_check_job_test.rs`, `tests/weekly_baseline_recalculation_job_test.rs`
