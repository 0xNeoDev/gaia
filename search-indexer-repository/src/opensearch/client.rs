//! OpenSearch client implementation.
//!
//! This module provides the concrete implementation of `SearchEngineClient`
//! using the OpenSearch Rust client.

use async_trait::async_trait;
use opensearch::{
    http::{
        request::JsonBody,
        transport::{SingleNodeConnectionPool, TransportBuilder},
    },
    indices::{IndicesCreateParts, IndicesExistsParts},
    BulkParts, DeleteParts, IndexParts, OpenSearch, SearchParts, UpdateParts,
};
use serde_json::{json, Value};
use tracing::{debug, error, info, instrument, warn};
use url::Url;
use uuid::Uuid;

use crate::errors::{SearchError, SearchIndexError};
use crate::interfaces::UpdateEntityRequest as OldUpdateEntityRequest;
use crate::interfaces::{SearchEngineClient, SearchIndexProvider};
use crate::opensearch::index_config::{get_index_settings, INDEX_NAME};
use crate::opensearch::queries::build_search_query;
use crate::types::{
    BatchOperationResult, BatchOperationSummary, DeleteEntityRequest, UpdateEntityRequest,
};
use search_indexer_shared::{EntityDocument, SearchQuery, SearchResponse, SearchResult};

/// OpenSearch client implementation.
///
/// Provides full-text search capabilities using OpenSearch as the backend.
///
/// # Example
///
/// ```ignore
/// let client = OpenSearchClient::new("http://localhost:9200").await?;
/// client.ensure_index_exists().await?;
///
/// let doc = EntityDocument::new(
///     Uuid::new_v4(),
///     Uuid::new_v4(),
///     "Test Entity".to_string(),
///     Some("Description".to_string()),
/// );
/// client.index_document(&doc).await?;
/// ```
pub struct OpenSearchClient {
    client: OpenSearch,
    index_name: String,
}

impl OpenSearchClient {
    /// Create a new OpenSearch client connected to the specified URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The OpenSearch server URL (e.g., "http://localhost:9200")
    ///
    /// # Returns
    ///
    /// * `Ok(OpenSearchClient)` - A new client instance
    /// * `Err(SearchError)` - If connection setup fails
    pub async fn new(url: &str) -> Result<Self, SearchError> {
        let parsed_url =
            Url::parse(url).map_err(|e| SearchError::ConnectionError(e.to_string()))?;

        let conn_pool = SingleNodeConnectionPool::new(parsed_url);
        let transport = TransportBuilder::new(conn_pool)
            .disable_proxy()
            .build()
            .map_err(|e| SearchError::ConnectionError(e.to_string()))?;

        let client = OpenSearch::new(transport);

        info!(url = %url, "Created OpenSearch client");

        Ok(Self {
            client,
            index_name: INDEX_NAME.to_string(),
        })
    }

    /// Create a client with a custom index name.
    ///
    /// Useful for testing or multi-tenant scenarios.
    ///
    /// # Arguments
    ///
    /// * `url` - The OpenSearch server URL
    /// * `index_name` - Custom index name to use
    pub async fn with_index_name(url: &str, index_name: &str) -> Result<Self, SearchError> {
        let mut client = Self::new(url).await?;
        client.index_name = index_name.to_string();
        Ok(client)
    }

    /// Generate a document ID from entity and space IDs.
    ///
    /// Uses format: `{entity_id}_{space_id}` to ensure uniqueness.
    fn document_id(entity_id: &Uuid, space_id: &Uuid) -> String {
        format!("{}_{}", entity_id, space_id)
    }

