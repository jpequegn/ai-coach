# Feature #3: Computer Vision Training Analysis

## Overview
Implement computer vision capabilities to analyze exercise form, provide movement quality scoring, and assess injury risk through video analysis. This transforms AI Coach into a comprehensive training platform with visual feedback.

## Business Value
- **Injury Prevention**: Detect poor form before injuries occur
- **Form Optimization**: Help athletes perfect technique
- **Differentiation**: Unique value proposition vs. competitors
- **Premium Feature**: High-value capability for subscription tiers
- **Coach Augmentation**: Scales coaching expertise

## Technical Architecture

### Components
1. **Video Upload API** (multipart file handling)
2. **Video Processing Pipeline** (FFmpeg)
3. **Pose Estimation Engine** (ML models)
4. **Movement Analysis Service** (biomechanics)
5. **Feedback Generation** (scoring and recommendations)

### Technology Stack
- `axum-extra` multipart for video uploads
- `ffmpeg` (via `ffmpeg-next`) for video processing
- `tract` or `burn` for ONNX model inference
- `ndarray` for tensor operations
- S3-compatible storage (MinIO/AWS) for video files
- PostgreSQL for metadata and analysis results

## Implementation Tasks

### Phase 1: Video Upload Infrastructure (Week 1-2)
**Task 1.1: Video Upload API**
- Create `POST /api/v1/vision/upload` endpoint with multipart support
- Implement file validation (format, size, duration limits)
- Add virus scanning for uploaded files
- Generate unique video IDs and storage paths
- Return upload progress for large files

**Task 1.2: Storage Integration**
- Set up MinIO/S3 bucket for video storage
- Implement `VideoStorageService` for upload/download
- Add signed URL generation for secure access
- Implement storage quota management per user
- Add automatic cleanup of old/processed videos

**Task 1.3: Video Processing Pipeline**
- Install and configure FFmpeg
- Create `VideoProcessingService` for video manipulation
- Implement video format conversion (to MP4)
- Extract frames at specified intervals (e.g., 30 FPS)
- Generate video thumbnails
- Add video metadata extraction (resolution, duration, codec)

**Task 1.4: Database Schema**
```sql
CREATE TABLE vision_analyses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    video_url TEXT NOT NULL,
    video_duration_seconds DOUBLE PRECISION,
    status VARCHAR(20) NOT NULL, -- uploaded, processing, completed, failed
    exercise_type VARCHAR(50), -- squat, deadlift, running, etc.
    upload_timestamp TIMESTAMPTZ DEFAULT NOW(),
    processing_started_at TIMESTAMPTZ,
    processing_completed_at TIMESTAMPTZ,
    metadata JSONB
);

CREATE TABLE pose_detections (
    id BIGSERIAL PRIMARY KEY,
    analysis_id UUID NOT NULL REFERENCES vision_analyses(id),
    frame_number INTEGER NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    keypoints JSONB NOT NULL, -- Array of {x, y, confidence, joint_name}
    confidence_score DOUBLE PRECISION,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE movement_scores (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    analysis_id UUID NOT NULL REFERENCES vision_analyses(id),
    overall_score DOUBLE PRECISION NOT NULL, -- 0-100
    form_quality DOUBLE PRECISION,
    injury_risk DOUBLE PRECISION,
    range_of_motion DOUBLE PRECISION,
    tempo_consistency DOUBLE PRECISION,
    issues JSONB, -- Array of detected issues
    recommendations JSONB, -- Array of improvement suggestions
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_vision_analyses_user ON vision_analyses(user_id, upload_timestamp);
CREATE INDEX idx_pose_detections_analysis ON pose_detections(analysis_id, frame_number);
```

### Phase 2: Pose Estimation Engine (Week 3-5)
**Task 2.1: Model Selection & Integration**
- Research pose estimation models:
  - MediaPipe Pose (Google) - lightweight, real-time
  - OpenPose - high accuracy, heavier
  - MoveNet (TensorFlow) - balance of speed/accuracy
- Choose ONNX-compatible model for Rust integration
- Download and convert model to ONNX format
- Create model versioning system

