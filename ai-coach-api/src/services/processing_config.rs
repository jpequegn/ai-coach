use serde::{Deserialize, Serialize};

/// Processing configuration with performance optimizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Frame sampling strategy
    pub frame_sampling: FrameSamplingConfig,

    /// Resolution optimization
    pub resolution: ResolutionConfig,

    /// Inference optimization
    pub inference: InferenceConfig,

    /// Caching configuration
    pub caching: CachingConfig,

    /// Storage optimization
    pub storage: StorageConfig,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            frame_sampling: FrameSamplingConfig::default(),
            resolution: ResolutionConfig::default(),
            inference: InferenceConfig::default(),
            caching: CachingConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

impl ProcessingConfig {
    /// Create optimized configuration for low latency
    pub fn low_latency() -> Self {
        Self {
            frame_sampling: FrameSamplingConfig {
                strategy: SamplingStrategy::Fixed,
                target_fps: 10,
                adaptive_min_fps: 5,
                adaptive_max_fps: 15,
            },
            resolution: ResolutionConfig {
                strategy: ResolutionStrategy::FixedDownscale,
                target_height: 480,
                max_height: 720,
                maintain_aspect_ratio: true,
            },
            inference: InferenceConfig {
                batch_size: 4,
                enable_gpu: true,
                quantization: QuantizationType::FP16,
                enable_tensorrt: false,
            },
            caching: CachingConfig {
                enabled: true,
                cache_pose_results: true,
                cache_ttl_seconds: 3600,
            },
            storage: StorageConfig {
                compress_after_analysis: true,
                cleanup_temp_files: true,
                retention_days: 30,
            },
        }
    }

    /// Create optimized configuration for high accuracy
    pub fn high_accuracy() -> Self {
        Self {
            frame_sampling: FrameSamplingConfig {
                strategy: SamplingStrategy::Adaptive,
                target_fps: 30,
                adaptive_min_fps: 15,
                adaptive_max_fps: 30,
            },
            resolution: ResolutionConfig {
                strategy: ResolutionStrategy::Adaptive,
                target_height: 720,
                max_height: 1080,
                maintain_aspect_ratio: true,
            },
            inference: InferenceConfig {
                batch_size: 1,
                enable_gpu: true,
                quantization: QuantizationType::FP32,
                enable_tensorrt: true,
            },
            caching: CachingConfig {
                enabled: true,
                cache_pose_results: true,
                cache_ttl_seconds: 7200,
            },
            storage: StorageConfig {
                compress_after_analysis: false,
                cleanup_temp_files: true,
                retention_days: 90,
            },
        }
    }

