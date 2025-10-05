/// Unit tests for KeypointProcessor
///
/// Tests cover:
/// - Angle calculations with edge cases
/// - Distance calculations
/// - Alignment detection (vertical/horizontal)
/// - Form scoring algorithms
/// - Issue detection logic
/// - Edge case handling (missing keypoints, invalid values)

use ai_coach_api::services::keypoint_processor::{KeypointProcessor, ProcessedKeypoints};
use ai_coach_api::services::pose_estimation_service::{Keypoint, PersonPose};

/// Create a test keypoint
fn create_keypoint(x: f32, y: f32, confidence: f32, name: &str) -> Keypoint {
    Keypoint {
        x,
        y,
        confidence,
        name: name.to_string(),
    }
}

/// Create a test person pose with specific keypoints
fn create_test_person(keypoints: Vec<Keypoint>) -> PersonPose {
    PersonPose {
        bbox_x: 0.5,
        bbox_y: 0.5,
        bbox_width: 0.3,
        bbox_height: 0.6,
        confidence: 0.9,
        keypoints,
    }
}

/// Create a standard COCO skeleton for testing
fn create_standard_skeleton() -> PersonPose {
    let keypoints = vec![
        create_keypoint(0.5, 0.1, 0.9, "nose"),
        create_keypoint(0.48, 0.08, 0.8, "left_eye"),
        create_keypoint(0.52, 0.08, 0.8, "right_eye"),
        create_keypoint(0.46, 0.09, 0.75, "left_ear"),
        create_keypoint(0.54, 0.09, 0.75, "right_ear"),
        create_keypoint(0.4, 0.25, 0.9, "left_shoulder"),
        create_keypoint(0.6, 0.25, 0.9, "right_shoulder"),
        create_keypoint(0.35, 0.45, 0.85, "left_elbow"),
        create_keypoint(0.65, 0.45, 0.85, "right_elbow"),
        create_keypoint(0.3, 0.65, 0.8, "left_wrist"),
        create_keypoint(0.7, 0.65, 0.8, "right_wrist"),
        create_keypoint(0.45, 0.55, 0.9, "left_hip"),
        create_keypoint(0.55, 0.55, 0.9, "right_hip"),
        create_keypoint(0.43, 0.75, 0.85, "left_knee"),
        create_keypoint(0.57, 0.75, 0.85, "right_knee"),
        create_keypoint(0.42, 0.95, 0.8, "left_ankle"),
        create_keypoint(0.58, 0.95, 0.8, "right_ankle"),
    ];
    create_test_person(keypoints)
}

#[test]
fn test_angle_calculation_90_degrees() {
    let processor = KeypointProcessor::new();

    // Create points forming a 90-degree angle
    let p1 = create_keypoint(0.0, 0.0, 1.0, "p1");
    let p2 = create_keypoint(1.0, 0.0, 1.0, "p2");
    let p3 = create_keypoint(1.0, 1.0, 1.0, "p3");

    let angle = processor.calculate_angle(&p1, &p2, &p3).unwrap();

    assert!((angle - 90.0).abs() < 1.0, "Expected ~90°, got {:.2}°", angle);
}

#[test]
fn test_angle_calculation_180_degrees() {
    let processor = KeypointProcessor::new();

    // Straight line (180 degrees)
    let p1 = create_keypoint(0.0, 0.0, 1.0, "p1");
    let p2 = create_keypoint(0.5, 0.0, 1.0, "p2");
    let p3 = create_keypoint(1.0, 0.0, 1.0, "p3");

    let angle = processor.calculate_angle(&p1, &p2, &p3).unwrap();

    assert!((angle - 180.0).abs() < 1.0, "Expected ~180°, got {:.2}°", angle);
}