**Task 2.2: Inference Service**
- Create `PoseEstimationService` using `tract` ONNX runtime
- Implement frame-by-frame pose detection
- Extract 2D/3D keypoints (33 body landmarks)
- Calculate keypoint confidence scores
- Implement batched inference for efficiency
- Add GPU acceleration support (if available)

**Task 2.3: Keypoint Processing**
- Normalize keypoints to consistent coordinate system
- Implement temporal smoothing for jittery detections
- Calculate joint angles from keypoints
- Detect pose landmarks:
  - Shoulders, elbows, wrists
  - Hips, knees, ankles
  - Spine alignment
  - Head position

**Task 2.4: Background Processing**
- Implement async job queue for video processing
- Create worker service for processing videos
- Add retry logic for failed analyses
- Implement progress tracking
- Send notifications on completion/failure

### Phase 3: Movement Analysis (Week 6-8)
**Task 3.1: Exercise-Specific Analysis**
- Implement analyzers for common exercises:
  - **Squat**: Depth, knee tracking, back angle
  - **Deadlift**: Back rounding, bar path, hip hinge
  - **Running**: Cadence, stride length, foot strike
  - **Push-up**: Elbow angle, body alignment, depth
- Create `MovementAnalyzer` trait for extensibility
- Calculate exercise-specific metrics

**Task 3.2: Form Quality Scoring**
- Define quality criteria per exercise type
- Implement scoring algorithm (0-100 scale):
  - Range of motion (ROM)
  - Joint alignment
  - Movement symmetry
  - Tempo consistency
- Weight different criteria by importance
- Generate frame-by-frame quality scores

**Task 3.3: Injury Risk Assessment**
- Detect dangerous movement patterns:
  - Knee valgus (inward collapse)
  - Excessive spinal flexion
  - Asymmetric loading
  - Hyperextension
- Calculate injury risk score per frame
- Identify highest risk moments in video
- Prioritize issues by severity

**Task 3.4: Biomechanics Calculations**
- Calculate joint angles over time
- Detect range of motion limitations
- Identify compensatory patterns
- Track movement velocity and acceleration
- Analyze left/right symmetry

### Phase 4: Feedback Generation (Week 9)
**Task 4.1: Issue Detection**
- Create rule-based system for common issues:
  - "Knees caving inward during squat"
  - "Lower back rounding in deadlift"
  - "Overstriding while running"
- Assign severity levels (critical, warning, minor)
- Include timestamp of issue occurrence
- Add visual markers on problematic frames

**Task 4.2: Recommendation Engine**
- Generate actionable recommendations:
  - Mobility exercises for ROM issues
  - Cueing for form corrections
  - Load reduction suggestions
- Prioritize recommendations by impact
- Link to educational content/videos
- Provide progressive correction plan

**Task 4.3: Visualization API**
- Create `GET /api/v1/vision/{id}/overlay` endpoint
- Generate annotated video with skeleton overlay
- Add form quality heatmap on timeline
- Include issue markers and text overlays
- Return processed video or frame images

### Phase 5: Exercise Classification (Week 10-11)
**Task 5.1: Exercise Type Detection**
- Train/integrate classifier to detect exercise type:
  - Squat vs. deadlift vs. lunge
  - Running vs. cycling
  - Upper body vs. lower body
- Use pose sequence patterns for classification
- Implement confidence scoring
- Support manual override/correction

**Task 5.2: Rep Counting**
- Detect repetition cycles automatically
- Count reps based on movement patterns
- Calculate tempo (eccentric/concentric duration)
- Detect incomplete reps
- Track rest periods between sets

**Task 5.3: Auto-Tagging**
- Extract metadata from video analysis:
  - Exercise type
  - Rep count
  - Quality score
  - Issues detected
- Auto-populate training session data
- Link to existing training log

### Phase 6: API & Client Integration (Week 12)
**Task 6.1: REST API Completion**
- `POST /api/v1/vision/upload` - Upload video
- `GET /api/v1/vision/{id}` - Get analysis results
- `GET /api/v1/vision/{id}/overlay` - Get annotated video
- `DELETE /api/v1/vision/{id}` - Delete analysis
- `GET /api/v1/vision/history` - User's analysis history
- `POST /api/v1/vision/{id}/feedback` - Submit user feedback

**Task 6.2: Webhook Integration**
- Send webhook on analysis completion
- Include summary data in webhook payload
- Support retry logic for failed webhooks
- Add webhook signature verification

