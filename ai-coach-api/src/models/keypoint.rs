/// Keypoint processing models and data structures
///
/// This module provides comprehensive keypoint representation, normalization,
/// and temporal smoothing for pose estimation data.

use serde::{Deserialize, Serialize};

/// Enhanced keypoint structure with additional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keypoint {
    /// X coordinate (normalized 0-1 or pixel coordinates)
    pub x: f32,
    /// Y coordinate (normalized 0-1 or pixel coordinates)
    pub y: f32,
    /// Detection confidence (0-1)
    pub confidence: f32,
    /// Keypoint name (e.g., "left_shoulder")
    pub name: String,
    /// Whether this keypoint is visible (confidence > threshold)
    pub visible: bool,
}

impl Keypoint {
    /// Create a new keypoint
    pub fn new(x: f32, y: f32, confidence: f32, name: String) -> Self {
        Self {
            x,
            y,
            confidence,
            name,
            visible: confidence > 0.5,
        }
    }

    /// Check if keypoint is valid (within bounds and has sufficient confidence)
    pub fn is_valid(&self, min_confidence: f32) -> bool {
        self.confidence >= min_confidence && self.x >= 0.0 && self.y >= 0.0
    }

    /// Calculate Euclidean distance to another keypoint
    pub fn distance_to(&self, other: &Keypoint) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Normalize coordinates to [0, 1] range
    pub fn normalize(&mut self, width: f32, height: f32) {
        self.x /= width;
        self.y /= height;
    }

    /// Denormalize coordinates to pixel space
    pub fn denormalize(&mut self, width: f32, height: f32) {
        self.x *= width;
        self.y *= height;
    }
}

/// COCO keypoint indices for joint calculations
#[derive(Debug, Clone, Copy)]
pub enum CocoKeypoint {
    Nose = 0,
    LeftEye = 1,
    RightEye = 2,
    LeftEar = 3,
    RightEar = 4,
    LeftShoulder = 5,
    RightShoulder = 6,
    LeftElbow = 7,
    RightElbow = 8,
    LeftWrist = 9,
    RightWrist = 10,
    LeftHip = 11,
    RightHip = 12,
    LeftKnee = 13,
    RightKnee = 14,
    LeftAnkle = 15,
    RightAnkle = 16,
}

impl CocoKeypoint {
    /// Get keypoint name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Nose => "nose",
            Self::LeftEye => "left_eye",
            Self::RightEye => "right_eye",
            Self::LeftEar => "left_ear",
            Self::RightEar => "right_ear",
            Self::LeftShoulder => "left_shoulder",
            Self::RightShoulder => "right_shoulder",
            Self::LeftElbow => "left_elbow",
            Self::RightElbow => "right_elbow",
            Self::LeftWrist => "left_wrist",
            Self::RightWrist => "right_wrist",
            Self::LeftHip => "left_hip",
            Self::RightHip => "right_hip",
            Self::LeftKnee => "left_knee",
            Self::RightKnee => "right_knee",
            Self::LeftAnkle => "left_ankle",
            Self::RightAnkle => "right_ankle",
        }
    }

    /// Get all keypoint indices
    pub fn all() -> Vec<Self> {
        vec![
            Self::Nose,
            Self::LeftEye,
            Self::RightEye,
            Self::LeftEar,
            Self::RightEar,
            Self::LeftShoulder,
            Self::RightShoulder,
            Self::LeftElbow,
            Self::RightElbow,
            Self::LeftWrist,
            Self::RightWrist,
            Self::LeftHip,
            Self::RightHip,
            Self::LeftKnee,
            Self::RightKnee,
            Self::LeftAnkle,
            Self::RightAnkle,
        ]
    }
}

/// Joint angle calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointAngle {
    /// Joint name (e.g., "left_knee")
    pub name: String,
    /// Angle in degrees
    pub angle_degrees: f32,
    /// Angle in radians
    pub angle_radians: f32,
    /// Confidence (minimum of the three keypoints used)
    pub confidence: f32,
}

/// Pose frame with temporal information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseFrame {
    /// Frame timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Frame number in sequence
    pub frame_number: u32,
    /// All keypoints for this frame (17 COCO keypoints)
    pub keypoints: Vec<crate::models::keypoint::Keypoint>,
    /// Calculated joint angles
    pub joint_angles: Vec<JointAngle>,
}

impl PoseFrame {
    /// Create a new pose frame
    pub fn new(timestamp_ms: u64, frame_number: u32, keypoints: Vec<Keypoint>) -> Self {
        Self {
            timestamp_ms,
            frame_number,
            keypoints,
            joint_angles: Vec::new(),
        }
    }

    /// Get keypoint by name
    pub fn get_keypoint(&self, name: &str) -> Option<&Keypoint> {
        self.keypoints.iter().find(|kp| kp.name == name)
    }

