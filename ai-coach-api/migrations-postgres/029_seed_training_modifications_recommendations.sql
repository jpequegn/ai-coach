-- Migration: Seed Training Modification Recommendations
-- Description: Initial 10+ training modification recommendations for recovery optimization
-- Issue: #129 - Phase 8.1: Recommendation Library Foundation

-- Volume Adjustments (3 recommendations)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'training_modification',
    'Reduce Training Volume 20-30%',
    'Decrease weekly training volume by 20-30% when recovery is compromised',
    'Cut total training time/distance by 20-30% this week. Maintain intensity zones but reduce duration. Example: 60min→45min sessions.',
    'high',
    'moderate',
    'Improved recovery, reduced injury risk, restored readiness within 3-7 days',
    0,
    '{"poor_recovery": true, "low_hrv": true, "high_fatigue": true, "readiness_score_below": 60}',
    '{"training_plan_flexibility": "required", "self_coaching": true}',
    '{"evidence_level": "strong", "volume_reduction_percent": [20, 30], "duration_days": [3, 7]}'
),
(
    'training_modification',
    'Add Extra Rest Day',
    'Insert complete rest day when recovery markers are poor',
    'Take full day off from training. Replace with very light activity only (gentle walk <30min) or complete rest.',
    'critical',
    'easy',
    'Rapid recovery improvement, reduced accumulated fatigue, better subsequent training quality',
    0,
    '{"consecutive_poor_recovery": true, "readiness_score_below": 55, "illness_symptoms": true}',
    '{"schedule_flexibility": "required", "training_plan_adjustment": true}',
    '{"evidence_level": "strong", "complete_rest_preferred": true, "light_activity_max_minutes": 30}'
),
(
    'training_modification',
    'Convert Hard Session to Easy',
    'Replace planned hard workout with easy aerobic session',
    'If hard session planned but recovery poor, do easy 30-45min aerobic at 65-75% max HR instead. Reschedule hard session.',
    'high',
    'moderate',
    'Maintained training stimulus without overload, improved recovery trajectory',
    0,
    '{"poor_morning_readiness": true, "hard_session_scheduled": true, "low_hrv": true}',
    '{"same_day_decision": true, "training_plan_flexibility": "required"}',
    '{"evidence_level": "strong", "replacement_heart_rate_percent": [65, 75], "duration_minutes": [30, 45]}'
);

-- Intensity Modifications (3 recommendations)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'training_modification',
    'Lower Intensity Zones 5-10%',
    'Reduce training intensity by 5-10% when fatigue is elevated',
    'Lower HR/pace targets: If zone 3 = 150-160bpm, train at 145-155bpm instead. Same duration, reduced intensity.',
    'medium',
    'moderate',
    'Reduced physiological stress, maintained aerobic stimulus, improved recovery quality',
    0,
    '{"elevated_fatigue": true, "poor_session_performance": true, "readiness_score_below": 65}',
    '{"heart_rate_monitor": "required", "zone_awareness": true}',
    '{"evidence_level": "strong", "intensity_reduction_percent": [5, 10], "maintain_duration": true}'
),
(
    'training_modification',
    'Skip High-Intensity Intervals',
    'Replace interval session with tempo or steady-state work',
    'Instead of intervals, do continuous effort at tempo pace (80-85% max HR) for same total work duration.',
    'medium',
    'moderate',
    'Reduced neuromuscular stress, maintained cardiovascular stimulus, lower recovery cost',
    0,
    '{"poor_recovery": true, "interval_session_scheduled": true, "muscle_soreness": true}',
    '{"training_plan_modification": true, "tempo_pace_knowledge": "required"}',
    '{"evidence_level": "strong", "replacement_intensity_percent": [80, 85], "continuous_effort": true}'
),
(
    'training_modification',
    'Deload Week Protocol',
    'Implement planned deload week: 40-50% volume, 70-80% intensity',
    'Reduce volume to 40-50% normal, intensity to 70-80%. Maintain frequency. Focus on technique and recovery.',
    'high',
    'challenging',
    'Complete recovery, supercompensation, reduced injury risk, readiness for next training block',
    0,
    '{"accumulated_fatigue": true, "3-4_weeks_hard_training": true, "multiple_poor_recovery_days": true}',
    '{"planned_deload": true, "week_long_commitment": true, "training_cycle_awareness": true}',
    '{"evidence_level": "strong", "volume_percent": [40, 50], "intensity_percent": [70, 80], "maintain_frequency": true}'
);

-- Session Structure Changes (4 recommendations)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'training_modification',
    'Split Long Session into Two',
    'Divide single long workout into two shorter sessions',
    'Instead of one 2-hour session, do two 1-hour sessions (AM + PM). Reduces per-session stress, improves recovery.',
    'medium',
    'challenging',
    'Reduced single-session stress, better glycogen management, improved recovery between efforts',
    0,
    '{"long_session_scheduled": true, "poor_recovery": true, "schedule_allows_double": true}',
    '{"schedule_flexibility": "high", "two_session_logistics": "required"}',
    '{"evidence_level": "moderate", "session_split_benefit": "reduced_per_session_stress"}'
),
(
    'training_modification',
    'Add Extra Warm-Up Time',
    'Extend warm-up by 10-15 minutes when recovery is compromised',
    'Add 10-15 min easy aerobic + dynamic mobility before main session. Gradual intensity ramp. Better preparation when fatigued.',
    'medium',
    'easy',
    'Improved session quality, reduced injury risk, better neuromuscular readiness',
    15,
    '{"poor_morning_readiness": true, "muscle_stiffness": true, "hard_session_scheduled": true}',
    '{"time_availability": "15_extra_minutes", "warm_up_knowledge": "basic"}',
    '{"evidence_level": "strong", "extra_warmup_minutes": [10, 15], "gradual_intensity_ramp": true}'
),
(
    'training_modification',
    'Increase Recovery Between Intervals',
    'Extend rest periods between intervals by 50-100%',
    'If plan says 2min rest, take 3-4min instead. Maintain interval quality over total volume when fatigued.',
    'medium',
    'easy',
    'Better interval quality, reduced accumulated fatigue, maintained training stimulus',
    0,
    '{"poor_interval_quality": true, "elevated_fatigue": true, "interval_session": true}',
    '{"workout_modification": "on_the_fly", "quality_over_volume_mindset": true}',
    '{"evidence_level": "strong", "rest_extension_percent": [50, 100], "quality_priority": true}'
),
(
    'training_modification',
    'Substitute Cross-Training',
    'Replace impact training with low-impact alternative',
    'Swap run/court sport with bike, swim, or elliptical. Match duration and effort level, reduce impact stress.',
    'medium',
    'moderate',
    'Reduced impact stress, maintained aerobic fitness, injury prevention',
    0,
    '{"joint_pain": true, "impact_sensitivity": true, "injury_risk": true}',
    '{"alternative_equipment_access": true, "cross_training_experience": "basic"}',
    '{"evidence_level": "strong", "impact_alternatives": ["bike", "swim", "elliptical"], "match_effort_duration": true}'
);

-- Comments for documentation
COMMENT ON TABLE recommendation_templates IS 'Training modification recommendations seeded with 10 evidence-based interventions';
