-- Migration: Seed Stress Management Recommendations
-- Description: Initial 10+ stress management recommendations for recovery optimization
-- Issue: #129 - Phase 8.1: Recommendation Library Foundation

-- Breathing & Meditation (4 recommendations)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'stress_management',
    'Box Breathing for Acute Stress',
    'Practice 5-10 minutes of box breathing (4-4-4-4) to reduce acute stress',
    'Inhale 4 counts, hold 4, exhale 4, hold 4. Repeat for 5-10 minutes. Use before bed, post-training, or when stressed.',
    'high',
    'easy',
    'Rapid stress reduction, improved HRV, better emotional regulation within minutes',
    10,
    '{"acute_stress": true, "low_hrv": true, "poor_sleep_onset": true, "anxiety": true}',
    '{"location": "anywhere", "equipment_needed": [], "guidance": "basic"}',
    '{"evidence_level": "strong", "pattern": "4-4-4-4", "duration_minutes": [5, 10]}'
),
(
    'stress_management',
    'Morning Meditation Practice',
    'Start day with 10-20 minute mindfulness meditation',
    'Sit comfortably, focus on breath. When mind wanders, gently return to breath. Use app guidance if needed (Headspace, Calm).',
    'medium',
    'moderate',
    'Reduced baseline stress, improved focus, better emotional resilience over 2-4 weeks',
    15,
    '{"chronic_stress": true, "poor_focus": true, "anxiety": true, "low_readiness": true}',
    '{"time_of_day": "morning", "location": "quiet_space", "guidance": "app_optional"}',
    '{"evidence_level": "strong", "duration_minutes": [10, 20], "consistency_key": true}'
),
(
    'stress_management',
    '4-7-8 Breathing for Sleep',
    'Use 4-7-8 breathing pattern before bed to activate parasympathetic system',
    'Inhale 4 counts through nose, hold 7, exhale 8 through mouth. Do 4-8 cycles. Practice 30-60 min before bed.',
    'medium',
    'easy',
    'Faster sleep onset, reduced nighttime anxiety, activated relaxation response',
    10,
    '{"poor_sleep_onset": true, "bedtime_anxiety": true, "racing_thoughts": true}',
    '{"time_of_day": "evening", "location": "bedroom", "equipment_needed": []}',
    '{"evidence_level": "moderate", "pattern": "4-7-8", "cycles": [4, 8], "timing": "30-60_min_pre_bed"}'
),
(
    'stress_management',
    'Heart Rate Variability Biofeedback',
    'Practice 10-15 minute HRV biofeedback using resonant frequency breathing',
    'Use HRV app (Elite HRV, HRV4Training). Breathe at personal resonant frequency (~5-6 breaths/min). Track HRV improvements.',
    'medium',
    'moderate',
    'Improved HRV, enhanced autonomic balance, better stress resilience within 1-2 weeks',
    15,
    '{"low_hrv": true, "poor_recovery": true, "chronic_stress": true}',
    '{"equipment_needed": ["hrv_app", "heart_rate_monitor"], "guidance": "app_provided"}',
    '{"evidence_level": "strong", "resonant_frequency_breaths_per_min": [5, 6], "duration_minutes": [10, 15]}'
);

-- Psychological Recovery (3 recommendations)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'stress_management',
    'Gratitude Journaling',
    'Write 3-5 things you''re grateful for each evening',
    'Before bed, write down 3-5 specific things you''re grateful for today. Be specific. Focus on experiences, not just things.',
    'low',
    'easy',
    'Improved mood, better sleep quality, enhanced psychological recovery over 1-2 weeks',
    10,
    '{"low_mood": true, "poor_sleep_quality": true, "negative_mindset": true}',
    '{"time_of_day": "evening", "equipment_needed": ["journal_or_phone"], "consistency_required": true}',
    '{"evidence_level": "moderate", "items_per_day": [3, 5], "specificity_important": true}'
),
(
    'stress_management',
    'Digital Detox Evening',
    'Take complete break from screens and digital devices 2+ hours before bed',
    'Set "digital sunset" time (e.g., 8pm). No phone, TV, computer. Replace with reading, conversation, gentle activities.',
    'medium',
    'challenging',
    'Reduced mental stimulation, improved sleep quality, better psychological recovery',
    120,
    '{"poor_sleep": true, "mental_fatigue": true, "screen_dependency": true, "evening_stress": true}',
    '{"time_of_day": "evening", "lifestyle": "requires_commitment", "alternatives_needed": true}',
    '{"evidence_level": "moderate", "screen_free_hours_before_bed": 2, "replacement_activities": "essential"}'
),
(
    'stress_management',
    'Progressive Muscle Relaxation',
    'Practice 15-20 minute PMR session to release physical tension',
    'Systematically tense and relax muscle groups: feet→calves→thighs→core→arms→face. Hold tension 5 sec, release 30 sec.',
    'medium',
    'easy',
    'Reduced muscle tension, improved body awareness, better sleep onset, stress relief',
    20,
    '{"muscle_tension": true, "stress": true, "poor_sleep_onset": true, "body_disconnection": true}',
    '{"location": "quiet_comfortable_space", "time_of_day": "evening_or_post_training"}',
    '{"evidence_level": "strong", "tension_hold_seconds": 5, "relaxation_seconds": 30, "duration_minutes": [15, 20]}'
);

-- Lifestyle Integration (3 recommendations)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'stress_management',
    'Nature Exposure for Recovery',
    'Spend 20-30 minutes in nature daily for psychological restoration',
    'Walk in park, forest, or natural setting. Leave phone behind. Focus on sensory experience: sights, sounds, smells.',
    'medium',
    'easy',
    'Reduced stress hormones, improved mood, enhanced psychological recovery, better focus',
    30,
    '{"chronic_stress": true, "mental_fatigue": true, "low_mood": true, "urban_environment": true}',
    '{"location": "outdoor_natural", "weather": "mild_to_good", "accessibility": "park_or_trail"}',
    '{"evidence_level": "strong", "duration_minutes": [20, 30], "attention_restoration_theory": true}'
),
(
    'stress_management',
    'Social Connection Time',
    'Schedule 30-60 minutes quality time with supportive friends or family',
    'Plan regular social activity: meal together, walk, conversation. Focus on positive, supportive relationships.',
    'medium',
    'moderate',
    'Improved mood, reduced stress, enhanced emotional support, better recovery perception',
    45,
    '{"social_isolation": true, "low_mood": true, "chronic_stress": true, "lack_support": true}',
    '{"lifestyle": "requires_planning", "relationship_quality": "supportive"}',
    '{"evidence_level": "moderate", "duration_minutes": [30, 60], "quality_over_quantity": true}'
),
(
    'stress_management',
    'Creative Expression Activity',
    'Engage in 20-30 minute creative activity for stress relief',
    'Choose creative outlet: drawing, music, writing, crafts. Focus on process, not outcome. No judgment or performance pressure.',
    'low',
    'easy',
    'Reduced stress, improved mood, enhanced psychological recovery, mental restoration',
    30,
    '{"mental_fatigue": true, "stress": true, "need_mental_break": true}',
    '{"equipment_needed": ["varies_by_activity"], "judgment_free": true, "process_focused": true}',
    '{"evidence_level": "moderate", "duration_minutes": [20, 30], "flow_state_beneficial": true}'
);

-- Comments for documentation
COMMENT ON TABLE recommendation_templates IS 'Stress management recommendations seeded with 10 evidence-based interventions';
