//! Search index client implementation.
//!
//! This module provides the main client for interacting with the search index.
//! Application code uses this to query, create, update, and delete documents.

use crate::config::SearchIndexConfig;
use crate::errors::SearchIndexError;
use crate::interfaces::SearchIndexProvider;
use crate::types::{
    BatchOperationSummary, CreateEntityRequest, DeleteEntityRequest, UpdateEntityRequest,
};
use search_indexer_shared::{EntityDocument, SearchQuery, SearchResult};

/// The main client for interacting with the search index.
/// Application code uses this to query, create, update, and delete documents.
pub struct SearchIndexClient {
    provider: Box<dyn SearchIndexProvider>,
    config: SearchIndexConfig,
}

impl SearchIndexClient {
    /// Create a new SearchIndexClient with default configuration.
    pub fn new(provider: Box<dyn SearchIndexProvider>) -> Self {
        Self {
            provider,
            config: SearchIndexConfig::default(),
        }
    }

    /// Create a new SearchIndexClient with custom configuration.
    pub fn with_config(provider: Box<dyn SearchIndexProvider>, config: SearchIndexConfig) -> Self {
        Self { provider, config }
    }

    /// Check if batch size exceeds the configured limit.
    fn validate_batch_size(&self, size: usize) -> Result<(), SearchIndexError> {
        if let Some(max) = self.config.max_batch_size {
            if size > max {
                return Err(SearchIndexError::batch_size_exceeded(size, max));
            }
        }
        Ok(())
    }

