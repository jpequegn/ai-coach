-- Migration: Seed Progressive Recommendation Variants
-- Description: Create beginner/intermediate/advanced versions of core recommendations
-- SQLite-compatible version

-- Sleep Recommendations Progression (Template IDs: 10000000-0000-0000-0000-000000000001 through 000000000005)

-- Sleep Duration Extension
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000001-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000001', 'beginner', 0.5, 0.5, 'Start by going to bed just 15 minutes earlier for easy wins'),
('90000001-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000001', 'intermediate', 1.0, 1.0, 'Gradually extend sleep by 30-45 minutes with consistent bedtime routine'),
('90000001-0000-0000-0000-000000000003', '10000000-0000-0000-0000-000000000001', 'advanced', 1.5, 1.5, 'Optimize full sleep duration to 8-9 hours with strict sleep schedule and environment control');

-- Sleep Timing Optimization
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000002-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000002', 'beginner', 0.5, 0.5, 'Choose a consistent bedtime within a 1-hour window'),
('90000002-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000002', 'intermediate', 1.0, 1.0, 'Maintain bedtime within ±30 minutes and align with natural circadian rhythm'),
('90000002-0000-0000-0000-000000000003', '10000000-0000-0000-0000-000000000002', 'advanced', 1.5, 1.5, 'Track and optimize sleep cycles using chronotype and targeted sleep timing');

-- Sleep Environment Optimization
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000003-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000003', 'beginner', 0.5, 0.5, 'Start with one environmental factor: either light blocking or temperature adjustment'),
('90000003-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000003', 'intermediate', 1.0, 1.0, 'Optimize 2-3 factors: temperature (60-67°F), darkness, and noise reduction'),
('90000003-0000-0000-0000-000000000003', '10000000-0000-0000-0000-000000000003', 'advanced', 1.5, 1.5, 'Full sleep sanctuary: optimize all factors including air quality, bedding, and technology-free zone');

-- Evening Routine
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000004-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000004', 'beginner', 0.5, 0.5, 'Simple 10-minute wind-down: dim lights and avoid screens 30 minutes before bed'),
('90000004-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000004', 'intermediate', 1.0, 1.0, 'Structured 30-minute routine with relaxation techniques and screen-free time'),
('90000004-0000-0000-0000-000000000003', '10000000-0000-0000-0000-000000000004', 'advanced', 1.5, 1.5, 'Comprehensive 60-minute wind-down protocol with meditation, journaling, and nervous system regulation');

-- Strategic Napping
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000005-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000005', 'beginner', 0.5, 0.5, 'Optional 10-15 minute power nap if feeling fatigued'),
('90000005-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000005', 'intermediate', 1.0, 1.0, 'Strategic 20-minute nap at optimal time (1-3pm) without disrupting nighttime sleep'),
('90000005-0000-0000-0000-000000000003', '10000000-0000-0000-0000-000000000005', 'advanced', 1.5, 1.5, 'Precision nap timing based on sleep cycles and training schedule for peak recovery');

-- Nutrition Recommendations Progression (Template IDs: 20000000-0000-0000-0000-000000000001 through 000000000005)

-- Post-Workout Nutrition
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000006-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000001', 'beginner', 0.5, 0.5, 'Eat any meal with protein and carbs within 2 hours after workout'),
('90000006-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000001', 'intermediate', 1.0, 1.0, 'Consume 20-30g protein and carbs within 60 minutes post-workout'),
('90000006-0000-0000-0000-000000000003', '20000000-0000-0000-0000-000000000001', 'advanced', 1.5, 1.5, 'Optimize macros (3:1 carb:protein ratio) and timing (within 30 min) based on workout intensity');

-- Hydration Strategy
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000007-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000002', 'beginner', 0.5, 0.5, 'Drink water when thirsty and have a glass with each meal'),
('90000007-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000002', 'intermediate', 1.0, 1.0, 'Target 2-3 liters daily with consistent intake throughout the day'),
('90000007-0000-0000-0000-000000000003', '20000000-0000-0000-0000-000000000002', 'advanced', 1.5, 1.5, 'Calculated hydration based on body weight, training load, and sweat rate testing');

-- Anti-Inflammatory Foods
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000008-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000003', 'beginner', 0.5, 0.5, 'Add one anti-inflammatory food daily (berries, leafy greens, or fatty fish)'),
('90000008-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000003', 'intermediate', 1.0, 1.0, 'Include 3-4 anti-inflammatory foods daily with focus on variety'),
('90000008-0000-0000-0000-000000000003', '20000000-0000-0000-0000-000000000003', 'advanced', 1.5, 1.5, 'Structured anti-inflammatory diet with tracked polyphenol intake and inflammatory markers');

-- Protein Intake
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000009-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000004', 'beginner', 0.5, 0.5, 'Include protein source (palm-sized portion) with each main meal'),
('90000009-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000004', 'intermediate', 1.0, 1.0, 'Target 1.6-2.2g protein per kg bodyweight distributed across 3-4 meals'),
('90000009-0000-0000-0000-000000000003', '20000000-0000-0000-0000-000000000004', 'advanced', 1.5, 1.5, 'Optimized protein timing and leucine content (3g+ per meal) for maximum muscle protein synthesis');

