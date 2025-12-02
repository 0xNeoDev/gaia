//! Search error types.
//!
//! This module defines the error types that can occur during search operations.

use thiserror::Error;

/// Errors that can occur during search engine operations.
#[derive(Error, Debug)]
pub enum SearchError {
    /// Failed to establish connection to the search engine.
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Search query execution failed.
    #[error("Query error: {0}")]
    QueryError(String),

    /// Failed to index a single document.
    #[error("Index error: {0}")]
    IndexError(String),

    /// Bulk indexing operation had failures.
    #[error("Bulk index error: {0}")]
    BulkIndexError(String),

    /// Failed to update a document.
    #[error("Update error: {0}")]
    UpdateError(String),

    /// Failed to delete a document.
    #[error("Delete error: {0}")]
    DeleteError(String),

    /// Failed to create the search index.
    #[error("Index creation error: {0}")]
    IndexCreationError(String),

    /// Failed to parse response from search engine.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Failed to serialize data for the search engine.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// The provided query is invalid.
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// Document not found.
    #[error("Document not found: {0}")]
    NotFound(String),
}

impl SearchError {
    /// Create a connection error.
    pub fn connection(msg: impl Into<String>) -> Self {
        Self::ConnectionError(msg.into())
    }

    /// Create a query error.
    pub fn query(msg: impl Into<String>) -> Self {
        Self::QueryError(msg.into())
    }

    /// Create an index error.
    pub fn index(msg: impl Into<String>) -> Self {
        Self::IndexError(msg.into())
    }

    /// Create a bulk index error.
    pub fn bulk_index(msg: impl Into<String>) -> Self {
        Self::BulkIndexError(msg.into())
    }

    /// Create an invalid query error.
    pub fn invalid_query(msg: impl Into<String>) -> Self {
        Self::InvalidQuery(msg.into())
    }
}

