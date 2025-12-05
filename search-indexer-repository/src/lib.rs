//! # Search Indexer Repository
//!
//! This crate provides traits and implementations for interacting with the
//! search engine. It includes definitions for errors, interfaces, and a
//! concrete implementation for OpenSearch.

pub mod client;
pub mod config;
pub mod errors;
pub mod interfaces;
pub mod opensearch;
pub mod types;

pub use client::SearchIndexClient;
pub use config::SearchIndexConfig;
pub use errors::{SearchError, SearchIndexError};
pub use interfaces::{SearchEngineClient, SearchIndexProvider, UpdateEntityRequest};
pub use opensearch::OpenSearchClient;
pub use types::{
    BatchOperationResult, BatchOperationSummary, CreateEntityRequest, DeleteEntityRequest,
};
