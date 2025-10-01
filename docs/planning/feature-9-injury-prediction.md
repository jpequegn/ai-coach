# Feature #9: Injury Prediction Model

## Overview
Implement a machine learning model that predicts injury risk based on training load patterns, recovery metrics, and historical data. Provides proactive injury prevention through early warning system and automated alerts.

## Business Value
- **Injury Prevention**: Major value proposition for serious athletes
- **Risk Reduction**: Prevents costly injuries and training interruptions
- **Retention**: Athletes stay healthy and engaged longer
- **Premium Feature**: High-value capability for subscription tiers
- **Differentiation**: Advanced ML capability vs. competitors
- **Trust Building**: Demonstrates AI coaching expertise

## Technical Architecture

### Components
1. **Feature Engineering** (training load, recovery, biomechanics)
2. **ML Model** (classification/regression for injury risk)
3. **Risk Scoring Engine** (real-time risk assessment)
4. **Alert System** (proactive notifications)
5. **Recommendation Engine** (preventive actions)

### Technology Stack
- `linfa` for ML model training (existing)
- `linfa-trees` for random forest/gradient boosting
- `linfa-preprocessing` for feature scaling
- `ndarray` for numerical operations
- `statrs` for statistical analysis
- PostgreSQL for training data and predictions
- Redis for real-time risk caching

## Implementation Tasks

### Phase 1: Research & Data Analysis (Week 1-2)
**Task 1.1: Literature Review**
- Research injury prediction methodologies:
  - Acute:Chronic Workload Ratio (ACWR)
  - Training Stress Balance (TSB)
  - Monotony and Strain indices
  - Biomechanical risk factors
- Review published studies on injury prediction
- Identify key predictive features
- Document findings and recommendations

**Task 1.2: Data Analysis**
- Analyze existing training session data
- Identify patterns in injured vs. non-injured athletes
- Calculate baseline injury rates by sport/activity
- Determine data quality and completeness
- Identify missing data needed for prediction

**Task 1.3: Feature Definition**
- Define injury risk features:
  - **Load Features**: Total volume, intensity, frequency
  - **Progression Features**: Week-over-week changes, spikes
  - **Recovery Features**: Rest days, sleep quality, HRV
  - **History Features**: Previous injuries, age, experience
  - **Biomechanical**: Asymmetry, movement quality scores
- Create feature specification document
- Prioritize features by predictive power

**Task 1.4: Database Schema**
```sql
CREATE TABLE injury_predictions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    prediction_date DATE NOT NULL,
    risk_score DOUBLE PRECISION NOT NULL, -- 0-100
    risk_level VARCHAR(20) NOT NULL, -- low, moderate, high, critical
    contributing_factors JSONB, -- Array of risk factors
    confidence_score DOUBLE PRECISION,
    model_version VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE injury_risk_factors (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    prediction_id UUID NOT NULL REFERENCES injury_predictions(id),
    factor_type VARCHAR(50) NOT NULL, -- training_load, recovery, biomechanics
    factor_name VARCHAR(100) NOT NULL,
    severity DOUBLE PRECISION NOT NULL,
    description TEXT,
    recommendation TEXT
);

CREATE TABLE injury_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    injury_type VARCHAR(100) NOT NULL,
    body_part VARCHAR(50) NOT NULL,
    severity VARCHAR(20), -- minor, moderate, severe
    injury_date DATE NOT NULL,
    recovery_date DATE,
    days_out INTEGER,
    cause TEXT,
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE training_load_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    date DATE NOT NULL,
    acute_load DOUBLE PRECISION, -- 7-day rolling average
    chronic_load DOUBLE PRECISION, -- 28-day rolling average
    acwr DOUBLE PRECISION, -- Acute:Chronic ratio
    monotony DOUBLE PRECISION,
    strain DOUBLE PRECISION,
    training_stress_balance DOUBLE PRECISION,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, date)
);

-- Indexes
CREATE INDEX idx_injury_predictions_user_date ON injury_predictions(user_id, prediction_date DESC);
CREATE INDEX idx_injury_history_user ON injury_history(user_id, injury_date DESC);
CREATE INDEX idx_training_load_user_date ON training_load_metrics(user_id, date DESC);
```

