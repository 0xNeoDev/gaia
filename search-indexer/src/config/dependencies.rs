//! Dependency initialization and wiring for the search indexer.

use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::IndexingError;
use search_indexer_ingest::{
    consumer::KafkaConsumer, loader::SearchLoader, orchestrator::Orchestrator,
    processor::EntityProcessor,
};
use search_indexer_repository::{OpenSearchClient, SearchEngineClient};

/// Default OpenSearch URL.
const DEFAULT_OPENSEARCH_URL: &str = "http://localhost:9200";

/// Default Kafka broker address.
const DEFAULT_KAFKA_BROKER: &str = "localhost:9092";

/// Default Kafka consumer group ID.
const DEFAULT_KAFKA_GROUP_ID: &str = "search-indexer";

/// Default connection retry interval in seconds.
const DEFAULT_RETRY_INTERVAL_SECS: u64 = 15;

/// Connection mode for OpenSearch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    /// Fail immediately if connection fails.
    FailFast,
    /// Retry connection every 15 seconds until successful.
    Retry,
}

/// Container for all initialized dependencies.
pub struct Dependencies {
    /// The configured orchestrator ready to run.
    pub orchestrator: Orchestrator,
}

impl ConnectionMode {
    /// Parse connection mode from environment variable.
    ///
    /// Valid values: "fail-fast" or "retry" (case-insensitive)
    /// Defaults to "retry" if not set or invalid.
    fn from_env() -> Self {
        match env::var("OPENSEARCH_CONNECTION_MODE")
            .unwrap_or_else(|_| "retry".to_string())
            .to_lowercase()
            .as_str()
        {
            "fail-fast" | "failfast" | "fail_fast" => Self::FailFast,
            "retry" => Self::Retry,
            _ => {
                warn!("Invalid OPENSEARCH_CONNECTION_MODE, defaulting to 'retry'");
                Self::Retry
            }
        }
    }
}

impl Dependencies {
    /// Initialize all dependencies from environment variables.
    ///
    /// # Environment Variables
    ///
    /// - `OPENSEARCH_URL`: OpenSearch server URL (default: http://localhost:9200)
    /// - `KAFKA_BROKER`: Kafka broker address (default: localhost:9092)
    /// - `KAFKA_GROUP_ID`: Consumer group ID (default: search-indexer)
    /// - `OPENSEARCH_CONNECTION_MODE`: Connection mode - "fail-fast" or "retry" (default: retry)
    /// - `OPENSEARCH_RETRY_INTERVAL_SECS`: Retry interval in seconds (default: 15)
    ///
    /// # Returns
    ///
    /// * `Ok(Dependencies)` - Initialized dependencies
    /// * `Err(IndexingError)` - If initialization fails (only in fail-fast mode)
    pub async fn new() -> Result<Self, IndexingError> {
        let opensearch_url =
            env::var("OPENSEARCH_URL").unwrap_or_else(|_| DEFAULT_OPENSEARCH_URL.to_string());
        let kafka_broker =
            env::var("KAFKA_BROKER").unwrap_or_else(|_| DEFAULT_KAFKA_BROKER.to_string());
        let kafka_group_id =
            env::var("KAFKA_GROUP_ID").unwrap_or_else(|_| DEFAULT_KAFKA_GROUP_ID.to_string());
        let connection_mode = ConnectionMode::from_env();
        let retry_interval = env::var("OPENSEARCH_RETRY_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_RETRY_INTERVAL_SECS);

        info!(
            opensearch_url = %opensearch_url,
            kafka_broker = %kafka_broker,
            kafka_group_id = %kafka_group_id,
            connection_mode = ?connection_mode,
            retry_interval_secs = retry_interval,
            "Initializing dependencies"
        );

        // Initialize OpenSearch client with retry logic
        let search_client = Self::connect_to_opensearch(
            &opensearch_url,
            connection_mode,
            Duration::from_secs(retry_interval),
        )
        .await?;

        info!("OpenSearch connection verified");

        // Initialize Kafka consumer
        let consumer = KafkaConsumer::new(&kafka_broker, &kafka_group_id).map_err(|e| {
            IndexingError::config(format!("Failed to create Kafka consumer: {}", e))
        })?;

        info!("Kafka consumer created");

        // Initialize processor
        let processor = EntityProcessor::new();

        // Initialize loader with search client
        let loader = SearchLoader::new(Arc::new(search_client));

        // Create orchestrator
        let orchestrator = Orchestrator::new(consumer, processor, loader);

        Ok(Self { orchestrator })
    }

    /// Connect to OpenSearch with retry logic based on connection mode.
    async fn connect_to_opensearch(
        url: &str,
        mode: ConnectionMode,
        retry_interval: Duration,
    ) -> Result<OpenSearchClient, IndexingError> {
        loop {
            match Self::try_connect_opensearch(url).await {
                Ok(client) => return Ok(client),
                Err(e) => match mode {
                    ConnectionMode::FailFast => {
                        return Err(IndexingError::config(format!(
                            "Failed to connect to OpenSearch: {}",
                            e
                        )));
                    }
                    ConnectionMode::Retry => {
                        warn!(
                            opensearch_url = %url,
                            error = %e,
                            retry_interval_secs = retry_interval.as_secs(),
                            "Failed to connect to OpenSearch, retrying..."
                        );
                        sleep(retry_interval).await;
                    }
                },
            }
        }
    }

    /// Attempt to connect to OpenSearch and verify health.
    async fn try_connect_opensearch(url: &str) -> Result<OpenSearchClient, IndexingError> {
        // Initialize OpenSearch client
        let search_client = OpenSearchClient::new(url).await.map_err(|e| {
            IndexingError::config(format!("Failed to create OpenSearch client: {}", e))
        })?;

        // Verify OpenSearch is reachable
        let healthy = search_client
            .health_check()
            .await
            .map_err(|e| IndexingError::config(format!("OpenSearch health check failed: {}", e)))?;

        if !healthy {
            return Err(IndexingError::config("OpenSearch cluster is unhealthy"));
        }

        Ok(search_client)
    }
}
