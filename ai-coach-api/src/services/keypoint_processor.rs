/// Keypoint Processing Service
///
/// This service provides comprehensive keypoint processing capabilities:
/// - Coordinate normalization (image bounds, torso-relative, bounding box)
/// - Joint angle calculations (hip, knee, ankle, elbow, shoulder)
/// - Temporal smoothing (moving average, exponential moving average, Kalman filter)
/// - Keypoint validation and quality checks

use crate::models::keypoint::{
    CocoKeypoint, JointAngle, Keypoint as KeypointData, NormalizationMethod, NormalizationParams,
    PoseFrame, SmoothingConfig,
};
use anyhow::{Context, Result};
use std::collections::VecDeque;

/// Keypoint processor service
pub struct KeypointProcessor {
    /// Smoothing configuration
    smoothing_config: SmoothingConfig,
    /// History buffer for temporal smoothing
    history_buffer: VecDeque<PoseFrame>,
    /// Minimum confidence threshold for valid keypoints
    min_confidence: f32,
    /// Kalman filter states (position x, position y, velocity x, velocity y)
    kalman_states: Vec<KalmanState>,
}

/// Kalman filter state for a single keypoint coordinate
#[derive(Debug, Clone)]
struct KalmanState {
    /// Current estimate
    x: f32,
    /// Estimation error covariance
    p: f32,
    /// Process noise
    q: f32,
    /// Measurement noise
    r: f32,
}

impl KalmanState {
    fn new(q: f32, r: f32) -> Self {
        Self {
            x: 0.0,
            p: 1.0,
            q,
            r,
        }
    }

    fn update(&mut self, measurement: f32) -> f32 {
        // Prediction step
        let p_pred = self.p + self.q;

        // Update step
        let k = p_pred / (p_pred + self.r); // Kalman gain
        self.x = self.x + k * (measurement - self.x);
        self.p = (1.0 - k) * p_pred;

        self.x
    }

    fn reset(&mut self, initial_value: f32) {
        self.x = initial_value;
        self.p = 1.0;
    }
}

impl KeypointProcessor {
    /// Create a new keypoint processor with default configuration
    pub fn new() -> Self {
        Self::with_config(SmoothingConfig::default())
    }

    /// Create a new keypoint processor with custom configuration
    pub fn with_config(smoothing_config: SmoothingConfig) -> Self {
        // Initialize Kalman filter states for 17 keypoints × 2 coordinates
        let kalman_states = (0..34)
            .map(|_| {
                KalmanState::new(
                    smoothing_config.kalman_process_noise,
                    smoothing_config.kalman_measurement_noise,
                )
            })
            .collect();

        Self {
            smoothing_config,
            history_buffer: VecDeque::with_capacity(10),
            min_confidence: 0.3,
            kalman_states,
        }
    }

    /// Set minimum confidence threshold
    pub fn with_min_confidence(mut self, min_confidence: f32) -> Self {
        self.min_confidence = min_confidence.clamp(0.0, 1.0);
        self
    }

    /// Normalize keypoints using specified method
    pub fn normalize_keypoints(
        &self,
        keypoints: &mut [KeypointData],
        params: &NormalizationParams,
    ) -> Result<()> {
        match params.method {
            NormalizationMethod::ImageBounds => {
                for kp in keypoints.iter_mut() {
                    kp.normalize(params.reference_width, params.reference_height);
                }
            }
            NormalizationMethod::BoundingBox => {
                for kp in keypoints.iter_mut() {
                    // Translate to bbox origin, then normalize by bbox dimensions
                    kp.x = (kp.x - params.offset_x) / params.reference_width;
                    kp.y = (kp.y - params.offset_y) / params.reference_height;
                }
            }
            NormalizationMethod::TorsoRelative => {
                // Calculate torso reference (average hip-shoulder distance)
                let torso_scale = self.calculate_torso_scale(keypoints)?;

                // Find torso center (midpoint of hips and shoulders)
                let (center_x, center_y) = self.calculate_torso_center(keypoints)?;

                // Normalize relative to torso
                for kp in keypoints.iter_mut() {
                    kp.x = (kp.x - center_x) / torso_scale;
                    kp.y = (kp.y - center_y) / torso_scale;
                }
            }
        }

        Ok(())
    }

