use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use colored::*;

use crate::config::ResourceConfig;
use crate::metrics::{TestMetrics, LatencyMetrics, ThroughputMetrics, ErrorMetrics};

pub struct Reporter {
    output_dir: String,
    test_name: String,
    resource_config: Option<ResourceConfig>,
    time_series_data: Vec<TimeSeriesPoint>,
}

#[derive(Debug, Clone)]
struct TimeSeriesPoint {
    timestamp: String,
    indexing_rate: Option<f64>,
    querying_rate: Option<f64>,
    indexing_latency_p50: Option<f64>,
    querying_latency_p50: Option<f64>,
}

impl Reporter {
    pub fn new(output_dir: String, test_name: String, resource_config: Option<ResourceConfig>) -> Self {
        // Ensure output directory exists
        if let Err(e) = fs::create_dir_all(&output_dir) {
            eprintln!("Warning: Failed to create output directory {}: {}", output_dir, e);
        }

        Self {
            output_dir,
            test_name,
            resource_config,
            time_series_data: Vec::new(),
        }
    }

    pub fn add_time_series_point(
        &mut self,
        indexing_rate: Option<f64>,
        querying_rate: Option<f64>,
        indexing_latency_p50: Option<f64>,
        querying_latency_p50: Option<f64>,
    ) {
        self.time_series_data.push(TimeSeriesPoint {
            timestamp: chrono::Utc::now().to_rfc3339(),
            indexing_rate,
            querying_rate,
            indexing_latency_p50,
            querying_latency_p50,
        });
    }

    pub async fn generate_reports(&self, metrics: &TestMetrics) -> Result<()> {
        // Generate human-readable report
        let human_readable = self.generate_human_readable_report(metrics);
        let human_readable_path = Path::new(&self.output_dir).join(format!("{}-report.txt", self.test_name));
        fs::write(&human_readable_path, &human_readable)
            .with_context(|| format!("Failed to write report to {:?}", human_readable_path))?;
        println!("{}", "✓ Human-readable report saved to:".green());
        println!("  {}\n", human_readable_path.display());

        // Generate JSON report
        let json_report = self.generate_json_report(metrics);
        let json_path = Path::new(&self.output_dir).join(format!("{}-results.json", self.test_name));
        fs::write(&json_path, serde_json::to_string_pretty(&json_report)?)
            .with_context(|| format!("Failed to write JSON to {:?}", json_path))?;
        println!("{}", "✓ JSON report saved to:".green());
        println!("  {}\n", json_path.display());

        // Generate CSV report
        let csv_report = self.generate_csv_report(metrics);
        let csv_path = Path::new(&self.output_dir).join(format!("{}-results.csv", self.test_name));
        fs::write(&csv_path, &csv_report)
            .with_context(|| format!("Failed to write CSV to {:?}", csv_path))?;
        println!("{}", "✓ CSV report saved to:".green());
        println!("  {}\n", csv_path.display());

        // Generate time-series CSV if we have data
        if !self.time_series_data.is_empty() {
            let time_series_csv = self.generate_time_series_csv();
            let time_series_path = Path::new(&self.output_dir).join(format!("{}-timeseries.csv", self.test_name));
            fs::write(&time_series_path, &time_series_csv)
                .with_context(|| format!("Failed to write time-series CSV to {:?}", time_series_path))?;
            println!("{}", "✓ Time-series CSV saved to:".green());
            println!("  {}\n", time_series_path.display());
        }

        // Print summary to console
        println!("\n{}", human_readable);

        Ok(())
    }

    fn generate_human_readable_report(&self, metrics: &TestMetrics) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push("=".repeat(80));
        lines.push(format!("LOAD TEST RESULTS: {}", self.test_name.to_uppercase()));
        lines.push("=".repeat(80));
        lines.push(String::new());

        // Test configuration
        lines.push("Test Configuration:".to_string());
        lines.push(format!("  Duration: {:.1} seconds", metrics.duration_seconds));
        lines.push(format!("  Timestamp: {}", metrics.timestamp));
        if let Some(ref config) = self.resource_config {
            lines.push(format!("  Deployment: {}", config.deployment_type));
            lines.push(format!("  OpenSearch Memory: {}GB", config.opensearch_memory_gb));
            lines.push(format!("  OpenSearch CPU: {} cores", config.opensearch_cpu_cores));
            lines.push(format!("  OpenSearch JVM Heap: {}GB", config.opensearch_jvm_heap_gb));
        }
        lines.push(String::new());

