# Admin Runbook - Data Quality Monitoring

## Quick Reference

| Task | Command | Frequency |
|------|---------|-----------|
| Check system health | `curl $BASE/admin/data-quality/summary` | Daily |
| View poor quality users | `curl $BASE/admin/data-quality/users/poor` | Daily |
| Check job status | `curl $BASE/admin/jobs/status` | As needed |
| Trigger manual job run | See "Manual Job Execution" | As needed |
| Review baseline changes | `curl $BASE/admin/data-quality/baselines/changes` | Weekly |

## System Health Check

### Morning Health Check (5 minutes)

1. **Check Overall System Health**
   ```bash
   curl -H "Authorization: Bearer $ADMIN_TOKEN" \
     "$BASE_URL/api/v1/admin/data-quality/summary"
   ```

   **Expected Values**:
   - `avg_completeness_score`: ≥70%
   - `users_with_poor_quality`: <20% of total
   - `users_with_missing_data`: <15% of total

   **Action Items**:
   - If avg_completeness < 70%: Review trend and identify causes
   - If poor_quality_users > 20%: Check reminder delivery
   - If missing_data_users > 15%: Review wearable connections

2. **Check Job Execution**
   ```bash
   curl -H "Authorization: Bearer $ADMIN_TOKEN" \
     "$BASE_URL/api/v1/admin/jobs/health"
   ```

   **Expected**:
   - All jobs: `healthy: true`
   - `consecutive_failures`: 0

   **Action Items**:
   - If any job unhealthy: Check logs and consider manual trigger
   - If failures > 3: Escalate to engineering team

3. **Review Recent Baseline Changes**
   ```bash
   curl -H "Authorization: Bearer $ADMIN_TOKEN" \
     "$BASE_URL/api/v1/admin/data-quality/baselines/changes?limit=20"
   ```

   **Monitor For**:
   - Unusual number of major changes (>5% of users)
   - Patterns in changes (seasonal, widespread)

## Common Operations

### Manual Job Execution

**When to Run Manually**:
- Job failed and needs retry
- Testing after configuration change
- Backfilling missed execution

**DataQualityCheckJob**:
```bash
# Trigger via admin endpoint (when implemented)
curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" \
  "$BASE_URL/api/v1/admin/jobs/data_quality_check/trigger"

# Alternative: Restart application (job runs on schedule)
```

**WeeklyBaselineRecalculationJob**:
```bash
# Trigger via admin endpoint
curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" \
  "$BASE_URL/api/v1/admin/jobs/weekly_baseline_recalculation/trigger"
```

### Investigate Poor Quality User

1. **Get User Quality History**
   ```bash
   curl -H "Authorization: Bearer $ADMIN_TOKEN" \
     "$BASE_URL/api/v1/admin/data-quality/users/$USER_ID/history"
   ```

2. **Check User's Raw Data**
   ```sql
   -- HRV readings (last 30 days)
   SELECT measurement_date, rmssd, source
   FROM hrv_readings
   WHERE user_id = '$USER_ID' AND measurement_date >= CURRENT_DATE - 30
   ORDER BY measurement_date DESC;

   -- Sleep data
   SELECT sleep_date, total_sleep_hours, source
   FROM sleep_data
   WHERE user_id = '$USER_ID' AND sleep_date >= CURRENT_DATE - 30
   ORDER BY sleep_date DESC;

   -- Resting HR
   SELECT measurement_date, resting_hr, source
   FROM resting_hr_data
   WHERE user_id = '$USER_ID' AND measurement_date >= CURRENT_DATE - 30
   ORDER BY measurement_date DESC;
   ```

3. **Check Notification History**
   ```sql
   SELECT notification_type, sent_at, status, metadata
   FROM notifications
   WHERE user_id = '$USER_ID' AND notification_type = 'data_quality_reminder'
   ORDER BY sent_at DESC
   LIMIT 10;
   ```

4. **Action Items**:
   - If no data: Check if wearable is connected
   - If manual entries only: Consider engagement campaign
   - If notifications failed: Verify user preferences
   - If consistent gaps: Send personalized outreach

### Review Quality Trends

```bash
# Weekly trends
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "$BASE_URL/api/v1/admin/data-quality/trends?period=week"

# Monthly trends
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "$BASE_URL/api/v1/admin/data-quality/trends?period=month"
```

