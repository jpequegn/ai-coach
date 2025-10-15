-- Validation Framework Tables for Vision Analysis Testing & Validation (Issue #87)
--
-- This migration creates the infrastructure for:
-- 1. Test dataset management with ground truth labels
-- 2. Validation metrics tracking (pose accuracy, form scoring, issue detection)
-- 3. Expert annotations and ratings
-- 4. Threshold optimization tracking

-- ============================================================================
-- Test Dataset Management
-- ============================================================================

-- Test videos with ground truth labels
CREATE TABLE IF NOT EXISTS validation_test_videos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    video_url TEXT NOT NULL,
    video_name VARCHAR(255) NOT NULL,
    exercise_type VARCHAR(50) NOT NULL,
    quality_category VARCHAR(50) NOT NULL, -- 'good_form', 'poor_form', 'mixed'
    body_type VARCHAR(50), -- 'ectomorph', 'mesomorph', 'endomorph', 'average'
    lighting_condition VARCHAR(50), -- 'bright', 'dim', 'outdoor', 'indoor'
    camera_angle VARCHAR(50), -- 'front', 'side', 'diagonal', '45_degree'
    video_duration_seconds DECIMAL(10, 2),
    video_resolution VARCHAR(20), -- '720p', '1080p', '4k'
    fps INTEGER,
    notes TEXT,
    uploaded_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_validation_videos_exercise ON validation_test_videos(exercise_type);
CREATE INDEX idx_validation_videos_quality ON validation_test_videos(quality_category);

-- ============================================================================
-- Expert Annotations
-- ============================================================================

-- Expert coaches providing ground truth
CREATE TABLE IF NOT EXISTS validation_experts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    expert_name VARCHAR(255) NOT NULL,
    credentials TEXT,
    specialization TEXT[], -- Areas of expertise
    years_experience INTEGER,
    certification_level VARCHAR(50), -- 'certified', 'master', 'elite'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Manual keypoint annotations (ground truth for pose estimation)
CREATE TABLE IF NOT EXISTS validation_keypoint_annotations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    video_id UUID NOT NULL REFERENCES validation_test_videos(id) ON DELETE CASCADE,
    expert_id UUID NOT NULL REFERENCES validation_experts(id),
    frame_number INTEGER NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    keypoints JSONB NOT NULL, -- Array of {joint_name, x, y, confidence, visible}
    annotation_method VARCHAR(50), -- 'manual', 'semi_auto', 'verified'
    annotation_quality VARCHAR(20), -- 'high', 'medium', 'low'
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(video_id, frame_number, expert_id)
);

CREATE INDEX idx_keypoint_annotations_video ON validation_keypoint_annotations(video_id);
CREATE INDEX idx_keypoint_annotations_expert ON validation_keypoint_annotations(expert_id);

-- Expert form ratings (ground truth for form scoring)
CREATE TABLE IF NOT EXISTS validation_form_ratings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    video_id UUID NOT NULL REFERENCES validation_test_videos(id) ON DELETE CASCADE,
    expert_id UUID NOT NULL REFERENCES validation_experts(id),
    overall_score DECIMAL(3, 1) NOT NULL CHECK (overall_score >= 0 AND overall_score <= 10),
    form_quality DECIMAL(3, 1) CHECK (form_quality >= 0 AND form_quality <= 10),
    injury_risk DECIMAL(3, 1) CHECK (injury_risk >= 0 AND injury_risk <= 10),
    range_of_motion DECIMAL(3, 1) CHECK (range_of_motion >= 0 AND range_of_motion <= 10),
    tempo_consistency DECIMAL(3, 1) CHECK (tempo_consistency >= 0 AND tempo_consistency <= 10),
    rep_count INTEGER,
    rating_confidence VARCHAR(20), -- 'high', 'medium', 'low'
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(video_id, expert_id)
);

CREATE INDEX idx_form_ratings_video ON validation_form_ratings(video_id);
CREATE INDEX idx_form_ratings_expert ON validation_form_ratings(expert_id);

-- Expert issue annotations (ground truth for issue detection)
CREATE TABLE IF NOT EXISTS validation_issue_annotations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    video_id UUID NOT NULL REFERENCES validation_test_videos(id) ON DELETE CASCADE,
    expert_id UUID NOT NULL REFERENCES validation_experts(id),
    issue_type VARCHAR(100) NOT NULL, -- 'knee_valgus', 'back_rounding', 'elbow_flare', etc.
    severity VARCHAR(20) NOT NULL, -- 'critical', 'warning', 'minor'
    description TEXT NOT NULL,
    affected_frames INTEGER[], -- Array of frame numbers where issue occurs
    timestamp_ranges JSONB, -- [{start_ms, end_ms}]
    confidence DECIMAL(3, 2) CHECK (confidence >= 0 AND confidence <= 1),
    corrective_action TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_issue_annotations_video ON validation_issue_annotations(video_id);
CREATE INDEX idx_issue_annotations_type ON validation_issue_annotations(issue_type);

-- ============================================================================
-- Validation Results & Metrics
-- ============================================================================

-- Validation run metadata
CREATE TABLE IF NOT EXISTS validation_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_name VARCHAR(255) NOT NULL,
    model_version VARCHAR(100), -- Version of pose estimation model
    threshold_config JSONB, -- Current threshold configuration
    dataset_size INTEGER, -- Number of test videos in this run
    run_type VARCHAR(50), -- 'pose_accuracy', 'form_scoring', 'issue_detection', 'full'
    initiated_by UUID REFERENCES users(id),
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL DEFAULT 'running', -- 'running', 'completed', 'failed'
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_validation_runs_status ON validation_runs(status);
CREATE INDEX idx_validation_runs_type ON validation_runs(run_type);