    /// Query the search index.
    /// Input: SearchQuery (query text, scope, optional space_id)
    /// Output: Result<Vec<SearchResult>, SearchIndexError> (ordered by relevance)
    pub async fn query(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SearchIndexError> {
        // Validate query and space_id requirements based on scope
        // If the query text is a uuid, assume it is an entity_id
        //   return a direct search for that exact entity
        // Build search query with optional filters for space scope
        // Execute search against underlying provider
        // Map results to SearchResult format
        // Return ordered results
        self.provider.search(query).await
    }

    /// Create a new entity document in the search index.
    /// Input: CreateEntityRequest (entity_id, space_id, name required; all else optional)
    /// Output: Result<(), SearchIndexError>
    pub async fn create(&self, request: CreateEntityRequest) -> Result<(), SearchIndexError> {
        // Validate required fields
        if request.entity_id.is_empty() {
            return Err(SearchIndexError::validation("entity_id is required"));
        }
        if request.space_id.is_empty() {
            return Err(SearchIndexError::validation("space_id is required"));
        }

        // Build EntityDocument from request with current timestamp
        let document: EntityDocument = request.into();

        // Send index request to provider
        self.provider.index_document(&document).await
    }

    /// Update one or more properties of an existing entity document.
    /// Input: UpdateEntityRequest (entity_id, space_id required; all properties optional)
    /// Output: Result<(), SearchIndexError>
    pub async fn update(&self, request: UpdateEntityRequest) -> Result<(), SearchIndexError> {
        // Validate required fields
        if request.entity_id.is_empty() {
            return Err(SearchIndexError::validation("entity_id is required"));
        }
        if request.space_id.is_empty() {
            return Err(SearchIndexError::validation("space_id is required"));
        }

        // Build partial document update with only provided fields
        // Send update request to provider
        self.provider.update_document(&request).await
    }

    /// Delete an entity document from the search index.
    /// Input: DeleteEntityRequest (entity_id, space_id required)
    /// Output: Result<(), SearchIndexError>
    pub async fn delete(&self, request: DeleteEntityRequest) -> Result<(), SearchIndexError> {
        if request.entity_id.is_empty() {
            return Err(SearchIndexError::validation("entity_id is required"));
        }
        if request.space_id.is_empty() {
            return Err(SearchIndexError::validation("space_id is required"));
        }

        self.provider.delete_document(&request).await
    }

    /// Create multiple entity documents in the search index.
    /// Input: Vec<CreateEntityRequest> (batch of entities to create)
    /// Output: Result<BatchOperationSummary, SearchIndexError>
    ///
    /// Note: This operation is atomic at the batch level if supported by the
    /// underlying provider. Individual failures are reported in the summary.
    ///
    /// The batch size is limited by the configured max_batch_size (default: 1000).
    pub async fn batch_create(
        &self,
        requests: Vec<CreateEntityRequest>,
    ) -> Result<BatchOperationSummary, SearchIndexError> {
        if requests.is_empty() {
            return Ok(BatchOperationSummary {
                total: 0,
                succeeded: 0,
                failed: 0,
                results: vec![],
            });
        }

        self.validate_batch_size(requests.len())?;

        // Validate all requests
        for request in &requests {
            if request.entity_id.is_empty() {
                return Err(SearchIndexError::validation(
                    "All requests must have entity_id",
                ));
            }
            if request.space_id.is_empty() {
                return Err(SearchIndexError::validation(
                    "All requests must have space_id",
                ));
            }
        }

        let documents: Vec<EntityDocument> = requests.into_iter().map(Into::into).collect();

        self.provider.bulk_index_documents(&documents).await
    }

    /// Update multiple entity documents in the search index.
    /// Input: Vec<UpdateEntityRequest> (batch of partial updates)
    /// Output: Result<BatchOperationSummary, SearchIndexError>
    ///
    /// Note: Each update only modifies the fields that are Some.
    /// Individual failures are reported in the summary.
    ///
    /// The batch size is limited by the configured max_batch_size (default: 1000).
    pub async fn batch_update(
        &self,
        requests: Vec<UpdateEntityRequest>,
    ) -> Result<BatchOperationSummary, SearchIndexError> {
        if requests.is_empty() {
            return Ok(BatchOperationSummary {
                total: 0,
                succeeded: 0,
                failed: 0,
                results: vec![],
            });
        }

        self.validate_batch_size(requests.len())?;

        // Validate all requests
        for request in &requests {
            if request.entity_id.is_empty() {
                return Err(SearchIndexError::validation(
                    "All requests must have entity_id",
                ));
            }
            if request.space_id.is_empty() {
                return Err(SearchIndexError::validation(
                    "All requests must have space_id",
                ));
            }
        }

        self.provider.bulk_update_documents(&requests).await
    }

    /// Delete multiple entity documents from the search index.
    /// Input: Vec<DeleteEntityRequest> (batch of entities to delete)
    /// Output: Result<BatchOperationSummary, SearchIndexError>
    ///
    /// Note: Individual failures are reported in the summary.
    /// Documents that don't exist are considered successful deletes.
    ///
    /// The batch size is limited by the configured max_batch_size (default: 1000).
    pub async fn batch_delete(
        &self,
        requests: Vec<DeleteEntityRequest>,
    ) -> Result<BatchOperationSummary, SearchIndexError> {
        if requests.is_empty() {
            return Ok(BatchOperationSummary {
                total: 0,
                succeeded: 0,
                failed: 0,
                results: vec![],
            });
        }

        self.validate_batch_size(requests.len())?;

        // Validate all requests
        for request in &requests {
            if request.entity_id.is_empty() {
                return Err(SearchIndexError::validation(
                    "All requests must have entity_id",
                ));
            }
            if request.space_id.is_empty() {
                return Err(SearchIndexError::validation(
                    "All requests must have space_id",
                ));
            }
        }

        self.provider.bulk_delete_documents(&requests).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BatchOperationResult;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    /// Mock provider for testing
    struct MockProvider {
        indexed_documents: Arc<Mutex<Vec<EntityDocument>>>,
        update_requests: Arc<Mutex<Vec<UpdateEntityRequest>>>,
        delete_requests: Arc<Mutex<Vec<DeleteEntityRequest>>>,
        should_fail: bool,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                indexed_documents: Arc::new(Mutex::new(Vec::new())),
                update_requests: Arc::new(Mutex::new(Vec::new())),
                delete_requests: Arc::new(Mutex::new(Vec::new())),
                should_fail: false,
            }
        }
    }

    #[async_trait]
    impl SearchIndexProvider for MockProvider {
        async fn search(
            &self,
            _query: &SearchQuery,
        ) -> Result<Vec<SearchResult>, SearchIndexError> {
            Ok(vec![])
        }

        async fn index_document(&self, document: &EntityDocument) -> Result<(), SearchIndexError> {
            if self.should_fail {
                return Err(SearchIndexError::index("Mock failure"));
            }
            self.indexed_documents.lock().await.push(document.clone());
            Ok(())
        }

