//! Interface definitions for the search engine client.
//!
//! This module defines the abstract `SearchEngineClient` trait that allows
//! for dependency injection and swappable search backend implementations.

mod search_engine_client;
mod search_index_provider;

pub use search_engine_client::{SearchEngineClient, UpdateEntityRequest};
pub use search_index_provider::SearchIndexProvider;