    /// Parse a search hit into a SearchResult.
    fn parse_hit(hit: &Value) -> Option<SearchResult> {
        let source = &hit["_source"];
        let score = hit["_score"].as_f64().unwrap_or(0.0);

        Some(SearchResult {
            entity_id: Uuid::parse_str(source["entity_id"].as_str()?).ok()?,
            space_id: Uuid::parse_str(source["space_id"].as_str()?).ok()?,
            name: source["name"].as_str().map(String::from),
            description: source["description"].as_str().map(String::from),
            avatar: source["avatar"].as_str().map(String::from),
            cover: source["cover"].as_str().map(String::from),
            entity_global_score: source["entity_global_score"].as_f64(),
            space_score: source["space_score"].as_f64(),
            entity_space_score: source["entity_space_score"].as_f64(),
            relevance_score: score,
        })
    }
}

#[async_trait]
impl SearchEngineClient for OpenSearchClient {
    #[instrument(skip(self), fields(query = %query.query, scope = ?query.scope))]
    async fn search(&self, query: &SearchQuery) -> Result<SearchResponse, SearchError> {
        // Validate query
        query.validate().map_err(|e| SearchError::InvalidQuery(e))?;

        let search_body = build_search_query(query);

        debug!(body = %search_body, "Executing search");

        let response = self
            .client
            .search(SearchParts::Index(&[&self.index_name]))
            .from(query.offset as i64)
            .size(query.limit as i64)
            .body(search_body)
            .send()
            .await
            .map_err(|e| SearchError::QueryError(e.to_string()))?;

        let status = response.status_code();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            error!(status = %status, body = %error_body, "Search request failed");
            return Err(SearchError::QueryError(format!(
                "Search failed with status {}: {}",
                status, error_body
            )));
        }

        let response_body: Value = response
            .json()
            .await
            .map_err(|e| SearchError::ParseError(e.to_string()))?;

        let took = response_body["took"].as_u64().unwrap_or(0);
        let total = response_body["hits"]["total"]["value"]
            .as_u64()
            .unwrap_or(0);

        let results: Vec<SearchResult> = response_body["hits"]["hits"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(Self::parse_hit)
            .collect();

        debug!(
            total = total,
            returned = results.len(),
            took_ms = took,
            "Search completed"
        );

        Ok(SearchResponse::new(results, total, took))
    }

    #[instrument(skip(self, document), fields(entity_id = %document.entity_id, space_id = %document.space_id))]
    async fn index_document(&self, document: &EntityDocument) -> Result<(), SearchError> {
        let doc_id = Self::document_id(&document.entity_id, &document.space_id);

        let response = self
            .client
            .index(IndexParts::IndexId(&self.index_name, &doc_id))
            .body(document)
            .send()
            .await
            .map_err(|e| SearchError::IndexError(e.to_string()))?;

        let status = response.status_code();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            error!(status = %status, body = %error_body, "Index request failed");
            return Err(SearchError::IndexError(format!(
                "Index failed with status {}: {}",
                status, error_body
            )));
        }

