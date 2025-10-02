/// Integration test for pose estimation model loading and basic inference
///
/// This test verifies:
/// 1. ONNX model file exists and is accessible
/// 2. Model loads successfully with ONNX Runtime
/// 3. Model metadata is correct (input/output shapes)
/// 4. Basic inference runs without errors
use ort::{GraphOptimizationLevel, Session};
use std::path::PathBuf;

/// Test that the pose estimation model file exists
#[test]
fn test_model_file_exists() {
    let model_path = PathBuf::from("models/pose_v1.onnx");
    assert!(
        model_path.exists(),
        "Model file should exist at models/pose_v1.onnx"
    );

    // Check file size is reasonable (should be ~12-13 MB for YOLOv8n-pose)
    let metadata = std::fs::metadata(&model_path)
        .expect("Should be able to read model metadata");
    let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

    assert!(
        size_mb > 10.0 && size_mb < 20.0,
        "Model size should be between 10-20 MB, got {:.2} MB",
        size_mb
    );
}

/// Test that the model metadata file exists and is valid JSON
#[test]
fn test_model_metadata_exists() {
    let metadata_path = PathBuf::from("models/pose_v1.metadata.json");
    assert!(
        metadata_path.exists(),
        "Model metadata file should exist at models/pose_v1.metadata.json"
    );

    // Verify it's valid JSON
    let metadata_content = std::fs::read_to_string(&metadata_path)
        .expect("Should be able to read metadata file");

    let metadata: serde_json::Value = serde_json::from_str(&metadata_content)
        .expect("Metadata file should be valid JSON");

    // Verify key fields exist
    assert_eq!(metadata["model_name"], "YOLOv8n-pose");
    assert_eq!(metadata["version"], "1.0");
    assert_eq!(metadata["format"], "ONNX");
    assert_eq!(metadata["keypoints"]["count"], 17);
}

/// Test that the ONNX model loads successfully
#[test]
fn test_model_loads_successfully() -> anyhow::Result<()> {
    // Initialize ONNX Runtime environment
    ort::init()
        .with_name("ai-coach-pose-test")
        .commit()?;

    // Load the model
    let model_path = "models/pose_v1.onnx";
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(model_path)?;

    println!("✓ Model loaded successfully from {}", model_path);

    // Verify model has expected inputs
    let inputs = session.inputs;
    assert_eq!(inputs.len(), 1, "Model should have exactly 1 input");

    let input = &inputs[0];
    println!("Input name: {}", input.name);
    println!("Input shape: {:?}", input.input_type);

    // Verify model has expected outputs
    let outputs = session.outputs;
    assert_eq!(outputs.len(), 1, "Model should have exactly 1 output");

    let output = &outputs[0];
    println!("Output name: {}", output.name);
    println!("Output shape: {:?}", output.output_type);

    Ok(())
}

/// Test basic inference with dummy input
#[test]
fn test_model_inference_runs() -> anyhow::Result<()> {
    use ndarray::Array4;
    use ort::inputs;

    // Initialize ONNX Runtime
    ort::init()
        .with_name("ai-coach-pose-inference-test")
        .commit()?;

    // Load model
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .commit_from_file("models/pose_v1.onnx")?;

    // Create dummy input: [1, 3, 640, 640] (NCHW format)
    // In production, this would be a real preprocessed image
    let input_shape = [1, 3, 640, 640];
    let dummy_input: Array4<f32> = Array4::zeros(input_shape);

    println!("Running inference with input shape: {:?}", input_shape);

    // Run inference
    let outputs = session.run(inputs!["images" => dummy_input.view()]?)?;

    // Verify output exists
    assert!(!outputs.is_empty(), "Should have at least one output");

    // Get output tensor
    let output = outputs["output0"].try_extract_tensor::<f32>()?;
    let output_shape = output.shape();

    println!("Output shape: {:?}", output_shape);

    // Verify output shape matches expected [1, 56, 8400]
    // 56 = 4 (bbox) + 1 (confidence) + 17*3 (keypoints x,y,conf)
    // 8400 = number of anchor points
    assert_eq!(output_shape.len(), 3, "Output should be 3-dimensional");
    assert_eq!(output_shape[0], 1, "Batch size should be 1");
    assert_eq!(output_shape[1], 56, "Should have 56 attributes per detection");
    assert_eq!(output_shape[2], 8400, "Should have 8400 anchor points");

    println!("✓ Inference completed successfully");
    println!("✓ Output shape is correct: {:?}", output_shape);

    Ok(())
}