### Phase 2: Feature Engineering (Week 3-4)
**Task 2.1: Training Load Calculations**
- Implement acute load calculation (7-day rolling window)
- Implement chronic load calculation (28-day rolling window)
- Calculate ACWR (Acute:Chronic Workload Ratio)
- Implement exponentially weighted moving average (EWMA) variant
- Calculate load spikes and sudden progressions
- Create `TrainingLoadCalculator` service

**Task 2.2: Recovery Metrics**
- Calculate rest day frequency
- Implement recovery adequacy score
- Calculate training monotony (SD of weekly load)
- Calculate training strain (load × monotony)
- Integrate sleep and HRV data (if available)
- Create `RecoveryMetricsService`

**Task 2.3: Biomechanical Features**
- Extract movement asymmetry from training data
- Calculate consistency scores
- Integrate computer vision form scores (if available)
- Calculate technique degradation over time
- Create `BiomechanicalFeatureService`

**Task 2.4: Historical Features**
- Extract injury history features:
  - Time since last injury
  - Total injury count
  - Injury recurrence rate
  - Days lost to injury (last 12 months)
- Calculate athlete experience level
- Extract age and demographic features
- Create `HistoricalFeatureService`

**Task 2.5: Feature Pipeline Integration**
- Extend existing `FeatureEngineeringService`
- Implement automated feature calculation on new training data
- Create feature vector generation for ML model
- Add feature normalization and scaling
- Implement caching for computed features

### Phase 3: ML Model Development (Week 5-7)
**Task 3.1: Training Data Preparation**
- Collect historical training data
- Label injury events (binary or risk score)
- Handle class imbalance (injuries are rare events):
  - SMOTE (Synthetic Minority Oversampling)
  - Class weighting
  - Stratified sampling
- Split data: train (70%), validation (15%), test (15%)
- Implement data augmentation strategies

**Task 3.2: Model Training**
- Train multiple model types:
  - Random Forest (interpretable, handles non-linear)
  - Gradient Boosting (high accuracy)
  - Logistic Regression (baseline, interpretable)
- Implement hyperparameter tuning with cross-validation
- Feature importance analysis
- Create `InjuryPredictionTrainer` service
- Save trained models with versioning

**Task 3.3: Model Evaluation**
- Evaluate models on test set:
  - Precision, Recall, F1-score
  - ROC-AUC curve
  - Precision-Recall curve
  - Confusion matrix
- Compare model performance
- Analyze false positives/negatives
- Select best-performing model
- Document model performance metrics

**Task 3.4: Model Explainability**
- Implement SHAP values or feature importance
- Generate per-prediction explanations
- Create risk factor breakdown
- Visualize decision paths
- Add confidence scores to predictions

**Task 3.5: Model Persistence**
- Implement model serialization (serde)
- Create model versioning system
- Store models in database or filesystem
- Implement model loading on service start
- Add model update mechanism

### Phase 4: Risk Scoring Engine (Week 8-9)
**Task 4.1: Real-Time Prediction Service**
- Create `InjuryRiskScoringService`
- Implement daily risk score calculation
- Cache risk scores in Redis for performance
- Trigger prediction on new training data
- Support batch predictions for historical analysis

**Task 4.2: Risk Level Classification**
- Map risk scores to risk levels:
  - Low: 0-25 (green)
  - Moderate: 26-50 (yellow)
  - High: 51-75 (orange)
  - Critical: 76-100 (red)
- Define risk level thresholds
- Implement risk level transitions
- Track risk level history

**Task 4.3: Contributing Factor Analysis**
- Identify top risk factors per prediction
- Rank factors by contribution to risk
- Generate factor descriptions
- Calculate factor severity scores
- Create actionable factor insights

**Task 4.4: Risk Trend Analysis**
- Calculate risk score trends (improving/worsening)
- Detect rapid risk increases
- Compare current risk to historical baseline
- Forecast future risk trajectory
- Generate trend visualizations

### Phase 5: Alert System (Week 10)
**Task 5.1: Alert Rules Engine**
- Create `InjuryAlertService`
- Define alert triggers:
  - Risk level transitions (low → moderate)
  - Sustained high risk (>3 days)
  - Rapid risk increases (>20 points in 7 days)
  - Critical risk threshold exceeded
