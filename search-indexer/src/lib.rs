//! # Search Indexer
//!
//! Main library for the Geo Knowledge Graph search indexer.
//!
//! This crate provides the entry point and configuration for running
//! the search indexer pipeline.

pub mod config;

pub use config::Dependencies;

use thiserror::Error;

/// Errors that can occur during indexer initialization or execution.
#[derive(Error, Debug)]
pub enum IndexingError {
    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Pipeline error.
    #[error("Pipeline error: {0}")]
    PipelineError(#[from] search_indexer_pipeline::PipelineError),

    /// Search error.
    #[error("Search error: {0}")]
    SearchError(#[from] search_indexer_repository::SearchError),

    /// IO error.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl IndexingError {
    /// Create a configuration error.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::ConfigError(msg.into())
    }
}

