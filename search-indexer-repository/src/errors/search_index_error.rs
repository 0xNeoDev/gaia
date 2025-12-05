//! Search index error types.
//!
//! This module defines the error types that can occur during search index operations.

use thiserror::Error;

/// Errors that can occur during search index operations.
#[derive(Debug, Clone, Error)]
pub enum SearchIndexError {
    /// Validation error (e.g., missing required fields).
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Failed to establish connection to the search engine.
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Failed to index a document.
    #[error("Index error: {0}")]
    IndexError(String),

    /// Document not found.
    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    /// Bulk operation had failures.
    #[error("Bulk operation error: {0}")]
    BulkOperationError(String),

    /// Batch size exceeds configured maximum.
    #[error("Batch size {provided} exceeds maximum {max}")]
    BatchSizeExceeded { provided: usize, max: usize },

    /// Unknown error.
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl SearchIndexError {
    /// Create a validation error.
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::ValidationError(msg.into())
    }

    /// Create a connection error.
    pub fn connection(msg: impl Into<String>) -> Self {
        Self::ConnectionError(msg.into())
    }

    /// Create an index error.
    pub fn index(msg: impl Into<String>) -> Self {
        Self::IndexError(msg.into())
    }

    /// Create a document not found error.
    pub fn document_not_found(entity_id: &str, space_id: &str) -> Self {
        Self::DocumentNotFound(format!("entity_id={}, space_id={}", entity_id, space_id))
    }

    /// Create a bulk operation error.
    pub fn bulk_operation(msg: impl Into<String>) -> Self {
        Self::BulkOperationError(msg.into())
    }

    /// Create a batch size exceeded error.
    pub fn batch_size_exceeded(provided: usize, max: usize) -> Self {
        Self::BatchSizeExceeded { provided, max }
    }

    /// Create an unknown error.
    pub fn unknown(msg: impl Into<String>) -> Self {
        Self::Unknown(msg.into())
    }
}