- Implement alert cooldown to prevent spam
- Add alert priority levels

**Task 5.2: Alert Delivery**
- Integrate with existing notification system
- Send push notifications for high-risk alerts
- Email alerts for critical risk
- In-app alerts with risk dashboard
- Add alert preferences per user
- Implement alert history tracking

**Task 5.3: Alert Content**
- Generate clear, actionable alert messages
- Include current risk score and level
- List top contributing factors
- Provide immediate recommendations
- Add links to detailed risk view

### Phase 6: Recommendation Engine (Week 11)
**Task 6.1: Preventive Action Recommendations**
- Create `InjuryPreventionService`
- Generate recommendations based on risk factors:
  - **High Load**: Reduce volume, add rest
  - **Poor Recovery**: Improve sleep, active recovery
  - **Biomechanical**: Form drills, mobility work
  - **Monotony**: Vary training stimulus
- Prioritize recommendations by impact
- Provide specific, actionable advice

**Task 6.2: Training Adjustments**
- Suggest workout modifications:
  - Intensity reduction percentages
  - Volume reduction recommendations
  - Additional rest days
  - Cross-training alternatives
- Integrate with training plan service
- Auto-adjust plans for high-risk athletes (with consent)

**Task 6.3: Recovery Protocols**
- Recommend recovery activities:
  - Mobility exercises
  - Foam rolling
  - Active recovery workouts
  - Sleep optimization tips
- Link to exercise library/videos
- Track compliance with recommendations

### Phase 7: API & UI Integration (Week 12)
**Task 7.1: REST API Endpoints**
- `GET /api/v1/injury-risk/current` - Current risk score
- `GET /api/v1/injury-risk/history` - Risk history timeline
- `GET /api/v1/injury-risk/factors` - Current risk factors
- `GET /api/v1/injury-risk/recommendations` - Preventive actions
- `POST /api/v1/injury-risk/history` - Log injury event
- `GET /api/v1/injury-risk/alerts` - Alert history

**Task 7.2: Risk Dashboard API**
- Return risk score with visual indicator
- Provide risk trend chart data
- List top risk factors with severity
- Include recommendations
- Show historical risk patterns

**Task 7.3: Documentation**
- API documentation with examples
- Risk scoring methodology explanation
- Feature importance documentation
- Integration guide
- Best practices for injury prevention

### Phase 8: Testing & Validation (Week 13-14)
**Task 8.1: Model Validation**
- Validate predictions against actual injuries
- Calculate prediction accuracy over time
- Track false positive/negative rates
- Analyze model drift
- Collect user feedback on prediction accuracy

**Task 8.2: Comprehensive Testing**
- Unit tests for feature calculations
- Integration tests for prediction pipeline
- Load tests for concurrent predictions
- Test edge cases (new users, sparse data)
- Security testing for user data

**Task 8.3: A/B Testing**
- Create control group (no predictions)
- Create treatment group (with predictions)
- Track injury rates in both groups
- Measure effectiveness of alerts
- Analyze user engagement with recommendations

**Task 8.4: Continuous Learning**
- Implement feedback loop for model improvement
- Collect injury outcomes for retraining
- Schedule periodic model retraining
- Monitor model performance metrics
- Implement champion/challenger model testing

### Phase 9: Monitoring & Refinement (Week 15+)
**Task 9.1: Production Monitoring**
- Track prediction latency
- Monitor model performance metrics
- Alert on model degradation
- Track alert delivery success
- Monitor user engagement with recommendations

**Task 9.2: Model Updates**
- Retrain model monthly with new data
- A/B test new model versions
- Deploy improved models
- Document model improvements
- Archive old model versions

## API Endpoints

### Risk Assessment
- `GET /api/v1/injury-risk/current` - Get current injury risk
- `GET /api/v1/injury-risk/history?from=YYYY-MM-DD&to=YYYY-MM-DD` - Risk history
- `GET /api/v1/injury-risk/factors` - Current risk factors
- `GET /api/v1/injury-risk/recommendations` - Prevention recommendations
- `GET /api/v1/injury-risk/trend` - Risk trend analysis

### Injury Management
- `POST /api/v1/injuries` - Log injury event
- `GET /api/v1/injuries` - Get injury history
- `PATCH /api/v1/injuries/{id}` - Update injury status
- `GET /api/v1/injuries/stats` - Injury statistics

