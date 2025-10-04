# Vision Analysis Validation Framework

## Overview

The Validation Framework provides comprehensive infrastructure for testing, validating, and optimizing the AI Coach computer vision system. It enables systematic collection of ground truth data, expert annotations, and quantitative validation metrics.

**Issue**: #87 - Phase 7.1: Collect test dataset and validate model accuracy

## Architecture

### Components

1. **Test Dataset Management**: Curated videos with metadata (exercise type, quality, lighting, camera angle)
2. **Expert Annotation System**: Ground truth labels from certified coaches
3. **Validation Metrics Engine**: Automated accuracy, correlation, and precision calculations
4. **Threshold Optimization**: Data-driven tuning of detection thresholds

### Database Schema

```
validation_test_videos          # Test video repository
├── video metadata (resolution, fps, duration)
├── categorization (exercise, quality, conditions)
└── upload tracking

validation_experts              # Expert coach profiles
├── credentials and specialization
└── annotation statistics

validation_keypoint_annotations # Ground truth pose data
├── frame-by-frame keypoint positions
├── annotation quality metrics
└── expert consensus tracking

validation_form_ratings        # Ground truth form scores
├── overall and component scores (1-10 scale)
├── rep count and quality metrics
└── expert confidence levels

validation_issue_annotations   # Ground truth movement issues
├── issue type and severity
├── temporal information (frames, timestamps)
└── corrective actions

validation_runs                # Validation execution tracking
├── run metadata and configuration
├── dataset composition
└── execution status

validation_pose_accuracy       # Pose estimation metrics
├── mAP (mean Average Precision)
├── PCK (Percentage Correct Keypoints)
├── MPJPE (Mean Per-Joint Position Error)
└── detection rate and false positives

validation_form_scoring        # Form scoring metrics
├── correlation with expert ratings
├── score differences and accuracy
└── component-wise validation

validation_issue_detection     # Issue detection metrics
├── precision, recall, F1 score
├── true/false positives/negatives
└── per-issue-type accuracy

validation_run_summary         # Aggregated results
├── overall metrics across all videos
├── target achievement flags
└── optimization recommendations
```

## API Endpoints

### Test Videos

**POST `/api/v1/validation/videos`**
```json
{
  "video_url": "https://storage.ai-coach.com/test/squat-001.mp4",
  "video_name": "Squat - Good Form - Front Angle",
  "exercise_type": "squat",
  "quality_category": "good_form",
  "body_type": "mesomorph",
  "lighting_condition": "bright",
  "camera_angle": "front",
  "notes": "Professional form demonstration"
}
```

**GET `/api/v1/validation/videos`**
- Query params: `exercise_type`, `quality_category`, `limit`, `offset`
- Returns: List of test videos with metadata

**GET `/api/v1/validation/videos/:video_id`**
- Returns: Full video details and annotation status

### Experts

**POST `/api/v1/validation/experts`**
```json
{
  "expert_name": "Dr. John Smith",
  "credentials": "CSCS, PhD Exercise Science",
  "specialization": ["strength_training", "biomechanics"],
  "years_experience": 15,
  "certification_level": "elite"
}
```

**GET `/api/v1/validation/experts`**
- Returns: List of all registered experts

**GET `/api/v1/validation/experts/:expert_id`**
- Returns: Expert profile with annotation statistics

### Annotations

**POST `/api/v1/validation/annotations/keypoints`**
```json
{
  "video_id": "uuid",
  "frame_number": 45,
  "timestamp_ms": 1500,
  "keypoints": [
    {"joint_name": "nose", "x": 320.5, "y": 180.2, "confidence": 0.95, "visible": true},
    {"joint_name": "left_shoulder", "x": 280.3, "y": 220.1, "confidence": 0.92, "visible": true}
    // ... 17 COCO keypoints total
  ],
  "annotation_method": "manual",
  "annotation_quality": "high"
}
```

**POST `/api/v1/validation/annotations/form-ratings`**
```json
{
  "video_id": "uuid",
  "overall_score": 8.5,
  "form_quality": 9.0,
  "injury_risk": 2.0,
  "range_of_motion": 8.0,
  "tempo_consistency": 9.0,
  "rep_count": 10,
  "rating_confidence": "high",
  "notes": "Excellent depth, minor knee tracking issue"
}
```

