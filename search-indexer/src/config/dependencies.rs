//! Dependency initialization and wiring for the search indexer.

use std::env;
use std::sync::Arc;
use tracing::info;

use crate::IndexingError;
use search_indexer_pipeline::{
    consumer::KafkaConsumer,
    loader::SearchLoader,
    orchestrator::Orchestrator,
    processor::EntityProcessor,
};
use search_indexer_repository::OpenSearchClient;

/// Default OpenSearch URL.
const DEFAULT_OPENSEARCH_URL: &str = "http://localhost:9200";

/// Default Kafka broker address.
const DEFAULT_KAFKA_BROKER: &str = "localhost:9092";

/// Default Kafka consumer group ID.
const DEFAULT_KAFKA_GROUP_ID: &str = "search-indexer";

/// Container for all initialized dependencies.
pub struct Dependencies {
    /// The configured orchestrator ready to run.
    pub orchestrator: Orchestrator,
}

impl Dependencies {
    /// Initialize all dependencies from environment variables.
    ///
    /// # Environment Variables
    ///
    /// - `OPENSEARCH_URL`: OpenSearch server URL (default: http://localhost:9200)
    /// - `KAFKA_BROKER`: Kafka broker address (default: localhost:9092)
    /// - `KAFKA_GROUP_ID`: Consumer group ID (default: search-indexer)
    ///
    /// # Returns
    ///
    /// * `Ok(Dependencies)` - Initialized dependencies
    /// * `Err(IndexingError)` - If initialization fails
    pub async fn new() -> Result<Self, IndexingError> {
        let opensearch_url =
            env::var("OPENSEARCH_URL").unwrap_or_else(|_| DEFAULT_OPENSEARCH_URL.to_string());
        let kafka_broker =
            env::var("KAFKA_BROKER").unwrap_or_else(|_| DEFAULT_KAFKA_BROKER.to_string());
        let kafka_group_id =
            env::var("KAFKA_GROUP_ID").unwrap_or_else(|_| DEFAULT_KAFKA_GROUP_ID.to_string());

        info!(
            opensearch_url = %opensearch_url,
            kafka_broker = %kafka_broker,
            kafka_group_id = %kafka_group_id,
            "Initializing dependencies"
        );

        // Initialize OpenSearch client
        let search_client = OpenSearchClient::new(&opensearch_url)
            .await
            .map_err(|e| IndexingError::config(format!("Failed to create OpenSearch client: {}", e)))?;

        // Verify OpenSearch is reachable
        let healthy = search_client
            .health_check()
            .await
            .map_err(|e| IndexingError::config(format!("OpenSearch health check failed: {}", e)))?;

        if !healthy {
            return Err(IndexingError::config("OpenSearch cluster is unhealthy"));
        }

        info!("OpenSearch connection verified");

        // Initialize Kafka consumer
        let consumer = KafkaConsumer::new(&kafka_broker, &kafka_group_id)
            .map_err(|e| IndexingError::config(format!("Failed to create Kafka consumer: {}", e)))?;

        info!("Kafka consumer created");

        // Initialize processor
        let processor = EntityProcessor::new();

        // Initialize loader with search client
        let loader = SearchLoader::new(Arc::new(search_client));

        // Create orchestrator
        let orchestrator = Orchestrator::new(consumer, processor, loader);

        Ok(Self { orchestrator })
    }
}