**Task 6.3: Documentation**
- API documentation with examples
- Supported exercises and analysis capabilities
- Video requirements (format, quality, angle)
- Integration guide for mobile apps
- Best practices for recording videos

### Phase 7: Testing & Optimization (Week 13-14)
**Task 7.1: Model Accuracy Testing**
- Collect test dataset of labeled videos
- Measure pose estimation accuracy
- Validate form scoring against expert coaches
- Test across different body types and lighting
- Optimize model thresholds

**Task 7.2: Performance Optimization**
- Benchmark inference speed (target <2x real-time)
- Optimize frame sampling rate
- Implement model quantization for speed
- Add GPU acceleration
- Test with various video resolutions

**Task 7.3: Comprehensive Testing**
- Unit tests for pose processing logic
- Integration tests for upload → analysis flow
- Load tests for concurrent video processing
- Test edge cases (poor lighting, partial body, obstruction)
- Security testing for file uploads

## API Endpoints

### Video Analysis
- `POST /api/v1/vision/upload` - Upload video for analysis
- `GET /api/v1/vision/{id}` - Get analysis results
- `GET /api/v1/vision/{id}/status` - Check processing status
- `GET /api/v1/vision/{id}/overlay` - Get annotated video
- `DELETE /api/v1/vision/{id}` - Delete analysis
- `GET /api/v1/vision/history` - List user's analyses
- `PATCH /api/v1/vision/{id}` - Update analysis metadata

### Exercise Library
- `GET /api/v1/vision/exercises` - List supported exercises
- `GET /api/v1/vision/exercises/{type}` - Get exercise details

## Response Schema Examples

```json
// Analysis Result
{
  "id": "uuid",
  "user_id": "uuid",
  "video_url": "https://...",
  "status": "completed",
  "exercise_type": "squat",
  "duration_seconds": 15.5,
  "processing_time_seconds": 8.2,
  "scores": {
    "overall": 78.5,
    "form_quality": 82.0,
    "injury_risk": 25.0,
    "range_of_motion": 75.0,
    "tempo_consistency": 88.0
  },
  "rep_count": 10,
  "issues": [
    {
      "severity": "warning",
      "type": "knee_valgus",
      "description": "Knees caving inward during descent",
      "frames": [45, 78, 112],
      "confidence": 0.85
    }
  ],
  "recommendations": [
    {
      "priority": "high",
      "issue": "knee_valgus",
      "suggestion": "Focus on pushing knees outward during descent",
      "exercises": ["banded_squats", "clamshells"],
      "cue": "Think 'knees out' at the bottom position"
    }
  ],
  "overlay_url": "https://.../overlay.mp4",
  "keypoints_data_url": "https://.../keypoints.json"
}
```

## Supported Exercises (Initial)
1. **Squat** - Depth, knee tracking, back angle
2. **Deadlift** - Hip hinge, back position, bar path
3. **Push-up** - Elbow angle, body alignment, depth
4. **Running** - Cadence, stride, foot strike pattern
5. **Plank** - Body alignment, hip position

## Success Metrics
- Pose detection accuracy >90% (vs. human annotations)
- Form scoring correlation >0.8 with expert coaches
- Processing time <2x video duration
- Issue detection precision >85%
- User satisfaction score >4.0/5.0

## Dependencies
- FFmpeg installation
- ONNX runtime (`tract` or `burn`)
- Object storage (MinIO/S3)
- Existing user authentication
- Background job queue

## Risks & Mitigations
- **Risk**: Model accuracy varies with video quality
  - **Mitigation**: Provide recording guidelines, validate video quality on upload
- **Risk**: High computational cost
  - **Mitigation**: GPU acceleration, optimized models, usage quotas
- **Risk**: Privacy concerns with video data
  - **Mitigation**: Encryption at rest/transit, auto-deletion policies, user consent
- **Risk**: Limited exercise library
  - **Mitigation**: Start with 5 common exercises, expand based on demand

## Future Enhancements
- Real-time analysis during live workouts
- 3D pose estimation for depth accuracy
- Multi-person analysis for group training
- AR overlay for mobile app
- Coach annotation tools
- Comparative analysis (before/after)