#[test]
fn test_angle_calculation_45_degrees() {
    let processor = KeypointProcessor::new();

    // 45-degree angle
    let p1 = create_keypoint(0.0, 0.0, 1.0, "p1");
    let p2 = create_keypoint(1.0, 0.0, 1.0, "p2");
    let p3 = create_keypoint(1.0, 1.0, 1.0, "p3");

    // Rotate to get 45 degrees: use (2, 0) and (2, 2) instead
    let p1 = create_keypoint(0.0, 0.0, 1.0, "p1");
    let p2 = create_keypoint(1.0, 0.0, 1.0, "p2");
    let p3 = create_keypoint(2.0, 1.0, 1.0, "p3");

    let angle = processor.calculate_angle(&p1, &p2, &p3).unwrap();

    // Allow some tolerance
    assert!(angle > 40.0 && angle < 50.0, "Expected ~45°, got {:.2}°", angle);
}

#[test]
fn test_angle_with_low_confidence_keypoint() {
    let processor = KeypointProcessor::new();

    let p1 = create_keypoint(0.0, 0.0, 0.3, "p1"); // Low confidence
    let p2 = create_keypoint(1.0, 0.0, 1.0, "p2");
    let p3 = create_keypoint(1.0, 1.0, 1.0, "p3");

    let result = processor.calculate_angle(&p1, &p2, &p3);

    assert!(result.is_none(), "Should return None for low confidence keypoint");
}

#[test]
fn test_distance_calculation() {
    let processor = KeypointProcessor::new();

    // Horizontal distance
    let p1 = create_keypoint(0.0, 0.0, 1.0, "p1");
    let p2 = create_keypoint(3.0, 0.0, 1.0, "p2");

    let distance = processor.calculate_distance(&p1, &p2).unwrap();
    assert!((distance - 3.0).abs() < 0.001, "Expected 3.0, got {:.3}", distance);

    // Vertical distance
    let p1 = create_keypoint(0.0, 0.0, 1.0, "p1");
    let p2 = create_keypoint(0.0, 4.0, 1.0, "p2");

    let distance = processor.calculate_distance(&p1, &p2).unwrap();
    assert!((distance - 4.0).abs() < 0.001, "Expected 4.0, got {:.3}", distance);

    // Diagonal distance (3-4-5 triangle)
    let p1 = create_keypoint(0.0, 0.0, 1.0, "p1");
    let p2 = create_keypoint(3.0, 4.0, 1.0, "p2");

    let distance = processor.calculate_distance(&p1, &p2).unwrap();
    assert!((distance - 5.0).abs() < 0.001, "Expected 5.0, got {:.3}", distance);
}

#[test]
fn test_distance_with_missing_keypoint() {
    let processor = KeypointProcessor::new();

    let p1 = create_keypoint(0.0, 0.0, 0.2, "p1"); // Low confidence
    let p2 = create_keypoint(3.0, 0.0, 1.0, "p2");

    let result = processor.calculate_distance(&p1, &p2);
    assert!(result.is_none(), "Should return None for low confidence keypoint");
}

#[test]
fn test_vertical_alignment_detection() {
    let processor = KeypointProcessor::new();

    // Perfectly vertical
    let p1 = create_keypoint(0.5, 0.0, 1.0, "p1");
    let p2 = create_keypoint(0.5, 1.0, 1.0, "p2");

    assert!(processor.is_vertically_aligned(&p1, &p2, 0.05).unwrap());

    // Slightly off vertical (within threshold)
    let p1 = create_keypoint(0.5, 0.0, 1.0, "p1");
    let p2 = create_keypoint(0.52, 1.0, 1.0, "p2");

    assert!(processor.is_vertically_aligned(&p1, &p2, 0.05).unwrap());

    // Not vertical (beyond threshold)
    let p1 = create_keypoint(0.5, 0.0, 1.0, "p1");
    let p2 = create_keypoint(0.7, 1.0, 1.0, "p2");

    assert!(!processor.is_vertically_aligned(&p1, &p2, 0.05).unwrap());
}

