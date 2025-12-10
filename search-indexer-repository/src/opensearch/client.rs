//! OpenSearch client implementation.
//!
//! This module provides the concrete implementation of `SearchIndexProvider`
//! using the OpenSearch Rust client.

use async_trait::async_trait;
use opensearch::{
    http::transport::{SingleNodeConnectionPool, TransportBuilder},
    DeleteParts, OpenSearch, UpdateParts,
};
use serde_json::{json, Value};
use tracing::{debug, error, info, instrument};
use url::Url;
use uuid::Uuid;

use crate::errors::SearchIndexError;
use crate::interfaces::SearchIndexProvider;
use crate::opensearch::index_config::IndexConfig;
use crate::types::{
    BatchOperationResult, BatchOperationSummary, DeleteEntityRequest, UpdateEntityRequest,
};

/// OpenSearch client implementation.
///
/// Provides full-text search capabilities using OpenSearch as the backend.
///
/// # Example
///
/// ```ignore
/// use search_indexer_repository::opensearch::IndexConfig;
/// use search_indexer_repository::types::UpdateEntityRequest;
/// let config = IndexConfig::new("entities", 0);
/// let client = OpenSearchClient::new("http://localhost:9200", config).await?;
///
/// let request = UpdateEntityRequest {
///     entity_id: Uuid::new_v4().to_string(),
///     space_id: Uuid::new_v4().to_string(),
///     name: Some("Test Entity".to_string()),
///     description: Some("Description".to_string()),
///     ..Default::default()
/// };
/// // This will create the document if it doesn't exist, or update it if it does
/// client.update_document(&request).await?;
/// ```
pub struct OpenSearchClient {
    client: OpenSearch,
    index_config: IndexConfig,
}

impl OpenSearchClient {
    /// Create a new OpenSearch client connected to the specified URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The OpenSearch server URL (e.g., "http://localhost:9200")
    /// * `index_config` - The index configuration containing alias and version
    ///
    /// # Returns
    ///
    /// * `Ok(OpenSearchClient)` - A new client instance
    /// * `Err(SearchIndexError)` - If connection setup fails
    pub async fn new(url: &str, index_config: IndexConfig) -> Result<Self, SearchIndexError> {
        let parsed_url =
            Url::parse(url).map_err(|e| SearchIndexError::connection(e.to_string()))?;

        let conn_pool = SingleNodeConnectionPool::new(parsed_url);
        let transport = TransportBuilder::new(conn_pool)
            .disable_proxy()
            .build()
            .map_err(|e| SearchIndexError::connection(e.to_string()))?;

        let client = OpenSearch::new(transport);

        info!(
            url = %url,
            alias = %index_config.alias,
            version = index_config.version,
            "Created OpenSearch client"
        );

        Ok(Self {
            client,
            index_config,
        })
    }

    /// Generate a document ID from entity and space IDs.
    ///
    /// Uses format: `{entity_id}_{space_id}` to ensure uniqueness.
    fn document_id(entity_id: &Uuid, space_id: &Uuid) -> String {
        format!("{}_{}", entity_id, space_id)
    }
}

