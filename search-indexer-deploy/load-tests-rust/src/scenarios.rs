use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::{info, warn};

use crate::clients::{APITestClient, OpenSearchTestClient};
use crate::config::{get_resource_config, TestConfig};
use crate::generators::generate_documents;
use crate::loaders::{IndexLoader, QueryLoader};
use crate::metrics::MetricsCollector;
use crate::reporter::Reporter;

pub async fn run_indexing(config: TestConfig) -> Result<()> {
    info!("🔵 Starting Pure Indexing Load Test");

    let indexing_workers = config
        .indexing_workers
        .context("indexing_workers is required for indexing scenario")?;
    let batch_size = config
        .batch_size
        .context("batch_size is required for indexing scenario")?;

    // Initialize client
    info!("Connecting to OpenSearch at: {}", config.opensearch_url);
    let client = Arc::new(
        OpenSearchTestClient::new(&config.opensearch_url, &config.index_name)
            .await
            .context("Failed to create OpenSearch client")?,
    );

    // Health check
    info!("Checking OpenSearch health...");
    let healthy = client
        .health_check()
        .await
        .context("OpenSearch health check failed")?;

    if !healthy {
        return Err(anyhow::anyhow!(
            "OpenSearch is not healthy. Please check the connection."
        ));
    }
    info!("✓ OpenSearch is healthy");

    // Initialize metrics and reporter
    let metrics = Arc::new(MetricsCollector::new());
    let resource_config = get_resource_config();
    let reporter = Reporter::new(
        config.output_dir.clone(),
        format!("indexing-{}", chrono::Utc::now().timestamp()),
        Some(resource_config),
    );

    // Create loader
    info!(
        "Starting test with {} workers, batch size {}, duration {}s",
        indexing_workers, batch_size, config.duration_seconds
    );

    let loader = IndexLoader::new(
        Arc::clone(&client),
        Arc::clone(&metrics),
        batch_size,
        indexing_workers,
        config.duration_seconds,
    );

    // Run test
    let start_time = std::time::Instant::now();
    loader.start().await?;
    metrics.stop();

    let duration = start_time.elapsed().as_secs_f64();
    println!("\n\nTest completed in {:.1} seconds", duration);

    // Get index statistics
    let index_stats = client.get_index_statistics().await?;

    // Generate reports
    let mut final_metrics = metrics.get_metrics();
    final_metrics.index_statistics = Some(index_stats);
    reporter.generate_reports(&final_metrics).await?;

    Ok(())
}

pub async fn run_querying(config: TestConfig) -> Result<()> {
    info!("🟢 Starting Pure Querying Load Test");

    let query_workers = config
        .query_workers
        .context("query_workers is required for querying scenario")?;

    // Initialize client (prefer API if available, otherwise direct OpenSearch)
    let opensearch_client: Option<Arc<OpenSearchTestClient>>;
    let api_client: Option<Arc<APITestClient>>;

    if let Some(ref api_url) = config.api_url {
        info!("Using API client for queries at: {}", api_url);
        let client = Arc::new(APITestClient::new(api_url));

        // Health check
        info!("Checking API health...");
        match client.health_check().await {
            Ok(healthy) => {
                if !healthy {
                    warn!("API server is not healthy, falling back to direct OpenSearch");
                    opensearch_client = Some(Arc::new(
                        OpenSearchTestClient::new(&config.opensearch_url, &config.index_name)
                            .await
                            .context("Failed to create OpenSearch client")?,
                    ));
                    api_client = None;
                } else {
                    info!("✓ API server is healthy");
                    opensearch_client = None;
                    api_client = Some(client);
                }
            }
            Err(e) => {
                warn!(
                    "API health check failed: {}, falling back to direct OpenSearch",
                    e
                );
                opensearch_client = Some(Arc::new(
                    OpenSearchTestClient::new(&config.opensearch_url, &config.index_name)
                        .await
                        .context("Failed to create OpenSearch client")?,
                ));
                api_client = None;
            }
        }
    } else {
        info!("Using direct OpenSearch client for queries");
        opensearch_client = Some(Arc::new(
            OpenSearchTestClient::new(&config.opensearch_url, &config.index_name)
                .await
                .context("Failed to create OpenSearch client")?,
        ));
        api_client = None;
    }

    // Health check OpenSearch if we're using it
    if let Some(ref client) = opensearch_client {
        info!("Checking OpenSearch health...");
        let healthy = client
            .health_check()
            .await
            .context("OpenSearch health check failed")?;

        if !healthy {
            return Err(anyhow::anyhow!(
                "OpenSearch is not healthy. Please check the connection."
            ));
        }
        info!("✓ OpenSearch is healthy");
    }

    // Initialize metrics and reporter
    let metrics = Arc::new(MetricsCollector::new());
    let resource_config = get_resource_config();
    let reporter = Reporter::new(
        config.output_dir.clone(),
        format!("querying-{}", chrono::Utc::now().timestamp()),
        Some(resource_config),
    );

    // Create loader (clone client for stats retrieval later)
    info!(
        "Starting test with {} workers, duration {}s",
        query_workers, config.duration_seconds
    );

    // Clone client for stats retrieval before moving into loader
    let stats_client = opensearch_client.as_ref().map(|c| Arc::clone(c));

    let loader = QueryLoader::new(
        opensearch_client,
        api_client,
        Arc::clone(&metrics),
        query_workers,
        config.duration_seconds,
        Vec::new(), // Empty documents for now
    );

    // Run test
    let start_time = std::time::Instant::now();
    loader.start().await?;
    metrics.stop();

    let duration = start_time.elapsed().as_secs_f64();
    println!("\n\nTest completed in {:.1} seconds", duration);

    // Get index statistics (use OpenSearch client if available)
    let index_stats = if let Some(ref client) = stats_client {
        client.get_index_statistics().await?
    } else {
        // If using API only, we can't get stats - return zeros
        crate::clients::IndexStatistics {
            document_count: 0,
            average_doc_size_kb: 0.0,
            total_storage_gb: 0.0,
            primary_shards: 0,
            replica_shards: 0,
        }
    };

    // Generate reports
    let mut final_metrics = metrics.get_metrics();
    final_metrics.index_statistics = Some(index_stats);
    reporter.generate_reports(&final_metrics).await?;

    Ok(())
}

