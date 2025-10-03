

# Recovery Analysis MVP

## Overview

This document describes the MVP implementation of Recovery Analysis (Issue #55), which builds on the recovery data collection foundation (#54) to provide actionable insights, trend analysis, and personalized recommendations.

## Status

**Phase:** MVP Complete - Statistical Analysis (trainrs-independent)
**Dependencies:** Recovery data collection (#54) ✅
**Future:** Will integrate with trainrs recovery module when available

## Architecture

### Components

1. **Recovery Analysis Service**: Statistical calculations for recovery metrics
2. **Recovery Scores Table**: Daily calculated recovery scores
3. **REST API**: Endpoints for status, trends, and insights
4. **Recommendation Engine**: Personalized recovery advice

### Data Flow

```
Recovery Data (HRV, Sleep, RHR)
    ↓
Recovery Analysis Service
    ↓
Daily Recovery Score Calculation
    ↓
API Endpoints → Frontend Dashboard
```

## Database Schema

### Recovery Scores Table

```sql
CREATE TABLE recovery_scores (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id),
    score_date DATE NOT NULL,
    readiness_score DOUBLE PRECISION NOT NULL (0-100),
    hrv_trend VARCHAR(20) NOT NULL, -- improving/stable/declining/insufficient_data
    hrv_deviation DOUBLE PRECISION, -- % deviation from baseline
    sleep_quality_score DOUBLE PRECISION (0-100),
    recovery_adequacy DOUBLE PRECISION (0-100),
    rhr_deviation DOUBLE PRECISION, -- % deviation from baseline
    training_strain DOUBLE PRECISION,
    recovery_status VARCHAR(20) NOT NULL, -- optimal/good/fair/poor/critical
    recommended_tss_adjustment DOUBLE PRECISION, -- 0.5-1.1 multiplier
    model_version VARCHAR(20),
    UNIQUE(user_id, score_date)
);
```

### Recovery Alerts Table

```sql
CREATE TABLE recovery_alerts (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id),
    alert_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL, -- info/warning/critical
    recovery_score_id UUID REFERENCES recovery_scores(id),
    message TEXT NOT NULL,
    recommendations JSONB,
    acknowledged_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ
);
```

## Recovery Calculations

### HRV Trend Detection

**Algorithm:**
1. Fetch last 7 days of HRV data
2. Split into two halves (recent vs older)
3. Compare averages:
   - Change >5%: Improving
   - Change <-5%: Declining
   - Otherwise: Stable

**HRV Deviation:**
```
deviation = ((recent_avg - baseline) / baseline) * 100
```

### Sleep Quality Score

**Formula (0-100):**
```
base_score = 50

// Hours score (optimal 7-9 hours)
if 7 <= hours <= 9:
    hours_score = 25
elif 6 <= hours <= 10:
    hours_score = 15
else:
    hours_score = 0

// Efficiency score
efficiency_score = (efficiency / 100) * 25

total = base_score + hours_score + efficiency_score
```

### Recovery Adequacy

**Composite Score:**
```
components = []

if hrv_deviation exists:
    components.push(50 + (hrv_deviation * 0.5))  // Positive deviation is good

if sleep_quality exists:
    components.push(sleep_quality)

if rhr_deviation exists:
    components.push(50 - (rhr_deviation * 0.5))  // Negative deviation is good

recovery_adequacy = average(components).clamp(0, 100)
```

### Readiness Score

**Primary metric:**
```
if recovery_adequacy exists:
    readiness_score = recovery_adequacy
elif sleep_quality exists:
    readiness_score = sleep_quality
else:
    readiness_score = 50  // Default neutral
```

### Recovery Status Classification

```
score >= 85:  Optimal
score >= 70:  Good
score >= 50:  Fair
score >= 30:  Poor
score < 30:   Critical
```

### TSS Adjustment Recommendation

```
score >= 85: 1.1  (increase 10%)
score >= 70: 1.0  (no change)
score >= 50: 0.9  (reduce 10%)
score >= 30: 0.7  (reduce 30%)
score < 30:  0.5  (reduce 50%)
```

## API Endpoints

### Base Path

`/api/v1/recovery/analysis`

### GET /status

**Description:** Get current recovery status with recommendations

**Authentication:** Required (JWT)

**Response:**
```json
{
  "date": "2025-10-03",
  "readiness_score": 78.5,
  "recovery_status": "good",
  "hrv_trend": "stable",
  "hrv_deviation": -2.3,
  "sleep_quality": 85.0,
  "recovery_adequacy": 72.0,
  "rhr_deviation": 1.5,
  "recommended_tss_adjustment": 1.0,
  "recommendations": [
    {
      "priority": "low",
      "category": "general",
      "message": "Recovery is within normal range",
      "action": "Continue with your planned training schedule."
    }
  ]
}
```

**Recommendations Categories:**
- `recovery`: General recovery advice
- `sleep`: Sleep-specific recommendations
- `training`: Training adjustments
- `general`: General guidance

**Priority Levels:**
- `critical`: Immediate action required
- `high`: Important, address soon
- `medium`: Should consider
- `low`: Informational

### GET /trends

**Description:** Get recovery trends over a time period

**Authentication:** Required (JWT)

**Query Parameters:**
- `period_days` (optional): Number of days (1-365, default: 30)

**Response:**
```json
{
  "period_days": 30,
  "average_readiness": 72.5,
  "trend_direction": "stable",
  "data_points": [
    {
      "date": "2025-10-03",
      "readiness_score": 78.5,
      "recovery_status": "good"
    }
  ],
  "patterns": [
    {
      "pattern_type": "weekend_recovery",
      "description": "Recovery scores are significantly better on weekends",
      "confidence": 0.7
    }
  ]
}
```

**Trend Directions:**
- `improving`: Average increasing >5 points
- `declining`: Average decreasing >5 points
- `stable`: Relatively consistent
- `insufficient_data`: <3 days of data

**Detected Patterns:**
- `consecutive_poor_recovery`: 3+ consecutive poor days
- `weekend_recovery`: Higher scores on weekends

### GET /insights

**Description:** Get AI-generated recovery insights

**Authentication:** Required (JWT)

**Response:**
```json
{
  "insights": [
    {
      "category": "HRV",
      "title": "Declining Heart Rate Variability",
      "description": "Your HRV has been trending downward, indicating increased stress or fatigue.",
      "impact": "negative"
    }
  ],
  "key_factors": [
    {
      "factor": "HRV",
      "current_value": 45.2,
      "baseline_value": 48.5,
      "deviation_percent": -6.8
    }
  ],
  "suggestions": [
    "Consider reducing training intensity and prioritizing recovery activities.",
    "Aim for 8+ hours of quality sleep tonight."
  ]
}
```

**Insight Categories:**
- `HRV`: Heart rate variability insights
- `Sleep`: Sleep quality insights
- `Recovery`: Overall recovery insights

**Impact Levels:**
- `positive`: Good trend or status
- `negative`: Concerning trend
- `neutral`: Informational

## Usage Examples

### 1. Get Current Recovery Status

```bash
curl -X GET http://localhost:3000/api/v1/recovery/analysis/status \
  -H "Authorization: Bearer <JWT_TOKEN>"
```

### 2. Get 7-Day Trends

```bash
curl -X GET "http://localhost:3000/api/v1/recovery/analysis/trends?period_days=7" \
  -H "Authorization: Bearer <JWT_TOKEN>"
```

### 3. Get Recovery Insights

```bash
curl -X GET http://localhost:3000/api/v1/recovery/analysis/insights \
  -H "Authorization: Bearer <JWT_TOKEN>"
```

## Recommendation Logic

### HRV-Based Recommendations

**Declining Trend:**
- Priority: High
- Message: "Your HRV is declining, indicating increased stress or fatigue"
- Action: "Consider taking a rest day or reducing training intensity by 30%"

### Sleep-Based Recommendations

**Poor Sleep Quality (<70):**
- Priority: Medium
- Message: "Sleep quality is below optimal"
- Action: "Aim for 8+ hours of quality sleep tonight. Consider improving sleep hygiene."

### Overall Recovery Recommendations

**Critical (<30):**
- Priority: Critical
- Message: "Critical recovery status detected"
- Action: "Take a complete rest day. Avoid any strenuous activity."

**Excellent (>=85):**
- Priority: Low
- Message: "Excellent recovery - you're ready for high-intensity training"
- Action: "This is a good day for your hardest workout of the week."

## Pattern Detection

### Consecutive Poor Recovery

**Detection:** 3+ consecutive days with readiness <50

**Insight:** Indicates overtraining or inadequate recovery

**Recommendation:** Extended rest period or deload week

### Weekend Recovery

**Detection:** Weekend scores >10 points higher than weekday average

**Insight:** Work-related stress or training distribution issue

**Recommendation:** Consider stress management or training schedule adjustment

## Error Handling

### Insufficient Data (404)

```json
{
  "error_code": "INSUFFICIENT_DATA",
  "message": "Not enough recovery data to calculate status. Please log HRV, sleep, or resting HR data."
}
```

**Cause:** No recovery data logged in recent days

**Solution:** Log recovery data using `/api/v1/recovery` endpoints

### Database Error (500)

```json
{
  "error_code": "DATABASE_ERROR",
  "message": "Failed to retrieve recovery status"
}
```

## Integration Points

### Recovery Data Collection (#54)

**Input Data:**
- HRV readings from `/api/v1/recovery/hrv`
- Sleep data from `/api/v1/recovery/sleep`
- Resting HR from `/api/v1/recovery/resting-hr`
- Baselines from `/api/v1/recovery/baseline`

### Future Integrations

**trainrs Module (when available):**
- Replace simple calculations with trainrs functions
- Add advanced metrics (TSB, ACWR, training strain)
- Improve trend analysis accuracy

**Training Plans:**
- Auto-adjust TSS based on recommendations
- Schedule workouts based on recovery status

**Alerts System:**
- Generate alerts for poor recovery
- Send notifications for critical status

## Testing

### Manual Testing Steps

1. **Ensure recovery data exists:**
   ```bash
   # Log some HRV data
   curl -X POST http://localhost:3000/api/v1/recovery/hrv \
     -H "Authorization: Bearer <TOKEN>" \
     -H "Content-Type: application/json" \
     -d '{"rmssd": 45.5}'

   # Log sleep data
   curl -X POST http://localhost:3000/api/v1/recovery/sleep \
     -H "Authorization: Bearer <TOKEN>" \
     -H "Content-Type: application/json" \
     -d '{"total_sleep_hours": 7.5, "sleep_efficiency": 92.5}'
   ```

2. **Test recovery status:**
   ```bash
   curl -X GET http://localhost:3000/api/v1/recovery/analysis/status \
     -H "Authorization: Bearer <TOKEN>"
   ```

3. **Test trends:**
   ```bash
   curl -X GET "http://localhost:3000/api/v1/recovery/analysis/trends?period_days=7" \
     -H "Authorization: Bearer <TOKEN>"
   ```

4. **Test insights:**
   ```bash
   curl -X GET http://localhost:3000/api/v1/recovery/analysis/insights \
     -H "Authorization: Bearer <TOKEN>"
   ```

### Expected Outcomes

- ✅ Status endpoint returns readiness score and recommendations
- ✅ Trends endpoint shows data points and patterns
- ✅ Insights endpoint provides personalized suggestions
- ✅ All endpoints require authentication
- ✅ Error handling for insufficient data

## Performance Considerations

### Caching Strategy

**Current:** No caching implemented (MVP)

**Future:**
- Cache recovery scores in Redis (1 hour TTL)
- Cache trends in Redis (30 minutes TTL)
- Invalidate cache on new data entry

### Query Optimization

- Indexes on `recovery_scores(user_id, score_date)`
- Indexes on recovery data tables for date-range queries
- Use of connection pooling

## Future Enhancements

### Phase 2: trainrs Integration

- Replace statistical calculations with trainrs functions
- Add advanced metrics (TSB, CTL, ATL, ACWR)
- Improve accuracy with scientific algorithms

### Phase 3: Alert Service

- Automated alerts for poor recovery
- Email and push notifications
- Alert cooldown and preferences

### Phase 4: Training Plan Integration

- Auto-adjust training plans based on recovery
- Schedule workouts optimally
- Implement floating rest days

### Phase 5: Background Jobs

- Daily recovery calculation cron job
- Weekly baseline recalculation
- Automated pattern detection

### Phase 6: Advanced Analytics

- ML-based pattern recognition
- Predictive recovery forecasting
- Personalized baseline calculation

## Migration Information

**Migration:** `019_create_recovery_analysis_tables.sql`

**Run automatically on server start**

## API Documentation

Full API docs available at: `/api/v1/docs`

## Related Issues

- **#54**: Recovery data collection (dependency) ✅
- **#110**: Data quality validation (future)
- **#111**: Advanced analytics (future)
- **#30**: Injury prediction (uses recovery metrics)

## Support

For issues:
1. Check that recovery data is logged
2. Verify baselines are calculated
3. Ensure minimum 1 day of data exists
4. Check authentication token validity

## Changelog

### v1.0.0 (MVP - Current)
- ✅ Statistical recovery score calculation
- ✅ HRV trend detection
- ✅ Sleep quality scoring
- ✅ Recovery adequacy calculation
- ✅ Readiness score and status
- ✅ TSS adjustment recommendations
- ✅ Basic pattern detection
- ✅ Personalized recommendations
- ✅ REST API endpoints

### v2.0.0 (Planned - trainrs integration)
- ⏳ trainrs recovery functions
- ⏳ Advanced metrics (TSB, ACWR)
- ⏳ Alert service
- ⏳ Background jobs
- ⏳ Training plan integration
