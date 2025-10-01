use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{error, info, warn};

/// Service for video processing operations using FFmpeg
pub struct VideoProcessingService {
    ffmpeg_path: String,
    ffprobe_path: String,
}

impl VideoProcessingService {
    /// Create a new VideoProcessingService
    pub fn new() -> Self {
        Self {
            ffmpeg_path: "ffmpeg".to_string(),
            ffprobe_path: "ffprobe".to_string(),
        }
    }

    /// Create service with custom FFmpeg paths
    pub fn with_paths(ffmpeg_path: String, ffprobe_path: String) -> Self {
        Self {
            ffmpeg_path,
            ffprobe_path,
        }
    }

    /// Extract video metadata (duration, resolution, format, codec)
    pub async fn extract_metadata(&self, video_path: &Path) -> Result<VideoInfo> {
        info!("Extracting metadata from video: {:?}", video_path);

        let output = Command::new(&self.ffprobe_path)
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=width,height,duration,codec_name,r_frame_rate",
                "-show_entries",
                "format=duration,size,format_name",
                "-of",
                "json",
                video_path.to_str().unwrap(),
            ])
            .output()
            .context("Failed to execute ffprobe")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("FFprobe failed: {}", stderr));
        }

        let json_output = String::from_utf8(output.stdout)?;
        let metadata: FfprobeOutput = serde_json::from_str(&json_output)
            .context("Failed to parse ffprobe output")?;

        self.parse_video_info(metadata)
    }

    /// Convert video to MP4 format with optimal settings
    pub async fn convert_to_mp4(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        info!("Converting video to MP4: {:?} -> {:?}", input_path, output_path);

        let status = Command::new(&self.ffmpeg_path)
            .args([
                "-i",
                input_path.to_str().unwrap(),
                "-c:v",
                "libx264", // H.264 codec
                "-preset",
                "medium", // Balance speed/quality
                "-crf",
                "23", // Quality setting (lower = better)
                "-c:a",
                "aac", // AAC audio codec
                "-b:a",
                "128k", // Audio bitrate
                "-movflags",
                "+faststart", // Enable streaming
                "-y", // Overwrite output
                output_path.to_str().unwrap(),
            ])
            .status()
            .context("Failed to execute ffmpeg")?;

        if !status.success() {
            return Err(anyhow::anyhow!("FFmpeg conversion failed"));
        }

        info!("Successfully converted video to MP4");
        Ok(())
    }

    /// Extract frames from video at specified FPS
    pub async fn extract_frames(
        &self,
        video_path: &Path,
        output_dir: &Path,
        fps: u32,
    ) -> Result<Vec<PathBuf>> {
        info!("Extracting frames at {}fps from {:?}", fps, video_path);

        std::fs::create_dir_all(output_dir)?;

        let frame_pattern = output_dir.join("frame_%04d.jpg");

        let status = Command::new(&self.ffmpeg_path)
            .args([
                "-i",
                video_path.to_str().unwrap(),
                "-vf",
                &format!("fps={}", fps),
                "-q:v",
                "2", // JPEG quality
                "-y",
                frame_pattern.to_str().unwrap(),
            ])
            .status()
            .context("Failed to execute ffmpeg for frame extraction")?;

        if !status.success() {
            return Err(anyhow::anyhow!("FFmpeg frame extraction failed"));
        }

        // Collect extracted frame paths
        let mut frames = Vec::new();
        let entries = std::fs::read_dir(output_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("jpg") {
                frames.push(path);
            }
        }

        frames.sort();
        info!("Extracted {} frames", frames.len());
        Ok(frames)
    }

    /// Generate thumbnail from video at specified timestamp
    pub async fn generate_thumbnail(
        &self,
        video_path: &Path,
        output_path: &Path,
        timestamp_seconds: f64,
    ) -> Result<()> {
        info!(
            "Generating thumbnail at {}s from {:?}",
            timestamp_seconds, video_path
        );

        let status = Command::new(&self.ffmpeg_path)
            .args([
                "-ss",
                &timestamp_seconds.to_string(),
                "-i",
                video_path.to_str().unwrap(),
                "-vframes",
                "1",
                "-q:v",
                "2",
                "-y",
                output_path.to_str().unwrap(),
            ])
            .status()
            .context("Failed to execute ffmpeg for thumbnail")?;

        if !status.success() {
            return Err(anyhow::anyhow!("FFmpeg thumbnail generation failed"));
        }

        info!("Successfully generated thumbnail");
        Ok(())
    }

    /// Validate video file format and quality
    pub async fn validate_video(&self, video_path: &Path) -> Result<ValidationResult> {
        let metadata = self.extract_metadata(video_path).await?;

        let mut issues = Vec::new();

        // Check minimum resolution (360p)
        if metadata.width < 640 || metadata.height < 360 {
            issues.push("Video resolution too low (minimum 640x360)".to_string());
        }

        // Check maximum resolution (4K)
        if metadata.width > 3840 || metadata.height > 2160 {
            warn!("Video resolution very high ({}x{}), may impact processing speed",
                  metadata.width, metadata.height);
        }

        // Check duration (minimum 1s, maximum 10 minutes)
        if metadata.duration_seconds < 1.0 {
            issues.push("Video too short (minimum 1 second)".to_string());
        }
        if metadata.duration_seconds > 600.0 {
            issues.push("Video too long (maximum 10 minutes)".to_string());
        }

        // Check codec compatibility
        if !Self::is_supported_codec(&metadata.video_codec) {
            issues.push(format!(
                "Unsupported video codec: {} (supported: h264, h265, vp8, vp9)",
                metadata.video_codec
            ));
        }

        let is_valid = issues.is_empty();
        Ok(ValidationResult {
            is_valid,
            issues,
            metadata,
        })
    }

    /// Check if codec is supported
    fn is_supported_codec(codec: &str) -> bool {
        matches!(
            codec.to_lowercase().as_str(),
            "h264" | "h265" | "hevc" | "vp8" | "vp9" | "av1"
        )
    }

    /// Parse ffprobe output into VideoInfo
    fn parse_video_info(&self, metadata: FfprobeOutput) -> Result<VideoInfo> {
        let stream = metadata
            .streams
            .first()
            .context("No video stream found")?;

        let format = metadata.format;

        Ok(VideoInfo {
            width: stream.width,
            height: stream.height,
            duration_seconds: stream
                .duration
                .clone()
                .or(format.duration.clone())
                .and_then(|d| d.parse::<f64>().ok())
                .unwrap_or(0.0),
            video_codec: stream.codec_name.clone(),
            fps: Self::parse_frame_rate(&stream.r_frame_rate),
            size_bytes: format
                .size
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0),
            format_name: format.format_name,
        })
    }

    /// Parse frame rate string (e.g., "30/1" -> 30.0)
    fn parse_frame_rate(rate_str: &str) -> f64 {
        if let Some((num, den)) = rate_str.split_once('/') {
            if let (Ok(n), Ok(d)) = (num.parse::<f64>(), den.parse::<f64>()) {
                if d != 0.0 {
                    return n / d;
                }
            }
        }
        0.0
    }
}