        async fn update_document(
            &self,
            request: &UpdateEntityRequest,
        ) -> Result<(), SearchIndexError> {
            if self.should_fail {
                return Err(SearchIndexError::index("Mock failure"));
            }
            self.update_requests.lock().await.push(request.clone());
            Ok(())
        }

        async fn delete_document(
            &self,
            request: &DeleteEntityRequest,
        ) -> Result<(), SearchIndexError> {
            if self.should_fail {
                return Err(SearchIndexError::index("Mock failure"));
            }
            self.delete_requests.lock().await.push(request.clone());
            Ok(())
        }

        async fn bulk_index_documents(
            &self,
            documents: &[EntityDocument],
        ) -> Result<BatchOperationSummary, SearchIndexError> {
            if self.should_fail {
                return Err(SearchIndexError::bulk_operation("Mock failure"));
            }

            let mut results = Vec::new();
            let mut succeeded = 0;
            let failed = 0;

            for doc in documents {
                let result = BatchOperationResult {
                    entity_id: doc.entity_id.to_string(),
                    space_id: doc.space_id.to_string(),
                    success: true,
                    error: None,
                };
                results.push(result);
                succeeded += 1;
                self.indexed_documents.lock().await.push(doc.clone());
            }

            Ok(BatchOperationSummary {
                total: documents.len(),
                succeeded,
                failed,
                results,
            })
        }

        async fn bulk_update_documents(
            &self,
            requests: &[UpdateEntityRequest],
        ) -> Result<BatchOperationSummary, SearchIndexError> {
            if self.should_fail {
                return Err(SearchIndexError::bulk_operation("Mock failure"));
            }

            let mut results = Vec::new();
            let mut succeeded = 0;
            let failed = 0;

            for req in requests {
                let result = BatchOperationResult {
                    entity_id: req.entity_id.clone(),
                    space_id: req.space_id.clone(),
                    success: true,
                    error: None,
                };
                results.push(result);
                succeeded += 1;
                self.update_requests.lock().await.push(req.clone());
            }

            Ok(BatchOperationSummary {
                total: requests.len(),
                succeeded,
                failed,
                results,
            })
        }

        async fn bulk_delete_documents(
            &self,
            requests: &[DeleteEntityRequest],
        ) -> Result<BatchOperationSummary, SearchIndexError> {
            if self.should_fail {
                return Err(SearchIndexError::bulk_operation("Mock failure"));
            }

            let mut results = Vec::new();
            let mut succeeded = 0;
            let failed = 0;

            for req in requests {
                let result = BatchOperationResult {
                    entity_id: req.entity_id.clone(),
                    space_id: req.space_id.clone(),
                    success: true,
                    error: None,
                };
                results.push(result);
                succeeded += 1;
                self.delete_requests.lock().await.push(req.clone());
            }

            Ok(BatchOperationSummary {
                total: requests.len(),
                succeeded,
                failed,
                results,
            })
        }
    }

    fn create_test_request(entity_id: &str, space_id: &str, name: &str) -> CreateEntityRequest {
        CreateEntityRequest {
            entity_id: entity_id.to_string(),
            space_id: space_id.to_string(),
            name: Some(name.to_string()),
            description: Some("Test description".to_string()),
            avatar: None,
            cover: None,
            entity_global_score: Some(1.0),
            space_score: Some(2.0),
            entity_space_score: Some(3.0),
        }
    }

    fn create_test_update_request(entity_id: &str, space_id: &str) -> UpdateEntityRequest {
        UpdateEntityRequest {
            entity_id: entity_id.to_string(),
            space_id: space_id.to_string(),
            name: Some("Updated name".to_string()),
            description: None,
            avatar: None,
            cover: None,
            entity_global_score: None,
            space_score: None,
            entity_space_score: None,
        }
    }

    fn create_test_delete_request(entity_id: &str, space_id: &str) -> DeleteEntityRequest {
        DeleteEntityRequest {
            entity_id: entity_id.to_string(),
            space_id: space_id.to_string(),
        }
    }

    #[tokio::test]
    async fn test_batch_create_empty() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        let result = client.batch_create(vec![]).await.unwrap();

