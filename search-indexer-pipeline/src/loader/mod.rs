//! Loader module for the search indexer pipeline.
//!
//! Loads processed documents into the search index.

use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};

use crate::errors::PipelineError;
use crate::processor::ProcessedEvent;
use search_indexer_repository::SearchEngineClient;
use search_indexer_shared::EntityDocument;

/// Configuration for the search loader.
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Number of documents to batch before flushing.
    pub batch_size: usize,
    /// Maximum time to wait before flushing a partial batch (in milliseconds).
    pub flush_interval_ms: u64,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            flush_interval_ms: 5000,
        }
    }
}

/// Loader that indexes documents into the search engine.
///
/// The loader is responsible for:
/// - Batching documents for efficient bulk indexing
/// - Handling retries on failures
/// - Managing cursor persistence
pub struct SearchLoader {
    client: Arc<dyn SearchEngineClient>,
    config: LoaderConfig,
    pending_docs: Vec<EntityDocument>,
    pending_deletes: Vec<(uuid::Uuid, uuid::Uuid)>,
}

impl SearchLoader {
    /// Create a new search loader with the given client.
    pub fn new(client: Arc<dyn SearchEngineClient>) -> Self {
        Self {
            client,
            config: LoaderConfig::default(),
            pending_docs: Vec::new(),
            pending_deletes: Vec::new(),
        }
    }

    /// Create a new search loader with custom configuration.
    pub fn with_config(client: Arc<dyn SearchEngineClient>, config: LoaderConfig) -> Self {
        Self {
            client,
            config,
            pending_docs: Vec::with_capacity(config.batch_size),
            pending_deletes: Vec::new(),
        }
    }

    /// Load a batch of processed events.
    ///
    /// Documents are batched and flushed when the batch size is reached.
    #[instrument(skip(self, events), fields(event_count = events.len()))]
    pub async fn load(&mut self, events: Vec<ProcessedEvent>) -> Result<(), PipelineError> {
        for event in events {
            match event {
                ProcessedEvent::Index(doc) => {
                    self.pending_docs.push(doc);
                }
                ProcessedEvent::Delete { entity_id, space_id } => {
                    self.pending_deletes.push((entity_id, space_id));
                }
            }
        }

        // Flush if we've reached batch size
        if self.pending_docs.len() >= self.config.batch_size {
            self.flush().await?;
        }

        // Process deletes immediately (they're usually less frequent)
        if !self.pending_deletes.is_empty() {
            self.process_deletes().await?;
        }

        Ok(())
    }

    /// Flush all pending documents to the search index.
    #[instrument(skip(self))]
    pub async fn flush(&mut self) -> Result<(), PipelineError> {
        if self.pending_docs.is_empty() {
            return Ok(());
        }

        let docs: Vec<EntityDocument> = self.pending_docs.drain(..).collect();
        let count = docs.len();

        info!(count = count, "Flushing documents to search index");

        match self.client.bulk_index(&docs).await {
            Ok(()) => {
                debug!(count = count, "Successfully indexed documents");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, count = count, "Failed to index documents");

                // On bulk failure, try indexing individually
                warn!("Attempting individual document indexing");
                let mut success_count = 0;
                let mut error_count = 0;

                for doc in docs {
                    match self.client.index_document(&doc).await {
                        Ok(()) => success_count += 1,
                        Err(e) => {
                            error!(
                                entity_id = %doc.entity_id,
                                error = %e,
                                "Failed to index individual document"
                            );
                            error_count += 1;
                        }
                    }
                }

                info!(
                    success = success_count,
                    errors = error_count,
                    "Individual indexing completed"
                );

                if error_count > 0 {
                    Err(PipelineError::loader(format!(
                        "Failed to index {} documents",
                        error_count
                    )))
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Process pending delete operations.
    async fn process_deletes(&mut self) -> Result<(), PipelineError> {
        let deletes: Vec<(uuid::Uuid, uuid::Uuid)> = self.pending_deletes.drain(..).collect();

        for (entity_id, space_id) in deletes {
            if let Err(e) = self.client.delete_document(&entity_id, &space_id).await {
                // Log but don't fail - document might not exist
                warn!(
                    entity_id = %entity_id,
                    space_id = %space_id,
                    error = %e,
                    "Failed to delete document"
                );
            }
        }

        Ok(())
    }

    /// Ensure the search index exists.
    pub async fn ensure_index(&self) -> Result<(), PipelineError> {
        self.client
            .ensure_index_exists()
            .await
            .map_err(|e| PipelineError::LoaderError(e.to_string()))
    }

    /// Check if the search engine is healthy.
    pub async fn health_check(&self) -> Result<bool, PipelineError> {
        self.client
            .health_check()
            .await
            .map_err(|e| PipelineError::LoaderError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use search_indexer_repository::{SearchError, UpdateEntityRequest};
    use search_indexer_shared::{SearchQuery, SearchResponse};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uuid::Uuid;

    /// Mock search client for testing.
    struct MockSearchClient {
        indexed_count: AtomicUsize,
        deleted_count: AtomicUsize,
    }

    impl MockSearchClient {
        fn new() -> Self {
            Self {
                indexed_count: AtomicUsize::new(0),
                deleted_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl SearchEngineClient for MockSearchClient {
        async fn search(&self, _query: &SearchQuery) -> Result<SearchResponse, SearchError> {
            Ok(SearchResponse::empty())
        }

        async fn index_document(&self, _doc: &EntityDocument) -> Result<(), SearchError> {
            self.indexed_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn bulk_index(&self, docs: &[EntityDocument]) -> Result<(), SearchError> {
            self.indexed_count.fetch_add(docs.len(), Ordering::SeqCst);
            Ok(())
        }

        async fn update_document(&self, _request: &UpdateEntityRequest) -> Result<(), SearchError> {
            Ok(())
        }

        async fn delete_document(&self, _entity_id: &Uuid, _space_id: &Uuid) -> Result<(), SearchError> {
            self.deleted_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn ensure_index_exists(&self) -> Result<(), SearchError> {
            Ok(())
        }

        async fn health_check(&self) -> Result<bool, SearchError> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_load_and_flush() {
        let client = Arc::new(MockSearchClient::new());
        let mut loader = SearchLoader::new(client.clone());

        let events = vec![
            ProcessedEvent::Index(EntityDocument::new(
                Uuid::new_v4(),
                Uuid::new_v4(),
                "Test 1".to_string(),
                None,
            )),
            ProcessedEvent::Index(EntityDocument::new(
                Uuid::new_v4(),
                Uuid::new_v4(),
                "Test 2".to_string(),
                None,
            )),
        ];

        loader.load(events).await.unwrap();
        loader.flush().await.unwrap();

        assert_eq!(client.indexed_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_delete_processing() {
        let client = Arc::new(MockSearchClient::new());
        let mut loader = SearchLoader::new(client.clone());

        let events = vec![ProcessedEvent::Delete {
            entity_id: Uuid::new_v4(),
            space_id: Uuid::new_v4(),
        }];

        loader.load(events).await.unwrap();

        assert_eq!(client.deleted_count.load(Ordering::SeqCst), 1);
    }
}

