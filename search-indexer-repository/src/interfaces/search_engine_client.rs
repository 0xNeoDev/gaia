//! Search engine client trait definition.
//!
//! This module defines the abstract interface for search engine operations,
//! allowing for different backend implementations (OpenSearch, Elasticsearch, etc.).

use async_trait::async_trait;
use uuid::Uuid;

use crate::errors::SearchError;
use search_indexer_shared::{EntityDocument, SearchQuery, SearchResponse};

/// Request to update specific fields of an entity document.
///
/// Only fields that are `Some` will be updated. Fields that are `None`
/// will be left unchanged in the search index.
#[derive(Debug, Clone, Default)]
pub struct UpdateEntityRequest {
    /// The entity's unique identifier (required).
    pub entity_id: Uuid,
    /// The space this entity belongs to (required).
    pub space_id: Uuid,
    /// New name for the entity.
    pub name: Option<String>,
    /// New description for the entity.
    pub description: Option<String>,
    /// New avatar URL.
    pub avatar: Option<String>,
    /// New cover image URL.
    pub cover: Option<String>,
}

impl UpdateEntityRequest {
    /// Create a new update request for the given entity and space.
    pub fn new(entity_id: Uuid, space_id: Uuid) -> Self {
        Self {
            entity_id,
            space_id,
            name: None,
            description: None,
            avatar: None,
            cover: None,
        }
    }

    /// Set the name to update.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the description to update.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the avatar URL to update.
    pub fn with_avatar(mut self, avatar: impl Into<String>) -> Self {
        self.avatar = Some(avatar.into());
        self
    }

    /// Set the cover URL to update.
    pub fn with_cover(mut self, cover: impl Into<String>) -> Self {
        self.cover = Some(cover.into());
        self
    }

    /// Check if any fields are set for update.
    pub fn has_updates(&self) -> bool {
        self.name.is_some()
            || self.description.is_some()
            || self.avatar.is_some()
            || self.cover.is_some()
    }
}

/// Abstract interface for search engine operations.
///
/// This trait defines all the operations required to interact with a search engine.
/// Implementations can be swapped for different backends (OpenSearch, mock, etc.)
/// enabling easy testing and potential future migrations.
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to allow use across async tasks.
///
/// # Error Handling
///
/// All methods return `Result<T, SearchError>` for consistent error handling.
#[async_trait]
pub trait SearchEngineClient: Send + Sync {
    /// Execute a search query against the index.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query parameters including query text, scope, and pagination
    ///
    /// # Returns
    ///
    /// * `Ok(SearchResponse)` - The search results with metadata
    /// * `Err(SearchError)` - If the search fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let query = SearchQuery::global("blockchain");
    /// let response = client.search(&query).await?;
    /// println!("Found {} results", response.total);
    /// ```
    async fn search(&self, query: &SearchQuery) -> Result<SearchResponse, SearchError>;

    /// Index a single document in the search engine.
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
    /// * `Err(SearchError)` - If indexing fails
    async fn index_document(&self, document: &EntityDocument) -> Result<(), SearchError>;

    /// Index multiple documents in a single bulk operation.
    ///
    /// This is more efficient than calling `index_document` multiple times.
    ///
    /// # Arguments
    ///
    /// * `documents` - Slice of entity documents to index
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If all documents were indexed successfully
    /// * `Err(SearchError::BulkIndexError)` - If any documents failed to index
    async fn bulk_index(&self, documents: &[EntityDocument]) -> Result<(), SearchError>;

    /// Update specific fields of an existing document.
    ///
    /// Only the fields specified in the request will be updated.
    /// The document must already exist in the index.
    ///
    /// # Arguments
    ///
    /// * `request` - The update request with entity ID and fields to update
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the document was updated successfully
    /// * `Err(SearchError)` - If the update fails
    async fn update_document(&self, request: &UpdateEntityRequest) -> Result<(), SearchError>;

    /// Delete a document from the search index.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity's unique identifier
    /// * `space_id` - The space the entity belongs to
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the document was deleted (or didn't exist)
    /// * `Err(SearchError)` - If the deletion fails
    async fn delete_document(&self, entity_id: &Uuid, space_id: &Uuid) -> Result<(), SearchError>;

    /// Ensure the search index exists with proper mappings.
    ///
    /// If the index doesn't exist, it will be created with the appropriate
    /// settings and mappings for entity search.
    ///
    /// This should be called during application startup.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the index exists or was created successfully
    /// * `Err(SearchError)` - If index creation fails
    async fn ensure_index_exists(&self) -> Result<(), SearchError>;

    /// Check if the search engine is healthy and reachable.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - If the search engine is healthy
    /// * `Ok(false)` - If the search engine is unhealthy
    /// * `Err(SearchError)` - If the health check fails to execute
    async fn health_check(&self) -> Result<bool, SearchError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_request_builder() {
        let entity_id = Uuid::new_v4();
        let space_id = Uuid::new_v4();

        let request = UpdateEntityRequest::new(entity_id, space_id)
            .with_name("New Name")
            .with_description("New Description");

        assert_eq!(request.entity_id, entity_id);
        assert_eq!(request.space_id, space_id);
        assert_eq!(request.name, Some("New Name".to_string()));
        assert_eq!(request.description, Some("New Description".to_string()));
        assert!(request.avatar.is_none());
        assert!(request.has_updates());
    }

    #[test]
    fn test_update_request_has_updates() {
        let request = UpdateEntityRequest::new(Uuid::new_v4(), Uuid::new_v4());
        assert!(!request.has_updates());

        let request = request.with_name("Name");
        assert!(request.has_updates());
    }
}

