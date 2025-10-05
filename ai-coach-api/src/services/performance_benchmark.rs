use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::performance_profiler::{PerformanceMetrics, PipelineProfiler};
use super::processing_config::{ProcessingConfig, OptimizationRecommendation};

/// Benchmark results for vision processing pipeline
#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkResults {
    pub test_name: String,
    pub video_info: VideoInfo,
    pub configuration: ProcessingConfig,
    pub metrics: BenchmarkMetrics,
    pub recommendations: OptimizationRecommendation,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub duration_seconds: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub size_mb: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    pub total_processing_time_ms: u64,
    pub processing_ratio: f64, // Processing time / Video duration
    pub frames_processed: u32,
    pub avg_frame_time_ms: f64,
    pub peak_memory_mb: f64,
    pub throughput_fps: f64,
    pub operation_breakdown: HashMap<String, u64>,
    pub bottlenecks: Vec<String>,
    pub targets_met: TargetStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TargetStatus {
    pub processing_ratio_target: f64,
    pub processing_ratio_met: bool,
    pub memory_target_mb: f64,
    pub memory_target_met: bool,
    pub throughput_target_fps: f64,
    pub throughput_met: bool,
}

/// Performance benchmark suite
pub struct PerformanceBenchmark {
    profiler: PipelineProfiler,
    video_info: VideoInfo,
    config: ProcessingConfig,
    test_name: String,
}

impl PerformanceBenchmark {
    pub fn new(test_name: impl Into<String>, video_info: VideoInfo, config: ProcessingConfig) -> Self {
        Self {
            profiler: PipelineProfiler::new(),
            video_info,
            config,
            test_name: test_name.into(),
        }
    }

    /// Record a performance metric
    pub fn record(&mut self, metric: PerformanceMetrics) {
        self.profiler.record(metric);
    }