        // Index statistics
        if let Some(ref index_stats) = metrics.index_statistics {
            lines.push("Index Statistics:".to_string());
            lines.push(format!("  Document Count: {}", index_stats.document_count));
            lines.push(format!("  Average Document Size: {:.2} kB", index_stats.average_doc_size_kb));
            lines.push(format!("  Total Storage: {:.3} GB", index_stats.total_storage_gb));
            lines.push(format!("  Primary Shards: {}", index_stats.primary_shards));
            lines.push(format!("  Replica Shards: {}", index_stats.replica_shards));
            lines.push(String::new());
        }

        // Indexing metrics
        if let Some(ref indexing) = metrics.indexing {
            lines.push("Indexing Performance:".to_string());
            lines.extend(self.format_throughput(&indexing.throughput, "docs"));
            lines.extend(self.format_latency(&indexing.latency));
            lines.extend(self.format_errors(&indexing.errors));
            lines.push(String::new());
        }

        // Querying metrics
        if let Some(ref querying) = metrics.querying {
            lines.push("Querying Performance:".to_string());
            lines.extend(self.format_throughput(&querying.throughput, "queries"));
            lines.extend(self.format_latency(&querying.latency));
            lines.extend(self.format_errors(&querying.errors));
            lines.push(String::new());
        }

        // Summary
        lines.push("Summary:".to_string());
        match (&metrics.indexing, &metrics.querying) {
            (Some(indexing), Some(querying)) => {
                lines.push(format!(
                    "  Total Operations: {}",
                    indexing.throughput.total + querying.throughput.total
                ));
                lines.push(format!(
                    "  Combined Throughput: {:.1} ops/sec",
                    indexing.throughput.per_second + querying.throughput.per_second
                ));
            }
            (Some(indexing), None) => {
                lines.push(format!("  Total Documents Indexed: {}", indexing.throughput.total));
                lines.push(format!("  Average Rate: {:.1} docs/sec", indexing.throughput.per_second));
            }
            (None, Some(querying)) => {
                lines.push(format!("  Total Queries Executed: {}", querying.throughput.total));
                lines.push(format!("  Average Rate: {:.1} queries/sec", querying.throughput.per_second));
            }
            (None, None) => {
                lines.push("  No metrics collected".to_string());
            }
        }

        lines.push(String::new());
        lines.push("=".repeat(80));

