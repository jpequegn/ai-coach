/// Integration tests for PoseEstimationService
///
/// Tests cover:
/// - Service initialization and model loading
/// - Image preprocessing pipeline
/// - Inference execution
/// - Keypoint extraction
/// - Performance benchmarking
use ai_coach_api::services::pose_estimation_service::{PoseEstimationService, COCO_KEYPOINT_NAMES};
use image::{DynamicImage, ImageBuffer, Rgb};

/// Test that the service can be created and loads the model successfully
#[test]
fn test_service_creation() {
    let result = PoseEstimationService::new("models/pose_v1.onnx");

    assert!(
        result.is_ok(),
        "Service should load model successfully: {:?}",
        result.err()
    );

    let service = result.unwrap();
    println!("✓ PoseEstimationService created successfully");
}

/// Test service with custom thresholds
#[test]
fn test_service_with_custom_thresholds() {
    let service = PoseEstimationService::new("models/pose_v1.onnx")
        .expect("Failed to load model")
        .with_confidence_threshold(0.3)
        .with_nms_threshold(0.5);

    println!("✓ Service created with custom thresholds");
}

/// Test inference with a simple test image (solid color)
#[test]
fn test_inference_with_test_image() {
    let service = PoseEstimationService::new("models/pose_v1.onnx")
        .expect("Failed to load model");

    // Create a simple test image (gray)
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(640, 640, Rgb([128, 128, 128]));
    let img = DynamicImage::ImageRgb8(img);

    // Run inference
    let result = service.estimate_pose(&img);

    assert!(
        result.is_ok(),
        "Inference should succeed: {:?}",
        result.err()
    );

    let pose_result = result.unwrap();

    println!("✓ Inference completed successfully");
    println!("  - Inference time: {} ms", pose_result.inference_time_ms);
    println!("  - Persons detected: {}", pose_result.persons.len());
    println!("  - Image dimensions: {}x{}", pose_result.image_width, pose_result.image_height);

    // Inference should complete in reasonable time (<100ms target)
    assert!(
        pose_result.inference_time_ms < 1000,
        "Inference should complete in < 1000ms (got {}ms)",
        pose_result.inference_time_ms
    );
}

/// Test inference with different image sizes
#[test]
fn test_inference_with_various_sizes() {
    let service = PoseEstimationService::new("models/pose_v1.onnx")
        .expect("Failed to load model");

    let test_sizes = vec![
        (320, 240),  // Small
        (640, 480),  // Medium
        (1280, 720), // HD
        (1920, 1080), // Full HD
    ];

    for (width, height) in test_sizes {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(width, height, Rgb([100, 100, 100]));
        let img = DynamicImage::ImageRgb8(img);

        let result = service.estimate_pose(&img);
        assert!(
            result.is_ok(),
            "Inference should work with {}x{} image",
            width,
            height
        );

        let pose_result = result.unwrap();
        println!(
            "  {}x{}: {} ms, {} persons",
            width, height, pose_result.inference_time_ms, pose_result.persons.len()
        );
    }

    println!("✓ Inference works with various image sizes");
}

/// Test keypoint structure
#[test]
fn test_keypoint_structure() {
    let service = PoseEstimationService::new("models/pose_v1.onnx")
        .expect("Failed to load model")
        .with_confidence_threshold(0.1); // Lower threshold to detect something

    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(640, 640, Rgb([128, 128, 128]));
    let img = DynamicImage::ImageRgb8(img);

    let result = service.estimate_pose(&img).expect("Inference failed");

    // Even if no persons detected, structure should be valid
    println!("Persons detected: {}", result.persons.len());

    if !result.persons.is_empty() {
        let person = &result.persons[0];

        // Each person should have 17 keypoints
        assert_eq!(person.keypoints.len(), 17, "Should have 17 COCO keypoints");

        // Verify keypoint names match COCO standard
        for (i, kp) in person.keypoints.iter().enumerate() {
            assert_eq!(
                kp.name,
                COCO_KEYPOINT_NAMES[i],
                "Keypoint {} should be named {}",
                i,
                COCO_KEYPOINT_NAMES[i]
            );

            // Keypoints should have valid coordinates (normalized [0,1])
            assert!(
                kp.x >= 0.0 && kp.x <= 1.0,
                "Keypoint X should be normalized"
            );
            assert!(
                kp.y >= 0.0 && kp.y <= 1.0,
                "Keypoint Y should be normalized"
            );
            assert!(
                kp.confidence >= 0.0 && kp.confidence <= 1.0,
                "Keypoint confidence should be in [0,1]"
            );
        }

        println!("✓ Keypoint structure is valid");
        println!("  - Bounding box: ({:.2}, {:.2}, {:.2}, {:.2})",
                 person.bbox_x, person.bbox_y, person.bbox_width, person.bbox_height);
        println!("  - Confidence: {:.2}", person.confidence);
    }
}

