# Pose Estimation Model Selection

**Date**: 2025-10-02
**Version**: 1.0
**Author**: AI Coach Development Team

## Executive Summary

After comprehensive evaluation of available ONNX-compatible pose estimation models, **YOLOv8-pose (nano variant)** has been selected as the optimal solution for the AI Coach platform. This decision balances accuracy, inference speed, model size, and ease of integration with the Rust backend.

## Models Evaluated

### 1. MediaPipe Pose (Google)
- **Architecture**: CNN-based with BlazePose
- **Keypoints**: 33 body landmarks (superset of COCO 17)
- **Accuracy**: High (real-time capable)
- **Inference Speed**: 30 FPS on CPU
- **Model Size**: ~3-5 MB
- **ONNX Support**: ⚠️ Limited - primarily TFLite/TF.js
- **License**: Apache 2.0 (✅ Commercial friendly)

**Pros**:
- Excellent accuracy with detailed 33-point tracking
- Optimized for mobile and edge devices
- Strong performance on single-person scenarios
- Battle-tested in production (Google Fit, etc.)

**Cons**:
- ONNX conversion requires additional tooling (tf2onnx)
- Primarily designed for TensorFlow ecosystem
- Single-person detection only
- Less active ONNX community support

### 2. MoveNet (Google/TensorFlow)
- **Architecture**: MobileNetV2 backbone
- **Keypoints**: 17 COCO keypoints
- **Variants**:
  - Lightning (speed-optimized)
  - Thunder (accuracy-optimized)
- **Accuracy**: 75-81% (Lightning: 75.1%, Thunder: 80.6%)
- **Inference Speed**: Lightning fastest among traditional models
- **Model Size**: 5-7 MB (Lightning), 12-16 MB (Thunder)
- **ONNX Support**: ⚠️ Requires conversion (TFLite → ONNX)
- **License**: Apache 2.0 (✅ Commercial friendly)

**Pros**:
- Extremely fast (Lightning variant)
- Designed for resource-constrained devices
- Good accuracy/speed tradeoff
- Robust to various body types

**Cons**:
- Single-person detection only
- ONNX conversion adds complexity
- Fewer keypoints than MediaPipe (17 vs 33)
- Primarily TensorFlow ecosystem

### 3. OpenPose (CMU)
- **Architecture**: VGG-19 backbone with PAFs (Part Affinity Fields)
- **Keypoints**: 18 landmarks (full body)
- **Accuracy**: 86.2% (highest among traditional models)
- **Inference Speed**: ⚠️ Slowest (not real-time on CPU)
- **Model Size**: ~200+ MB
- **ONNX Support**: ✅ Available but complex
- **License**: ⚠️ **Academic/Non-profit only** ($25K/year commercial)

**Pros**:
- Highest accuracy
- Multi-person detection
- Detailed full-body tracking
- Includes face and hand keypoints

**Cons**:
- **Prohibitive commercial license** ($25,000/year)
- Very large model size
- Slowest inference speed
- High computational requirements
- Not suitable for real-time applications

### 4. YOLOv8-pose (Ultralytics)
- **Architecture**: Single-stage, anchor-free, heatmap-free
- **Keypoints**: 17 COCO keypoints
- **Variants**: nano (n), small (s), medium (m), large (l), extra-large (x)
- **Accuracy**: High (state-of-the-art for real-time)
- **Inference Speed**: ✅ Real-time on CPU/GPU
- **Model Size**:
  - Nano: ~6 MB
  - Small: ~11 MB
  - Medium: ~26 MB
  - Large: ~52 MB
  - XL: ~90 MB
- **ONNX Support**: ✅ **Native export** via `model.export(format='onnx')`
- **License**: AGPL-3.0 (⚠️ with commercial options available)

**Pros**:
- **Modern architecture** (2023+, actively maintained)
- **Native ONNX export** - one command
- Multi-person detection
- Excellent speed/accuracy balance
- Multiple model sizes for different use cases
- Strong Rust ecosystem support (ort crate examples)
- Trained on COCO dataset (standard benchmark)
- Automatic model downloads

**Cons**:
- AGPL-3.0 requires open-source or commercial license
- Fewer keypoints than MediaPipe (17 vs 33)

## Comparison Matrix

