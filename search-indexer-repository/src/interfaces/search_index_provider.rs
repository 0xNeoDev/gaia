//! Search index provider trait definition.
//!
//! This module defines the abstract interface for search index operations,
//! allowing for different backend implementations (OpenSearch, Elasticsearch, etc.).

use async_trait::async_trait;

use crate::errors::SearchIndexError;
use crate::types::{BatchOperationSummary, DeleteEntityRequest, UpdateEntityRequest};
use search_indexer_shared::EntityDocument;

/// Abstracts the underlying search index implementation (OpenSearch, Elasticsearch, etc.).
///
/// This trait defines the interface for all search index backend implementations. Implementations
/// are injected into `SearchIndexClient` to enable dependency injection and easy testing with
/// mock implementations.
///
/// All methods return `Result<T, SearchIndexError>` for consistent error handling across
/// different backend implementations.
#[async_trait]
pub trait SearchIndexProvider: Send + Sync {
    /// Index a single document in the search index.
    ///
    /// If a document with the same ID already exists, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `document` - The entity document to index
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the document was indexed successfully
    /// * `Err(SearchIndexError)` - If indexing fails
    async fn index_document(&self, document: &EntityDocument) -> Result<(), SearchIndexError>;

    /// Update specific fields of an existing document.
    ///
    /// Only fields that are `Some` in the request will be updated. The document must
    /// already exist in the index.
    ///
    /// # Arguments
    ///
    /// * `request` - The update request containing entity_id, space_id, and optional fields
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the document was updated successfully
    /// * `Err(SearchIndexError::DocumentNotFound)` - If the document doesn't exist
    /// * `Err(SearchIndexError)` - If the update fails
    async fn update_document(&self, request: &UpdateEntityRequest) -> Result<(), SearchIndexError>;

    /// Delete a document from the search index.
    ///
    /// If the document doesn't exist, the operation is considered successful.
    ///
    /// # Arguments
    ///
    /// * `request` - The delete request containing entity_id and space_id
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the document was deleted (or didn't exist)
    /// * `Err(SearchIndexError)` - If the deletion fails
    async fn delete_document(&self, request: &DeleteEntityRequest) -> Result<(), SearchIndexError>;

    /// Index multiple documents in bulk and return a summary of successful and failed operations.
    ///
    /// This is more efficient than calling `index_document` multiple times. The function
    /// returns a detailed summary including which documents succeeded and which failed.
    ///
    /// # Arguments
    ///
    /// * `documents` - Slice of entity documents to index
    ///
    /// # Returns
    ///
    /// * `Ok(BatchOperationSummary)` - Contains aggregate statistics and individual results
    /// * `Err(SearchIndexError)` - If the bulk operation fails entirely
    async fn bulk_index_documents(
        &self,
        documents: &[EntityDocument],
    ) -> Result<BatchOperationSummary, SearchIndexError>;

    /// Update multiple documents in bulk and return a summary of successful and failed operations.
    ///
    /// Processes each update request individually and collects results. Returns a summary
    /// indicating which updates succeeded and which failed.
    ///
    /// # Arguments
    ///
    /// * `requests` - Slice of update requests
    ///
    /// # Returns
    ///
    /// * `Ok(BatchOperationSummary)` - Contains aggregate statistics and individual results
    /// * `Err(SearchIndexError)` - If the bulk operation fails entirely
    async fn bulk_update_documents(
        &self,
        requests: &[UpdateEntityRequest],
    ) -> Result<BatchOperationSummary, SearchIndexError>;

    /// Delete multiple documents in bulk and return a summary of successful and failed operations.
    ///
    /// Processes each delete request individually and collects results. Documents that don't
    /// exist are considered successful deletions.
    ///
    /// # Arguments
    ///
    /// * `requests` - Slice of delete requests
    ///
    /// # Returns
    ///
    /// * `Ok(BatchOperationSummary)` - Contains aggregate statistics and individual results
    /// * `Err(SearchIndexError)` - If the bulk operation fails entirely
    async fn bulk_delete_documents(
        &self,
        requests: &[DeleteEntityRequest],
    ) -> Result<BatchOperationSummary, SearchIndexError>;
}