-- Pose estimation accuracy metrics
CREATE TABLE IF NOT EXISTS validation_pose_accuracy (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES validation_runs(id) ON DELETE CASCADE,
    video_id UUID NOT NULL REFERENCES validation_test_videos(id),
    frame_count INTEGER NOT NULL,
    mean_per_joint_position_error DECIMAL(10, 4), -- Average pixel error per joint
    percentage_correct_keypoints DECIMAL(5, 2), -- PCK (Percentage of Correct Keypoints)
    mean_average_precision DECIMAL(5, 4), -- mAP for pose detection
    keypoint_accuracy_by_joint JSONB, -- Per-joint accuracy scores
    detection_rate DECIMAL(5, 2), -- Percentage of frames with successful detection
    false_positive_rate DECIMAL(5, 4),
    processing_time_ms INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(run_id, video_id)
);

CREATE INDEX idx_pose_accuracy_run ON validation_pose_accuracy(run_id);

-- Form scoring validation metrics
CREATE TABLE IF NOT EXISTS validation_form_scoring (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES validation_runs(id) ON DELETE CASCADE,
    video_id UUID NOT NULL REFERENCES validation_test_videos(id),
    predicted_overall_score DECIMAL(3, 1),
    expert_overall_score DECIMAL(3, 1),
    score_difference DECIMAL(3, 1), -- absolute difference
    pearson_correlation DECIMAL(5, 4), -- correlation with expert ratings
    score_category_accuracy DECIMAL(5, 2), -- % correct category (excellent/good/fair/poor)
    component_scores JSONB, -- {form_quality, injury_risk, range_of_motion, tempo}
    expert_scores JSONB, -- Expert ratings for same components
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(run_id, video_id)
);

CREATE INDEX idx_form_scoring_run ON validation_form_scoring(run_id);

-- Issue detection validation metrics
CREATE TABLE IF NOT EXISTS validation_issue_detection (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES validation_runs(id) ON DELETE CASCADE,
    video_id UUID NOT NULL REFERENCES validation_test_videos(id),
    detected_issues JSONB, -- Array of detected issues with confidence
    expert_issues JSONB, -- Array of expert-annotated issues
    true_positives INTEGER, -- Correctly detected issues
    false_positives INTEGER, -- Incorrectly detected issues
    false_negatives INTEGER, -- Missed issues
    precision DECIMAL(5, 4), -- TP / (TP + FP)
    recall DECIMAL(5, 4), -- TP / (TP + FN)
    f1_score DECIMAL(5, 4), -- 2 * (precision * recall) / (precision + recall)
    issue_type_accuracy JSONB, -- Per-issue-type accuracy metrics
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(run_id, video_id)
);

CREATE INDEX idx_issue_detection_run ON validation_issue_detection(run_id);

-- ============================================================================
-- Threshold Optimization Tracking
-- ============================================================================

-- Track threshold configurations and their performance
CREATE TABLE IF NOT EXISTS validation_threshold_experiments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES validation_runs(id) ON DELETE CASCADE,
    threshold_name VARCHAR(100) NOT NULL, -- e.g., 'pose_confidence', 'nms_threshold'
    threshold_value DECIMAL(10, 6) NOT NULL,
    metric_name VARCHAR(100) NOT NULL, -- e.g., 'mAP', 'precision', 'f1_score'
    metric_value DECIMAL(10, 6) NOT NULL,
    dataset_subset VARCHAR(100), -- Which subset was tested
    is_optimal BOOLEAN DEFAULT false,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_threshold_experiments_run ON validation_threshold_experiments(run_id);
CREATE INDEX idx_threshold_experiments_metric ON validation_threshold_experiments(metric_name);

-- ============================================================================
-- Aggregated Validation Summary
-- ============================================================================

-- Summary statistics for each validation run
CREATE TABLE IF NOT EXISTS validation_run_summary (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES validation_runs(id) ON DELETE CASCADE UNIQUE,

    -- Pose Accuracy Summary
    avg_pose_map DECIMAL(5, 4), -- Average mAP across all videos
    avg_pck DECIMAL(5, 2), -- Average PCK across all videos
    avg_mpjpe DECIMAL(10, 4), -- Average mean per-joint position error
    pose_target_met BOOLEAN, -- Did we achieve >90% accuracy target

    -- Form Scoring Summary
    avg_pearson_correlation DECIMAL(5, 4), -- Average correlation with experts
    avg_score_difference DECIMAL(3, 1), -- Average absolute difference
    form_target_met BOOLEAN, -- Did we achieve >0.8 correlation target

    -- Issue Detection Summary
    avg_precision DECIMAL(5, 4), -- Average precision across all videos
    avg_recall DECIMAL(5, 4), -- Average recall across all videos
    avg_f1_score DECIMAL(5, 4), -- Average F1 score
    issue_target_met BOOLEAN, -- Did we achieve >85% precision target

    -- Overall
    all_targets_met BOOLEAN, -- Did we meet all acceptance criteria
    recommendations TEXT, -- Suggested next steps

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_run_summary_run ON validation_run_summary(run_id);
CREATE INDEX idx_run_summary_targets ON validation_run_summary(all_targets_met);

-- ============================================================================
-- Triggers for Updated Timestamps
-- ============================================================================

CREATE OR REPLACE FUNCTION update_validation_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_validation_test_videos_timestamp
    BEFORE UPDATE ON validation_test_videos
    FOR EACH ROW EXECUTE FUNCTION update_validation_timestamp();

CREATE TRIGGER update_validation_run_summary_timestamp
    BEFORE UPDATE ON validation_run_summary
    FOR EACH ROW EXECUTE FUNCTION update_validation_timestamp();