| Criteria | MediaPipe | MoveNet | OpenPose | **YOLOv8-pose** |
|----------|-----------|---------|----------|-----------------|
| **Accuracy** | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Speed (CPU)** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Model Size** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐ | ⭐⭐⭐⭐⭐ |
| **ONNX Native** | ⭐⭐ | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Rust Support** | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Multi-person** | ❌ | ❌ | ✅ | ✅ |
| **Commercial License** | ✅ Free | ✅ Free | ❌ $25K/year | ⚠️ AGPL/Commercial |
| **Keypoints** | 33 | 17 | 18 | 17 |
| **Active Development** | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ |

## Decision: YOLOv8-pose (nano)

### Selection Rationale

**Primary Reasons**:

1. **Native ONNX Support**: One-command export with no conversion complexity
   ```python
   model = YOLO('yolov8n-pose.pt')
   model.export(format='onnx')
   ```

2. **Modern Architecture**: Latest advancements in pose estimation (2023-2024)
   - Anchor-free design
   - Heatmap-free detection
   - Single-stage inference

3. **Excellent Rust Ecosystem**:
   - Proven examples with `ort` crate (ONNX Runtime wrapper)
   - Active community support
   - Multiple production implementations

4. **Multi-person Detection**: Critical for gym/training scenarios
   - Detect multiple athletes simultaneously
   - Scalable to group training sessions

5. **Optimal Speed/Accuracy Balance**:
   - Real-time inference on modern CPUs
   - GPU acceleration when available
   - Nano variant: <100ms per frame target achievable

6. **Model Size Flexibility**:
   - Start with nano (6 MB) for development
   - Scale up to larger variants if accuracy requires
   - Easy A/B testing between model sizes

7. **Production-Ready**:
   - Battle-tested in various applications
   - Comprehensive documentation
   - Regular updates and bug fixes

### Trade-offs Accepted

1. **License Consideration**: AGPL-3.0 requires either:
   - Open-source the AI Coach platform (aligns with our goals)
   - Purchase Ultralytics commercial license (future option)
   - For MVP/demo phase, AGPL is acceptable

2. **Keypoint Count**: 17 COCO keypoints vs 33 (MediaPipe)
   - COCO 17 covers all essential athletic tracking points
   - Standard benchmark makes integration easier
   - Can augment with additional models later if needed

3. **Dependency on Ultralytics**: Mitigated by:
   - ONNX model is portable
   - Active project with strong community
   - Can retrain or fine-tune if needed

### ONNX Runtime Choice: `ort` vs `tract`

**Selected**: `ort` (ONNX Runtime wrapper)