pub async fn run_mixed(config: TestConfig) -> Result<()> {
    info!("🟡 Starting Mixed Workload Load Test");

    let indexing_workers = config
        .indexing_workers
        .context("indexing_workers is required for mixed scenario")?;
    let query_workers = config
        .query_workers
        .context("query_workers is required for mixed scenario")?;
    let batch_size = config
        .batch_size
        .context("batch_size is required for mixed scenario")?;

    // Initialize clients
    info!("Connecting to OpenSearch at: {}", config.opensearch_url);
    let opensearch_client = Arc::new(
        OpenSearchTestClient::new(&config.opensearch_url, &config.index_name)
            .await
            .context("Failed to create OpenSearch client")?,
    );

    let mut api_client: Option<Arc<APITestClient>> = None;

    if let Some(ref api_url) = config.api_url {
        info!("Creating API client at: {}", api_url);
        let client = Arc::new(APITestClient::new(api_url));

        info!("Checking API health...");
        match client.health_check().await {
            Ok(healthy) => {
                if healthy {
                    info!("✓ API server is healthy");
                    api_client = Some(client);
                } else {
                    warn!("⚠ API server is not healthy, queries will use direct OpenSearch");
                }
            }
            Err(e) => {
                warn!(
                    "⚠ API health check failed: {}, queries will use direct OpenSearch",
                    e
                );
            }
        }
    }

    info!("Checking OpenSearch health...");
    let healthy = opensearch_client
        .health_check()
        .await
        .context("OpenSearch health check failed")?;

    if !healthy {
        return Err(anyhow::anyhow!(
            "OpenSearch is not healthy. Please check the connection."
        ));
    }
    info!("✓ OpenSearch is healthy");

    // Generate seed documents for realistic query generation
    info!("Generating seed documents for query generation...");
    let seed_documents = generate_documents(1000, None);
    info!("✓ Generated {} seed documents", seed_documents.len());

    // Initialize metrics and reporter
    let metrics = Arc::new(MetricsCollector::new());
    let resource_config = get_resource_config();
    let reporter = Reporter::new(
        config.output_dir.clone(),
        format!("mixed-{}", chrono::Utc::now().timestamp()),
        Some(resource_config),
    );

    // Create loaders
    info!(
        "Starting test with {} indexing workers, {} query workers, batch size {}, duration {}s",
        indexing_workers, query_workers, batch_size, config.duration_seconds
    );

    let index_loader = IndexLoader::new(
        Arc::clone(&opensearch_client),
        Arc::clone(&metrics),
        batch_size,
        indexing_workers,
        config.duration_seconds,
    );

    let query_loader = QueryLoader::new(
        Some(Arc::clone(&opensearch_client)),
        api_client,
        Arc::clone(&metrics),
        query_workers,
        config.duration_seconds,
        seed_documents,
    );

    // Run both loaders simultaneously
    let start_time = std::time::Instant::now();

    let index_handle = {
        let loader = index_loader;
        tokio::spawn(async move { loader.start().await })
    };

    let query_handle = {
        let loader = query_loader;
        tokio::spawn(async move { loader.start().await })
    };

    let (index_result, query_result) = tokio::try_join!(index_handle, query_handle)?;
    index_result.context("Index loader task failed")?;
    query_result.context("Query loader task failed")?;
    metrics.stop();

    let duration = start_time.elapsed().as_secs_f64();
    println!("\n\nTest completed in {:.1} seconds", duration);

    // Get index statistics
    let index_stats = opensearch_client.get_index_statistics().await?;

    // Generate reports
    let mut final_metrics = metrics.get_metrics();
    final_metrics.index_statistics = Some(index_stats);
    reporter.generate_reports(&final_metrics).await?;

    Ok(())
}

pub async fn run_sustained(config: TestConfig) -> Result<()> {
    info!("🟠 Starting Sustained Load Test");

    let sustained_config = TestConfig {
        duration_seconds: config.duration_seconds.max(3600), // At least 1 hour
        ..config
    };

    info!(
        "⚠ Running sustained test for {:.1} minutes",
        sustained_config.duration_seconds as f64 / 60.0
    );

    run_mixed(sustained_config).await
}

pub async fn run_burst(config: TestConfig) -> Result<()> {
    info!("🔴 Starting Burst Load Test");

    let burst_config = TestConfig {
        indexing_workers: config.indexing_workers.map(|w| w * 3), // 3x for burst
        query_workers: config.query_workers.map(|w| w * 5),       // 5x for burst
        duration_seconds: config.duration_seconds.min(300),       // Max 5 minutes
        ..config
    };

    if let (Some(idx), Some(qry)) = (burst_config.indexing_workers, burst_config.query_workers) {
        info!(
            "⚠ Running burst test with {} indexing workers and {} query workers",
            idx, qry
        );
    }

    run_mixed(burst_config).await
}