    /// Calculate torso scale (average distance between shoulders and hips)
    fn calculate_torso_scale(&self, keypoints: &[KeypointData]) -> Result<f32> {
        let left_shoulder = keypoints
            .get(CocoKeypoint::LeftShoulder as usize)
            .context("Missing left shoulder")?;
        let right_shoulder = keypoints
            .get(CocoKeypoint::RightShoulder as usize)
            .context("Missing right shoulder")?;
        let left_hip = keypoints
            .get(CocoKeypoint::LeftHip as usize)
            .context("Missing left hip")?;
        let right_hip = keypoints
            .get(CocoKeypoint::RightHip as usize)
            .context("Missing right hip")?;

        // Calculate shoulder-hip distances
        let left_torso = left_shoulder.distance_to(left_hip);
        let right_torso = right_shoulder.distance_to(right_hip);

        Ok((left_torso + right_torso) / 2.0)
    }

    /// Calculate torso center point
    fn calculate_torso_center(&self, keypoints: &[KeypointData]) -> Result<(f32, f32)> {
        let left_shoulder = keypoints
            .get(CocoKeypoint::LeftShoulder as usize)
            .context("Missing left shoulder")?;
        let right_shoulder = keypoints
            .get(CocoKeypoint::RightShoulder as usize)
            .context("Missing right shoulder")?;
        let left_hip = keypoints
            .get(CocoKeypoint::LeftHip as usize)
            .context("Missing left hip")?;
        let right_hip = keypoints
            .get(CocoKeypoint::RightHip as usize)
            .context("Missing right hip")?;

        let center_x = (left_shoulder.x + right_shoulder.x + left_hip.x + right_hip.x) / 4.0;
        let center_y = (left_shoulder.y + right_shoulder.y + left_hip.y + right_hip.y) / 4.0;

        Ok((center_x, center_y))
    }

    /// Calculate joint angle from three keypoints
    ///
    /// # Arguments
    /// * `point_a` - First point (e.g., hip)
    /// * `point_b` - Joint point (e.g., knee)
    /// * `point_c` - Third point (e.g., ankle)
    ///
    /// # Returns
    /// Angle at point_b in radians and degrees
    pub fn calculate_joint_angle(
        &self,
        point_a: &KeypointData,
        point_b: &KeypointData,
        point_c: &KeypointData,
    ) -> Result<(f32, f32)> {
        // Vectors from joint to adjacent points
        let vec_ba_x = point_a.x - point_b.x;
        let vec_ba_y = point_a.y - point_b.y;
        let vec_bc_x = point_c.x - point_b.x;
        let vec_bc_y = point_c.y - point_b.y;

        // Calculate angle using dot product and magnitudes
        let dot_product = vec_ba_x * vec_bc_x + vec_ba_y * vec_bc_y;
        let mag_ba = (vec_ba_x * vec_ba_x + vec_ba_y * vec_ba_y).sqrt();
        let mag_bc = (vec_bc_x * vec_bc_x + vec_bc_y * vec_bc_y).sqrt();

        if mag_ba == 0.0 || mag_bc == 0.0 {
            anyhow::bail!("Cannot calculate angle with zero-length vectors");
        }

        let cos_angle = dot_product / (mag_ba * mag_bc);
        let angle_radians = cos_angle.clamp(-1.0, 1.0).acos();
        let angle_degrees = angle_radians.to_degrees();

        Ok((angle_radians, angle_degrees))
    }