        lines.join("\n")
    }

    fn format_throughput(&self, throughput: &ThroughputMetrics, unit: &str) -> Vec<String> {
        vec![
            format!("  Total {}: {}", unit, throughput.total),
            format!("  Rate: {:.1} {}/sec", throughput.per_second, unit),
        ]
    }

    fn format_latency(&self, latency: &LatencyMetrics) -> Vec<String> {
        vec![
            "  Latency (ms):".to_string(),
            format!("    Mean:   {:.1}", latency.mean),
            format!("    P50:    {:.1}", latency.p50),
            format!("    P90:    {:.1}", latency.p90),
            format!("    P95:    {:.1}", latency.p95),
            format!("    P99:    {:.1}", latency.p99),
            format!("    P99.9:  {:.1}", latency.p99_9),
            format!("    Min:    {:.1}", latency.min),
            format!("    Max:    {:.1}", latency.max),
        ]
    }

    fn format_errors(&self, errors: &ErrorMetrics) -> Vec<String> {
        let mut lines = vec![format!("  Errors: {} ({:.2}%)", errors.total, errors.rate)];
        if !errors.errors.is_empty() {
            lines.push("  Error Breakdown:".to_string());
            for (error_type, count) in &errors.errors {
                lines.push(format!("    {}: {}", error_type, count));
            }
        }
        lines
    }

    fn generate_json_report(&self, metrics: &TestMetrics) -> serde_json::Value {
        let mut report = serde_json::json!({
            "test_name": self.test_name,
            "timestamp": metrics.timestamp,
            "duration_seconds": metrics.duration_seconds,
        });

        if let Some(ref config) = self.resource_config {
            report["resource_config"] = serde_json::json!({
                "deployment_type": config.deployment_type,
                "opensearch_memory_gb": config.opensearch_memory_gb,
                "opensearch_cpu_cores": config.opensearch_cpu_cores,
                "opensearch_jvm_heap_gb": config.opensearch_jvm_heap_gb,
            });
        }

        if let Some(ref index_stats) = metrics.index_statistics {
            report["index_statistics"] = serde_json::json!({
                "document_count": index_stats.document_count,
                "average_doc_size_kb": index_stats.average_doc_size_kb,
                "total_storage_gb": index_stats.total_storage_gb,
                "primary_shards": index_stats.primary_shards,
                "replica_shards": index_stats.replica_shards,
            });
        }

        let mut results = serde_json::json!({});

        if let Some(ref indexing) = metrics.indexing {
            results["indexing"] = serde_json::json!({
                "throughput": {
                    "total": indexing.throughput.total,
                    "per_second": indexing.throughput.per_second,
                },
                "latency_ms": {
                    "mean": indexing.latency.mean,
                    "p50": indexing.latency.p50,
                    "p90": indexing.latency.p90,
                    "p95": indexing.latency.p95,
                    "p99": indexing.latency.p99,
                    "p99_9": indexing.latency.p99_9,
                    "min": indexing.latency.min,
                    "max": indexing.latency.max,
                },
                "errors": {
                    "total": indexing.errors.total,
                    "rate_percent": indexing.errors.rate,
                    "breakdown": indexing.errors.errors,
                },
            });
        }

        if let Some(ref querying) = metrics.querying {
            results["querying"] = serde_json::json!({
                "throughput": {
                    "total": querying.throughput.total,
                    "per_second": querying.throughput.per_second,
                },
                "latency_ms": {
                    "mean": querying.latency.mean,
                    "p50": querying.latency.p50,
                    "p90": querying.latency.p90,
                    "p95": querying.latency.p95,
                    "p99": querying.latency.p99,
                    "p99_9": querying.latency.p99_9,
                    "min": querying.latency.min,
                    "max": querying.latency.max,
                },
                "errors": {
                    "total": querying.errors.total,
                    "rate_percent": querying.errors.rate,
                    "breakdown": querying.errors.errors,
                },
            });
        }

        report["results"] = results;
        report
    }

    fn generate_csv_report(&self, metrics: &TestMetrics) -> String {
        let mut lines = vec!["metric,value,unit".to_string()];

        lines.push(format!("duration,{:.1},seconds", metrics.duration_seconds));

        if let Some(ref index_stats) = metrics.index_statistics {
            lines.push(format!("index_document_count,{},documents", index_stats.document_count));
            lines.push(format!("index_avg_doc_size,{:.2},kB", index_stats.average_doc_size_kb));
            lines.push(format!("index_total_storage,{:.3},GB", index_stats.total_storage_gb));
            lines.push(format!("index_primary_shards,{},shards", index_stats.primary_shards));
            lines.push(format!("index_replica_shards,{},shards", index_stats.replica_shards));
        }

        if let Some(ref indexing) = metrics.indexing {
            lines.push(format!("indexing_total,{},documents", indexing.throughput.total));
            lines.push(format!("indexing_rate,{:.1},docs_per_second", indexing.throughput.per_second));
            lines.push(format!("indexing_latency_mean,{:.1},ms", indexing.latency.mean));
            lines.push(format!("indexing_latency_p50,{:.1},ms", indexing.latency.p50));
            lines.push(format!("indexing_latency_p90,{:.1},ms", indexing.latency.p90));
            lines.push(format!("indexing_latency_p95,{:.1},ms", indexing.latency.p95));
            lines.push(format!("indexing_latency_p99,{:.1},ms", indexing.latency.p99));
            lines.push(format!("indexing_latency_p99_9,{:.1},ms", indexing.latency.p99_9));
            lines.push(format!("indexing_errors_total,{},count", indexing.errors.total));
            lines.push(format!("indexing_errors_rate,{:.2},percent", indexing.errors.rate));
        }

        if let Some(ref querying) = metrics.querying {
            lines.push(format!("querying_total,{},queries", querying.throughput.total));
            lines.push(format!("querying_rate,{:.1},queries_per_second", querying.throughput.per_second));
            lines.push(format!("querying_latency_mean,{:.1},ms", querying.latency.mean));
            lines.push(format!("querying_latency_p50,{:.1},ms", querying.latency.p50));
            lines.push(format!("querying_latency_p90,{:.1},ms", querying.latency.p90));
            lines.push(format!("querying_latency_p95,{:.1},ms", querying.latency.p95));
            lines.push(format!("querying_latency_p99,{:.1},ms", querying.latency.p99));
            lines.push(format!("querying_latency_p99_9,{:.1},ms", querying.latency.p99_9));
            lines.push(format!("querying_errors_total,{},count", querying.errors.total));
            lines.push(format!("querying_errors_rate,{:.2},percent", querying.errors.rate));
        }

        lines.join("\n")
    }

    fn generate_time_series_csv(&self) -> String {
        let mut lines = vec!["timestamp,indexing_rate,querying_rate,indexing_latency_p50,querying_latency_p50".to_string()];

        for point in &self.time_series_data {
            let row = vec![
                point.timestamp.clone(),
                point.indexing_rate.map(|r| format!("{:.1}", r)).unwrap_or_default(),
                point.querying_rate.map(|r| format!("{:.1}", r)).unwrap_or_default(),
                point.indexing_latency_p50.map(|l| format!("{:.1}", l)).unwrap_or_default(),
                point.querying_latency_p50.map(|l| format!("{:.1}", l)).unwrap_or_default(),
            ];
            lines.push(row.join(","));
        }

        lines.join("\n")
    }
}