-- Carbohydrate Timing
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000010-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000005', 'beginner', 0.5, 0.5, 'Include carbs with breakfast and post-workout meals'),
('90000010-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000005', 'intermediate', 1.0, 1.0, 'Strategic carb intake: higher on training days, moderate on rest days'),
('90000010-0000-0000-0000-000000000003', '20000000-0000-0000-0000-000000000005', 'advanced', 1.5, 1.5, 'Periodized carb intake matched to training phases with glycogen supercompensation protocols');

-- Active Recovery Recommendations Progression (Template IDs: 30000000-0000-0000-0000-000000000001 through 000000000005)

-- Post-Workout Walk
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000011-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000001', 'beginner', 0.5, 0.5, '5-10 minute easy walk after intense training'),
('90000011-0000-0000-0000-000000000002', '30000000-0000-0000-0000-000000000001', 'intermediate', 1.0, 1.0, '15-20 minute walk at conversational pace focusing on active recovery'),
('90000011-0000-0000-0000-000000000003', '30000000-0000-0000-0000-000000000001', 'advanced', 1.5, 1.5, '30-minute structured walk with heart rate monitoring in Zone 1 (50-60% max HR)');

-- Dynamic Stretching
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000012-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000002', 'beginner', 0.5, 0.5, '5 minutes of basic dynamic stretches targeting major muscle groups'),
('90000012-0000-0000-0000-000000000002', '30000000-0000-0000-0000-000000000002', 'intermediate', 1.0, 1.0, '15-minute dynamic stretching routine with sport-specific movements'),
('90000012-0000-0000-0000-000000000003', '30000000-0000-0000-0000-000000000002', 'advanced', 1.5, 1.5, '25-minute comprehensive mobility flow with progressive ROM work and muscle activation');

-- Foam Rolling
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000013-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000003', 'beginner', 0.5, 0.5, '5 minutes of foam rolling on most sore areas'),
('90000013-0000-0000-0000-000000000002', '30000000-0000-0000-0000-000000000003', 'intermediate', 1.0, 1.0, '10-15 minutes systematic foam rolling of all major muscle groups'),
('90000013-0000-0000-0000-000000000003', '30000000-0000-0000-0000-000000000003', 'advanced', 1.5, 1.5, '20-minute myofascial release protocol with specific trigger point work and tissue mobilization');

-- Yoga/Mobility Session
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000014-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000004', 'beginner', 0.5, 0.5, '10-minute gentle yoga or stretching video'),
('90000014-0000-0000-0000-000000000002', '30000000-0000-0000-0000-000000000004', 'intermediate', 1.0, 1.0, '20-30 minute restorative yoga or mobility class'),
('90000014-0000-0000-0000-000000000003', '30000000-0000-0000-0000-000000000004', 'advanced', 1.5, 1.5, '45-60 minute targeted mobility session with progressive loading and end-range strengthening');

-- Swimming/Aquatic Recovery
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000015-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000005', 'beginner', 0.5, 0.5, '10-15 minutes easy swimming or water walking'),
('90000015-0000-0000-0000-000000000002', '30000000-0000-0000-0000-000000000005', 'intermediate', 1.0, 1.0, '20-30 minutes easy swimming focusing on technique and low intensity'),
('90000015-0000-0000-0000-000000000003', '30000000-0000-0000-0000-000000000005', 'advanced', 1.5, 1.5, '45-minute structured aquatic recovery session with varied strokes and active rest intervals');

-- Stress Management Recommendations Progression (Template IDs: 40000000-0000-0000-0000-000000000001 through 000000000005)

-- Breathing Exercises
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000016-0000-0000-0000-000000000001', '40000000-0000-0000-0000-000000000001', 'beginner', 0.5, 0.5, '2-3 minutes of simple box breathing (4-4-4-4 count)'),
('90000016-0000-0000-0000-000000000002', '40000000-0000-0000-0000-000000000001', 'intermediate', 1.0, 1.0, '5-10 minutes of structured breathwork with heart rate variability focus'),
('90000016-0000-0000-0000-000000000003', '40000000-0000-0000-0000-000000000001', 'advanced', 1.5, 1.5, '15-20 minute comprehensive breathwork protocol with parasympathetic activation and HRV biofeedback');

-- Mindfulness Meditation
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000017-0000-0000-0000-000000000001', '40000000-0000-0000-0000-000000000002', 'beginner', 0.5, 0.5, '3-5 minutes guided meditation focusing on breath awareness'),
('90000017-0000-0000-0000-000000000002', '40000000-0000-0000-0000-000000000002', 'intermediate', 1.0, 1.0, '10-15 minutes focused attention or body scan meditation'),
('90000017-0000-0000-0000-000000000003', '40000000-0000-0000-0000-000000000002', 'advanced', 1.5, 1.5, '20-30 minutes open awareness meditation with non-reactive observation practice');

