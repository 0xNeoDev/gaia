//! OpenSearch implementation of the search index provider.
//!
//! This module provides a concrete implementation of `SearchIndexProvider`
//! using OpenSearch as the backend.

mod client;
mod index_config;

pub use client::OpenSearchClient;
pub use index_config::IndexConfig;