        assert_eq!(result.total, 0);
        assert_eq!(result.succeeded, 0);
        assert_eq!(result.failed, 0);
        assert!(result.results.is_empty());
    }

    #[tokio::test]
    async fn test_batch_create_single() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        let entity_id = Uuid::new_v4().to_string();
        let space_id = Uuid::new_v4().to_string();
        let requests = vec![create_test_request(&entity_id, &space_id, "Test Entity")];

        let result = client.batch_create(requests).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(result.results.len(), 1);
        assert!(result.results[0].success);
        assert_eq!(result.results[0].entity_id, entity_id);
        assert_eq!(result.results[0].space_id, space_id);
    }

    #[tokio::test]
    async fn test_batch_create_multiple() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        let requests = vec![
            create_test_request(
                &Uuid::new_v4().to_string(),
                &Uuid::new_v4().to_string(),
                "Entity 1",
            ),
            create_test_request(
                &Uuid::new_v4().to_string(),
                &Uuid::new_v4().to_string(),
                "Entity 2",
            ),
            create_test_request(
                &Uuid::new_v4().to_string(),
                &Uuid::new_v4().to_string(),
                "Entity 3",
            ),
        ];

        let result = client.batch_create(requests).await.unwrap();

        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 3);
        assert_eq!(result.failed, 0);
        assert_eq!(result.results.len(), 3);
        assert!(result.results.iter().all(|r| r.success));
    }

    #[tokio::test]
    async fn test_batch_create_validation_empty_entity_id() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        let requests = vec![CreateEntityRequest {
            entity_id: "".to_string(),
            space_id: Uuid::new_v4().to_string(),
            name: Some("Test".to_string()),
            description: None,
            avatar: None,
            cover: None,
            entity_global_score: None,
            space_score: None,
            entity_space_score: None,
        }];

        let result = client.batch_create(requests).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SearchIndexError::ValidationError(_)
        ));
    }


    #[tokio::test]
    async fn test_batch_create_batch_size_exceeded() {
        let provider = MockProvider::new();
        let config = SearchIndexConfig::with_max_batch_size(5);
        let client = SearchIndexClient::with_config(Box::new(provider), config);

        let requests: Vec<CreateEntityRequest> = (0..10)
            .map(|i| {
                create_test_request(
                    &Uuid::new_v4().to_string(),
                    &Uuid::new_v4().to_string(),
                    &format!("Entity {}", i),
                )
            })
            .collect();

        let result = client.batch_create(requests).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SearchIndexError::BatchSizeExceeded {
                provided: 10,
                max: 5
            }
        ));
    }

    #[tokio::test]
    async fn test_batch_update_empty() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        let result = client.batch_update(vec![]).await.unwrap();

        assert_eq!(result.total, 0);
        assert_eq!(result.succeeded, 0);
        assert_eq!(result.failed, 0);
        assert!(result.results.is_empty());
    }

    #[tokio::test]
    async fn test_batch_update_single() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        let entity_id = Uuid::new_v4().to_string();
        let space_id = Uuid::new_v4().to_string();
        let requests = vec![create_test_update_request(&entity_id, &space_id)];

        let result = client.batch_update(requests).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(result.results.len(), 1);
        assert!(result.results[0].success);
    }

    #[tokio::test]
    async fn test_batch_update_multiple() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        let requests = vec![
            create_test_update_request(&Uuid::new_v4().to_string(), &Uuid::new_v4().to_string()),
            create_test_update_request(&Uuid::new_v4().to_string(), &Uuid::new_v4().to_string()),
            create_test_update_request(&Uuid::new_v4().to_string(), &Uuid::new_v4().to_string()),
        ];

        let result = client.batch_update(requests).await.unwrap();

        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 3);
        assert_eq!(result.failed, 0);
        assert_eq!(result.results.len(), 3);
    }

    #[tokio::test]
    async fn test_batch_delete_empty() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        let result = client.batch_delete(vec![]).await.unwrap();

        assert_eq!(result.total, 0);
        assert_eq!(result.succeeded, 0);
        assert_eq!(result.failed, 0);
        assert!(result.results.is_empty());
    }

    #[tokio::test]
    async fn test_batch_delete_single() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        let entity_id = Uuid::new_v4().to_string();
        let space_id = Uuid::new_v4().to_string();
        let requests = vec![create_test_delete_request(&entity_id, &space_id)];

        let result = client.batch_delete(requests).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(result.results.len(), 1);
        assert!(result.results[0].success);
    }

    #[tokio::test]
    async fn test_batch_delete_multiple() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        let requests = vec![
            create_test_delete_request(&Uuid::new_v4().to_string(), &Uuid::new_v4().to_string()),
            create_test_delete_request(&Uuid::new_v4().to_string(), &Uuid::new_v4().to_string()),
            create_test_delete_request(&Uuid::new_v4().to_string(), &Uuid::new_v4().to_string()),
        ];

        let result = client.batch_delete(requests).await.unwrap();

        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 3);
        assert_eq!(result.failed, 0);
        assert_eq!(result.results.len(), 3);
    }

    #[tokio::test]
    async fn test_create_validation() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        // Test empty entity_id
        let request = CreateEntityRequest {
            entity_id: "".to_string(),
            space_id: Uuid::new_v4().to_string(),
            name: Some("Test".to_string()),
            description: None,
            avatar: None,
            cover: None,
            entity_global_score: None,
            space_score: None,
            entity_space_score: None,
        };
        assert!(client.create(request).await.is_err());

        // Test empty space_id
        let request = CreateEntityRequest {
            entity_id: Uuid::new_v4().to_string(),
            space_id: "".to_string(),
            name: Some("Test".to_string()),
            description: None,
            avatar: None,
            cover: None,
            entity_global_score: None,
            space_score: None,
            entity_space_score: None,
        };
        assert!(client.create(request).await.is_err());

        // Test with None name (should be valid)
        let request = CreateEntityRequest {
            entity_id: Uuid::new_v4().to_string(),
            space_id: Uuid::new_v4().to_string(),
            name: None,
            description: None,
            avatar: None,
            cover: None,
            entity_global_score: None,
            space_score: None,
            entity_space_score: None,
        };
        assert!(client.create(request).await.is_ok());
    }

    #[tokio::test]
    async fn test_create_success() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        let request = create_test_request(
            &Uuid::new_v4().to_string(),
            &Uuid::new_v4().to_string(),
            "Test Entity",
        );

        let result = client.create(request).await;
        assert!(result.is_ok());

        // Test create without name
        let request = CreateEntityRequest {
            entity_id: Uuid::new_v4().to_string(),
            space_id: Uuid::new_v4().to_string(),
            name: None,
            description: None,
            avatar: None,
            cover: None,
            entity_global_score: None,
            space_score: None,
            entity_space_score: None,
        };

        let result = client.create(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_validation() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        // Test empty entity_id
        let request = UpdateEntityRequest {
            entity_id: "".to_string(),
            space_id: Uuid::new_v4().to_string(),
            name: None,
            description: None,
            avatar: None,
            cover: None,
            entity_global_score: None,
            space_score: None,
            entity_space_score: None,
        };
        assert!(client.update(request).await.is_err());

        // Test empty space_id
        let request = UpdateEntityRequest {
            entity_id: Uuid::new_v4().to_string(),
            space_id: "".to_string(),
            name: None,
            description: None,
            avatar: None,
            cover: None,
            entity_global_score: None,
            space_score: None,
            entity_space_score: None,
        };
        assert!(client.update(request).await.is_err());
    }

    #[tokio::test]
    async fn test_delete_validation() {
        let provider = MockProvider::new();
        let client = SearchIndexClient::new(Box::new(provider));

        // Test empty entity_id
        let request = DeleteEntityRequest {
            entity_id: "".to_string(),
            space_id: Uuid::new_v4().to_string(),
        };
        assert!(client.delete(request).await.is_err());

        // Test empty space_id
        let request = DeleteEntityRequest {
            entity_id: Uuid::new_v4().to_string(),
            space_id: "".to_string(),
        };
        assert!(client.delete(request).await.is_err());
    }

    #[tokio::test]
    async fn test_batch_size_unlimited() {
        let provider = MockProvider::new();
        let config = SearchIndexConfig::unlimited();
        let client = SearchIndexClient::with_config(Box::new(provider), config);

        // Should allow any batch size
        let requests: Vec<CreateEntityRequest> = (0..10000)
            .map(|i| {
                create_test_request(
                    &Uuid::new_v4().to_string(),
                    &Uuid::new_v4().to_string(),
                    &format!("Entity {}", i),
                )
            })
            .collect();

        // This should not fail due to batch size (though it might fail for other reasons)
        let result = client.batch_create(requests).await;
        // If it fails, it should not be BatchSizeExceeded
        if let Err(SearchIndexError::BatchSizeExceeded { .. }) = result {
            panic!("Batch size should not be limited with unlimited config");
        }
    }
}
