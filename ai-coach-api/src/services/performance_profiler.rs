use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Performance metrics for a single operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub operation: String,
    pub duration_ms: u64,
    pub memory_usage_mb: Option<f64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, String>,
}

/// Performance profiler for tracking operation metrics
#[derive(Debug, Clone)]
pub struct PerformanceProfiler {
    operation_name: String,
    start_time: Instant,
    metadata: HashMap<String, String>,
}

impl PerformanceProfiler {
    /// Start profiling an operation
    pub fn start(operation_name: impl Into<String>) -> Self {
        Self {
            operation_name: operation_name.into(),
            start_time: Instant::now(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the profiling session
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// End profiling and return metrics
    pub fn end(self) -> PerformanceMetrics {
        let duration = self.start_time.elapsed();
        let duration_ms = duration.as_millis() as u64;

        let metrics = PerformanceMetrics {
            operation: self.operation_name.clone(),
            duration_ms,
            memory_usage_mb: Self::get_current_memory_usage(),
            timestamp: chrono::Utc::now(),
            metadata: self.metadata,
        };

        info!(
            operation = %self.operation_name,
            duration_ms = duration_ms,
            "Performance metric recorded"
        );

        metrics
    }

    /// Get current memory usage in MB (platform-specific)
    fn get_current_memory_usage() -> Option<f64> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<f64>() {
                                return Some(kb / 1024.0); // Convert KB to MB
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

/// Pipeline performance tracker
#[derive(Debug, Default)]
pub struct PipelineProfiler {
    metrics: Vec<PerformanceMetrics>,
    pipeline_start: Option<Instant>,
}

impl PipelineProfiler {
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
            pipeline_start: Some(Instant::now()),
        }
    }

    /// Record a performance metric
    pub fn record(&mut self, metric: PerformanceMetrics) {
        self.metrics.push(metric);
    }

    /// Get total pipeline duration
    pub fn total_duration_ms(&self) -> u64 {
        if let Some(start) = self.pipeline_start {
            start.elapsed().as_millis() as u64
        } else {
            0
        }
    }

    /// Get summary statistics
    pub fn summary(&self) -> PipelineSummary {
        let total_duration = self.total_duration_ms();
        let operation_durations: HashMap<String, u64> = self
            .metrics
            .iter()
            .map(|m| (m.operation.clone(), m.duration_ms))
            .collect();

        let max_memory = self
            .metrics
            .iter()
            .filter_map(|m| m.memory_usage_mb)
            .fold(0.0f64, f64::max);

        PipelineSummary {
            total_duration_ms: total_duration,
            operation_count: self.metrics.len(),
            operation_durations,
            peak_memory_mb: max_memory,
            bottlenecks: self.identify_bottlenecks(),
        }
    }

    /// Identify performance bottlenecks (operations taking >20% of total time)
    fn identify_bottlenecks(&self) -> Vec<String> {
        let total = self.total_duration_ms() as f64;
        if total == 0.0 {
            return vec![];
        }

        self.metrics
            .iter()
            .filter(|m| (m.duration_ms as f64 / total) > 0.2)
            .map(|m| format!("{} ({}ms, {:.1}%)",
                m.operation,
                m.duration_ms,
                (m.duration_ms as f64 / total) * 100.0
            ))
            .collect()
    }

    /// Check if performance targets are met
    pub fn check_targets(&self, video_duration_seconds: f64) -> PerformanceTargets {
        let total_ms = self.total_duration_ms();
        let video_ms = (video_duration_seconds * 1000.0) as u64;
        let processing_ratio = total_ms as f64 / video_ms as f64;

        PerformanceTargets {
            target_processing_ratio: 2.0,
            actual_processing_ratio: processing_ratio,
            target_met: processing_ratio < 2.0,
            target_memory_mb: 2048.0,
            actual_peak_memory_mb: self.summary().peak_memory_mb,
            memory_target_met: self.summary().peak_memory_mb < 2048.0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineSummary {
    pub total_duration_ms: u64,
    pub operation_count: usize,
    pub operation_durations: HashMap<String, u64>,
    pub peak_memory_mb: f64,
    pub bottlenecks: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceTargets {
    pub target_processing_ratio: f64,
    pub actual_processing_ratio: f64,
    pub target_met: bool,
    pub target_memory_mb: f64,
    pub actual_peak_memory_mb: f64,
    pub memory_target_met: bool,
}

/// Macro for easy profiling
#[macro_export]
macro_rules! profile_operation {
    ($name:expr, $block:block) => {{
        let mut profiler = $crate::services::performance_profiler::PerformanceProfiler::start($name);
        let result = $block;
        let _metrics = profiler.end();
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler_basic() {
        let mut profiler = PerformanceProfiler::start("test_operation");
        thread::sleep(Duration::from_millis(10));
        profiler.add_metadata("test_key", "test_value");

        let metrics = profiler.end();

        assert_eq!(metrics.operation, "test_operation");
        assert!(metrics.duration_ms >= 10);
        assert_eq!(metrics.metadata.get("test_key").unwrap(), "test_value");
    }

    #[test]
    fn test_pipeline_profiler() {
        let mut pipeline = PipelineProfiler::new();

        let metric1 = PerformanceMetrics {
            operation: "step1".to_string(),
            duration_ms: 100,
            memory_usage_mb: Some(512.0),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        let metric2 = PerformanceMetrics {
            operation: "step2".to_string(),
            duration_ms: 200,
            memory_usage_mb: Some(1024.0),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        pipeline.record(metric1);
        pipeline.record(metric2);

        let summary = pipeline.summary();
        assert_eq!(summary.operation_count, 2);
        assert_eq!(summary.peak_memory_mb, 1024.0);
    }

    #[test]
    fn test_performance_targets() {
        let mut pipeline = PipelineProfiler::new();
        thread::sleep(Duration::from_millis(100));

        let targets = pipeline.check_targets(1.0); // 1 second video

        // 100ms processing on 1s video = 0.1 ratio (< 2.0 target)
        assert!(targets.target_met);
    }
}