        debug!(doc_id = %doc_id, "Document indexed");
        Ok(())
    }

    #[instrument(skip(self, documents), fields(count = documents.len()))]
    async fn bulk_index(&self, documents: &[EntityDocument]) -> Result<(), SearchError> {
        if documents.is_empty() {
            return Ok(());
        }

        let mut body: Vec<JsonBody<Value>> = Vec::with_capacity(documents.len() * 2);

        for doc in documents {
            let doc_id = Self::document_id(&doc.entity_id, &doc.space_id);
            body.push(json!({"index": {"_id": doc_id}}).into());
            body.push(
                serde_json::to_value(doc)
                    .map_err(|e| SearchError::SerializationError(e.to_string()))?
                    .into(),
            );
        }

        let response = self
            .client
            .bulk(BulkParts::Index(&self.index_name))
            .body(body)
            .send()
            .await
            .map_err(|e| SearchError::IndexError(e.to_string()))?;

        let status = response.status_code();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            error!(status = %status, body = %error_body, "Bulk index request failed");
            return Err(SearchError::BulkIndexError(format!(
                "Bulk index failed with status {}: {}",
                status, error_body
            )));
        }

        let response_body: Value = response
            .json()
            .await
            .map_err(|e| SearchError::ParseError(e.to_string()))?;

        if response_body["errors"].as_bool().unwrap_or(false) {
            // Extract first error for reporting
            let first_error = response_body["items"]
                .as_array()
                .and_then(|items| {
                    items.iter().find_map(|item| {
                        item["index"]["error"]["reason"].as_str().map(String::from)
                    })
                })
                .unwrap_or_else(|| "Unknown bulk index error".to_string());

            warn!(error = %first_error, "Some documents failed to index");
            return Err(SearchError::BulkIndexError(first_error));
        }

        debug!(count = documents.len(), "Bulk index completed");
        Ok(())
    }

    #[instrument(skip(self, request), fields(entity_id = %request.entity_id, space_id = %request.space_id))]
    async fn update_document(&self, request: &OldUpdateEntityRequest) -> Result<(), SearchError> {
        if !request.has_updates() {
            return Ok(());
        }

        let doc_id = Self::document_id(&request.entity_id, &request.space_id);

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

        let response = self
            .client
            .update(UpdateParts::IndexId(&self.index_name, &doc_id))
            .body(json!({"doc": doc}))
            .send()
            .await
            .map_err(|e| SearchError::UpdateError(e.to_string()))?;

        let status = response.status_code();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();

            // 404 means document doesn't exist - could be expected
            if status.as_u16() == 404 {
                return Err(SearchError::NotFound(doc_id));
            }

            error!(status = %status, body = %error_body, "Update request failed");
            return Err(SearchError::UpdateError(format!(
                "Update failed with status {}: {}",
                status, error_body
            )));
        }

        debug!(doc_id = %doc_id, "Document updated");
        Ok(())
    }

    #[instrument(skip(self), fields(entity_id = %entity_id, space_id = %space_id))]
    async fn delete_document(&self, entity_id: &Uuid, space_id: &Uuid) -> Result<(), SearchError> {
        let doc_id = Self::document_id(entity_id, space_id);

        let response = self
            .client
            .delete(DeleteParts::IndexId(&self.index_name, &doc_id))
            .send()
            .await
            .map_err(|e| SearchError::DeleteError(e.to_string()))?;

        let status = response.status_code();

        // 404 is acceptable - document may not exist
        if !status.is_success() && status.as_u16() != 404 {
            let error_body = response.text().await.unwrap_or_default();
            error!(status = %status, body = %error_body, "Delete request failed");
            return Err(SearchError::DeleteError(format!(
                "Delete failed with status {}: {}",
                status, error_body
            )));
        }

        debug!(doc_id = %doc_id, "Document deleted");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn ensure_index_exists(&self) -> Result<(), SearchError> {
        let exists_response = self
            .client
            .indices()
            .exists(IndicesExistsParts::Index(&[&self.index_name]))
            .send()
            .await
            .map_err(|e| SearchError::ConnectionError(e.to_string()))?;

        if exists_response.status_code().is_success() {
            debug!(index = %self.index_name, "Index already exists");
            return Ok(());
        }

        info!(index = %self.index_name, "Creating index");

        let settings = get_index_settings(None);

        let create_response = self
            .client
            .indices()
            .create(IndicesCreateParts::Index(&self.index_name))
            .body(settings)
            .send()
            .await
            .map_err(|e| SearchError::IndexCreationError(e.to_string()))?;

        let status = create_response.status_code();
        if !status.is_success() {
            let error_body = create_response.text().await.unwrap_or_default();
            error!(status = %status, body = %error_body, "Index creation failed");
            return Err(SearchError::IndexCreationError(format!(
                "Index creation failed with status {}: {}",
                status, error_body
            )));
        }

        info!(index = %self.index_name, "Index created successfully");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn health_check(&self) -> Result<bool, SearchError> {
        let response = self
            .client
            .cluster()
            .health(opensearch::cluster::ClusterHealthParts::None)
            .send()
            .await
            .map_err(|e| SearchError::ConnectionError(e.to_string()))?;

        let healthy = response.status_code().is_success();

        if healthy {
            debug!("OpenSearch cluster is healthy");
        } else {
            warn!("OpenSearch cluster health check failed");
        }

        Ok(healthy)
    }
}

#[async_trait]
impl SearchIndexProvider for OpenSearchClient {
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, SearchIndexError> {
        // Use the existing SearchEngineClient implementation
        let response = <Self as SearchEngineClient>::search(self, query)
            .await
            .map_err(|e| match e {
                SearchError::ConnectionError(msg) => SearchIndexError::connection(msg),
                SearchError::QueryError(msg) => SearchIndexError::index(msg),
                SearchError::ParseError(msg) => SearchIndexError::unknown(msg),
                _ => SearchIndexError::unknown(e.to_string()),
            })?;

        Ok(response.results)
    }

    async fn index_document(&self, document: &EntityDocument) -> Result<(), SearchIndexError> {
        <Self as SearchEngineClient>::index_document(self, document)
            .await
            .map_err(|e| match e {
                SearchError::ConnectionError(msg) => SearchIndexError::connection(msg),
                SearchError::IndexError(msg) => SearchIndexError::index(msg),
                _ => SearchIndexError::unknown(e.to_string()),
            })
    }

    async fn update_document(&self, request: &UpdateEntityRequest) -> Result<(), SearchIndexError> {
        // Convert from new UpdateEntityRequest to old UpdateEntityRequest
        let old_request = crate::interfaces::UpdateEntityRequest {
            entity_id: Uuid::parse_str(&request.entity_id)
                .map_err(|e| SearchIndexError::validation(format!("Invalid entity_id: {}", e)))?,
            space_id: Uuid::parse_str(&request.space_id)
                .map_err(|e| SearchIndexError::validation(format!("Invalid space_id: {}", e)))?,
            name: request.name.clone(),
            description: request.description.clone(),
            avatar: request.avatar.clone(),
            cover: request.cover.clone(),
        };

        <Self as SearchEngineClient>::update_document(self, &old_request)
            .await
            .map_err(|e| match e {
                SearchError::NotFound(_msg) => {
                    SearchIndexError::document_not_found(&request.entity_id, &request.space_id)
                }
                SearchError::UpdateError(msg) => SearchIndexError::index(msg),
                SearchError::ConnectionError(msg) => SearchIndexError::connection(msg),
                _ => SearchIndexError::unknown(e.to_string()),
            })
    }

    async fn delete_document(&self, request: &DeleteEntityRequest) -> Result<(), SearchIndexError> {
        let entity_id = Uuid::parse_str(&request.entity_id)
            .map_err(|e| SearchIndexError::validation(format!("Invalid entity_id: {}", e)))?;
        let space_id = Uuid::parse_str(&request.space_id)
            .map_err(|e| SearchIndexError::validation(format!("Invalid space_id: {}", e)))?;

        <Self as SearchEngineClient>::delete_document(self, &entity_id, &space_id)
            .await
            .map_err(|e| match e {
                SearchError::ConnectionError(msg) => SearchIndexError::connection(msg),
                SearchError::DeleteError(msg) => SearchIndexError::index(msg),
                _ => SearchIndexError::unknown(e.to_string()),
            })
    }

    async fn bulk_index_documents(
        &self,
        documents: &[EntityDocument],
    ) -> Result<BatchOperationSummary, SearchIndexError> {
        // Use the existing bulk_index implementation
        <Self as SearchEngineClient>::bulk_index(self, documents)
            .await
            .map_err(|e| SearchIndexError::bulk_operation(e.to_string()))?;

        // Build success summary
        let results: Vec<BatchOperationResult> = documents
            .iter()
            .map(|doc| BatchOperationResult {
                entity_id: doc.entity_id.to_string(),
                space_id: doc.space_id.to_string(),
                success: true,
                error: None,
            })
            .collect();

        Ok(BatchOperationSummary {
            total: documents.len(),
            succeeded: documents.len(),
            failed: 0,
            results,
        })
    }

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
