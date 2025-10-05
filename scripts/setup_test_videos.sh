#!/bin/bash

# Setup Test Video Library for Vision Analysis Testing
# Creates comprehensive test video suite including edge cases and performance tests

set -e

TEST_DATA_DIR="test_data"
VIDEO_DIR="$TEST_DATA_DIR/videos"

echo "🎬 Setting up test video library..."

# Create directories
mkdir -p "$VIDEO_DIR"
mkdir -p "$VIDEO_DIR/edge_cases"
mkdir -p "$VIDEO_DIR/performance"
mkdir -p "$VIDEO_DIR/standard"

# Check for FFmpeg
if ! command -v ffmpeg &> /dev/null; then
    echo "❌ FFmpeg is required but not installed."
    echo "Install with: brew install ffmpeg (macOS) or apt-get install ffmpeg (Linux)"
    exit 1
fi

echo "✅ FFmpeg found"

# Standard Test Videos
echo ""
echo "📹 Creating standard test videos..."

# 720p 30fps squat simulation (10 seconds)
ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=30 \
    -pix_fmt yuv420p -y "$VIDEO_DIR/standard/test_squat.mp4" 2>/dev/null
echo "  ✓ test_squat.mp4 (720p, 30fps, 10s)"

# 720p 30fps deadlift simulation
ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=30 \
    -pix_fmt yuv420p -y "$VIDEO_DIR/standard/test_deadlift.mp4" 2>/dev/null
echo "  ✓ test_deadlift.mp4 (720p, 30fps, 10s)"

# 720p 30fps bench press simulation
ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=30 \
    -pix_fmt yuv420p -y "$VIDEO_DIR/standard/test_bench_press.mp4" 2>/dev/null
echo "  ✓ test_bench_press.mp4 (720p, 30fps, 10s)"

# Performance Test Videos
echo ""
echo "⚡ Creating performance test videos..."

# 480p low quality (fast processing)
ffmpeg -f lavfi -i testsrc=duration=30:size=854x480:rate=30 \
    -pix_fmt yuv420p -y "$VIDEO_DIR/performance/test_480p.mp4" 2>/dev/null
echo "  ✓ test_480p.mp4 (480p, 30fps, 30s)"

# 1080p Full HD
ffmpeg -f lavfi -i testsrc=duration=30:size=1920x1080:rate=30 \
    -pix_fmt yuv420p -y "$VIDEO_DIR/performance/test_1080p.mp4" 2>/dev/null
echo "  ✓ test_1080p.mp4 (1080p, 30fps, 30s)"

# 4K UHD (stress test)
ffmpeg -f lavfi -i testsrc=duration=10:size=3840x2160:rate=30 \
    -pix_fmt yuv420p -y "$VIDEO_DIR/performance/test_4k.mp4" 2>/dev/null
echo "  ✓ test_4k.mp4 (4K, 30fps, 10s)"

# 60fps high frame rate
ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=60 \
    -pix_fmt yuv420p -y "$VIDEO_DIR/performance/test_60fps.mp4" 2>/dev/null
echo "  ✓ test_60fps.mp4 (720p, 60fps, 10s)"

# Long video for memory testing
ffmpeg -f lavfi -i testsrc=duration=300:size=1280x720:rate=30 \
    -pix_fmt yuv420p -y "$VIDEO_DIR/performance/test_long.mp4" 2>/dev/null
echo "  ✓ test_long.mp4 (720p, 30fps, 5min)"

# Edge Cases
echo ""
echo "🔧 Creating edge case test files..."

# Very short video (1 second)
ffmpeg -f lavfi -i testsrc=duration=1:size=1280x720:rate=30 \
    -pix_fmt yuv420p -y "$VIDEO_DIR/edge_cases/test_1second.mp4" 2>/dev/null
echo "  ✓ test_1second.mp4 (1 second video)"

# Low resolution (144p)
ffmpeg -f lavfi -i testsrc=duration=10:size=256x144:rate=30 \
    -pix_fmt yuv420p -y "$VIDEO_DIR/edge_cases/test_144p.mp4" 2>/dev/null
echo "  ✓ test_144p.mp4 (very low resolution)"

# Variable frame rate (15fps)
ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=15 \
    -pix_fmt yuv420p -y "$VIDEO_DIR/edge_cases/test_15fps.mp4" 2>/dev/null
echo "  ✓ test_15fps.mp4 (low frame rate)"

# Different codec (H.265/HEVC)
ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=30 \
    -c:v libx265 -pix_fmt yuv420p -y "$VIDEO_DIR/edge_cases/test_h265.mp4" 2>/dev/null || \
    echo "  ⚠️  test_h265.mp4 (H.265 not available, skipped)"

