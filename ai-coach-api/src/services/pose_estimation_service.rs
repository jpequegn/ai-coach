/// Pose Estimation Service using ONNX Runtime
///
/// This service provides human pose estimation capabilities using the YOLOv8n-pose model.
/// It handles:
/// - Model loading and initialization
/// - Image preprocessing (letterbox resize, normalization)
/// - ONNX inference execution
/// - Keypoint extraction and confidence scoring
/// - NMS (Non-Maximum Suppression) for multi-person detection
///
/// Model Details:
/// - Input: [1, 3, 640, 640] FP32 (NCHW, RGB, normalized [0,1])
/// - Output: [1, 56, 8400] FP32 (56 = 4 bbox + 1 conf + 51 keypoints)
/// - Keypoints: 17 COCO format (nose, eyes, ears, shoulders, elbows, wrists, hips, knees, ankles)
use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb};
use ndarray::{s, Array, Array2, Array3, Array4};
use ort::{GraphOptimizationLevel, Session};
use std::path::Path;

/// COCO keypoint names (17 keypoints)
pub const COCO_KEYPOINT_NAMES: [&str; 17] = [
    "nose",
    "left_eye",
    "right_eye",
    "left_ear",
    "right_ear",
    "left_shoulder",
    "right_shoulder",
    "left_elbow",
    "right_elbow",
    "left_wrist",
    "right_wrist",
    "left_hip",
    "right_hip",
    "left_knee",
    "right_knee",
    "left_ankle",
    "right_ankle",
];

/// A single detected keypoint with position and confidence
#[derive(Debug, Clone)]
pub struct Keypoint {
    pub x: f32,
    pub y: f32,
    pub confidence: f32,
    pub name: String,
}

/// A detected person with bounding box and keypoints
#[derive(Debug, Clone)]
pub struct PersonPose {
    /// Bounding box center x (normalized 0-1)
    pub bbox_x: f32,
    /// Bounding box center y (normalized 0-1)
    pub bbox_y: f32,
    /// Bounding box width (normalized 0-1)
    pub bbox_width: f32,
    /// Bounding box height (normalized 0-1)
    pub bbox_height: f32,
    /// Overall detection confidence
    pub confidence: f32,
    /// 17 COCO keypoints
    pub keypoints: Vec<Keypoint>,
}

impl PersonPose {
    /// Convert normalized coordinates to pixel coordinates
    pub fn to_pixel_coords(&self, img_width: u32, img_height: u32) -> PersonPose {
        PersonPose {
            bbox_x: self.bbox_x * img_width as f32,
            bbox_y: self.bbox_y * img_height as f32,
            bbox_width: self.bbox_width * img_width as f32,
            bbox_height: self.bbox_height * img_height as f32,
            confidence: self.confidence,
            keypoints: self
                .keypoints
                .iter()
                .map(|kp| Keypoint {
                    x: kp.x * img_width as f32,
                    y: kp.y * img_height as f32,
                    confidence: kp.confidence,
                    name: kp.name.clone(),
                })
                .collect(),
        }
    }
}

/// Result of pose estimation on an image
#[derive(Debug)]
pub struct PoseEstimationResult {
    /// Detected persons with their poses
    pub persons: Vec<PersonPose>,
    /// Inference time in milliseconds
    pub inference_time_ms: u64,
    /// Image dimensions used for inference
    pub image_width: u32,
    pub image_height: u32,
}

/// Pose Estimation Service
pub struct PoseEstimationService {
    session: Session,
    model_input_size: u32,
    confidence_threshold: f32,
    nms_iou_threshold: f32,
}

impl PoseEstimationService {
    /// Create a new PoseEstimationService by loading the ONNX model
    ///
    /// # Arguments
    /// * `model_path` - Path to the ONNX model file (e.g., "models/pose_v1.onnx")
    ///
    /// # Example
    /// ```no_run
    /// use ai_coach_api::services::pose_estimation_service::PoseEstimationService;
    ///
    /// let service = PoseEstimationService::new("models/pose_v1.onnx")
    ///     .expect("Failed to load model");
    /// ```
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        // Initialize ONNX Runtime
        ort::init()
            .with_name("ai-coach-pose-estimation")
            .commit()
            .context("Failed to initialize ONNX Runtime")?;

