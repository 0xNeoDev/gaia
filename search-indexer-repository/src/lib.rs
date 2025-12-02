//! # Search Indexer Repository
//!
//! This crate provides traits and implementations for interacting with the
//! search engine. It includes definitions for errors, interfaces, and a
//! concrete implementation for OpenSearch.

pub mod errors;
pub mod interfaces;
pub mod opensearch;

pub use errors::SearchError;
pub use interfaces::{SearchEngineClient, UpdateEntityRequest};
pub use opensearch::OpenSearchClient;