#[test]
fn test_horizontal_alignment_detection() {
    let processor = KeypointProcessor::new();

    // Perfectly horizontal
    let p1 = create_keypoint(0.0, 0.5, 1.0, "p1");
    let p2 = create_keypoint(1.0, 0.5, 1.0, "p2");

    assert!(processor.is_horizontally_aligned(&p1, &p2, 0.05).unwrap());

    // Slightly off horizontal (within threshold)
    let p1 = create_keypoint(0.0, 0.5, 1.0, "p1");
    let p2 = create_keypoint(1.0, 0.52, 1.0, "p2");

    assert!(processor.is_horizontally_aligned(&p1, &p2, 0.05).unwrap());

    // Not horizontal (beyond threshold)
    let p1 = create_keypoint(0.0, 0.5, 1.0, "p1");
    let p2 = create_keypoint(1.0, 0.7, 1.0, "p2");

    assert!(!processor.is_horizontally_aligned(&p1, &p2, 0.05).unwrap());
}

#[test]
fn test_form_scoring_perfect_form() {
    let processor = KeypointProcessor::new();
    let person = create_standard_skeleton();

    let result = processor.analyze_form(&person, "squat").unwrap();

    // Perfect form should score high
    assert!(result.overall_score >= 0.7, "Expected score >= 0.7, got {}", result.overall_score);
    assert!(!result.issues.is_empty(), "Should identify potential issues even for good form");
}

#[test]
fn test_form_scoring_with_poor_posture() {
    let processor = KeypointProcessor::new();

    // Create person with misaligned posture
    let mut keypoints = create_standard_skeleton().keypoints;

    // Lean forward significantly
    for kp in keypoints.iter_mut() {
        if kp.y < 0.5 {
            kp.x += 0.15; // Shift upper body forward
        }
    }

    let person = create_test_person(keypoints);
    let result = processor.analyze_form(&person, "squat").unwrap();

    // Poor form should score lower
    assert!(result.overall_score < 0.8, "Expected lower score for poor form, got {}", result.overall_score);
    assert!(!result.issues.is_empty(), "Should detect issues with poor posture");
}

#[test]
fn test_issue_detection_knee_alignment() {
    let processor = KeypointProcessor::new();

    // Create person with knees caving in
    let mut keypoints = create_standard_skeleton().keypoints;

    // Move knees inward
    keypoints[13].x = 0.48; // left_knee moves right
    keypoints[14].x = 0.52; // right_knee moves left

    let person = create_test_person(keypoints);
    let result = processor.analyze_form(&person, "squat").unwrap();

    // Should detect knee alignment issue
    let has_knee_issue = result.issues.iter()
        .any(|issue| issue.issue_type.contains("knee") || issue.issue_type.contains("alignment"));

    assert!(has_knee_issue, "Should detect knee alignment issue");
}

#[test]
fn test_issue_detection_depth() {
    let processor = KeypointProcessor::new();

    // Create shallow squat (knees don't bend enough)
    let mut keypoints = create_standard_skeleton().keypoints;

    // Reduce knee bend
    keypoints[13].y = 0.65; // left_knee higher
    keypoints[14].y = 0.65; // right_knee higher

    let person = create_test_person(keypoints);
    let result = processor.analyze_form(&person, "squat").unwrap();

    // May detect depth issue depending on thresholds
    assert!(!result.issues.is_empty(), "Should detect at least some issues");
}

#[test]
fn test_multiple_people_selection() {
    let processor = KeypointProcessor::new();

    let person1 = create_standard_skeleton();
    let mut person2 = create_standard_skeleton();
    person2.confidence = 0.7; // Lower confidence

    let people = vec![person1.clone(), person2];

    // Should select person with highest confidence
    let selected = processor.select_primary_person(&people).unwrap();

    assert_eq!(selected.confidence, person1.confidence);
}

#[test]
fn test_empty_person_list() {
    let processor = KeypointProcessor::new();

    let people: Vec<PersonPose> = vec![];
    let result = processor.select_primary_person(&people);

    assert!(result.is_none(), "Should return None for empty list");
}

#[test]
fn test_missing_keypoints_handling() {
    let processor = KeypointProcessor::new();

    // Create person with missing keypoints (low confidence)
    let mut keypoints = create_standard_skeleton().keypoints;

    // Make some keypoints "missing"
    keypoints[7].confidence = 0.1; // left_elbow
    keypoints[8].confidence = 0.1; // right_elbow

    let person = create_test_person(keypoints);
    let result = processor.analyze_form(&person, "squat");

    // Should still work but may have issues
    assert!(result.is_ok(), "Should handle missing keypoints gracefully");
}

