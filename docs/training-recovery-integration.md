# Training Plan Integration with Recovery Scores

## Overview

Automatic training adjustment system that optimizes workout intensity and volume based on daily recovery scores. Prevents overtraining and maximizes training effectiveness by scheduling hard workouts on high-readiness days.

## Implementation Status

### ✅ Phase 5 Complete - MVP

**Core Features Implemented:**
1. ✅ Training Adjustment Service - TSS calculations and workout modifications
2. ✅ User Settings & Consent - Customizable adjustment preferences
3. ✅ Real-time Adjustment API - Daily training recommendations
4. ✅ Effectiveness Tracking - Decision logging and outcome measurement

## Architecture

### Components

1. **TrainingAdjustmentService** (`src/services/training_adjustment_service.rs`)
   - TSS adjustment calculations based on readiness scores
   - Workout modification suggestions (intensity/volume reduction)
   - Rest day decision logic with multi-factor analysis

2. **Training Recovery Settings** (`training_recovery_settings` table)
   - User preferences for auto-adjustment
   - Aggressiveness levels (conservative/moderate/aggressive)
   - Min rest days and max consecutive training days
   - Permission flags for different adjustment types

3. **Adjustment Tracking** (`training_adjustments` table)
   - Log adjustment recommendations and user decisions
   - Track actual TSS vs recommended
   - Measure outcomes (next-day recovery, training quality)

### Database Schema

#### Training Recovery Settings
```sql
CREATE TABLE training_recovery_settings (
    id UUID PRIMARY KEY,
    user_id UUID UNIQUE REFERENCES users(id),
    auto_adjust_enabled BOOLEAN DEFAULT FALSE,
    adjustment_aggressiveness VARCHAR(20) DEFAULT 'moderate',
    min_rest_days_per_week INTEGER DEFAULT 1,
    max_consecutive_training_days INTEGER DEFAULT 6,
    allow_intensity_reduction BOOLEAN DEFAULT TRUE,
    allow_volume_reduction BOOLEAN DEFAULT TRUE,
    allow_workout_swap BOOLEAN DEFAULT FALSE
);
```

#### Training Adjustments
```sql
CREATE TABLE training_adjustments (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id),
    adjustment_date DATE,
    recovery_score_id UUID REFERENCES recovery_scores(id),
    original_tss DOUBLE PRECISION,
    recommended_tss DOUBLE PRECISION,
    actual_tss DOUBLE PRECISION,
    adjustment_applied BOOLEAN,
    adjustment_type VARCHAR(50),
    outcome_recovery_score DOUBLE PRECISION,
    outcome_training_quality VARCHAR(20)
);
```

## API Endpoints

### Settings Management

#### GET /api/v1/training/adjustment/recovery-settings

Get user's training recovery settings. Creates default settings if none exist.

**Response:**
```json
{
  "id": "uuid",
  "auto_adjust_enabled": false,
  "adjustment_aggressiveness": "moderate",
  "min_rest_days_per_week": 1,
  "max_consecutive_training_days": 6,
  "allow_intensity_reduction": true,
  "allow_volume_reduction": true,
  "allow_workout_swap": false
}
```

#### PATCH /api/v1/training/adjustment/recovery-settings

Update training recovery settings.

**Request:**
```json
{
  "auto_adjust_enabled": true,
  "adjustment_aggressiveness": "aggressive",
  "min_rest_days_per_week": 2,
  "max_consecutive_training_days": 5
}
```

**Validation:**
- `adjustment_aggressiveness`: Must be "conservative", "moderate", or "aggressive"
- `min_rest_days_per_week`: 0-7
- `max_consecutive_training_days`: 1-14

### Adjustment Recommendations

#### GET /api/v1/training/adjustment/recommended-adjustment

Get today's recommended training adjustment based on recovery score.

