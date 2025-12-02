//! Interface definitions for the search engine client.
//!
//! This module defines the abstract `SearchEngineClient` trait that allows
//! for dependency injection and swappable search backend implementations.

mod search_engine_client;

pub use search_engine_client::{SearchEngineClient, UpdateEntityRequest};