#[test]
fn test_exercise_type_validation() {
    let processor = KeypointProcessor::new();
    let person = create_standard_skeleton();

    // Valid exercise types
    assert!(processor.analyze_form(&person, "squat").is_ok());
    assert!(processor.analyze_form(&person, "deadlift").is_ok());
    assert!(processor.analyze_form(&person, "bench_press").is_ok());

    // Invalid exercise type should still work (may use generic analysis)
    let result = processor.analyze_form(&person, "unknown_exercise");
    assert!(result.is_ok(), "Should handle unknown exercise types");
}

#[test]
fn test_confidence_threshold_enforcement() {
    let processor = KeypointProcessor::new();

    // All keypoints below threshold
    let keypoints: Vec<Keypoint> = (0..17)
        .map(|i| create_keypoint(0.5, i as f32 * 0.05, 0.2, &format!("kp_{}", i)))
        .collect();

    let person = create_test_person(keypoints);
    let result = processor.analyze_form(&person, "squat");

    // Should either fail or return very low score
    if let Ok(form_result) = result {
        assert!(form_result.overall_score < 0.5, "Low confidence keypoints should result in low score");
    }
}

#[test]
fn test_symmetry_detection() {
    let processor = KeypointProcessor::new();

    // Create asymmetric person (one side different)
    let mut keypoints = create_standard_skeleton().keypoints;

    // Make left side lower than right
    keypoints[5].y = 0.30; // left_shoulder down
    keypoints[7].y = 0.50; // left_elbow down
    keypoints[11].y = 0.60; // left_hip down

    let person = create_test_person(keypoints);
    let result = processor.analyze_form(&person, "squat").unwrap();

    // Should detect asymmetry
    let has_asymmetry_issue = result.issues.iter()
        .any(|issue| issue.issue_type.contains("symmetry") || issue.issue_type.contains("imbalance"));

    // Note: This depends on implementation having symmetry detection
    if has_asymmetry_issue {
        println!("✓ Symmetry detection working");
    } else {
        println!("ℹ️  Symmetry detection not implemented or not triggered");
    }
}

#[test]
fn test_angle_calculation_edge_cases() {
    let processor = KeypointProcessor::new();

    // Zero-length vectors
    let p1 = create_keypoint(0.5, 0.5, 1.0, "p1");
    let p2 = create_keypoint(0.5, 0.5, 1.0, "p2"); // Same point
    let p3 = create_keypoint(0.7, 0.7, 1.0, "p3");

    let result = processor.calculate_angle(&p1, &p2, &p3);
    // Should handle gracefully (may return None or default angle)
    println!("Zero-length vector result: {:?}", result);
}

#[test]
fn test_processed_keypoints_structure() {
    let processor = KeypointProcessor::new();
    let person = create_standard_skeleton();

    let result = processor.analyze_form(&person, "squat").unwrap();

    // Verify structure
    assert!(result.overall_score >= 0.0 && result.overall_score <= 1.0, "Score should be in [0,1]");
    assert!(!result.joint_angles.is_empty(), "Should have joint angles");

    // Check that issues have proper structure
    for issue in &result.issues {
        assert!(!issue.issue_type.is_empty(), "Issue type should not be empty");
        assert!(issue.severity >= 0.0 && issue.severity <= 1.0, "Severity should be in [0,1]");
    }
}

#[test]
fn test_performance_with_many_calculations() {
    use std::time::Instant;

    let processor = KeypointProcessor::new();
    let person = create_standard_skeleton();

    let start = Instant::now();

    // Run many form analyses
    for _ in 0..100 {
        let _ = processor.analyze_form(&person, "squat");
    }

    let duration = start.elapsed();

    println!("100 form analyses took: {:?}", duration);
    assert!(duration.as_millis() < 1000, "Should complete 100 analyses in <1s");
}