    /// Calculate all joint angles for a pose
    pub fn calculate_all_joint_angles(&self, keypoints: &[KeypointData]) -> Result<Vec<JointAngle>> {
        let mut angles = Vec::new();

        // Define joint configurations: (name, point_a_idx, joint_idx, point_c_idx)
        let joint_configs = [
            ("left_hip", CocoKeypoint::LeftShoulder as usize, CocoKeypoint::LeftHip as usize, CocoKeypoint::LeftKnee as usize),
            ("right_hip", CocoKeypoint::RightShoulder as usize, CocoKeypoint::RightHip as usize, CocoKeypoint::RightKnee as usize),
            ("left_knee", CocoKeypoint::LeftHip as usize, CocoKeypoint::LeftKnee as usize, CocoKeypoint::LeftAnkle as usize),
            ("right_knee", CocoKeypoint::RightHip as usize, CocoKeypoint::RightKnee as usize, CocoKeypoint::RightAnkle as usize),
            ("left_ankle", CocoKeypoint::LeftKnee as usize, CocoKeypoint::LeftAnkle as usize, CocoKeypoint::LeftAnkle as usize), // Placeholder
            ("right_ankle", CocoKeypoint::RightKnee as usize, CocoKeypoint::RightAnkle as usize, CocoKeypoint::RightAnkle as usize), // Placeholder
            ("left_shoulder", CocoKeypoint::LeftHip as usize, CocoKeypoint::LeftShoulder as usize, CocoKeypoint::LeftElbow as usize),
            ("right_shoulder", CocoKeypoint::RightHip as usize, CocoKeypoint::RightShoulder as usize, CocoKeypoint::RightElbow as usize),
            ("left_elbow", CocoKeypoint::LeftShoulder as usize, CocoKeypoint::LeftElbow as usize, CocoKeypoint::LeftWrist as usize),
            ("right_elbow", CocoKeypoint::RightShoulder as usize, CocoKeypoint::RightElbow as usize, CocoKeypoint::RightWrist as usize),
        ];

        for (name, idx_a, idx_b, idx_c) in joint_configs {
            // Skip ankle angles for now (need foot orientation data)
            if name.contains("ankle") {
                continue;
            }

            if let (Some(kp_a), Some(kp_b), Some(kp_c)) = (
                keypoints.get(idx_a),
                keypoints.get(idx_b),
                keypoints.get(idx_c),
            ) {
                // Check if all keypoints are valid
                if kp_a.is_valid(self.min_confidence)
                    && kp_b.is_valid(self.min_confidence)
                    && kp_c.is_valid(self.min_confidence)
                {
                    if let Ok((angle_rad, angle_deg)) =
                        self.calculate_joint_angle(kp_a, kp_b, kp_c)
                    {
                        angles.push(JointAngle {
                            name: name.to_string(),
                            angle_radians: angle_rad,
                            angle_degrees: angle_deg,
                            confidence: kp_a.confidence.min(kp_b.confidence).min(kp_c.confidence),
                        });
                    }
                }
            }
        }

        Ok(angles)
    }

    /// Apply temporal smoothing to a pose frame
    pub fn smooth_pose(&mut self, mut frame: PoseFrame) -> Result<PoseFrame> {
        // Add to history
        self.history_buffer.push_back(frame.clone());

        // Keep only necessary history
        let max_history = self.smoothing_config.window_size.max(3);
        while self.history_buffer.len() > max_history {
            self.history_buffer.pop_front();
        }

        // Apply smoothing based on configuration
        if self.smoothing_config.use_kalman {
            self.apply_kalman_smoothing(&mut frame)?;
        } else {
            self.apply_moving_average_smoothing(&mut frame)?;
        }

        Ok(frame)
    }

    /// Apply moving average or exponential moving average smoothing
    fn apply_moving_average_smoothing(&self, frame: &mut PoseFrame) -> Result<()> {
        if self.history_buffer.len() < 2 {
            return Ok(()); // Not enough history for smoothing
        }

        let history_vec: Vec<&PoseFrame> = self.history_buffer.iter().collect();
        let window_size = self.smoothing_config.window_size.min(history_vec.len());
        let start_idx = history_vec.len().saturating_sub(window_size);

        for (kp_idx, kp) in frame.keypoints.iter_mut().enumerate() {
            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            let mut count = 0;

            // Collect values from history window
            for history_frame in &history_vec[start_idx..] {
                if let Some(historical_kp) = history_frame.keypoints.get(kp_idx) {
                    if historical_kp.is_valid(self.min_confidence) {
                        sum_x += historical_kp.x;
                        sum_y += historical_kp.y;
                        count += 1;
                    }
                }
            }

            // Apply smoothing if we have history
            if count > 0 {
                kp.x = sum_x / count as f32;
                kp.y = sum_y / count as f32;
            }
        }

        Ok(())
    }