#[async_trait]
impl SearchIndexProvider for OpenSearchClient {
    /// Update specific fields of a document, creating it if it doesn't exist (upsert).
    ///
    /// This function performs an upsert operation: if the document exists, only fields that are
    /// `Some` in the request will be updated; if the document doesn't exist, it will be created
    /// with the provided fields. Fields that are `None` in the request will be left unchanged
    /// (for existing documents) or omitted (for new documents).
    ///
    /// # Arguments
    ///
    /// * `request` - The update request containing entity_id, space_id, and optional fields
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the document was updated or created successfully
    /// * `Err(SearchIndexError)` - If the operation fails
    async fn update_document(&self, request: &UpdateEntityRequest) -> Result<(), SearchIndexError> {
        // Validate UUIDs
        let entity_id = Uuid::parse_str(&request.entity_id)
            .map_err(|e| SearchIndexError::validation(format!("Invalid entity_id: {}", e)))?;
        let space_id = Uuid::parse_str(&request.space_id)
            .map_err(|e| SearchIndexError::validation(format!("Invalid space_id: {}", e)))?;

        let doc_id = Self::document_id(&entity_id, &space_id);

        // Build update document with only provided fields
        let mut doc = serde_json::Map::new();
        if let Some(ref name) = request.name {
            doc.insert("name".to_string(), json!(name));
        }
        if let Some(ref description) = request.description {
            doc.insert("description".to_string(), json!(description));
        }
        if let Some(ref avatar) = request.avatar {
            doc.insert("avatar".to_string(), json!(avatar));
        }
        if let Some(ref cover) = request.cover {
            doc.insert("cover".to_string(), json!(cover));
        }
        if let Some(entity_global_score) = request.entity_global_score {
            doc.insert(
                "entity_global_score".to_string(),
                json!(entity_global_score),
            );
        }
        if let Some(space_score) = request.space_score {
            doc.insert("space_score".to_string(), json!(space_score));
        }
        if let Some(entity_space_score) = request.entity_space_score {
            doc.insert("entity_space_score".to_string(), json!(entity_space_score));
        }

        if doc.is_empty() {
            // No fields to update
            return Ok(());
        }

        // Use upsert to create document if it doesn't exist
        // API reference: https://docs.opensearch.org/latest/api-reference/document-apis/update-document/#using-the-upsert-operation
        let response = self
            .client
            .update(UpdateParts::IndexId(&self.index_config.alias, &doc_id))
            .body(json!({
                "doc": doc,
                "doc_as_upsert": true
            }))
            .send()
            .await
            .map_err(|e| SearchIndexError::update(e.to_string()))?;

        let status = response.status_code();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            error!(status = %status, body = %error_body, "Update request failed");
            return Err(SearchIndexError::update(format!(
                "Update failed with status {}: {}",
                status, error_body
            )));
        }

