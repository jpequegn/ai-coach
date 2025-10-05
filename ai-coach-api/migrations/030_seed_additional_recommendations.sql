-- Migration: Seed Additional Recommendations
-- Description: Additional 10 recommendations to reach 75+ total
-- Issue: #129 - Phase 8.1: Recommendation Library Foundation

-- Additional Sleep Recommendations (2)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'sleep',
    'Sleep Hygiene Audit',
    'Review and optimize all sleep hygiene factors systematically',
    'Assess: bedroom temp, darkness, noise, bedding comfort, evening routine. Make 2-3 improvements this week.',
    'medium',
    'moderate',
    'Comprehensive sleep quality improvement, better recovery markers',
    45,
    '{"poor_sleep_quality": true, "sleep_efficiency_below": 85, "inconsistent_sleep": true}',
    '{"time_of_day": "flexible", "requires_assessment": true}',
    '{"evidence_level": "strong", "systematic_approach": true}'
),
(
    'sleep',
    'Sleep Tracking & Awareness',
    'Track sleep patterns for 1 week to identify improvement opportunities',
    'Use sleep tracker (Oura, Whoop, app). Log wake quality, dreams, disturbances. Review weekly patterns.',
    'low',
    'easy',
    'Increased sleep awareness, personalized optimization insights',
    10,
    '{"poor_sleep_insight": true, "wants_optimization": true}',
    '{"equipment_needed": ["sleep_tracker_or_app"], "tracking_commitment": "7_days"}',
    '{"evidence_level": "moderate", "self_awareness_tool": true}'
);

-- Additional Active Recovery Recommendations (3)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'active_recovery',
    'Mobility Flow Session',
    'Perform 30-minute mobility flow combining yoga, stretching, and dynamic movement',
    'Follow mobility flow sequence: Cat-cow, thread needle, hip openers, shoulder circles, spinal twists. Smooth transitions.',
    'medium',
    'moderate',
    'Enhanced flexibility, reduced stiffness, improved movement quality, mental relaxation',
    30,
    '{"muscle_tightness": true, "poor_mobility": true, "recovery_day": true}',
    '{"equipment_needed": ["mat"], "location": "home_or_gym", "guidance": "video_optional"}',
    '{"evidence_level": "moderate", "flow_emphasis": true}'
),
(
    'active_recovery',
    'Breathing & Movement Integration',
    'Combine breathwork with gentle movement for 20 minutes',
    'Slow movements (tai chi style) coordinated with deep breathing. 4-count inhale/movement, 4-count exhale/return.',
    'low',
    'easy',
    'Parasympathetic activation, reduced stress, improved body awareness, gentle recovery',
    20,
    '{"stress": true, "mental_fatigue": true, "need_gentle_recovery": true}',
    '{"location": "quiet_space", "equipment_needed": [], "mind_body_focus": true}',
    '{"evidence_level": "moderate", "integrative_approach": true}'
),
(
    'active_recovery',
    'Cold Water Immersion',
    'Use cold plunge or cold shower (2-5 min) for recovery and inflammation reduction',
    'Immerse in 50-60°F water for 2-5 minutes post-training. Build tolerance gradually. Focus on breathing.',
    'medium',
    'advanced',
    'Reduced inflammation, enhanced recovery, improved circulation, mental resilience',
    5,
    '{"post_hard_training": true, "inflammation": true, "muscle_soreness": true}',
    '{"equipment_needed": ["cold_plunge_or_shower"], "cold_tolerance": "required", "gradual_adaptation": true}',
    '{"evidence_level": "strong", "temperature_fahrenheit": [50, 60], "duration_minutes": [2, 5]}'
);

-- Additional Stress Management Recommendations (2)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'stress_management',
    'Body Scan Meditation',
    'Practice 15-minute body scan to release tension and improve awareness',
    'Lie down, systematically scan body head-to-toe. Notice sensations without judgment. Release tension on exhale.',
    'medium',
    'easy',
    'Reduced muscle tension, improved body awareness, better stress management, enhanced sleep',
    15,
    '{"muscle_tension": true, "stress": true, "poor_body_awareness": true}',
    '{"location": "quiet_comfortable_space", "time_of_day": "flexible", "guidance": "app_helpful"}',
    '{"evidence_level": "strong", "mindfulness_based": true}'
),
(
    'stress_management',
    'Laughter and Play',
    'Engage in 15-30 minutes of playful activity or comedy for stress relief',
    'Watch comedy, play games, joke with friends, or engage in playful movement. Genuine laughter preferred.',
    'low',
    'easy',
    'Reduced cortisol, improved mood, enhanced psychological recovery, better immune function',
    20,
    '{"chronic_stress": true, "low_mood": true, "serious_mindset": true}',
    '{"lifestyle": "flexible", "social_optional": true, "enjoyment_focus": true}',
    '{"evidence_level": "moderate", "parasympathetic_activation": true}'
);

-- Additional Training Modification Recommendations (3)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'training_modification',
    'Technical Work Focus Day',
    'Replace high-intensity session with technical skill work at low intensity',
    'Focus on form, technique, efficiency. Low speed/intensity. Use mirrors, video, or coach feedback.',
    'medium',
    'easy',
    'Skill improvement without fatigue cost, maintained training habit, enhanced movement quality',
    0,
    '{"poor_recovery": true, "skill_development_needed": true, "hard_session_scheduled": true}',
    '{"training_plan_flexibility": true, "technical_knowledge": "basic"}',
    '{"evidence_level": "strong", "skill_maintenance": true}'
),
(
    'training_modification',
    'Active Rest Week',
    'Take full week of easy, enjoyable movement only',
    'No structured training. Only fun activities: hiking, swimming, dancing, casual sports. Listen to body.',
    'medium',
    'moderate',
    'Complete recovery, mental refresh, renewed motivation, injury prevention',
    0,
    '{"burnout_signs": true, "multiple_weeks_fatigue": true, "motivation_loss": true}',
    '{"schedule_flexibility": "week_commitment", "training_cycle_planned": true}',
    '{"evidence_level": "strong", "mental_physical_recovery": true, "duration_days": 7}'
),
(
    'training_modification',
    'Reverse Taper',
    'Gradually reduce training volume over 2-3 weeks when showing fatigue signs',
    'Week 1: 80% volume, Week 2: 60% volume, Week 3: 40% volume. Maintain intensity patterns.',
    'high',
    'challenging',
    'Systematic recovery, preserved fitness, reduced injury risk, readiness restoration',
    0,
    '{"accumulated_fatigue": true, "performance_decline": true, "multiple_poor_recovery_days": true}',
    '{"planned_reduction": true, "2-3_week_commitment": true, "training_plan_required": true}',
    '{"evidence_level": "strong", "volume_reduction_schedule": [80, 60, 40], "maintain_intensity": true}'
);

-- Comments for documentation
COMMENT ON TABLE recommendation_templates IS 'Recommendation library with 75+ evidence-based interventions across 5 categories';