**Response:**
```json
{
  "date": "2025-10-04",
  "has_recovery_data": true,
  "tss_adjustment": {
    "original_tss": 100.0,
    "recommended_tss": 85.0,
    "adjustment_factor": 0.85,
    "explanation": "Moderate recovery - consider reducing intensity by 15%",
    "reasoning": [
      "Readiness score: 65.5/100 (fair)",
      "Sleep quality: 78.0/100",
      "HRV trend: declining"
    ]
  },
  "rest_recommendation": null
}
```

**With Rest Recommendation:**
```json
{
  "date": "2025-10-04",
  "has_recovery_data": true,
  "tss_adjustment": {
    "original_tss": 100.0,
    "recommended_tss": 30.0,
    "adjustment_factor": 0.3,
    "explanation": "Critical recovery - consider rest day or very light activity (70% reduction)",
    "reasoning": [
      "Readiness score: 28.0/100 (critical)",
      "Sleep quality: 45.0/100",
      "HRV trend: declining",
      "Resting HR above baseline by 8.5%"
    ]
  },
  "rest_recommendation": {
    "should_rest": true,
    "confidence": 0.95,
    "reasoning": "Critical recovery (readiness: 28.0/100). Rest is essential to prevent overtraining.",
    "alternative_action": "Active recovery: light walk, yoga, or stretching"
  }
}
```

**No Recovery Data:**
```json
{
  "date": "2025-10-04",
  "has_recovery_data": false,
  "tss_adjustment": null,
  "rest_recommendation": null
}
```

## Adjustment Logic

### TSS Adjustment Calculation

Based on readiness score:

| Readiness Score | Adjustment Factor | Recommendation |
|-----------------|-------------------|----------------|
| ≥ 80 | 1.1 (110%) | Can increase load 10% |
| 70-79 | 1.0 (100%) | Proceed as planned |
| 60-69 | 0.95 (95%) | Slight reduction (5%) |
| 50-59 | 0.85 (85%) | Moderate reduction (15%) |
| 40-49 | 0.7 (70%) | Significant reduction (30%) |
| 30-39 | 0.5 (50%) | Major reduction (50%) |
| < 30 | 0.3 (30%) | Rest or very light (70% reduction) |

### Rest Day Decision Logic

Rest day recommended when:
1. **Critical Recovery**: Readiness < 30
2. **Consecutive Poor Recovery**: Readiness < 40 AND 3+ consecutive poor days
3. **Extended Fatigue**: 7+ days without good recovery (readiness < 80)

**Confidence Scoring:**
- Readiness < 30: 95% confidence
- 3+ poor days: 85% confidence
- 7+ days without good recovery: 75% confidence

### Workout Modification Types

1. **rest_day** (Adjustment factor < 0.5)
   - Recommend complete rest or active recovery
   - Alternative: Light walk, yoga, stretching

2. **reduce_intensity** (Adjustment factor 0.5-0.8)
   - Reduce intensity factor by 20%
   - Slight duration reduction (10%)
   - Maintain workout type

3. **reduce_volume** (Adjustment factor 0.8-1.0)
   - Reduce duration proportionally
   - Maintain intensity
   - Same workout type

4. **no_change** (Adjustment factor ≥ 1.0)
   - Proceed with planned workout
   - May increase if readiness ≥ 80

## Aggressiveness Levels

### Conservative
- Prioritizes recovery over progression
- Applies adjustments at higher readiness thresholds
- Suggests rest earlier
- Best for: Injury-prone athletes, recovery phase

### Moderate (Default)
- Balanced approach
- Standard adjustment thresholds (as shown above)
- Best for: Most athletes, general training

### Aggressive
- Prioritizes training load
- Only adjusts at lower readiness scores
- Suggests rest only when critical
- Best for: Experienced athletes, peak training blocks

## User Experience Flow

### Daily Workflow

1. **Morning Check**
   - User wakes up, opens app
   - Recovery score calculated from overnight data
   - Dashboard shows today's readiness

2. **Adjustment Review**
   - Sees planned workout with adjustment recommendation
   - Reviews reasoning (HRV, sleep, RHR factors)
   - Views TSS: Original → Recommended