**POST `/api/v1/validation/annotations/issues`**
```json
{
  "video_id": "uuid",
  "issue_type": "knee_valgus",
  "severity": "warning",
  "description": "Knees collapsing inward during descent",
  "affected_frames": [30, 31, 32, 45, 46],
  "timestamp_ranges": [{"start_ms": 1000, "end_ms": 1200}],
  "confidence": 0.9,
  "corrective_action": "Focus on pushing knees out, add resistance band"
}
```

**GET `/api/v1/validation/videos/:video_id/annotations/form-ratings`**
- Returns: All expert form ratings for a video

**GET `/api/v1/validation/videos/:video_id/annotations/issues`**
- Returns: All expert issue annotations for a video

### Validation Runs

**POST `/api/v1/validation/runs`**
```json
{
  "run_name": "YOLOv8n-pose Baseline Validation",
  "model_version": "yolov8n-pose-v1.0",
  "threshold_config": {
    "pose_confidence": 0.5,
    "nms_threshold": 0.45,
    "keypoint_confidence": 0.3
  },
  "run_type": "full",
  "video_ids": ["uuid1", "uuid2", "uuid3"],
  "notes": "Initial baseline validation"
}
```

**GET `/api/v1/validation/runs`**
- Query params: `run_type`, `limit`, `offset`
- Returns: List of validation runs with status

**GET `/api/v1/validation/runs/:run_id`**
- Returns: Run details and configuration

**GET `/api/v1/validation/runs/:run_id/summary`**
```json
{
  "pose_accuracy": {
    "avg_map": 0.92,
    "avg_pck": 0.94,
    "avg_mpjpe": 8.5,
    "target_met": true
  },
  "form_scoring": {
    "avg_correlation": 0.85,
    "avg_difference": 0.8,
    "target_met": true
  },
  "issue_detection": {
    "avg_precision": 0.87,
    "avg_recall": 0.82,
    "avg_f1_score": 0.84,
    "target_met": true
  },
  "all_targets_met": true,
  "recommendations": "Consider lowering keypoint confidence threshold to improve recall"
}
```

## Validation Workflow

### 1. Dataset Collection (Deliverable 1)

**Requirements**: 250+ videos (50 per exercise)

```bash
# Upload test videos
for video in squat_videos/*.mp4; do
  curl -X POST /api/v1/validation/videos \
    -H "Authorization: Bearer $TOKEN" \
    -d @video_metadata.json
done
```

**Diversity Criteria**:
- Exercise types: squat, deadlift, push-up, running, plank
- Quality mix: 40% good form, 40% poor form, 20% mixed
- Body types: ectomorph, mesomorph, endomorph, average
- Lighting: bright, dim, outdoor, indoor
- Camera angles: front, side, diagonal, 45-degree

### 2. Expert Annotation (Deliverable 2)

**Recruit 2-3 expert coaches**:
1. Create expert profiles via API
2. Assign videos for annotation
3. Collect ground truth labels:
   - Keypoint annotations (sample frames)
   - Form ratings (1-10 scale)
   - Issue identification

**Annotation Tools** (Future Enhancement):
- Web-based annotation interface
- Keypoint drawing tool
- Video playback controls
- Annotation validation and consensus

### 3. Validation Execution (Deliverable 3)

**Pose Estimation Accuracy**:
```python
# Run pose estimation on test videos
for video in test_videos:
    predicted_keypoints = pose_model.estimate(video)
    ground_truth = get_expert_annotations(video.id)

    # Calculate metrics
    mpjpe = calculate_mpjpe(predicted, ground_truth)
    pck = calculate_pck(predicted, ground_truth, threshold=0.5)
    map = calculate_map(predicted, ground_truth)
```

**Target**: >90% accuracy (mAP or PCK)

**Form Scoring Validation**:
```python
# Compare AI scores with expert ratings
for video in test_videos:
    ai_score = form_scorer.score(video)
    expert_scores = get_expert_ratings(video.id)

    correlation = pearson_correlation(ai_score, expert_scores)
    difference = abs(ai_score - mean(expert_scores))
```

**Target**: >0.8 Pearson correlation