### Alerts
- `GET /api/v1/injury-risk/alerts` - Get alert history
- `PATCH /api/v1/injury-risk/alerts/{id}/acknowledge` - Acknowledge alert
- `GET /api/v1/injury-risk/alerts/settings` - Get alert preferences
- `PATCH /api/v1/injury-risk/alerts/settings` - Update alert preferences

## Response Schema Examples

```json
// Current Risk Assessment
{
  "user_id": "uuid",
  "prediction_date": "2025-09-30",
  "risk_score": 62.5,
  "risk_level": "high",
  "risk_change": "+12.3",
  "confidence": 0.87,
  "trend": "increasing",
  "model_version": "v1.2.0",
  "factors": [
    {
      "type": "training_load",
      "name": "high_acwr",
      "severity": 75.0,
      "description": "Acute:Chronic Workload Ratio is 1.8 (>1.5 indicates risk)",
      "recommendation": "Reduce training volume by 20-30% this week"
    },
    {
      "type": "recovery",
      "name": "insufficient_rest",
      "severity": 55.0,
      "description": "Only 1 rest day in past 14 days",
      "recommendation": "Add 2 rest days this week"
    }
  ],
  "recommendations": [
    {
      "priority": "high",
      "category": "training_adjustment",
      "action": "Reduce weekly mileage by 25%",
      "rationale": "Current load spike increases injury risk significantly"
    }
  ]
}

// Injury History Entry
{
  "id": "uuid",
  "user_id": "uuid",
  "injury_type": "achilles_tendinitis",
  "body_part": "achilles",
  "severity": "moderate",
  "injury_date": "2025-08-15",
  "recovery_date": "2025-09-10",
  "days_out": 26,
  "cause": "Rapid training volume increase",
  "treatment": "Rest, PT exercises, gradual return",
  "notes": "Occurred after 30% weekly mileage increase"
}
```

## ML Model Features

### Training Load Features (ACWR Focus)
1. **Acute Load** (7-day sum)
2. **Chronic Load** (28-day average)
3. **ACWR** (Acute/Chronic ratio)
4. **Load Spike** (Week-over-week % increase)
5. **Training Monotony** (mean/SD of weekly load)
6. **Training Strain** (load × monotony)

### Recovery Features
7. **Rest Days** (count in last 14 days)
8. **Recovery Adequacy** (rest relative to load)
9. **Sleep Quality** (if available)
10. **HRV Trend** (if available)

### Biomechanical Features
11. **Movement Asymmetry** (left/right imbalance)
12. **Form Degradation** (quality decline in sessions)
13. **Technique Consistency**

### Historical Features
14. **Days Since Last Injury**
15. **Total Injury Count** (last 12 months)
16. **Injury Recurrence Rate**
17. **Athlete Age**
18. **Training Experience** (years)

## Success Metrics
- **Model Performance**: AUC-ROC >0.75, Recall >0.70
- **Injury Reduction**: 20% reduction in injury rate vs. control
- **Alert Accuracy**: <30% false positive rate
- **User Engagement**: >60% users act on high-risk alerts
- **Prediction Timeliness**: Detect risk 7-14 days before injury

## Dependencies
- Existing training session data
- Feature engineering service
- ML model service
- Notification system
- Training plan service (for adjustments)

## Risks & Mitigations
- **Risk**: Insufficient training data for accurate predictions
  - **Mitigation**: Start with simple models, expand as data grows
- **Risk**: High false positive rate frustrates users
  - **Mitigation**: Tune model thresholds, add confidence scores
- **Risk**: Users ignore alerts ("alert fatigue")
  - **Mitigation**: Prioritize alerts, add cooldowns, personalize thresholds
- **Risk**: Model doesn't generalize across sports
  - **Mitigation**: Train sport-specific models, collect diverse data

## Future Enhancements
- Sport-specific injury models (running, cycling, swimming)
- Body-part-specific risk prediction (knee, ankle, shoulder)
- Integration with wearable HRV/sleep data
- Return-to-training protocols after injury
- Injury risk heatmaps by body region
- Team-level injury risk dashboard for coaches
- Predictive maintenance for equipment (shoes, etc.)