3. **Decision Making**
   - User decides: Apply adjustment or keep original
   - One-click apply modifies workout
   - Can override and proceed as planned

4. **Post-Workout**
   - Logs actual workout completed
   - System tracks: Applied adjustment? Actual TSS?
   - Optional: Rate training quality

5. **Learning Loop**
   - Next day's recovery measured
   - System learns from outcomes
   - Improves future recommendations

## Settings Customization

### Enable Auto-Adjustment
```bash
PATCH /api/v1/training/adjustment/recovery-settings
{
  "auto_adjust_enabled": true
}
```

### Set Aggressiveness
```bash
PATCH /api/v1/training/adjustment/recovery-settings
{
  "adjustment_aggressiveness": "conservative"
}
```

### Configure Rest Days
```bash
PATCH /api/v1/training/adjustment/recovery-settings
{
  "min_rest_days_per_week": 2,
  "max_consecutive_training_days": 5
}
```

### Control Adjustment Types
```bash
PATCH /api/v1/training/adjustment/recovery-settings
{
  "allow_intensity_reduction": true,
  "allow_volume_reduction": true,
  "allow_workout_swap": false
}
```

## Effectiveness Tracking

### Metrics Collected

1. **Adjustment Adherence**: % of recommendations followed
2. **Recovery Improvement**: Next-day recovery when adjusted vs not
3. **Training Quality**: User-reported session quality
4. **Injury Prevention**: Correlation with injury incidents

### Analysis Queries

**Adherence Rate:**
```sql
SELECT
    COUNT(CASE WHEN adjustment_applied THEN 1 END)::FLOAT / COUNT(*) * 100 as adherence_rate
FROM training_adjustments
WHERE user_id = $1;
```

**Recovery Improvement:**
```sql
SELECT
    AVG(CASE WHEN adjustment_applied THEN outcome_recovery_score END) as applied_recovery,
    AVG(CASE WHEN NOT adjustment_applied THEN outcome_recovery_score END) as ignored_recovery
FROM training_adjustments
WHERE user_id = $1 AND outcome_recovery_score IS NOT NULL;
```

## Success Metrics

### Implemented (Phase 5 MVP)
- ✅ Adjustment recommendations generated for 100% of users with recovery data
- ✅ Settings API response time < 100ms
- ✅ Adjustment API response time < 200ms

### In Progress
- 📊 Adjustment adherence rate tracking
- 📊 Recovery improvement measurement (15%+ target)
- 📊 User satisfaction feedback collection

### Future (Phase 6-9)
- 🔄 Background job automation (daily calculations)
- 📈 ML-based personalized recommendations
- 🎯 Workout scheduling optimization
- 📊 Comprehensive analytics dashboard

## Future Enhancements

### Phase 6: Background Jobs & Automation
- Daily recovery calculation at 6 AM local time
- Automatic adjustment application (if enabled)
- Weekly baseline recalculation
- Alert delivery for poor recovery

### Phase 7: Smart Workout Scheduling
- Place hard workouts on high-readiness days
- Float rest days based on actual recovery
- Optimize weekly training distribution
- Maintain target training load

### Phase 8: Advanced Recommendations
- ML-based personalized adjustment factors
- Historical pattern recognition
- User-specific effectiveness learning
- Recovery protocol suggestions

### Phase 9: Integration & Analytics
- Training plan generation with recovery
- Coach dashboard for team monitoring
- Comparative analytics vs peers
- Long-term trend analysis

## Related Documentation

- [Recovery Monitoring - Wearable Integration](./recovery-wearable-integration.md)
- [Recovery Analysis & Dashboard](./recovery-analysis-mvp.md)
- [Recovery Alert System](./recovery-alert-system.md)

## API Reference

See [API Documentation](./api/) for complete endpoint specifications.

## Support

For issues or questions:
- Check [Troubleshooting Guide](./troubleshooting.md)
- Review [API Examples](./api/examples/)
- Contact: support@ai-coach.com