impl Default for VideoProcessingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Video metadata information
#[derive(Debug, Clone)]
pub struct VideoInfo {
    pub width: i32,
    pub height: i32,
    pub duration_seconds: f64,
    pub video_codec: String,
    pub fps: f64,
    pub size_bytes: i64,
    pub format_name: String,
}

impl VideoInfo {
    pub fn resolution_string(&self) -> String {
        format!("{}x{}", self.width, self.height)
    }
}

/// Video validation result
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub issues: Vec<String>,
    pub metadata: VideoInfo,
}

// FFprobe JSON output structures
#[derive(Debug, serde::Deserialize)]
struct FfprobeOutput {
    streams: Vec<FfprobeStream>,
    format: FfprobeFormat,
}

#[derive(Debug, serde::Deserialize)]
struct FfprobeStream {
    width: i32,
    height: i32,
    duration: Option<String>,
    codec_name: String,
    r_frame_rate: String,
}

#[derive(Debug, serde::Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
    size: Option<String>,
    format_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frame_rate() {
        assert_eq!(VideoProcessingService::parse_frame_rate("30/1"), 30.0);
        assert_eq!(VideoProcessingService::parse_frame_rate("60/1"), 60.0);
        assert_eq!(VideoProcessingService::parse_frame_rate("24000/1001"), 23.976023976023978);
        assert_eq!(VideoProcessingService::parse_frame_rate("invalid"), 0.0);
    }

    #[test]
    fn test_is_supported_codec() {
        assert!(VideoProcessingService::is_supported_codec("h264"));
        assert!(VideoProcessingService::is_supported_codec("H264"));
        assert!(VideoProcessingService::is_supported_codec("hevc"));
        assert!(VideoProcessingService::is_supported_codec("vp9"));
        assert!(!VideoProcessingService::is_supported_codec("wmv"));
        assert!(!VideoProcessingService::is_supported_codec("unknown"));
    }
}
