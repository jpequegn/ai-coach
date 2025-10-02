# AI Coach - Pose Estimation Models

This directory contains ONNX models used for human pose estimation in the AI Coach platform.

## Current Models

### pose_v1.onnx
- **Model**: YOLOv8n-pose (nano variant)
- **Version**: 1.0
- **Size**: 12.89 MB
- **Format**: ONNX (opset 11)
- **Task**: Human pose estimation with 17 COCO keypoints
- **Capabilities**: Multi-person detection, real-time inference
- **License**: AGPL-3.0 (Ultralytics)

**Metadata**: See `pose_v1.metadata.json` for detailed specifications including:
- Input/output shapes and preprocessing requirements
- 17 COCO keypoint layout and skeleton connections
- Performance benchmarks and inference parameters
- Training information and runtime support

**Documentation**: See `../docs/models/pose-model-selection.md` for:
- Model selection rationale
- Comparison with alternatives (MediaPipe, MoveNet, OpenPose)
- Implementation guidelines
- Testing strategy

## Model Versioning

Models follow the naming convention: `pose_v{MAJOR}.{MINOR}.onnx`

- **MAJOR**: Model architecture change (e.g., YOLOv8 → YOLOv9)
- **MINOR**: Model size variant or training data update

### Planned Versions
- `pose_v1.0.onnx` - YOLOv8n-pose (current)
- `pose_v1.1.onnx` - YOLOv8s-pose (small variant, if accuracy requires upgrade)
- `pose_v1.2.onnx` - YOLOv8n-pose (fine-tuned on athletic training data)

## Usage

### Loading with ONNX Runtime (Rust)

```rust
use ort::{Environment, SessionBuilder};

// Create ONNX Runtime environment
let environment = Environment::builder()
    .with_name("ai-coach-pose")
    .build()?;

// Load model
let session = SessionBuilder::new(&environment)?
    .with_model_from_file("models/pose_v1.onnx")?;

// Run inference (see PoseEstimationService for full implementation)
```

### Preprocessing Requirements

1. **Resize**: Letterbox resize to 640x640 (maintain aspect ratio)
2. **Color Space**: Convert BGR → RGB (if using OpenCV)
3. **Normalize**: Divide pixel values by 255.0 → [0.0, 1.0]
4. **Format**: Transpose to NCHW (channels-first)
5. **Batch**: Add batch dimension [1, 3, 640, 640]

### Postprocessing Pipeline

1. **Transpose**: Output [1, 56, 8400] → [8400, 56]
2. **Filter**: Apply confidence threshold (default: 0.5)
3. **NMS**: Non-Maximum Suppression (IoU threshold: 0.45)
4. **Extract**: Parse keypoints (17 × 3 values per detection)
5. **Denormalize**: Convert normalized coords to original image size

## Model Downloads

Models are stored in this repository using Git LFS (Large File Storage). If you need to re-download:

```bash
# Install Git LFS (if not already installed)
git lfs install

# Pull models
git lfs pull

# Or download directly from source
python3 -c "
from ultralytics import YOLO
model = YOLO('yolov8n-pose.pt')
model.export(format='onnx', opset=11, simplify=True)
"
```

## Performance Expectations

| Metric | Value |
|--------|-------|
| Inference Speed (CPU) | ~50ms per frame |
| Inference Speed (GPU) | ~10ms per frame |
| Memory Usage | ~200 MB |
| VRAM Usage (GPU) | ~300 MB |
| Target FPS | 20+ (real-time capable) |
| Multi-person Support | Yes (10+ persons) |

## COCO Keypoints (17)

The model detects 17 body keypoints in COCO format:

**Facial** (5):
- 0: Nose
- 1: Left Eye
- 2: Right Eye
- 3: Left Ear
- 4: Right Ear

**Upper Body** (6):
- 5: Left Shoulder
- 6: Right Shoulder
- 7: Left Elbow
- 8: Right Elbow
- 9: Left Wrist
- 10: Right Wrist

**Lower Body** (6):
- 11: Left Hip
- 12: Right Hip
- 13: Left Knee
- 14: Right Knee
- 15: Left Ankle
- 16: Right Ankle

## Future Models

### Potential Upgrades
- **YOLOv8s-pose**: Small variant for better accuracy (11 MB)
- **YOLOv8m-pose**: Medium variant for highest accuracy (26 MB)
- **Custom Fine-tuned**: Trained on athletic/gym training footage

### Alternative Models (Research)
- **MediaPipe Pose**: 33 keypoints (if detailed tracking needed)
- **MoveNet Thunder**: High accuracy single-person (if multi-person not required)

## License Notes

**YOLOv8-pose License**: AGPL-3.0

This requires either:
1. **Open-source** the AI Coach platform under AGPL-3.0 (or compatible license)
2. **Purchase** Ultralytics commercial license for proprietary use

For MVP and demo purposes, AGPL-3.0 is acceptable. Commercial licensing can be evaluated later if needed.

## References

- [Ultralytics YOLOv8 Docs](https://docs.ultralytics.com/tasks/pose/)
- [ONNX Runtime Rust Bindings](https://github.com/pykeio/ort)
- [COCO Keypoints Dataset](https://cocodataset.org/#keypoints-2020)
- [Model Selection Document](../docs/models/pose-model-selection.md)

## Changelog

### 2025-10-02 - v1.0 Initial Release
- Added YOLOv8n-pose ONNX model (12.89 MB)
- Created comprehensive metadata file
- Documented model versioning system
- Established preprocessing/postprocessing pipelines
