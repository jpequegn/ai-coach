# Recovery Alert System

This document describes the Recovery Alert System implemented for AI Coach, which provides proactive recovery monitoring and automated alerts based on recovery metrics.

## Overview

The Recovery Alert System automatically evaluates daily recovery scores and generates alerts when recovery indicators suggest the athlete needs intervention. The system helps prevent overtraining and injury through early warning signals.

## Architecture

### Components

1. **RecoveryAlertService** (`src/services/recovery_alert_service.rs`)
   - Alert rule evaluation
   - Alert creation and management
   - Notification delivery
   - User preference management

2. **Alert Preferences** (Database: `recovery_alert_preferences`)
   - Per-user alert configuration
   - Notification channel preferences
   - Custom thresholds

3. **API Endpoints** (`src/api/recovery_analysis.rs`)
   - Alert history retrieval
   - Alert acknowledgment
   - Preference management

## Alert Types

### 1. Critical Recovery
- **Trigger**: Readiness score < 20 (default threshold)
- **Severity**: Critical
- **Action**: Take complete rest day, avoid strenuous activity
- **Notification**: Push + Email

### 2. Consecutive Poor Recovery
- **Trigger**: Readiness score < 40 for 3+ consecutive days
- **Severity**: Warning
- **Action**: Reduce training intensity by 30-50%
- **Notification**: Push (if enabled) + Email (if enabled)

### 3. Declining HRV Trend
- **Trigger**: HRV trend = "declining" for 7+ days
- **Severity**: Warning
- **Action**: Consider recovery day, reduce stress
- **Notification**: Push (if enabled) + Email (if enabled)

### 4. High Strain with Poor Recovery
- **Trigger**: Training strain > 1300 AND readiness < 60
- **Severity**: Warning
- **Action**: Risk of overtraining, take rest day
- **Notification**: Push (if enabled) + Email (if enabled)

### 5. Poor Sleep Quality
- **Trigger**: Sleep quality score < 60
- **Severity**: Info
- **Action**: Improve sleep hygiene, aim for 8+ hours
- **Notification**: Push (if enabled)

## Alert Cooldown

- **Duration**: 24 hours per alert type
- **Purpose**: Prevent alert fatigue from repeated notifications
- **Implementation**: Checks last alert timestamp before creating new alert

## User Preferences

### Default Settings
```json
{
  "enabled": true,
  "push_notifications": true,
  "email_notifications": false,
  "poor_recovery_threshold": 40.0,
  "critical_recovery_threshold": 20.0
}
```

### Customizable Parameters

1. **enabled** (boolean)
   - Master switch for all recovery alerts
   - Default: true

2. **push_notifications** (boolean)
   - Enable push notifications for non-critical alerts
   - Critical alerts always send push notifications
   - Default: true

3. **email_notifications** (boolean)
   - Enable email notifications for warnings and critical alerts
   - Default: false

4. **poor_recovery_threshold** (0-100)
   - Readiness score below this triggers poor recovery alerts
   - Default: 40.0

5. **critical_recovery_threshold** (0-100)
   - Readiness score below this triggers critical alerts
   - Default: 20.0

## API Endpoints

### Get Alerts
```http
GET /api/v1/recovery/analysis/alerts?limit=50&include_acknowledged=false
Authorization: Bearer <jwt_token>
```

**Query Parameters**:
- `limit` (optional): Max number of alerts to return (default: 50, max: 200)
- `include_acknowledged` (optional): Include acknowledged alerts (default: false)

**Response**:
```json
[
  {
    "id": "uuid",
    "user_id": "uuid",
    "alert_type": "critical_recovery",
    "severity": "critical",
    "recovery_score_id": "uuid",
    "message": "Critical recovery status detected",
    "recommendations": [
      {
        "priority": "critical",
        "category": "recovery",
        "message": "Critical recovery status detected",
        "action": "Take a complete rest day. Avoid any strenuous activity."
      }
    ],
    "acknowledged_at": null,
    "created_at": "2025-10-03T12:00:00Z"
  }
]
```

### Acknowledge Alert
```http
POST /api/v1/recovery/analysis/alerts/{alert_id}/acknowledge
Authorization: Bearer <jwt_token>
```

**Response**:
```json
{
  "id": "uuid",
  "user_id": "uuid",
  "alert_type": "critical_recovery",
  "severity": "critical",
  "message": "Critical recovery status detected",
  "acknowledged_at": "2025-10-03T12:30:00Z",
  "created_at": "2025-10-03T12:00:00Z"
}
```

### Get Alert Settings
```http
GET /api/v1/recovery/analysis/alerts/settings
Authorization: Bearer <jwt_token>
```

**Response**:
```json
{
  "id": "uuid",
  "user_id": "uuid",
  "enabled": true,
  "push_notifications": true,
  "email_notifications": false,
  "poor_recovery_threshold": 40.0,
  "critical_recovery_threshold": 20.0,
  "created_at": "2025-10-01T00:00:00Z",
  "updated_at": "2025-10-03T12:00:00Z"
}
```

### Update Alert Settings
```http
PATCH /api/v1/recovery/analysis/alerts/settings
Authorization: Bearer <jwt_token>
Content-Type: application/json

{
  "enabled": true,
  "push_notifications": true,
  "email_notifications": true,
  "poor_recovery_threshold": 35.0,
  "critical_recovery_threshold": 15.0
}
```

All fields are optional - only include fields you want to update.

**Response**: Returns updated preferences (same format as GET)

## Integration with Recovery Analysis

The alert system is automatically integrated into the recovery analysis workflow:

1. **Daily Calculation**: When `RecoveryAnalysisService::calculate_daily_recovery()` runs
2. **Alert Evaluation**: Alerts are evaluated against the newly calculated recovery score
3. **Alert Creation**: Matching alerts are created in the database
4. **Notification Delivery**: Notifications are sent based on user preferences
5. **Logging**: Alert creation and delivery are logged for monitoring

### Example Flow
```rust
// In RecoveryAnalysisService::calculate_daily_recovery()
let score = self.upsert_recovery_score(...).await?;

if let Some(alert_service) = &self.alert_service {
    alert_service.evaluate_alerts(user_id, &score).await?;
}
```

## Alert Rules Configuration

Alert rules are defined in `RecoveryAlertService::get_alert_rules()`:

```rust
RecoveryAlertRule {
    alert_type: "critical_recovery".to_string(),
    severity: Severity::Critical,
    condition: Box::new(move |score: &RecoveryScore| {
        score.readiness_score < preferences.critical_recovery_threshold
    }),
    message: "Critical recovery status detected".to_string(),
    recommendation: "Take a complete rest day...".to_string(),
}
```

### Adding New Alert Rules

To add a new alert rule:

1. Add rule to `get_alert_rules()` method
2. Define condition function
3. Set appropriate severity
4. Provide user-friendly message and recommendation
5. Consider cooldown requirements

## Notification Delivery

### Push Notifications
- **Critical alerts**: Always sent
- **Other alerts**: Only if `push_notifications` is enabled
- **Service**: `NotificationService::send_push_notification()`
- **Fallback**: Errors logged but don't fail alert creation

### Email Notifications
- **Critical/Warning alerts**: Sent if `email_notifications` is enabled
- **Info alerts**: Not sent via email
- **Service**: `NotificationService::send_email_notification()`
- **Fallback**: Errors logged but don't fail alert creation

## Database Schema

### recovery_alert_preferences
```sql
CREATE TABLE recovery_alert_preferences (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) UNIQUE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    push_notifications BOOLEAN NOT NULL DEFAULT TRUE,
    email_notifications BOOLEAN NOT NULL DEFAULT FALSE,
    poor_recovery_threshold DOUBLE PRECISION NOT NULL DEFAULT 40.0,
    critical_recovery_threshold DOUBLE PRECISION NOT NULL DEFAULT 20.0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### recovery_alerts (from migration 019)
```sql
CREATE TABLE recovery_alerts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    alert_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    recovery_score_id UUID REFERENCES recovery_scores(id),
    message TEXT NOT NULL,
    recommendations JSONB,
    acknowledged_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

## Testing

### Manual Testing

1. **Create User with Poor Recovery**:
   ```bash
   # Log HRV, sleep, and RHR data indicating poor recovery
   # Calculate recovery score (should be < 40)
   # Check that alert is created
   ```

2. **Test Alert Cooldown**:
   ```bash
   # Trigger same alert twice within 24 hours
   # Verify second alert is not created
   ```

3. **Test Preferences**:
   ```bash
   # Update alert preferences
   # Verify alerts respect new thresholds
   ```

### Integration Testing

Create tests for:
- Alert rule evaluation
- Cooldown logic
- Notification delivery
- Preference management
- API endpoint authorization

## Monitoring

### Logs to Monitor

```
INFO: Created recovery alert {alert_type} for user {user_id}: {message}
INFO: Generated {count} recovery alerts for user {user_id}
ERROR: Failed to evaluate recovery alerts: {error}
ERROR: Failed to send push notification: {error}
ERROR: Failed to send email notification: {error}
```

### Metrics to Track

1. **Alert Volume**: Number of alerts per type per day
2. **Acknowledgment Rate**: % of alerts acknowledged within 24h
3. **Notification Success Rate**: % of successful deliveries
4. **User Opt-Out Rate**: % of users disabling alerts

## Future Enhancements

1. **Machine Learning Alert Prioritization**
   - Learn which alerts users respond to
   - Adjust alert thresholds based on user behavior

2. **Custom Alert Rules**
   - Allow users to define custom alert conditions
   - Webhook integration for third-party notifications

3. **Alert Analytics Dashboard**
   - Admin view of alert trends
   - Identify users at high risk

4. **Multi-Channel Delivery**
   - SMS notifications
   - Slack/Discord integration
   - In-app banner notifications

5. **Smart Scheduling**
   - Deliver alerts at optimal times
   - Batch non-critical alerts into daily digest

## Troubleshooting

### Alerts Not Being Created

**Check**:
1. Are recovery scores being calculated?
2. Are alert preferences enabled?
3. Is the recovery score meeting alert conditions?
4. Has cooldown period expired?

**Debug**:
```bash
# Check recovery scores
SELECT * FROM recovery_scores WHERE user_id = '<user_id>' ORDER BY score_date DESC LIMIT 5;

# Check alert preferences
SELECT * FROM recovery_alert_preferences WHERE user_id = '<user_id>';

# Check recent alerts
SELECT * FROM recovery_alerts WHERE user_id = '<user_id>' ORDER BY created_at DESC LIMIT 10;
```

### Notifications Not Being Delivered

**Check**:
1. Are notification preferences enabled?
2. Is NotificationService configured?
3. Check error logs for delivery failures

## Related Documentation

- [Recovery Analysis MVP](./recovery-analysis-mvp.md) - Core recovery analysis service
- [Recovery Monitoring](./recovery-monitoring.md) - Data collection and wearable integration
- [Notification System](./notifications.md) - Notification service documentation

## Related Issues

- Implements Phase 4 of #55 (Recovery Monitoring - Analysis & Dashboard)
- Builds on #109 (Recovery Analysis MVP)
- Integrates with #54 (Wearable Integration)