    /// Generate benchmark results
    pub fn finish(self, frames_processed: u32) -> BenchmarkResults {
        let summary = self.profiler.summary();
        let total_time_ms = summary.total_duration_ms;
        let video_duration_ms = (self.video_info.duration_seconds * 1000.0) as u64;

        let processing_ratio = if video_duration_ms > 0 {
            total_time_ms as f64 / video_duration_ms as f64
        } else {
            0.0
        };

        let avg_frame_time = if frames_processed > 0 {
            total_time_ms as f64 / frames_processed as f64
        } else {
            0.0
        };

        let throughput_fps = if total_time_ms > 0 {
            (frames_processed as f64 * 1000.0) / total_time_ms as f64
        } else {
            0.0
        };

        // Check targets
        let targets_met = TargetStatus {
            processing_ratio_target: 2.0,
            processing_ratio_met: processing_ratio < 2.0,
            memory_target_mb: 2048.0,
            memory_target_met: summary.peak_memory_mb < 2048.0,
            throughput_target_fps: 10.0, // Process at least 10 frames per second
            throughput_met: throughput_fps >= 10.0,
        };

        let metrics = BenchmarkMetrics {
            total_processing_time_ms: total_time_ms,
            processing_ratio,
            frames_processed,
            avg_frame_time_ms: avg_frame_time,
            peak_memory_mb: summary.peak_memory_mb,
            throughput_fps,
            operation_breakdown: summary.operation_durations,
            bottlenecks: summary.bottlenecks,
            targets_met,
        };

        let recommendations = OptimizationRecommendation::from_video_info(
            self.video_info.duration_seconds,
            self.video_info.width,
            self.video_info.height,
            self.video_info.fps,
        );

        BenchmarkResults {
            test_name: self.test_name,
            video_info: self.video_info,
            configuration: self.config,
            metrics,
            recommendations,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Benchmark comparison between configurations
#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub baseline: BenchmarkResults,
    pub optimized: BenchmarkResults,
    pub improvements: PerformanceImprovements,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceImprovements {
    pub speedup_factor: f64,
    pub time_saved_ms: i64,
    pub memory_reduction_mb: f64,
    pub throughput_increase_fps: f64,
    pub summary: String,
}

impl BenchmarkComparison {
    pub fn new(baseline: BenchmarkResults, optimized: BenchmarkResults) -> Self {
        let speedup = if optimized.metrics.total_processing_time_ms > 0 {
            baseline.metrics.total_processing_time_ms as f64
                / optimized.metrics.total_processing_time_ms as f64
        } else {
            1.0
        };

        let time_saved = baseline.metrics.total_processing_time_ms as i64
            - optimized.metrics.total_processing_time_ms as i64;

        let memory_reduction =
            baseline.metrics.peak_memory_mb - optimized.metrics.peak_memory_mb;

        let throughput_increase =
            optimized.metrics.throughput_fps - baseline.metrics.throughput_fps;

        let summary = format!(
            "{}x speedup, saved {}ms ({:.1}%), memory reduced by {:.1}MB, throughput increased by {:.1}fps",
            speedup,
            time_saved,
            (time_saved as f64 / baseline.metrics.total_processing_time_ms as f64) * 100.0,
            memory_reduction,
            throughput_increase
        );

        Self {
            baseline,
            optimized,
            improvements: PerformanceImprovements {
                speedup_factor: speedup,
                time_saved_ms: time_saved,
                memory_reduction_mb: memory_reduction,
                throughput_increase_fps: throughput_increase,
                summary,
            },
        }
    }
}

/// Generate a comprehensive benchmark report
pub fn generate_report(comparisons: Vec<BenchmarkComparison>) -> String {
    let mut report = String::new();

    report.push_str("# Performance Benchmark Report\n\n");
    report.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

    report.push_str("## Summary\n\n");
    report.push_str(&format!("Total benchmarks: {}\n\n", comparisons.len()));

    for (i, comparison) in comparisons.iter().enumerate() {
        report.push_str(&format!("### Benchmark {}: {}\n\n", i + 1, comparison.baseline.test_name));

        report.push_str("**Video Info:**\n");
        report.push_str(&format!("- Duration: {:.1}s\n", comparison.baseline.video_info.duration_seconds));
        report.push_str(&format!("- Resolution: {}x{}\n", comparison.baseline.video_info.width, comparison.baseline.video_info.height));
        report.push_str(&format!("- FPS: {:.1}\n", comparison.baseline.video_info.fps));
        report.push_str(&format!("- Size: {:.1}MB\n\n", comparison.baseline.video_info.size_mb));

        report.push_str("**Baseline Performance:**\n");
        report.push_str(&format!("- Processing time: {}ms\n", comparison.baseline.metrics.total_processing_time_ms));
        report.push_str(&format!("- Processing ratio: {:.2}x\n", comparison.baseline.metrics.processing_ratio));
        report.push_str(&format!("- Frames processed: {}\n", comparison.baseline.metrics.frames_processed));
        report.push_str(&format!("- Throughput: {:.1}fps\n", comparison.baseline.metrics.throughput_fps));
        report.push_str(&format!("- Peak memory: {:.1}MB\n\n", comparison.baseline.metrics.peak_memory_mb));

        report.push_str("**Optimized Performance:**\n");
        report.push_str(&format!("- Processing time: {}ms\n", comparison.optimized.metrics.total_processing_time_ms));
        report.push_str(&format!("- Processing ratio: {:.2}x\n", comparison.optimized.metrics.processing_ratio));
        report.push_str(&format!("- Throughput: {:.1}fps\n", comparison.optimized.metrics.throughput_fps));
        report.push_str(&format!("- Peak memory: {:.1}MB\n\n", comparison.optimized.metrics.peak_memory_mb));

        report.push_str("**Improvements:**\n");
        report.push_str(&format!("- {}\n", comparison.improvements.summary));
        report.push_str(&format!("- Speedup: {:.2}x\n", comparison.improvements.speedup_factor));
        report.push_str(&format!("- Time saved: {}ms\n", comparison.improvements.time_saved_ms));
        report.push_str(&format!("- Memory reduction: {:.1}MB\n\n", comparison.improvements.memory_reduction_mb));

        report.push_str("**Targets Met:**\n");
        report.push_str(&format!("- Processing ratio < 2.0: {}\n",
            if comparison.optimized.metrics.targets_met.processing_ratio_met { "✅" } else { "❌" }));
        report.push_str(&format!("- Memory < 2GB: {}\n",
            if comparison.optimized.metrics.targets_met.memory_target_met { "✅" } else { "❌" }));
        report.push_str(&format!("- Throughput >= 10fps: {}\n\n",
            if comparison.optimized.metrics.targets_met.throughput_met { "✅" } else { "❌" }));

        if !comparison.optimized.metrics.bottlenecks.is_empty() {
            report.push_str("**Remaining Bottlenecks:**\n");
            for bottleneck in &comparison.optimized.metrics.bottlenecks {
                report.push_str(&format!("- {}\n", bottleneck));
            }
            report.push_str("\n");
        }

        if !comparison.optimized.recommendations.rationale.is_empty() {
            report.push_str("**Further Optimization Recommendations:**\n");
            for rec in &comparison.optimized.recommendations.rationale {
                report.push_str(&format!("- {}\n", rec));
            }
            report.push_str("\n");
        }

        report.push_str("---\n\n");
    }

    // Overall summary
    let avg_speedup: f64 = comparisons.iter()
        .map(|c| c.improvements.speedup_factor)
        .sum::<f64>() / comparisons.len() as f64;

    let all_targets_met = comparisons.iter()
        .all(|c| c.optimized.metrics.targets_met.processing_ratio_met
            && c.optimized.metrics.targets_met.memory_target_met
            && c.optimized.metrics.targets_met.throughput_met);

    report.push_str("## Overall Results\n\n");
    report.push_str(&format!("- Average speedup: {:.2}x\n", avg_speedup));
    report.push_str(&format!("- All targets met: {}\n", if all_targets_met { "✅ Yes" } else { "❌ No" }));

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_basic() {
        let video_info = VideoInfo {
            duration_seconds: 10.0,
            width: 1920,
            height: 1080,
            fps: 30.0,
            size_mb: 50.0,
        };

        let config = ProcessingConfig::default();
        let mut benchmark = PerformanceBenchmark::new("test", video_info, config);

        let metric = PerformanceMetrics {
            operation: "test_op".to_string(),
            duration_ms: 1000,
            memory_usage_mb: Some(512.0),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        benchmark.record(metric);
        let results = benchmark.finish(100);

        assert_eq!(results.metrics.frames_processed, 100);
        assert!(results.metrics.total_processing_time_ms >= 1000);
    }

    #[test]
    fn test_benchmark_comparison() {
        let video_info = VideoInfo {
            duration_seconds: 10.0,
            width: 1920,
            height: 1080,
            fps: 30.0,
            size_mb: 50.0,
        };

        let baseline = BenchmarkResults {
            test_name: "baseline".to_string(),
            video_info: video_info.clone(),
            configuration: ProcessingConfig::default(),
            metrics: BenchmarkMetrics {
                total_processing_time_ms: 20000,
                processing_ratio: 2.0,
                frames_processed: 300,
                avg_frame_time_ms: 66.67,
                peak_memory_mb: 1500.0,
                throughput_fps: 15.0,
                operation_breakdown: HashMap::new(),
                bottlenecks: vec![],
                targets_met: TargetStatus {
                    processing_ratio_target: 2.0,
                    processing_ratio_met: false,
                    memory_target_mb: 2048.0,
                    memory_target_met: true,
                    throughput_target_fps: 10.0,
                    throughput_met: true,
                },
            },
            recommendations: OptimizationRecommendation::from_video_info(10.0, 1920, 1080, 30.0),
            timestamp: chrono::Utc::now(),
        };

        let optimized = BenchmarkResults {
            test_name: "optimized".to_string(),
            video_info,
            configuration: ProcessingConfig::low_latency(),
            metrics: BenchmarkMetrics {
                total_processing_time_ms: 10000,
                processing_ratio: 1.0,
                frames_processed: 150,
                avg_frame_time_ms: 66.67,
                peak_memory_mb: 1000.0,
                throughput_fps: 15.0,
                operation_breakdown: HashMap::new(),
                bottlenecks: vec![],
                targets_met: TargetStatus {
                    processing_ratio_target: 2.0,
                    processing_ratio_met: true,
                    memory_target_mb: 2048.0,
                    memory_target_met: true,
                    throughput_target_fps: 10.0,
                    throughput_met: true,
                },
            },
            recommendations: OptimizationRecommendation::from_video_info(10.0, 1920, 1080, 30.0),
            timestamp: chrono::Utc::now(),
        };

        let comparison = BenchmarkComparison::new(baseline, optimized);

        assert_eq!(comparison.improvements.speedup_factor, 2.0);
        assert_eq!(comparison.improvements.time_saved_ms, 10000);
        assert_eq!(comparison.improvements.memory_reduction_mb, 500.0);
    }
}
