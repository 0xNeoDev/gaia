//! Search index provider trait definition.
//!
//! This module defines the abstract interface for search index operations,
//! allowing for different backend implementations (OpenSearch, Elasticsearch, etc.).

use async_trait::async_trait;

use crate::errors::SearchIndexError;
use crate::types::{BatchOperationSummary, DeleteEntityRequest, UpdateEntityRequest};
use search_indexer_shared::{EntityDocument, SearchQuery, SearchResult};

/// Abstracts the underlying search index implementation (OpenSearch, Elasticsearch, etc.).
/// Implementations of this trait are injected into SearchIndexClient.
#[async_trait]
pub trait SearchIndexProvider: Send + Sync {
    /// Execute a search query.
    async fn search(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, SearchIndexError>;

    /// Index a single document.
    async fn index_document(
        &self,
        document: &EntityDocument,
    ) -> Result<(), SearchIndexError>;

    /// Update a single document.
    async fn update_document(
        &self,
        request: &UpdateEntityRequest,
    ) -> Result<(), SearchIndexError>;

    /// Delete a single document.
    async fn delete_document(
        &self,
        request: &DeleteEntityRequest,
    ) -> Result<(), SearchIndexError>;

    /// Bulk index multiple documents.
    async fn bulk_index_documents(
        &self,
        documents: &[EntityDocument],
    ) -> Result<BatchOperationSummary, SearchIndexError>;

    /// Bulk update multiple documents.
    async fn bulk_update_documents(
        &self,
        requests: &[UpdateEntityRequest],
    ) -> Result<BatchOperationSummary, SearchIndexError>;

    /// Bulk delete multiple documents.
    async fn bulk_delete_documents(
        &self,
        requests: &[DeleteEntityRequest],
    ) -> Result<BatchOperationSummary, SearchIndexError>;
}