        debug!(doc_id = %doc_id, "Document updated/created");
        Ok(())
    }

    /// Delete a document from the search index.
    ///
    /// This function deletes a document identified by entity_id and space_id. If the
    /// document doesn't exist, the operation is considered successful (no error is returned).
    ///
    /// # Arguments
    ///
    /// * `request` - The delete request containing entity_id and space_id
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the document was deleted (or didn't exist)
    /// * `Err(SearchIndexError)` - If the deletion fails
    async fn delete_document(&self, request: &DeleteEntityRequest) -> Result<(), SearchIndexError> {
        let entity_id = Uuid::parse_str(&request.entity_id)
            .map_err(|e| SearchIndexError::validation(format!("Invalid entity_id: {}", e)))?;
        let space_id = Uuid::parse_str(&request.space_id)
            .map_err(|e| SearchIndexError::validation(format!("Invalid space_id: {}", e)))?;

        let doc_id = Self::document_id(&entity_id, &space_id);

        let response = self
            .client
            .delete(DeleteParts::IndexId(&self.index_config.alias, &doc_id))
            .send()
            .await
            .map_err(|e| SearchIndexError::delete(e.to_string()))?;

        let status = response.status_code();

        // 404 is acceptable - document may not exist
        if !status.is_success() && status.as_u16() != 404 {
            let error_body = response.text().await.unwrap_or_default();
            error!(status = %status, body = %error_body, "Delete request failed");
            return Err(SearchIndexError::delete(format!(
                "Delete failed with status {}: {}",
                status, error_body
            )));
        }

        debug!(doc_id = %doc_id, "Document deleted");
        Ok(())
    }

    /// Update multiple documents in bulk and return a summary of successful and failed operations.
    ///
    /// This function updates multiple documents by calling `update_document` for each request
    /// and collecting the results. Returns a summary indicating which updates succeeded and
    /// which failed, along with error details for failed operations.
    ///
    /// # Arguments
    ///
    /// * `requests` - Slice of update requests, each containing entity_id, space_id, and optional fields
    ///
    /// # Returns
    ///
    /// * `Ok(BatchOperationSummary)` - Contains total count, succeeded count, failed count,
    ///   and individual results for each request with success status and optional error
    async fn bulk_update_documents(
        &self,
        requests: &[UpdateEntityRequest],
    ) -> Result<BatchOperationSummary, SearchIndexError> {
        let mut results = Vec::new();
        let mut succeeded = 0;
        let mut failed = 0;

        for request in requests {
            match SearchIndexProvider::update_document(self, request).await {
                Ok(()) => {
                    succeeded += 1;
                    results.push(BatchOperationResult {
                        entity_id: request.entity_id.clone(),
                        space_id: request.space_id.clone(),
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    failed += 1;
                    results.push(BatchOperationResult {
                        entity_id: request.entity_id.clone(),
                        space_id: request.space_id.clone(),
                        success: false,
                        error: Some(e.clone()),
                    });
                }
            }
        }

        Ok(BatchOperationSummary {
            total: requests.len(),
            succeeded,
            failed,
            results,
        })
    }

    /// Delete multiple documents in bulk and return a summary of successful and failed operations.
    ///
    /// This function deletes multiple documents by calling `delete_document` for each request
    /// and collecting the results. Returns a summary indicating which deletions succeeded and
    /// which failed. Note that documents not found are considered successful deletions.
    ///
    /// # Arguments
    ///
    /// * `requests` - Slice of delete requests, each containing entity_id and space_id
    ///
    /// # Returns
    ///
    /// * `Ok(BatchOperationSummary)` - Contains total count, succeeded count, failed count,
    ///   and individual results for each request with success status and optional error
    ///
    /// # Note
    ///
    /// If a document doesn't exist, the deletion is considered successful (no error is recorded).
    async fn bulk_delete_documents(
        &self,
        requests: &[DeleteEntityRequest],
    ) -> Result<BatchOperationSummary, SearchIndexError> {
        let mut results = Vec::new();
        let mut succeeded = 0;
        let mut failed = 0;

        for request in requests {
            match SearchIndexProvider::delete_document(self, request).await {
                Ok(()) => {
                    succeeded += 1;
                    results.push(BatchOperationResult {
                        entity_id: request.entity_id.clone(),
                        space_id: request.space_id.clone(),
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    // Document not found is considered a successful delete
                    if matches!(e, SearchIndexError::DocumentNotFound(_)) {
                        succeeded += 1;
                        results.push(BatchOperationResult {
                            entity_id: request.entity_id.clone(),
                            space_id: request.space_id.clone(),
                            success: true,
                            error: None,
                        });
                    } else {
                        failed += 1;
                        results.push(BatchOperationResult {
                            entity_id: request.entity_id.clone(),
                            space_id: request.space_id.clone(),
                            success: false,
                            error: Some(e.clone()),
                        });
                    }
                }
            }
        }

        Ok(BatchOperationSummary {
            total: requests.len(),
            succeeded,
            failed,
            results,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_id() {
        let entity_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let space_id = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();

        let doc_id = OpenSearchClient::document_id(&entity_id, &space_id);

        assert_eq!(
            doc_id,
            "550e8400-e29b-41d4-a716-446655440000_6ba7b810-9dad-11d1-80b4-00c04fd430c8"
        );
    }

    #[test]
    fn test_parse_hit() {
        let hit = json!({
            "_source": {
                "entity_id": "550e8400-e29b-41d4-a716-446655440000",
                "space_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
                "name": "Test Entity",
                "description": "A test description"
            },
            "_score": 1.5
        });

        let result = OpenSearchClient::parse_hit(&hit).unwrap();

        assert_eq!(result.name, Some("Test Entity".to_string()));
        assert_eq!(result.description, Some("A test description".to_string()));
        assert_eq!(result.relevance_score, 1.5);
    }

    #[test]
    fn test_parse_hit_minimal() {
        let hit = json!({
            "_source": {
                "entity_id": "550e8400-e29b-41d4-a716-446655440000",
                "space_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
                "name": "Minimal"
            },
            "_score": 0.5
        });

        let result = OpenSearchClient::parse_hit(&hit).unwrap();

        assert_eq!(result.name, Some("Minimal".to_string()));
        assert!(result.description.is_none());
        assert!(result.avatar.is_none());
    }

    #[test]
    fn test_parse_hit_no_name() {
        let hit = json!({
            "_source": {
                "entity_id": "550e8400-e29b-41d4-a716-446655440000",
                "space_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
            },
            "_score": 0.5
        });

        let result = OpenSearchClient::parse_hit(&hit).unwrap();

        assert!(result.name.is_none());
        assert!(result.description.is_none());
    }

    #[test]
    fn test_parse_hit_invalid() {
        let hit = json!({
            "_source": {
                "name": "Missing IDs"
            },
            "_score": 1.0
        });

        let result = OpenSearchClient::parse_hit(&hit);
        assert!(result.is_none());
    }
}