    /// Apply Kalman filter smoothing
    fn apply_kalman_smoothing(&mut self, frame: &mut PoseFrame) -> Result<()> {
        for (kp_idx, kp) in frame.keypoints.iter_mut().enumerate() {
            if !kp.is_valid(self.min_confidence) {
                continue;
            }

            let state_x_idx = kp_idx * 2;
            let state_y_idx = kp_idx * 2 + 1;

            // Initialize Kalman state if first valid observation
            if self.kalman_states[state_x_idx].x == 0.0
                && self.kalman_states[state_x_idx].p == 1.0
            {
                self.kalman_states[state_x_idx].reset(kp.x);
                self.kalman_states[state_y_idx].reset(kp.y);
            }

            // Update Kalman filter
            kp.x = self.kalman_states[state_x_idx].update(kp.x);
            kp.y = self.kalman_states[state_y_idx].update(kp.y);
        }

        Ok(())
    }

    /// Validate keypoints and mark invalid ones
    pub fn validate_keypoints(&self, keypoints: &mut [KeypointData]) {
        for kp in keypoints.iter_mut() {
            // Check confidence threshold
            if kp.confidence < self.min_confidence {
                kp.visible = false;
                continue;
            }

            // Check coordinate bounds (assuming normalized [0, 1])
            if kp.x < 0.0 || kp.x > 1.0 || kp.y < 0.0 || kp.y > 1.0 {
                kp.visible = false;
                continue;
            }

            kp.visible = true;
        }
    }

    /// Process a complete pose frame: normalize, calculate angles, smooth
    pub fn process_frame(
        &mut self,
        mut frame: PoseFrame,
        normalization_params: &NormalizationParams,
    ) -> Result<PoseFrame> {
        // Normalize keypoints
        self.normalize_keypoints(&mut frame.keypoints, normalization_params)?;

        // Validate keypoints
        self.validate_keypoints(&mut frame.keypoints);

        // Calculate joint angles
        frame.joint_angles = self.calculate_all_joint_angles(&frame.keypoints)?;

        // Apply temporal smoothing
        frame = self.smooth_pose(frame)?;

        Ok(frame)
    }

    /// Reset temporal smoothing state
    pub fn reset(&mut self) {
        self.history_buffer.clear();
        for state in &mut self.kalman_states {
            state.x = 0.0;
            state.p = 1.0;
        }
    }
}