# Create corrupted video
dd if=/dev/urandom of="$VIDEO_DIR/edge_cases/corrupted.mp4" bs=1024 count=100 2>/dev/null
echo "  ✓ corrupted.mp4 (corrupted file)"

# Create wrong format file
echo "This is not a video file" > "$VIDEO_DIR/edge_cases/wrong_format.txt"
cp "$VIDEO_DIR/edge_cases/wrong_format.txt" "$VIDEO_DIR/edge_cases/wrong_format.mp4"
echo "  ✓ wrong_format.mp4 (invalid video data)"

# Create empty file
touch "$VIDEO_DIR/edge_cases/empty.mp4"
echo "  ✓ empty.mp4 (zero bytes)"

# Generate test metadata
cat > "$TEST_DATA_DIR/test_videos.json" <<EOF
{
  "standard": [
    {
      "name": "test_squat.mp4",
      "resolution": "1280x720",
      "fps": 30,
      "duration": 10,
      "exercise_type": "squat",
      "expected_keypoints": true
    },
    {
      "name": "test_deadlift.mp4",
      "resolution": "1280x720",
      "fps": 30,
      "duration": 10,
      "exercise_type": "deadlift",
      "expected_keypoints": true
    },
    {
      "name": "test_bench_press.mp4",
      "resolution": "1280x720",
      "fps": 30,
      "duration": 10,
      "exercise_type": "bench_press",
      "expected_keypoints": true
    }
  ],
  "performance": [
    {
      "name": "test_480p.mp4",
      "resolution": "854x480",
      "fps": 30,
      "duration": 30,
      "expected_processing_time_ms": 15000,
      "target": "fast_processing"
    },
    {
      "name": "test_1080p.mp4",
      "resolution": "1920x1080",
      "fps": 30,
      "duration": 30,
      "expected_processing_time_ms": 45000,
      "target": "quality_processing"
    },
    {
      "name": "test_4k.mp4",
      "resolution": "3840x2160",
      "fps": 30,
      "duration": 10,
      "expected_processing_time_ms": 20000,
      "target": "stress_test"
    },
    {
      "name": "test_60fps.mp4",
      "resolution": "1280x720",
      "fps": 60,
      "duration": 10,
      "expected_processing_time_ms": 15000,
      "target": "high_framerate"
    },
    {
      "name": "test_long.mp4",
      "resolution": "1280x720",
      "fps": 30,
      "duration": 300,
      "expected_processing_time_ms": 300000,
      "target": "memory_test"
    }
  ],
  "edge_cases": [
    {
      "name": "test_1second.mp4",
      "resolution": "1280x720",
      "fps": 30,
      "duration": 1,
      "expected_behavior": "should_process_successfully"
    },
    {
      "name": "test_144p.mp4",
      "resolution": "256x144",
      "fps": 30,
      "duration": 10,
      "expected_behavior": "should_process_with_warning"
    },
    {
      "name": "test_15fps.mp4",
      "resolution": "1280x720",
      "fps": 15,
      "duration": 10,
      "expected_behavior": "should_process_successfully"
    },
    {
      "name": "corrupted.mp4",
      "expected_behavior": "should_fail_gracefully"
    },
    {
      "name": "wrong_format.mp4",
      "expected_behavior": "should_reject_with_error"
    },
    {
      "name": "empty.mp4",
      "expected_behavior": "should_reject_with_error"
    }
  ]
}
EOF

echo ""
echo "📝 Created test metadata: $TEST_DATA_DIR/test_videos.json"

# Generate test summary
echo ""
echo "📊 Test Video Library Summary"
echo "=============================="
echo ""
echo "Standard Videos: $(ls -1 "$VIDEO_DIR/standard" | wc -l | tr -d ' ')"
ls -lh "$VIDEO_DIR/standard" | tail -n +2 | awk '{print "  " $9 " (" $5 ")"}'

echo ""
echo "Performance Videos: $(ls -1 "$VIDEO_DIR/performance" | wc -l | tr -d ' ')"
ls -lh "$VIDEO_DIR/performance" | tail -n +2 | awk '{print "  " $9 " (" $5 ")"}'

echo ""
echo "Edge Case Videos: $(ls -1 "$VIDEO_DIR/edge_cases" | wc -l | tr -d ' ')"
ls -lh "$VIDEO_DIR/edge_cases" | tail -n +2 | awk '{print "  " $9 " (" $5 ")"}'

echo ""
TOTAL_SIZE=$(du -sh "$VIDEO_DIR" | awk '{print $1}')
echo "Total Size: $TOTAL_SIZE"

echo ""
echo "✅ Test video library setup complete!"
echo ""
echo "Usage:"
echo "  Standard tests:     test_data/videos/standard/"
echo "  Performance tests:  test_data/videos/performance/"
echo "  Edge cases:         test_data/videos/edge_cases/"
echo "  Metadata:           test_data/test_videos.json"
