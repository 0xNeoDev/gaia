//! Request and response types for search index operations.

use chrono::Utc;
use uuid::Uuid;

use crate::errors::SearchIndexError;
use search_indexer_shared::EntityDocument;

/// Request to create a new entity document.
/// Same fields as EntityDocument (entity_id, space_id required; all else optional).
#[derive(Debug, Clone)]
pub struct CreateEntityRequest {
    /// The entity's unique identifier.
    pub entity_id: String,
    /// The space this entity belongs to.
    pub space_id: String,
    /// Optional entity display name.
    pub name: Option<String>,
    /// Optional description text.
    pub description: Option<String>,
    /// Optional avatar image URL.
    pub avatar: Option<String>,
    /// Optional cover image URL.
    pub cover: Option<String>,
    /// Global entity score.
    pub entity_global_score: Option<f64>,
    /// Space score.
    pub space_score: Option<f64>,
    /// Entity-space score.
    pub entity_space_score: Option<f64>,
}

impl From<CreateEntityRequest> for EntityDocument {
    fn from(req: CreateEntityRequest) -> Self {
        EntityDocument {
            entity_id: Uuid::parse_str(&req.entity_id).expect("entity_id must be a valid UUID"),
            space_id: Uuid::parse_str(&req.space_id).expect("space_id must be a valid UUID"),
            name: req.name.clone(),
            description: req.description,
            avatar: req.avatar,
            cover: req.cover,
            entity_global_score: req.entity_global_score,
            space_score: req.space_score,
            entity_space_score: req.entity_space_score,
            indexed_at: Utc::now(),
        }
    }
}

/// Request to update an existing entity document.
/// entity_id and space_id are required; all other properties are optional.
#[derive(Debug, Clone)]
pub struct UpdateEntityRequest {
    /// The entity's unique identifier.
    pub entity_id: String,
    /// The space this entity belongs to.
    pub space_id: String,
    /// The entity's display name.
    pub name: Option<String>,
    /// Optional description text.
    pub description: Option<String>,
    /// Optional avatar image URL.
    pub avatar: Option<String>,
    /// Optional cover image URL.
    pub cover: Option<String>,
    /// Global entity score.
    pub entity_global_score: Option<f64>,
    /// Space score.
    pub space_score: Option<f64>,
    /// Entity-space score.
    pub entity_space_score: Option<f64>,
}

/// Request to delete an entity document.
#[derive(Debug, Clone)]
pub struct DeleteEntityRequest {
    /// The entity's unique identifier.
    pub entity_id: String,
    /// The space this entity belongs to.
    pub space_id: String,
}

/// Result of a batch operation for a single item.
#[derive(Debug, Clone)]
pub struct BatchOperationResult {
    /// The entity's unique identifier.
    pub entity_id: String,
    /// The space this entity belongs to.
    pub space_id: String,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error if the operation failed.
    pub error: Option<SearchIndexError>,
}

/// Summary of a batch operation.
#[derive(Debug, Clone)]
pub struct BatchOperationSummary {
    /// Total number of items in the batch.
    pub total: usize,
    /// Number of successful operations.
    pub succeeded: usize,
    /// Number of failed operations.
    pub failed: usize,
    /// Individual results for each item.
    pub results: Vec<BatchOperationResult>,
}
