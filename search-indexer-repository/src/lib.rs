//! # Search Indexer Repository
//!
//! This crate provides traits and implementations for interacting with the
//! search index. It includes definitions for errors, interfaces, and a
//! concrete implementation for OpenSearch.

pub mod client;
pub mod config;
pub mod errors;
pub mod interfaces;
pub mod opensearch;
pub mod types;
pub mod utils;

pub use client::SearchIndexClient;
pub use config::SearchIndexConfig;
pub use errors::SearchIndexError;
pub use interfaces::SearchIndexProvider;
pub use opensearch::OpenSearchClient;
pub use types::{
    BatchOperationResult, BatchOperationSummary, DeleteEntityRequest, UnsetEntityPropertiesRequest,
    UpdateEntityRequest,
};
pub use utils::parse_entity_and_space_ids;