**Rationale**:
- More mature ONNX support (Microsoft's official runtime)
- Proven YOLOv8 pose examples in Rust ecosystem
- Better performance benchmarks
- GPU acceleration support
- Active maintenance and updates
- `tract` is excellent but less community support for pose models

## Implementation Plan

### Phase 1: Model Acquisition (Current)
- [x] Download YOLOv8n-pose PyTorch model
- [x] Export to ONNX format
- [ ] Store in `models/pose_v1.onnx`
- [ ] Create model metadata file

### Phase 2: Integration (Issue #64 - Next)
- [ ] Add `ort` dependency to Cargo.toml
- [ ] Implement basic ONNX model loading
- [ ] Verify inference on test image
- [ ] Extract 17 COCO keypoints
- [ ] Calculate confidence scores

### Phase 3: Optimization (Future)
- [ ] Benchmark inference speed
- [ ] Test GPU acceleration
- [ ] Evaluate model size upgrade if accuracy insufficient
- [ ] Implement temporal smoothing
- [ ] Multi-person tracking logic

## Model Metadata

### YOLOv8n-pose ONNX Model

**File**: `models/pose_v1.onnx`
**Version**: v1.0
**Source**: Ultralytics YOLOv8n-pose
**Size**: ~6 MB
**Format**: ONNX (opset 11+)

**Input**:
- **Name**: `images`
- **Shape**: `[1, 3, 640, 640]`
- **Type**: FP32
- **Format**: RGB
- **Normalization**: [0.0, 1.0] (divide by 255)
- **Preprocessing**:
  - Resize to 640x640 (letterbox padding)
  - RGB color space
  - Normalize to [0, 1]

**Output**:
- **Name**: `output0`
- **Shape**: `[1, 56, 8400]`
- **Type**: FP32
- **Format**:
  - Dimensions: `[batch, attributes, anchors]`
  - Attributes (56): `[x, y, w, h, confidence, kp1_x, kp1_y, kp1_conf, ..., kp17_x, kp17_y, kp17_conf]`
  - Anchors (8400): Detection grid positions

**Keypoints** (17 COCO format):
1. Nose
2. Left Eye
3. Right Eye
4. Left Ear
5. Right Ear
6. Left Shoulder
7. Right Shoulder
8. Left Elbow
9. Right Elbow
10. Left Wrist
11. Right Wrist
12. Left Hip
13. Right Hip
14. Left Knee
15. Right Knee
16. Left Ankle
17. Right Ankle

**Confidence Threshold**: 0.5 (recommended)
**NMS Threshold**: 0.45 (Non-Maximum Suppression)

## Model Versioning System

### Directory Structure
```
models/
├── pose_v1.onnx          # YOLOv8n-pose (nano)
├── pose_v1.metadata.json # Model metadata
└── README.md             # Model documentation
```

### Versioning Scheme

**Format**: `pose_v{MAJOR}.{MINOR}.onnx`

- **MAJOR**: Model architecture change (e.g., YOLOv8 → YOLOv9)
- **MINOR**: Model size variant or training data update

**Examples**:
- `pose_v1.0.onnx` - YOLOv8n-pose (initial)
- `pose_v1.1.onnx` - YOLOv8s-pose (small variant upgrade)
- `pose_v1.2.onnx` - YOLOv8n-pose (retrained on custom data)
- `pose_v2.0.onnx` - YOLOv9-pose (architecture upgrade)

### Storage Strategy

**Current (MVP)**: Git LFS (Large File Storage)
- Models stored in repository
- Version controlled
- Easy CI/CD integration

**Future (Production)**: S3/Object Storage
- Reduce repository size
- Faster downloads
- CDN distribution
- Model versioning via metadata

## Testing Strategy

### Unit Tests
- [x] Model file exists and loads
- [ ] Input shape validation
- [ ] Output shape validation
- [ ] Inference runs without errors

### Integration Tests
- [ ] End-to-end inference on test images
- [ ] Keypoint extraction accuracy
- [ ] Confidence score validation
- [ ] Multi-person detection

### Performance Tests
- [ ] Inference speed benchmarks (<100ms target)
- [ ] Memory usage profiling
- [ ] Batch processing efficiency
- [ ] GPU vs CPU comparison

### Acceptance Tests
- [ ] Works on various video qualities
- [ ] Handles different lighting conditions
- [ ] Accurate on different body types
- [ ] Robust to camera angles

## References

1. [Ultralytics YOLOv8 Pose Documentation](https://docs.ultralytics.com/tasks/pose/)
2. [ONNX Runtime Rust Bindings (ort)](https://github.com/pykeio/ort)
3. [YOLOv8 Rust Examples](https://github.com/ultralytics/ultralytics/issues/6580)
4. [COCO Keypoint Format](https://cocodataset.org/#keypoints-2020)
5. MediaPipe Pose: https://developers.google.com/mediapipe/solutions/vision/pose_landmarker
6. MoveNet: https://www.tensorflow.org/hub/tutorials/movenet
7. OpenPose: https://github.com/CMU-Perceptual-Computing-Lab/openpose

## Appendix A: Alternative Models Considered

If YOLOv8-pose proves insufficient, the following alternatives are recommended:

1. **YOLOv8s-pose** (small variant): Better accuracy, slightly slower
2. **MediaPipe Pose**: If 33 keypoints required, accept TFLite conversion
3. **MoveNet Thunder**: If single-person detection is sufficient

## Appendix B: Model Export Commands

### Export YOLOv8n-pose to ONNX

**Python**:
```python
from ultralytics import YOLO

# Load model
model = YOLO('yolov8n-pose.pt')

# Export to ONNX
model.export(format='onnx', opset=11, simplify=True)
```

**CLI**:
```bash
pip install ultralytics
yolo export model=yolov8n-pose.pt format=onnx opset=11 simplify=True
```

### Verify ONNX Model

```python
import onnx

# Load and check model
model = onnx.load('yolov8n-pose.onnx')
onnx.checker.check_model(model)

# Print model info
print(onnx.helper.printable_graph(model.graph))
```

## Change Log

- **2025-10-02**: Initial model selection document created
- Selected YOLOv8n-pose as primary model
- Defined versioning system and storage strategy