**Look For**:
- Overall trend direction (improving/declining)
- Correlation with product changes
- Seasonal patterns
- Reminder effectiveness

### Bulk Operations

**Get All Poor Quality Users for Outreach**:
```bash
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "$BASE_URL/api/v1/admin/data-quality/users/poor?threshold=40&limit=100" \
  > poor_quality_users.json
```

**Get Users with Long Data Gaps**:
```bash
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  "$BASE_URL/api/v1/admin/data-quality/users/missing?min_days=7&limit=100" \
  > missing_data_users.json
```

## Incident Response

### Incident: Job Failures

**Severity**: High (if >3 consecutive failures)

**Diagnosis**:
1. Check job execution history:
   ```bash
   curl -H "Authorization: Bearer $ADMIN_TOKEN" \
     "$BASE_URL/api/v1/admin/jobs/history/data_quality_check"
   ```

2. Review application logs:
   ```bash
   grep "data_quality_check\|weekly_baseline" /var/log/ai-coach/app.log | tail -100
   ```

3. Check database connectivity:
   ```sql
   SELECT 1; -- Basic connection test
   ```

**Resolution**:
1. If database issue: Verify connection pool settings
2. If timeout: Increase job timeout or batch size
3. If data issue: Review recent data changes
4. If persistent: Restart application

**Prevention**:
- Set up alerting for job failures
- Monitor job execution time trends
- Implement automatic retries (currently 3 retries)

### Incident: Reminder Delivery Failures

**Severity**: Medium

**Diagnosis**:
1. Check alert delivery queue:
   ```bash
   curl -H "Authorization: Bearer $ADMIN_TOKEN" \
     "$BASE_URL/api/v1/admin/alerts/delivery/pending"
   ```

2. Check notification service status:
   ```sql
   SELECT status, COUNT(*)
   FROM notifications
   WHERE created_at >= CURRENT_DATE - 1
   GROUP BY status;
   ```

**Resolution**:
1. If email failures: Verify SMTP configuration
2. If push failures: Check FCM/APNS credentials
3. If queue backed up: Process pending manually
4. If widespread: Check external service status

### Incident: Data Completeness Drop

**Severity**: Medium (if sudden >10% drop)

**Diagnosis**:
1. Check trend over time:
   ```bash
   curl -H "Authorization: Bearer $ADMIN_TOKEN" \
     "$BASE_URL/api/v1/admin/data-quality/trends?period=month"
   ```

2. Identify affected users:
   ```sql
   SELECT user_id, completeness_score, days_without_data
   FROM data_quality_metrics
   WHERE metric_date = CURRENT_DATE
   AND completeness_score < (
     SELECT AVG(completeness_score) - 10
     FROM data_quality_metrics
     WHERE metric_date = CURRENT_DATE - 7
   );
   ```

3. Check for pattern:
   - Specific wearable brand?
   - Recent app update?
   - API integration issue?

**Resolution**:
1. If wearable issue: Contact integration support
2. If app issue: Rollback or hotfix
3. If seasonal: Document and monitor
4. If user behavior: Launch engagement campaign

## Weekly Maintenance Tasks

### Monday Morning (15 minutes)

1. **Review Weekend Activity**
   - Check if WeeklyBaselineRecalculationJob ran successfully (Sunday 3 AM)
   - Review any major baseline changes detected
   - Verify notification delivery for baseline changes

2. **Set Weekly Goals**
   - Target completeness improvement: +0.5% from last week
   - Users to improve: Top 20 poor quality users
   - Wearable reconnections: Users with disconnected devices

### Wednesday (10 minutes)

1. **Mid-Week Check**
   - Current completeness vs. weekly goal
   - Reminder delivery success rate
   - Job execution stability

2. **Adjust if Needed**
   - Increase reminder frequency for persistently poor users
   - Lower threshold for reminders if many users declining
   - Manual outreach for high-value users

### Friday (20 minutes)

1. **Weekly Report Generation**
   - Export quality trends
   - Calculate reminder effectiveness
   - Identify success stories (biggest improvements)

2. **Planning for Next Week**
   - Schedule any manual interventions
   - Prepare engagement campaigns
   - Document learnings

## Monthly Operations

### First Monday of Month (60 minutes)

1. **Monthly Review**
   - Calculate achievement vs. 20% improvement goal
   - Review reminder effectiveness metrics
   - Analyze wearable adoption trends
   - Identify users for success stories

