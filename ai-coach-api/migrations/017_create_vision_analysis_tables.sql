-- Vision Analysis Tables for Computer Vision Training Analysis Feature
-- Phase 1: Video Upload Infrastructure

-- Enable UUID extension if not already enabled
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Vision Analyses: Main table for video analysis tracking
CREATE TABLE vision_analyses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    video_url TEXT NOT NULL,
    video_duration_seconds DOUBLE PRECISION,
    video_resolution VARCHAR(20), -- e.g., "1920x1080"
    video_format VARCHAR(10), -- e.g., "mp4", "mov"
    video_size_bytes BIGINT,
    status VARCHAR(20) NOT NULL DEFAULT 'uploaded', -- uploaded, processing, completed, failed
    exercise_type VARCHAR(50), -- squat, deadlift, running, push-up, plank, etc.
    upload_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processing_started_at TIMESTAMPTZ,
    processing_completed_at TIMESTAMPTZ,
    error_message TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Pose Detections: Frame-by-frame pose estimation results
CREATE TABLE pose_detections (
    id BIGSERIAL PRIMARY KEY,
    analysis_id UUID NOT NULL REFERENCES vision_analyses(id) ON DELETE CASCADE,
    frame_number INTEGER NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    keypoints JSONB NOT NULL, -- Array of {x, y, confidence, joint_name}
    confidence_score DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_analysis_frame UNIQUE (analysis_id, frame_number)
);

-- Movement Scores: Quality scoring and feedback per analysis
CREATE TABLE movement_scores (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    analysis_id UUID NOT NULL REFERENCES vision_analyses(id) ON DELETE CASCADE,
    overall_score DOUBLE PRECISION NOT NULL CHECK (overall_score >= 0 AND overall_score <= 100),
    form_quality DOUBLE PRECISION CHECK (form_quality >= 0 AND form_quality <= 100),
    injury_risk DOUBLE PRECISION CHECK (injury_risk >= 0 AND injury_risk <= 100),
    range_of_motion DOUBLE PRECISION CHECK (range_of_motion >= 0 AND range_of_motion <= 100),
    tempo_consistency DOUBLE PRECISION CHECK (tempo_consistency >= 0 AND tempo_consistency <= 100),
    rep_count INTEGER,
    issues JSONB DEFAULT '[]', -- Array of detected issues with severity, type, description, frames
    recommendations JSONB DEFAULT '[]', -- Array of improvement suggestions with priority and exercises
    biomechanics_data JSONB DEFAULT '{}', -- Joint angles, ROM data, symmetry metrics
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT one_score_per_analysis UNIQUE (analysis_id)
);

-- Indexes for performance optimization
CREATE INDEX idx_vision_analyses_user_id ON vision_analyses(user_id);
CREATE INDEX idx_vision_analyses_status ON vision_analyses(status);
CREATE INDEX idx_vision_analyses_upload_timestamp ON vision_analyses(upload_timestamp DESC);
CREATE INDEX idx_vision_analyses_user_upload ON vision_analyses(user_id, upload_timestamp DESC);
CREATE INDEX idx_vision_analyses_exercise_type ON vision_analyses(exercise_type) WHERE exercise_type IS NOT NULL;

CREATE INDEX idx_pose_detections_analysis_id ON pose_detections(analysis_id);
CREATE INDEX idx_pose_detections_frame ON pose_detections(analysis_id, frame_number);
CREATE INDEX idx_pose_detections_timestamp ON pose_detections(analysis_id, timestamp_ms);

CREATE INDEX idx_movement_scores_analysis_id ON movement_scores(analysis_id);
CREATE INDEX idx_movement_scores_overall_score ON movement_scores(overall_score DESC);

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_vision_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER vision_analyses_updated_at
    BEFORE UPDATE ON vision_analyses
    FOR EACH ROW
    EXECUTE FUNCTION update_vision_updated_at();

CREATE TRIGGER movement_scores_updated_at
    BEFORE UPDATE ON movement_scores
    FOR EACH ROW
    EXECUTE FUNCTION update_vision_updated_at();

-- Comments for documentation
COMMENT ON TABLE vision_analyses IS 'Stores metadata and status for video analysis requests';
COMMENT ON TABLE pose_detections IS 'Frame-by-frame pose estimation keypoint data';
COMMENT ON TABLE movement_scores IS 'Quality scoring, issues, and recommendations for analyzed movements';
COMMENT ON COLUMN vision_analyses.status IS 'Analysis workflow status: uploaded, processing, completed, failed';
COMMENT ON COLUMN vision_analyses.exercise_type IS 'Detected or user-specified exercise type';
COMMENT ON COLUMN pose_detections.keypoints IS 'JSON array of body landmarks with coordinates and confidence';
COMMENT ON COLUMN movement_scores.issues IS 'Detected form issues with severity, type, description, and frame numbers';
COMMENT ON COLUMN movement_scores.recommendations IS 'Actionable feedback with exercises and cues for improvement';