-- Journaling/Reflection
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000018-0000-0000-0000-000000000001', '40000000-0000-0000-0000-000000000003', 'beginner', 0.5, 0.5, '5 minutes of gratitude journaling (list 3 things)'),
('90000018-0000-0000-0000-000000000002', '40000000-0000-0000-0000-000000000003', 'intermediate', 1.0, 1.0, '10-15 minutes structured journaling with training reflection and mood tracking'),
('90000018-0000-0000-0000-000000000003', '40000000-0000-0000-0000-000000000003', 'advanced', 1.5, 1.5, '20-30 minutes comprehensive journaling with performance analysis, emotional processing, and goal setting');

-- Nature Time/Outdoor Exposure
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000019-0000-0000-0000-000000000001', '40000000-0000-0000-0000-000000000004', 'beginner', 0.5, 0.5, '10 minutes sitting outside or near a window with natural light'),
('90000019-0000-0000-0000-000000000002', '40000000-0000-0000-0000-000000000004', 'intermediate', 1.0, 1.0, '20-30 minutes outdoor walk or nature exposure'),
('90000019-0000-0000-0000-000000000003', '40000000-0000-0000-0000-000000000004', 'advanced', 1.5, 1.5, '60+ minutes of deliberate nature immersion with mindful awareness and grounding practices');

-- Social Connection
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000020-0000-0000-0000-000000000001', '40000000-0000-0000-0000-000000000005', 'beginner', 0.5, 0.5, '5-10 minutes text or call with friend or family member'),
('90000020-0000-0000-0000-000000000002', '40000000-0000-0000-0000-000000000005', 'intermediate', 1.0, 1.0, '30 minutes face-to-face or video conversation with meaningful connection'),
('90000020-0000-0000-0000-000000000003', '40000000-0000-0000-0000-000000000005', 'advanced', 1.5, 1.5, 'Planned social activity or group gathering fostering deep connection and community support');

-- Training Modifications Progression (Template IDs: 50000000-0000-0000-0000-000000000001 through 000000000005)

-- Volume Reduction
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000021-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000001', 'beginner', 0.5, 0.5, 'Reduce workout duration by 10-15%'),
('90000021-0000-0000-0000-000000000002', '50000000-0000-0000-0000-000000000001', 'intermediate', 1.0, 1.0, 'Cut training volume by 20-30% while maintaining intensity'),
('90000021-0000-0000-0000-000000000003', '50000000-0000-0000-0000-000000000001', 'advanced', 1.5, 1.5, 'Strategic volume reduction (30-50%) with precise load management and auto-regulation');

-- Intensity Modification
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000022-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000002', 'beginner', 0.5, 0.5, 'Replace one high-intensity session with moderate intensity'),
('90000022-0000-0000-0000-000000000002', '50000000-0000-0000-0000-000000000002', 'intermediate', 1.0, 1.0, 'Reduce intensity by 10-20% across all training sessions this week'),
('90000022-0000-0000-0000-000000000003', '50000000-0000-0000-0000-000000000002', 'advanced', 1.5, 1.5, 'Periodized intensity management with RPE/RIR-based auto-regulation and power meter guidance');

-- Additional Rest Day
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000023-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000003', 'beginner', 0.5, 0.5, 'Add one complete rest day this week'),
('90000023-0000-0000-0000-000000000002', '50000000-0000-0000-0000-000000000003', 'intermediate', 1.0, 1.0, 'Schedule 2 full rest days with active recovery options'),
('90000023-0000-0000-0000-000000000003', '50000000-0000-0000-0000-000000000003', 'advanced', 1.5, 1.5, 'Strategic rest week (deload) with 40-60% normal training load and recovery focus');

-- Cross-Training Substitution
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000024-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000004', 'beginner', 0.5, 0.5, 'Swap one hard workout for easy cross-training (swimming, cycling, etc.)'),
('90000024-0000-0000-0000-000000000002', '50000000-0000-0000-0000-000000000004', 'intermediate', 1.0, 1.0, 'Replace 2-3 sport-specific sessions with complementary cross-training'),
('90000024-0000-0000-0000-000000000003', '50000000-0000-0000-0000-000000000004', 'advanced', 1.5, 1.5, 'Structured cross-training block (1-2 weeks) maintaining fitness while reducing sport-specific stress');

-- Technique-Focused Sessions
INSERT INTO recommendation_progression (id, recommendation_template_id, version, difficulty_modifier, time_modifier, complexity_description) VALUES
('90000025-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000005', 'beginner', 0.5, 0.5, 'Dedicate 10 minutes of one workout to technique drills'),
('90000025-0000-0000-0000-000000000002', '50000000-0000-0000-0000-000000000005', 'intermediate', 1.0, 1.0, 'Replace one workout with 30-minute technical skills session'),
('90000025-0000-0000-0000-000000000003', '50000000-0000-0000-0000-000000000005', 'advanced', 1.5, 1.5, 'Full week of technique-focused training with video analysis and movement quality emphasis');