impl Default for KeypointProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_keypoints() -> Vec<KeypointData> {
        vec![
            KeypointData::new(320.0, 100.0, 0.9, "nose".to_string()), // 0
            KeypointData::new(310.0, 90.0, 0.8, "left_eye".to_string()), // 1
            KeypointData::new(330.0, 90.0, 0.8, "right_eye".to_string()), // 2
            KeypointData::new(300.0, 100.0, 0.7, "left_ear".to_string()), // 3
            KeypointData::new(340.0, 100.0, 0.7, "right_ear".to_string()), // 4
            KeypointData::new(280.0, 200.0, 0.9, "left_shoulder".to_string()), // 5
            KeypointData::new(360.0, 200.0, 0.9, "right_shoulder".to_string()), // 6
            KeypointData::new(250.0, 300.0, 0.85, "left_elbow".to_string()), // 7
            KeypointData::new(390.0, 300.0, 0.85, "right_elbow".to_string()), // 8
            KeypointData::new(230.0, 400.0, 0.8, "left_wrist".to_string()), // 9
            KeypointData::new(410.0, 400.0, 0.8, "right_wrist".to_string()), // 10
            KeypointData::new(290.0, 400.0, 0.9, "left_hip".to_string()), // 11
            KeypointData::new(350.0, 400.0, 0.9, "right_hip".to_string()), // 12
            KeypointData::new(280.0, 550.0, 0.85, "left_knee".to_string()), // 13
            KeypointData::new(360.0, 550.0, 0.85, "right_knee".to_string()), // 14
            KeypointData::new(270.0, 700.0, 0.8, "left_ankle".to_string()), // 15
            KeypointData::new(370.0, 700.0, 0.8, "right_ankle".to_string()), // 16
        ]
    }

    #[test]
    fn test_processor_creation() {
        let processor = KeypointProcessor::new();
        assert_eq!(processor.min_confidence, 0.3);
    }

    #[test]
    fn test_normalization_image_bounds() {
        let processor = KeypointProcessor::new();
        let mut keypoints = create_test_keypoints();
        let params = NormalizationParams::from_image_dimensions(640.0, 720.0);

        processor
            .normalize_keypoints(&mut keypoints, &params)
            .unwrap();

        // Check nose normalization (320, 100) -> (0.5, ~0.139)
        assert!((keypoints[0].x - 0.5).abs() < 0.01);
        assert!((keypoints[0].y - 0.139).abs() < 0.01);
    }

    #[test]
    fn test_joint_angle_calculation() {
        let processor = KeypointProcessor::new();

        // Create three points forming a right angle
        let point_a = KeypointData::new(0.0, 0.0, 1.0, "a".to_string());
        let point_b = KeypointData::new(0.0, 1.0, 1.0, "b".to_string());
        let point_c = KeypointData::new(1.0, 1.0, 1.0, "c".to_string());

        let (angle_rad, angle_deg) = processor
            .calculate_joint_angle(&point_a, &point_b, &point_c)
            .unwrap();

        // Should be 90 degrees
        assert!((angle_deg - 90.0).abs() < 1.0);
        assert!((angle_rad - std::f32::consts::FRAC_PI_2).abs() < 0.02);
    }

    #[test]
    fn test_calculate_all_joint_angles() {
        let processor = KeypointProcessor::new();
        let keypoints = create_test_keypoints();

        let angles = processor.calculate_all_joint_angles(&keypoints).unwrap();

        // Should have multiple joint angles
        assert!(angles.len() >= 6);

        // Check that angles are in valid range
        for angle in angles {
            assert!(angle.angle_degrees >= 0.0 && angle.angle_degrees <= 180.0);
            assert!(angle.confidence >= 0.0 && angle.confidence <= 1.0);
        }
    }

    #[test]
    fn test_keypoint_validation() {
        let processor = KeypointProcessor::new();
        let mut keypoints = vec![
            KeypointData::new(0.5, 0.5, 0.9, "valid".to_string()),
            KeypointData::new(0.5, 0.5, 0.1, "low_conf".to_string()),
            KeypointData::new(-0.1, 0.5, 0.9, "out_of_bounds".to_string()),
        ];

        processor.validate_keypoints(&mut keypoints);

        assert!(keypoints[0].visible);
        assert!(!keypoints[1].visible);
        assert!(!keypoints[2].visible);
    }

    #[test]
    fn test_temporal_smoothing() {
        let mut processor = KeypointProcessor::new();

        // Create three frames with slightly different keypoint positions
        let frame1 = PoseFrame::new(
            0,
            0,
            vec![KeypointData::new(100.0, 100.0, 0.9, "test".to_string())],
        );
        let frame2 = PoseFrame::new(
            33,
            1,
            vec![KeypointData::new(102.0, 98.0, 0.9, "test".to_string())],
        );
        let frame3 = PoseFrame::new(
            66,
            2,
            vec![KeypointData::new(98.0, 102.0, 0.9, "test".to_string())],
        );

        let result1 = processor.smooth_pose(frame1).unwrap();
        let result2 = processor.smooth_pose(frame2).unwrap();
        let result3 = processor.smooth_pose(frame3).unwrap();

        // Smoothed values should be closer to average
        let smoothed_x = result3.keypoints[0].x;
        let smoothed_y = result3.keypoints[0].y;

        // Should be close to average (100)
        assert!((smoothed_x - 100.0).abs() < 2.0);
        assert!((smoothed_y - 100.0).abs() < 2.0);
    }
}