/// Test pixel coordinate conversion
#[test]
fn test_pixel_coordinate_conversion() {
    use ai_coach_api::services::pose_estimation_service::{Keypoint, PersonPose};

    let normalized_person = PersonPose {
        bbox_x: 0.5,
        bbox_y: 0.5,
        bbox_width: 0.3,
        bbox_height: 0.4,
        confidence: 0.9,
        keypoints: vec![Keypoint {
            x: 0.5,
            y: 0.3,
            confidence: 0.8,
            name: "nose".to_string(),
        }],
    };

    let img_width = 1920;
    let img_height = 1080;

    let pixel_person = normalized_person.to_pixel_coords(img_width, img_height);

    // Verify conversion
    assert_eq!(pixel_person.bbox_x, 0.5 * img_width as f32);
    assert_eq!(pixel_person.bbox_y, 0.5 * img_height as f32);
    assert_eq!(pixel_person.keypoints[0].x, 0.5 * img_width as f32);
    assert_eq!(pixel_person.keypoints[0].y, 0.3 * img_height as f32);

    println!("✓ Pixel coordinate conversion works correctly");
}

/// Performance benchmark test
#[test]
#[ignore] // Ignore by default, run with --ignored
fn test_performance_benchmark() {
    let service = PoseEstimationService::new("models/pose_v1.onnx")
        .expect("Failed to load model");

    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(640, 640, Rgb([128, 128, 128]));
    let img = DynamicImage::ImageRgb8(img);

    // Warmup run
    let _ = service.estimate_pose(&img);

    // Benchmark multiple runs
    let num_runs = 100;
    let mut total_time = 0;

    for _ in 0..num_runs {
        let result = service.estimate_pose(&img).expect("Inference failed");
        total_time += result.inference_time_ms;
    }

    let avg_time = total_time / num_runs;

    println!("Performance Benchmark Results:");
    println!("  - Average inference time: {} ms", avg_time);
    println!("  - Throughput: {:.1} FPS", 1000.0 / avg_time as f64);
    println!("  - Total runs: {}", num_runs);

    // Performance target: <100ms per frame
    if avg_time > 100 {
        println!(
            "⚠️  Warning: Average inference time {}ms exceeds 100ms target",
            avg_time
        );
    } else {
        println!("✓ Performance target achieved: {}ms < 100ms", avg_time);
    }
}

/// Test with a gradient image (more realistic than solid color)
#[test]
fn test_inference_with_gradient_image() {
    let service = PoseEstimationService::new("models/pose_v1.onnx")
        .expect("Failed to load model");

    // Create gradient image
    let width = 640;
    let height = 480;
    let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let r = (x * 255 / width) as u8;
            let g = (y * 255 / height) as u8;
            let b = 128;
            img.put_pixel(x, y, Rgb([r, g, b]));
        }
    }

    let img = DynamicImage::ImageRgb8(img);
    let result = service.estimate_pose(&img).expect("Inference failed");

    println!("✓ Inference works with gradient image");
    println!("  - Inference time: {} ms", result.inference_time_ms);
    println!("  - Persons detected: {}", result.persons.len());
}

/// Test error handling with invalid model path
#[test]
fn test_invalid_model_path() {
    let result = PoseEstimationService::new("models/nonexistent_model.onnx");

    assert!(
        result.is_err(),
        "Should fail with non-existent model file"
    );

    println!("✓ Error handling works for invalid model path");
}

/// Test that COCO keypoint names are correct
#[test]
fn test_coco_keypoint_names() {
    assert_eq!(COCO_KEYPOINT_NAMES.len(), 17);

    // Test specific keypoints
    assert_eq!(COCO_KEYPOINT_NAMES[0], "nose");
    assert_eq!(COCO_KEYPOINT_NAMES[5], "left_shoulder");
    assert_eq!(COCO_KEYPOINT_NAMES[6], "right_shoulder");
    assert_eq!(COCO_KEYPOINT_NAMES[11], "left_hip");
    assert_eq!(COCO_KEYPOINT_NAMES[12], "right_hip");
    assert_eq!(COCO_KEYPOINT_NAMES[15], "left_ankle");
    assert_eq!(COCO_KEYPOINT_NAMES[16], "right_ankle");

    println!("✓ COCO keypoint names are correct");
}

/// Test multiple inference calls (stability test)
#[test]
fn test_multiple_inferences() {
    let service = PoseEstimationService::new("models/pose_v1.onnx")
        .expect("Failed to load model");

    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(640, 640, Rgb([128, 128, 128]));
    let img = DynamicImage::ImageRgb8(img);

    // Run multiple inferences to ensure stability
    for i in 0..10 {
        let result = service.estimate_pose(&img);
        assert!(
            result.is_ok(),
            "Inference {} should succeed",
            i + 1
        );
    }

    println!("✓ Multiple consecutive inferences work stably");
}