    /// Create balanced configuration
    pub fn balanced() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameSamplingConfig {
    pub strategy: SamplingStrategy,
    pub target_fps: u32,
    pub adaptive_min_fps: u32,
    pub adaptive_max_fps: u32,
}

impl Default for FrameSamplingConfig {
    fn default() -> Self {
        Self {
            strategy: SamplingStrategy::Adaptive,
            target_fps: 15,
            adaptive_min_fps: 10,
            adaptive_max_fps: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SamplingStrategy {
    /// Fixed FPS sampling
    Fixed,
    /// Adaptive sampling based on motion
    Adaptive,
    /// Key frame only
    KeyFrames,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionConfig {
    pub strategy: ResolutionStrategy,
    pub target_height: u32,
    pub max_height: u32,
    pub maintain_aspect_ratio: bool,
}

impl Default for ResolutionConfig {
    fn default() -> Self {
        Self {
            strategy: ResolutionStrategy::Adaptive,
            target_height: 720,
            max_height: 1080,
            maintain_aspect_ratio: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResolutionStrategy {
    /// No downscaling
    Original,
    /// Fixed downscale to target
    FixedDownscale,
    /// Adaptive based on source resolution
    Adaptive,
}

impl ResolutionConfig {
    /// Determine optimal processing resolution for input video
    pub fn optimal_resolution(&self, input_height: u32) -> u32 {
        match self.strategy {
            ResolutionStrategy::Original => input_height.min(self.max_height),
            ResolutionStrategy::FixedDownscale => self.target_height,
            ResolutionStrategy::Adaptive => {
                if input_height <= self.target_height {
                    input_height
                } else if input_height <= self.max_height {
                    // Downscale proportionally
                    (input_height + self.target_height) / 2
                } else {
                    self.target_height
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub batch_size: usize,
    pub enable_gpu: bool,
    pub quantization: QuantizationType,
    pub enable_tensorrt: bool,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            batch_size: 4,
            enable_gpu: true,
            quantization: QuantizationType::FP16,
            enable_tensorrt: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QuantizationType {
    /// Full precision
    FP32,
    /// Half precision (faster, good accuracy)
    FP16,
    /// 8-bit integer (fastest, some accuracy loss)
    INT8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachingConfig {
    pub enabled: bool,
    pub cache_pose_results: bool,
    pub cache_ttl_seconds: u64,
}

impl Default for CachingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_pose_results: true,
            cache_ttl_seconds: 3600, // 1 hour
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub compress_after_analysis: bool,
    pub cleanup_temp_files: bool,
    pub retention_days: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            compress_after_analysis: true,
            cleanup_temp_files: true,
            retention_days: 30,
        }
    }
}

/// Performance optimization recommendations based on video characteristics
pub struct OptimizationRecommendation {
    pub recommended_fps: u32,
    pub recommended_resolution: u32,
    pub recommended_batch_size: usize,
    pub estimated_speedup: f32,
    pub rationale: Vec<String>,
}

impl OptimizationRecommendation {
    /// Generate recommendations based on video properties
    pub fn from_video_info(
        duration_seconds: f64,
        width: u32,
        height: u32,
        original_fps: f32,
    ) -> Self {
        let mut rationale = Vec::new();
        let mut recommended_fps = 15;
        let mut recommended_resolution = 720;
        let mut recommended_batch_size = 4;
        let mut speedup_factors = Vec::new();

        // FPS optimization
        if original_fps > 30.0 {
            recommended_fps = 15;
            let fps_speedup = original_fps / recommended_fps as f32;
            speedup_factors.push(fps_speedup);
            rationale.push(format!(
                "Downsample from {:.0}fps to {}fps for {}x speedup",
                original_fps, recommended_fps, fps_speedup
            ));
        } else if original_fps > 15.0 {
            recommended_fps = 10;
            let fps_speedup = original_fps / recommended_fps as f32;
            speedup_factors.push(fps_speedup);
            rationale.push(format!(
                "Downsample from {:.0}fps to {}fps for {:.1}x speedup",
                original_fps, recommended_fps, fps_speedup
            ));
        }

        // Resolution optimization
        if height > 1080 {
            recommended_resolution = 720;
            let res_speedup = (height as f32 / recommended_resolution as f32).powi(2);
            speedup_factors.push(res_speedup);
            rationale.push(format!(
                "Downscale from {}p to {}p for {:.1}x speedup",
                height, recommended_resolution, res_speedup
            ));
        } else if height > 720 {
            recommended_resolution = 720;
            rationale.push("Process at 720p for optimal balance".to_string());
        } else {
            recommended_resolution = height;
            rationale.push("Process at original resolution".to_string());
        }

        // Batch size optimization based on video length
        if duration_seconds > 60.0 {
            recommended_batch_size = 8;
            rationale.push("Use batch size 8 for long videos".to_string());
        } else if duration_seconds > 30.0 {
            recommended_batch_size = 4;
            rationale.push("Use batch size 4 for medium videos".to_string());
        } else {
            recommended_batch_size = 2;
            rationale.push("Use batch size 2 for short videos".to_string());
        }

        let estimated_speedup = speedup_factors.iter().product::<f32>().max(1.0);

        Self {
            recommended_fps,
            recommended_resolution,
            recommended_batch_size,
            estimated_speedup,
            rationale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_adaptive() {
        let config = ResolutionConfig {
            strategy: ResolutionStrategy::Adaptive,
            target_height: 720,
            max_height: 1080,
            maintain_aspect_ratio: true,
        };

        assert_eq!(config.optimal_resolution(480), 480); // Keep original if lower
        assert_eq!(config.optimal_resolution(720), 720); // Keep if at target
        assert_eq!(config.optimal_resolution(1080), 900); // Proportional between target and max
        assert_eq!(config.optimal_resolution(2160), 720); // Downscale to target if over max
    }

    #[test]
    fn test_optimization_recommendation() {
        let rec = OptimizationRecommendation::from_video_info(
            120.0, // 2 minute video
            1920,  // 1080p width
            1080,  // 1080p height
            60.0,  // 60fps
        );

        assert_eq!(rec.recommended_fps, 15);
        assert_eq!(rec.recommended_resolution, 720);
        assert_eq!(rec.recommended_batch_size, 8);
        assert!(rec.estimated_speedup > 1.0);
        assert!(!rec.rationale.is_empty());
    }

    #[test]
    fn test_config_presets() {
        let low_latency = ProcessingConfig::low_latency();
        assert_eq!(low_latency.frame_sampling.target_fps, 10);
        assert_eq!(low_latency.resolution.target_height, 480);
        assert_eq!(low_latency.inference.batch_size, 4);

        let high_accuracy = ProcessingConfig::high_accuracy();
        assert_eq!(high_accuracy.frame_sampling.target_fps, 30);
        assert_eq!(high_accuracy.resolution.target_height, 720);
        assert_eq!(high_accuracy.inference.quantization, QuantizationType::FP32);
    }
}
