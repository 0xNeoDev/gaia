//! OpenSearch implementation of the search engine client.
//!
//! This module provides a concrete implementation of `SearchEngineClient`
//! using OpenSearch as the backend.

mod client;
mod index_config;
mod queries;

pub use client::OpenSearchClient;
pub use index_config::IndexConfig;
