use std::env;

#[derive(Debug, Clone)]
pub struct ResourceConfig {
    pub opensearch_memory_gb: f64,
    pub opensearch_cpu_cores: f64,
    pub opensearch_jvm_heap_gb: f64,
    pub indexer_memory_mb: f64,
    pub indexer_cpu_cores: f64,
    pub deployment_type: String,
}

#[derive(Debug, Clone)]
pub struct TestLimits {
    pub max_indexing_workers: usize,
    pub max_query_workers: usize,
    pub recommended_batch_size: usize,
    pub max_test_duration_minutes: usize,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TestConfig {
    pub scenario: String,
    pub duration_seconds: u64,
    pub indexing_workers: Option<usize>,
    pub query_workers: Option<usize>,
    pub batch_size: Option<usize>,
    pub opensearch_url: String,
    pub api_url: Option<String>,
    pub index_name: String,
    pub output_dir: String,
}

fn get_local_config() -> ResourceConfig {
    ResourceConfig {
        opensearch_memory_gb: 4.0,
        opensearch_cpu_cores: 2.0,
        opensearch_jvm_heap_gb: 1.0,
        indexer_memory_mb: 768.0,
        indexer_cpu_cores: 0.5,
        deployment_type: "local".to_string(),
    }
}

fn get_cloud_config() -> ResourceConfig {
    ResourceConfig {
        opensearch_memory_gb: 8.0,
        opensearch_cpu_cores: 2.0,
        opensearch_jvm_heap_gb: 3.0,
        indexer_memory_mb: 512.0,
        indexer_cpu_cores: 0.5,
        deployment_type: "cloud".to_string(),
    }
}

pub fn detect_deployment_type() -> String {
    // Check for explicit override
    if let Ok(explicit) = env::var("DEPLOYMENT_TYPE") {
        if explicit == "local" || explicit == "cloud" {
            return explicit;
        }
    }

    // Check for Kubernetes environment
    if env::var("KUBERNETES_SERVICE_HOST").is_ok() {
        return "cloud".to_string();
    }

    // Check for cloud-like OpenSearch URLs
    let opensearch_url = env::var("OPENSEARCH_URL").unwrap_or_default();
    if opensearch_url.contains("svc.cluster.local") || opensearch_url.contains("amazonaws.com") {
        return "cloud".to_string();
    }

    // Default to local
    "local".to_string()
}

pub fn get_resource_config() -> ResourceConfig {
    let deployment_type = detect_deployment_type();
    match deployment_type.as_str() {
        "cloud" => get_cloud_config(),
        _ => get_local_config(),
    }
}

pub fn get_test_limits(config: Option<&ResourceConfig>) -> TestLimits {
    let default_config = get_resource_config();
    let resources = config.unwrap_or(&default_config);

    if resources.deployment_type == "local" {
        TestLimits {
            max_indexing_workers: 10,
            max_query_workers: 50,
            recommended_batch_size: 100,
            max_test_duration_minutes: 15,
        }
    } else {
        TestLimits {
            max_indexing_workers: 50,
            max_query_workers: 500,
            recommended_batch_size: 500,
            max_test_duration_minutes: 60,
        }
    }
}

pub fn validate_test_config(
    indexing_workers: usize,
    query_workers: usize,
    batch_size: usize,
) -> ValidationResult {
    let limits = get_test_limits(None);
    let mut warnings = Vec::new();

    if indexing_workers > limits.max_indexing_workers {
        warnings.push(format!(
            "Indexing workers ({}) exceeds recommended limit ({})",
            indexing_workers, limits.max_indexing_workers
        ));
    }

    if query_workers > limits.max_query_workers {
        warnings.push(format!(
            "Query workers ({}) exceeds recommended limit ({})",
            query_workers, limits.max_query_workers
        ));
    }

    if batch_size > 1000 {
        warnings.push(format!(
            "Batch size ({}) is very large and may cause timeouts",
            batch_size
        ));
    }

    ValidationResult {
        valid: warnings.is_empty(),
        warnings,
    }
}