2. **Database Maintenance**
   ```sql
   -- Archive old quality metrics (>90 days)
   DELETE FROM data_quality_metrics
   WHERE metric_date < CURRENT_DATE - 90;

   -- Vacuum and analyze
   VACUUM ANALYZE data_quality_metrics;
   VACUUM ANALYZE recovery_baselines;
   ```

3. **Threshold Review**
   - Are current thresholds (50% poor, 3 days gap) appropriate?
   - Should reminder urgency levels be adjusted?
   - Review baseline change significance levels

4. **Executive Report**
   - System health summary
   - Key metrics trends
   - Success stories
   - Improvement opportunities
   - Resource recommendations

## Escalation Matrix

### Level 1: Self-Service (Admin)
- Job status checks
- Manual job triggers
- User investigation
- Routine maintenance

### Level 2: Engineering Team
- Job persistent failures (>5 consecutive)
- Database performance issues
- Integration problems
- Code bugs

### Level 3: Leadership
- System-wide data quality drops >15%
- Privacy/security incidents
- Resource scaling needs
- Strategic changes to thresholds

## Best Practices

### Do's
✅ Check system health daily
✅ Review trends weekly
✅ Document unusual patterns
✅ Celebrate improvements
✅ Respond to alerts within 1 hour
✅ Keep runbook updated

### Don'ts
❌ Ignore persistent job failures
❌ Change thresholds without data
❌ Skip weekly reviews
❌ Over-notify users (respect limits)
❌ Modify production data manually
❌ Share admin credentials

## Useful SQL Queries

### Top 10 Most Improved Users (Last 30 Days)
```sql
WITH recent AS (
  SELECT user_id, AVG(completeness_score) as recent_score
  FROM data_quality_metrics
  WHERE metric_date >= CURRENT_DATE - 7
  GROUP BY user_id
),
baseline AS (
  SELECT user_id, AVG(completeness_score) as baseline_score
  FROM data_quality_metrics
  WHERE metric_date BETWEEN CURRENT_DATE - 37 AND CURRENT_DATE - 30
  GROUP BY user_id
)
SELECT
  r.user_id,
  u.email,
  b.baseline_score,
  r.recent_score,
  (r.recent_score - b.baseline_score) as improvement
FROM recent r
JOIN baseline b ON r.user_id = b.user_id
JOIN users u ON r.user_id = u.id
WHERE (r.recent_score - b.baseline_score) > 10
ORDER BY improvement DESC
LIMIT 10;
```

### Users Due for Reminder (Not Sent in 24h)
```sql
SELECT
  m.user_id,
  u.email,
  m.completeness_score,
  m.days_without_data,
  MAX(n.sent_at) as last_reminder
FROM data_quality_metrics m
JOIN users u ON m.user_id = u.id
LEFT JOIN notifications n ON m.user_id = n.user_id
  AND n.notification_type = 'data_quality_reminder'
WHERE m.metric_date = CURRENT_DATE
AND (m.days_without_data >= 3 OR m.completeness_score < 50)
GROUP BY m.user_id, u.email, m.completeness_score, m.days_without_data
HAVING MAX(n.sent_at) < NOW() - INTERVAL '24 hours'
  OR MAX(n.sent_at) IS NULL
ORDER BY m.days_without_data DESC, m.completeness_score ASC
LIMIT 50;
```

### Wearable Disconnection Report
```sql
SELECT
  u.id as user_id,
  u.email,
  m.data_sources->>'wearable_connected' as connected,
  m.data_sources->>'last_sync' as last_sync,
  m.days_without_data
FROM users u
JOIN data_quality_metrics m ON u.id = m.user_id
WHERE m.metric_date = CURRENT_DATE
AND m.data_sources->>'wearable_connected' = 'false'
AND m.days_without_data >= 3
ORDER BY m.days_without_data DESC;
```

## Contact Information

**Primary On-Call**: [Engineering team rotation]
**Escalation**: [Engineering manager]
**Database Admin**: [DBA team]
**Product Owner**: [Product manager]

**Slack Channels**:
- `#ai-coach-alerts` - Automated alerts
- `#ai-coach-ops` - Operations discussion
- `#ai-coach-engineering` - Technical support

**Documentation**:
- System Overview: `docs/DATA_QUALITY_MONITORING.md`
- API Reference: `docs/API.md`
- Database Schema: `migrations/`
