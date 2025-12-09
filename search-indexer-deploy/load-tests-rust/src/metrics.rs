use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::clients::IndexStatistics;

#[derive(Debug, Clone)]
pub struct LatencyMetrics {
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub p99_9: f64,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone)]
pub struct ThroughputMetrics {
    pub total: usize,
    pub per_second: f64,
}

#[derive(Debug, Clone)]
pub struct ErrorMetrics {
    pub total: usize,
    pub rate: f64, // percentage
    pub errors: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct TestMetrics {
    pub indexing: Option<OperationMetrics>,
    pub querying: Option<OperationMetrics>,
    pub duration_seconds: f64,
    pub timestamp: String,
    pub index_statistics: Option<IndexStatistics>,
}

#[derive(Debug, Clone)]
pub struct OperationMetrics {
    pub throughput: ThroughputMetrics,
    pub latency: LatencyMetrics,
    pub errors: ErrorMetrics,
}

pub struct MetricsCollector {
    indexing_latencies: Arc<Mutex<Vec<u64>>>,
    querying_latencies: Arc<Mutex<Vec<u64>>>,
    indexing_errors: Arc<Mutex<HashMap<String, usize>>>,
    querying_errors: Arc<Mutex<HashMap<String, usize>>>,
    indexing_success_count: Arc<Mutex<usize>>,
    querying_success_count: Arc<Mutex<usize>>,
    start_time: std::time::Instant,
    end_time: Arc<Mutex<Option<std::time::Instant>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            indexing_latencies: Arc::new(Mutex::new(Vec::new())),
            querying_latencies: Arc::new(Mutex::new(Vec::new())),
            indexing_errors: Arc::new(Mutex::new(HashMap::new())),
            querying_errors: Arc::new(Mutex::new(HashMap::new())),
            indexing_success_count: Arc::new(Mutex::new(0)),
            querying_success_count: Arc::new(Mutex::new(0)),
            start_time: std::time::Instant::now(),
            end_time: Arc::new(Mutex::new(None)),
        }
    }

    pub fn record_indexing(&self, latency_ms: u64, success: bool, error: Option<&str>) {
        if success {
            self.indexing_latencies.lock().unwrap().push(latency_ms);
            *self.indexing_success_count.lock().unwrap() += 1;
        } else {
            let error_key = error.unwrap_or("unknown").to_string();
            *self.indexing_errors
                .lock()
                .unwrap()
                .entry(error_key)
                .or_insert(0) += 1;
        }
    }

    pub fn record_querying(&self, latency_ms: u64, success: bool, error: Option<&str>) {
        if success {
            self.querying_latencies.lock().unwrap().push(latency_ms);
            *self.querying_success_count.lock().unwrap() += 1;
        } else {
            let error_key = error.unwrap_or("unknown").to_string();
            *self.querying_errors
                .lock()
                .unwrap()
                .entry(error_key)
                .or_insert(0) += 1;
        }
    }

    pub fn stop(&self) {
        *self.end_time.lock().unwrap() = Some(std::time::Instant::now());
    }

    fn calculate_latency_metrics(&self, latencies: &[u64]) -> LatencyMetrics {
        if latencies.is_empty() {
            return LatencyMetrics {
                p50: 0.0,
                p90: 0.0,
                p95: 0.0,
                p99: 0.0,
                p99_9: 0.0,
                mean: 0.0,
                min: 0.0,
                max: 0.0,
            };
        }

        let mut sorted = latencies.to_vec();
        sorted.sort_unstable();
        let sum: u64 = sorted.iter().sum();

        LatencyMetrics {
            p50: Self::percentile(&sorted, 0.5),
            p90: Self::percentile(&sorted, 0.9),
            p95: Self::percentile(&sorted, 0.95),
            p99: Self::percentile(&sorted, 0.99),
            p99_9: Self::percentile(&sorted, 0.999),
            mean: sum as f64 / sorted.len() as f64,
            min: sorted[0] as f64,
            max: sorted[sorted.len() - 1] as f64,
        }
    }

    fn percentile(sorted: &[u64], p: f64) -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        let index = ((sorted.len() as f64 * p).ceil() as usize).max(1) - 1;
        sorted[index.min(sorted.len() - 1)] as f64
    }

    fn calculate_throughput_metrics(&self, success_count: usize, duration_seconds: f64) -> ThroughputMetrics {
        ThroughputMetrics {
            total: success_count,
            per_second: if duration_seconds > 0.0 {
                success_count as f64 / duration_seconds
            } else {
                0.0
            },
        }
    }

    fn calculate_error_metrics(&self, errors: &HashMap<String, usize>, total_operations: usize) -> ErrorMetrics {
        let total_errors: usize = errors.values().sum();
        ErrorMetrics {
            total: total_errors,
            rate: if total_operations > 0 {
                (total_errors as f64 / total_operations as f64) * 100.0
            } else {
                0.0
            },
            errors: errors.clone(),
        }
    }

    pub fn get_metrics(&self) -> TestMetrics {
        let end_time = self.end_time.lock().unwrap();
        let duration_seconds = if let Some(end) = *end_time {
            (end - self.start_time).as_secs_f64()
        } else {
            (std::time::Instant::now() - self.start_time).as_secs_f64()
        };

        let indexing_latencies = self.indexing_latencies.lock().unwrap().clone();
        let querying_latencies = self.querying_latencies.lock().unwrap().clone();
        let indexing_errors = self.indexing_errors.lock().unwrap().clone();
        let querying_errors = self.querying_errors.lock().unwrap().clone();
        let indexing_success = *self.indexing_success_count.lock().unwrap();
        let querying_success = *self.querying_success_count.lock().unwrap();

        let mut metrics = TestMetrics {
            indexing: None,
            querying: None,
            duration_seconds,
            timestamp: chrono::Utc::now().to_rfc3339(),
            index_statistics: None,
        };

        if !indexing_latencies.is_empty() || !indexing_errors.is_empty() {
            let total_indexing_ops = indexing_success + indexing_errors.values().sum::<usize>();
            metrics.indexing = Some(OperationMetrics {
                throughput: self.calculate_throughput_metrics(indexing_success, duration_seconds),
                latency: self.calculate_latency_metrics(&indexing_latencies),
                errors: self.calculate_error_metrics(&indexing_errors, total_indexing_ops),
            });
        }

        if !querying_latencies.is_empty() || !querying_errors.is_empty() {
            let total_querying_ops = querying_success + querying_errors.values().sum::<usize>();
            metrics.querying = Some(OperationMetrics {
                throughput: self.calculate_throughput_metrics(querying_success, duration_seconds),
                latency: self.calculate_latency_metrics(&querying_latencies),
                errors: self.calculate_error_metrics(&querying_errors, total_querying_ops),
            });
        }

        metrics
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