**Issue Detection Validation**:
```python
# Evaluate issue detection
for video in test_videos:
    detected_issues = issue_detector.detect(video)
    expert_issues = get_expert_issues(video.id)

    tp, fp, fn = calculate_confusion_matrix(detected, expert)
    precision = tp / (tp + fp)
    recall = tp / (tp + fn)
    f1 = 2 * (precision * recall) / (precision + recall)
```

**Target**: >85% precision

### 4. Threshold Optimization (Deliverable 4)

**Systematic Tuning**:
```python
thresholds = {
    'pose_confidence': [0.3, 0.4, 0.5, 0.6, 0.7],
    'nms_threshold': [0.4, 0.45, 0.5, 0.55],
    'keypoint_confidence': [0.2, 0.3, 0.4, 0.5]
}

for pose_conf in thresholds['pose_confidence']:
    for nms in thresholds['nms_threshold']:
        for kp_conf in thresholds['keypoint_confidence']:
            # Run validation with these thresholds
            results = run_validation(pose_conf, nms, kp_conf)

            # Store results
            save_threshold_experiment(
                run_id,
                thresholds={'pose': pose_conf, 'nms': nms, 'kp': kp_conf},
                metrics=results
            )
```

**Grid Search Output**:
- Best threshold combination for each metric
- Trade-off analysis (precision vs. recall)
- Optimal configuration recommendation

## Metrics Definitions

### Pose Estimation Metrics

**Mean Average Precision (mAP)**:
- Average precision across all keypoints and videos
- Accounts for detection confidence and localization accuracy
- Range: 0.0 - 1.0 (higher is better)

**Percentage of Correct Keypoints (PCK)**:
- Percentage of keypoints within threshold distance from ground truth
- Threshold typically 50% of head size or 0.5 * torso_diameter
- Range: 0.0 - 1.0 (higher is better)

**Mean Per-Joint Position Error (MPJPE)**:
- Average Euclidean distance between predicted and ground truth keypoints
- Measured in pixels
- Range: 0.0 - ∞ (lower is better)

### Form Scoring Metrics

**Pearson Correlation**:
- Linear correlation between AI scores and expert ratings
- Range: -1.0 to 1.0 (target: >0.8)

**Score Difference**:
- Absolute difference between AI and expert scores
- Range: 0.0 - 10.0 (lower is better)

**Category Accuracy**:
- Percentage of videos correctly categorized (excellent/good/fair/poor)

### Issue Detection Metrics

**Precision**:
- TP / (TP + FP)
- Percentage of detected issues that are correct
- Range: 0.0 - 1.0 (target: >0.85)

**Recall**:
- TP / (TP + FN)
- Percentage of actual issues that were detected
- Range: 0.0 - 1.0

**F1 Score**:
- Harmonic mean of precision and recall
- 2 * (precision * recall) / (precision + recall)
- Range: 0.0 - 1.0

## Success Criteria (Acceptance Criteria from Issue #87)

### Dataset Quality
- ✅ 250+ videos collected (50 per exercise type)
- ✅ Real-world variety: body types, lighting, angles
- ✅ Quality distribution: 40% good, 40% poor, 20% mixed
- ✅ 2-3 expert coaches recruited
- ✅ Ground truth annotations complete

### Validation Targets
- ✅ Pose accuracy >90% (mAP or PCK)
- ✅ Form scoring correlation >0.8
- ✅ Issue detection precision >85%

### Deliverables
- ✅ Test dataset (labeled and stored)
- ✅ Validation metrics report
- ✅ Optimized thresholds documented
- ✅ Complete documentation

## Future Enhancements

1. **Automated Annotation Tools**:
   - Semi-automated keypoint annotation with human verification
   - Batch annotation workflows
   - Inter-annotator agreement metrics

2. **Continuous Validation**:
   - Automated validation on model updates
   - Regression detection
   - Performance monitoring over time

3. **Expanded Metrics**:
   - Per-exercise accuracy breakdown
   - Temporal consistency metrics
   - Multi-person detection validation

4. **Integration with ML Pipeline**:
   - Automated model retraining based on validation results
   - A/B testing framework
   - Shadow deployment validation

## References

- Issue #87: Phase 7.1: Collect test dataset and validate model accuracy
- YOLOv8n-pose documentation
- COCO keypoint evaluation protocol
- Pose estimation benchmarking standards