    /// Get keypoint by COCO index
    pub fn get_keypoint_by_index(&self, index: usize) -> Option<&Keypoint> {
        self.keypoints.get(index)
    }

    /// Check if all required keypoints are visible
    pub fn has_visible_keypoints(&self, keypoint_names: &[&str]) -> bool {
        keypoint_names
            .iter()
            .all(|name| self.get_keypoint(name).map_or(false, |kp| kp.visible))
    }
}

/// Temporal smoothing configuration
#[derive(Debug, Clone)]
pub struct SmoothingConfig {
    /// Window size for moving average
    pub window_size: usize,
    /// Alpha parameter for exponential moving average (0-1)
    pub ema_alpha: f32,
    /// Enable Kalman filtering
    pub use_kalman: bool,
    /// Kalman process noise
    pub kalman_process_noise: f32,
    /// Kalman measurement noise
    pub kalman_measurement_noise: f32,
}

impl Default for SmoothingConfig {
    fn default() -> Self {
        Self {
            window_size: 5,
            ema_alpha: 0.3,
            use_kalman: false,
            kalman_process_noise: 0.01,
            kalman_measurement_noise: 0.1,
        }
    }
}

/// Coordinate normalization method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationMethod {
    /// Normalize to [0, 1] based on image dimensions
    ImageBounds,
    /// Normalize relative to torso size (hip-shoulder distance)
    TorsoRelative,
    /// Normalize relative to bounding box
    BoundingBox,
}

/// Normalization parameters
#[derive(Debug, Clone)]
pub struct NormalizationParams {
    /// Method to use
    pub method: NormalizationMethod,
    /// Reference width (image width or bbox width)
    pub reference_width: f32,
    /// Reference height (image height or bbox height)
    pub reference_height: f32,
    /// Offset X (for bbox normalization)
    pub offset_x: f32,
    /// Offset Y (for bbox normalization)
    pub offset_y: f32,
}

impl NormalizationParams {
    /// Create normalization params for image bounds
    pub fn from_image_dimensions(width: f32, height: f32) -> Self {
        Self {
            method: NormalizationMethod::ImageBounds,
            reference_width: width,
            reference_height: height,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }

    /// Create normalization params for bounding box
    pub fn from_bounding_box(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            method: NormalizationMethod::BoundingBox,
            reference_width: width,
            reference_height: height,
            offset_x: x,
            offset_y: y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypoint_creation() {
        let kp = Keypoint::new(100.0, 200.0, 0.9, "test".to_string());
        assert_eq!(kp.x, 100.0);
        assert_eq!(kp.y, 200.0);
        assert_eq!(kp.confidence, 0.9);
        assert!(kp.visible);
    }

    #[test]
    fn test_keypoint_validation() {
        let kp = Keypoint::new(100.0, 200.0, 0.9, "test".to_string());
        assert!(kp.is_valid(0.5));
        assert!(kp.is_valid(0.8));
        assert!(!kp.is_valid(0.95));
    }

    #[test]
    fn test_keypoint_distance() {
        let kp1 = Keypoint::new(0.0, 0.0, 1.0, "a".to_string());
        let kp2 = Keypoint::new(3.0, 4.0, 1.0, "b".to_string());
        assert!((kp1.distance_to(&kp2) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_keypoint_normalization() {
        let mut kp = Keypoint::new(640.0, 480.0, 0.9, "test".to_string());
        kp.normalize(1280.0, 960.0);
        assert!((kp.x - 0.5).abs() < 0.001);
        assert!((kp.y - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_coco_keypoint_names() {
        assert_eq!(CocoKeypoint::Nose.name(), "nose");
        assert_eq!(CocoKeypoint::LeftShoulder.name(), "left_shoulder");
        assert_eq!(CocoKeypoint::RightAnkle.name(), "right_ankle");
    }

    #[test]
    fn test_pose_frame_creation() {
        let keypoints = vec![
            Keypoint::new(100.0, 200.0, 0.9, "nose".to_string()),
            Keypoint::new(150.0, 250.0, 0.8, "left_shoulder".to_string()),
        ];
        let frame = PoseFrame::new(1000, 1, keypoints);
        assert_eq!(frame.timestamp_ms, 1000);
        assert_eq!(frame.frame_number, 1);
        assert_eq!(frame.keypoints.len(), 2);
    }

    #[test]
    fn test_get_keypoint_by_name() {
        let keypoints = vec![
            Keypoint::new(100.0, 200.0, 0.9, "nose".to_string()),
            Keypoint::new(150.0, 250.0, 0.8, "left_shoulder".to_string()),
        ];
        let frame = PoseFrame::new(1000, 1, keypoints);
        let kp = frame.get_keypoint("nose").unwrap();
        assert_eq!(kp.x, 100.0);
    }
}