        // Load the model with optimizations
        let session = Session::builder()
            .context("Failed to create session builder")?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path.as_ref())
            .context("Failed to load ONNX model")?;

        tracing::info!(
            "Loaded pose estimation model from {}",
            model_path.as_ref().display()
        );

        Ok(Self {
            session,
            model_input_size: 640,
            confidence_threshold: 0.5,
            nms_iou_threshold: 0.45,
        })
    }

    /// Set the confidence threshold for detection filtering
    ///
    /// Default: 0.5
    /// Range: 0.0 - 1.0
    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set the NMS IoU threshold for filtering overlapping detections
    ///
    /// Default: 0.45
    /// Range: 0.0 - 1.0
    pub fn with_nms_threshold(mut self, threshold: f32) -> Self {
        self.nms_iou_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Perform pose estimation on an image
    ///
    /// # Arguments
    /// * `image` - Input image (any size, will be resized)
    ///
    /// # Returns
    /// PoseEstimationResult containing detected persons and keypoints
    pub fn estimate_pose(&self, image: &DynamicImage) -> Result<PoseEstimationResult> {
        let start_time = std::time::Instant::now();

        // Store original dimensions
        let original_width = image.width();
        let original_height = image.height();

        // Preprocess image to model input format
        let (input_tensor, scale, pad_x, pad_y) = self.preprocess_image(image)?;

        // Run inference
        use ort::inputs;
        let outputs = self
            .session
            .run(inputs!["images" => input_tensor.view()]?)
            .context("Failed to run inference")?;

        // Extract output tensor
        let output = outputs["output0"]
            .try_extract_tensor::<f32>()
            .context("Failed to extract output tensor")?;

        // Parse detections and extract keypoints
        let persons = self.postprocess_output(
            &output,
            original_width,
            original_height,
            scale,
            pad_x,
            pad_y,
        )?;

        let inference_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(PoseEstimationResult {
            persons,
            inference_time_ms,
            image_width: original_width,
            image_height: original_height,
        })
    }

    /// Preprocess image to model input format
    ///
    /// Performs:
    /// 1. Letterbox resize to 640x640 (maintains aspect ratio with padding)
    /// 2. RGB color space
    /// 3. Normalization to [0, 1]
    /// 4. NCHW format conversion
    ///
    /// Returns: (tensor, scale, pad_x, pad_y) for coordinate transformation
    fn preprocess_image(
        &self,
        image: &DynamicImage,
    ) -> Result<(Array4<f32>, f32, u32, u32)> {
        let (width, height) = image.dimensions();
        let target_size = self.model_input_size;

        // Calculate letterbox parameters
        let scale = (target_size as f32 / width as f32).min(target_size as f32 / height as f32);
        let new_width = (width as f32 * scale) as u32;
        let new_height = (height as f32 * scale) as u32;
        let pad_x = (target_size - new_width) / 2;
        let pad_y = (target_size - new_height) / 2;

        // Resize image with aspect ratio preservation
        let resized = image.resize_exact(
            new_width,
            new_height,
            image::imageops::FilterType::Triangle,
        );

        // Create padded image (letterbox)
        let mut padded: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(target_size, target_size, Rgb([114, 114, 114]));

        // Copy resized image to center of padded image
        for y in 0..new_height {
            for x in 0..new_width {
                let pixel = resized.get_pixel(x, y);
                padded.put_pixel(
                    x + pad_x,
                    y + pad_y,
                    Rgb([pixel[0], pixel[1], pixel[2]]),
                );
            }
        }

        // Convert to NCHW tensor [1, 3, 640, 640] and normalize
        let mut input_tensor = Array4::<f32>::zeros((1, 3, target_size as usize, target_size as usize));

        for y in 0..target_size {
            for x in 0..target_size {
                let pixel = padded.get_pixel(x, y);
                // RGB channels, normalized to [0, 1]
                input_tensor[[0, 0, y as usize, x as usize]] = pixel[0] as f32 / 255.0;
                input_tensor[[0, 1, y as usize, x as usize]] = pixel[1] as f32 / 255.0;
                input_tensor[[0, 2, y as usize, x as usize]] = pixel[2] as f32 / 255.0;
            }
        }

        Ok((input_tensor, scale, pad_x, pad_y))
    }

    /// Postprocess model output to extract persons and keypoints
    ///
    /// Output format: [1, 56, 8400]
    /// - 56 attributes: [x, y, w, h, conf, kp1_x, kp1_y, kp1_conf, ..., kp17_x, kp17_y, kp17_conf]
    /// - 8400 anchor points
    fn postprocess_output(
        &self,
        output: &ndarray::ArrayView3<f32>,
        img_width: u32,
        img_height: u32,
        scale: f32,
        pad_x: u32,
        pad_y: u32,
    ) -> Result<Vec<PersonPose>> {
        // Transpose from [1, 56, 8400] to [8400, 56] for easier processing
        let output_2d: Array2<f32> = output.slice(s![0, .., ..]).t().to_owned();

        let mut detections = Vec::new();

        // Process each detection
        for i in 0..output_2d.shape()[0] {
            let row = output_2d.slice(s![i, ..]);

            // Extract bounding box and confidence
            let bbox_x = row[0];
            let bbox_y = row[1];
            let bbox_w = row[2];
            let bbox_h = row[3];
            let confidence = row[4];

            // Filter by confidence threshold
            if confidence < self.confidence_threshold {
                continue;
            }

            // Extract keypoints (17 keypoints × 3 values each)
            let mut keypoints = Vec::with_capacity(17);
            for kp_idx in 0..17 {
                let base_idx = 5 + kp_idx * 3;
                let kp_x = row[base_idx];
                let kp_y = row[base_idx + 1];
                let kp_conf = row[base_idx + 2];

                keypoints.push(Keypoint {
                    x: kp_x,
                    y: kp_y,
                    confidence: kp_conf,
                    name: COCO_KEYPOINT_NAMES[kp_idx].to_string(),
                });
            }

            detections.push(PersonPose {
                bbox_x,
                bbox_y,
                bbox_width: bbox_w,
                bbox_height: bbox_h,
                confidence,
                keypoints,
            });
        }

        // Apply Non-Maximum Suppression
        let filtered_detections = self.apply_nms(detections);

        // Transform coordinates back to original image space
        let final_detections = filtered_detections
            .into_iter()
            .map(|mut person| {
                // Adjust for letterbox padding and scale
                person.bbox_x = (person.bbox_x - pad_x as f32) / scale;
                person.bbox_y = (person.bbox_y - pad_y as f32) / scale;
                person.bbox_width = person.bbox_width / scale;
                person.bbox_height = person.bbox_height / scale;

                for kp in &mut person.keypoints {
                    kp.x = (kp.x - pad_x as f32) / scale;
                    kp.y = (kp.y - pad_y as f32) / scale;
                }

                // Normalize to [0, 1] range
                person.bbox_x /= img_width as f32;
                person.bbox_y /= img_height as f32;
                person.bbox_width /= img_width as f32;
                person.bbox_height /= img_height as f32;

                for kp in &mut person.keypoints {
                    kp.x /= img_width as f32;
                    kp.y /= img_height as f32;
                }

                person
            })
            .collect();

        Ok(final_detections)
    }

    /// Apply Non-Maximum Suppression to remove overlapping detections
    fn apply_nms(&self, mut detections: Vec<PersonPose>) -> Vec<PersonPose> {
        // Sort by confidence (descending)
        detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        let mut keep = Vec::new();

        while !detections.is_empty() {
            let best = detections.remove(0);
            let bbox_best = (
                best.bbox_x,
                best.bbox_y,
                best.bbox_width,
                best.bbox_height,
            );

            // Keep the best detection
            keep.push(best);

            // Remove overlapping detections
            detections.retain(|det| {
                let bbox_det = (det.bbox_x, det.bbox_y, det.bbox_width, det.bbox_height);
                let iou = self.calculate_iou(bbox_best, bbox_det);
                iou < self.nms_iou_threshold
            });
        }

        keep
    }

    /// Calculate Intersection over Union (IoU) for two bounding boxes
    fn calculate_iou(&self, bbox1: (f32, f32, f32, f32), bbox2: (f32, f32, f32, f32)) -> f32 {
        let (x1, y1, w1, h1) = bbox1;
        let (x2, y2, w2, h2) = bbox2;

        // Convert center coordinates to corners
        let x1_min = x1 - w1 / 2.0;
        let y1_min = y1 - h1 / 2.0;
        let x1_max = x1 + w1 / 2.0;
        let y1_max = y1 + h1 / 2.0;

        let x2_min = x2 - w2 / 2.0;
        let y2_min = y2 - h2 / 2.0;
        let x2_max = x2 + w2 / 2.0;
        let y2_max = y2 + h2 / 2.0;

        // Calculate intersection area
        let inter_x_min = x1_min.max(x2_min);
        let inter_y_min = y1_min.max(y2_min);
        let inter_x_max = x1_max.min(x2_max);
        let inter_y_max = y1_max.min(y2_max);

        let inter_width = (inter_x_max - inter_x_min).max(0.0);
        let inter_height = (inter_y_max - inter_y_min).max(0.0);
        let inter_area = inter_width * inter_height;

        // Calculate union area
        let bbox1_area = w1 * h1;
        let bbox2_area = w2 * h2;
        let union_area = bbox1_area + bbox2_area - inter_area;

        if union_area > 0.0 {
            inter_area / union_area
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coco_keypoint_names() {
        assert_eq!(COCO_KEYPOINT_NAMES.len(), 17);
        assert_eq!(COCO_KEYPOINT_NAMES[0], "nose");
        assert_eq!(COCO_KEYPOINT_NAMES[16], "right_ankle");
    }

    #[test]
    fn test_iou_calculation() {
        // Load actual model for test
        let service = PoseEstimationService::new("models/pose_v1.onnx")
            .expect("Failed to load model for test");

        // Same box
        let iou = service.calculate_iou((100.0, 100.0, 50.0, 50.0), (100.0, 100.0, 50.0, 50.0));
        assert!((iou - 1.0).abs() < 0.01);

        // No overlap
        let iou = service.calculate_iou((100.0, 100.0, 50.0, 50.0), (200.0, 200.0, 50.0, 50.0));
        assert!(iou < 0.01);

        // Partial overlap
        let iou = service.calculate_iou((100.0, 100.0, 50.0, 50.0), (120.0, 120.0, 50.0, 50.0));
        assert!(iou > 0.0 && iou < 1.0);
    }
}