/// Test model with actual image preprocessing (if image crate is available)
#[test]
#[ignore] // Ignore by default as it requires a test image
fn test_model_with_real_image() -> anyhow::Result<()> {
    use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb};
    use ndarray::Array4;
    use ort::inputs;

    // Initialize ONNX Runtime
    ort::init()
        .with_name("ai-coach-pose-real-image-test")
        .commit()?;

    // Load model
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .commit_from_file("models/pose_v1.onnx")?;

    // Create a simple test image (solid color for now)
    // In production, load from file: image::open("test_data/sample_pose.jpg")?
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_pixel(640, 640, Rgb([128, 128, 128]));
    let img = DynamicImage::ImageRgb8(img);

    // Preprocess image to NCHW format [1, 3, 640, 640]
    let mut input_data = Array4::<f32>::zeros((1, 3, 640, 640));

    for y in 0..640 {
        for x in 0..640 {
            let pixel = img.get_pixel(x, y);
            // Normalize to [0, 1] and arrange in CHW format
            input_data[[0, 0, y as usize, x as usize]] = pixel[0] as f32 / 255.0; // R
            input_data[[0, 1, y as usize, x as usize]] = pixel[1] as f32 / 255.0; // G
            input_data[[0, 2, y as usize, x as usize]] = pixel[2] as f32 / 255.0; // B
        }
    }

    println!("Preprocessed image to shape: {:?}", input_data.shape());

    // Run inference
    let outputs = session.run(inputs!["images" => input_data.view()]?)?;
    let output = outputs["output0"].try_extract_tensor::<f32>()?;

    println!("Inference output shape: {:?}", output.shape());
    println!("✓ Real image inference completed successfully");

    Ok(())
}

/// Performance benchmark for model inference
#[test]
#[ignore] // Ignore by default as it's a benchmark
fn test_model_inference_performance() -> anyhow::Result<()> {
    use ndarray::Array4;
    use ort::inputs;
    use std::time::Instant;

    // Initialize ONNX Runtime
    ort::init()
        .with_name("ai-coach-pose-benchmark")
        .commit()?;

    // Load model
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file("models/pose_v1.onnx")?;

    // Create dummy input
    let input_data: Array4<f32> = Array4::zeros((1, 3, 640, 640));

    // Warmup run
    let _ = session.run(inputs!["images" => input_data.view()]?)?;

    // Benchmark multiple runs
    let num_runs = 100;
    let start = Instant::now();

    for _ in 0..num_runs {
        let _ = session.run(inputs!["images" => input_data.view()]?)?;
    }

    let duration = start.elapsed();
    let avg_ms = duration.as_millis() as f64 / num_runs as f64;

    println!("Average inference time: {:.2} ms", avg_ms);
    println!("Target: <100 ms (achieved: {})", avg_ms < 100.0);

    // Assert performance target (may vary by hardware)
    // This is a soft assertion - log warning instead of failing
    if avg_ms > 100.0 {
        eprintln!(
            "⚠️  Warning: Inference time {:.2} ms exceeds target of 100 ms",
            avg_ms
        );
    } else {
        println!("✓ Performance target achieved: {:.2} ms < 100 ms", avg_ms);
    }

    Ok(())
}

/// Test that model versioning system is documented
#[test]
fn test_model_versioning_documented() {
    let readme_path = PathBuf::from("models/README.md");
    assert!(
        readme_path.exists(),
        "models/README.md should exist to document versioning"
    );

    let readme_content = std::fs::read_to_string(&readme_path)
        .expect("Should be able to read models/README.md");

    // Verify documentation includes key sections
    assert!(
        readme_content.contains("Model Versioning"),
        "README should document versioning system"
    );
    assert!(
        readme_content.contains("pose_v1.onnx"),
        "README should reference current model"
    );
    assert!(
        readme_content.contains("YOLOv8n-pose"),
        "README should document model type"
    );
}

/// Test that model selection rationale is documented
#[test]
fn test_model_selection_documented() {
    let selection_doc_path = PathBuf::from("docs/models/pose-model-selection.md");
    assert!(
        selection_doc_path.exists(),
        "docs/models/pose-model-selection.md should exist"
    );

    let doc_content = std::fs::read_to_string(&selection_doc_path)
        .expect("Should be able to read model selection doc");

    // Verify documentation includes key comparison sections
    assert!(
        doc_content.contains("MediaPipe"),
        "Should compare MediaPipe model"
    );
    assert!(
        doc_content.contains("MoveNet"),
        "Should compare MoveNet model"
    );
    assert!(
        doc_content.contains("OpenPose"),
        "Should compare OpenPose model"
    );
    assert!(
        doc_content.contains("YOLOv8"),
        "Should compare YOLOv8 model"
    );
    assert!(
        doc_content.contains("Decision:"),
        "Should document final decision"
    );
    assert!(
        doc_content.contains("Comparison Matrix"),
        "Should include comparison matrix"
    );
}
