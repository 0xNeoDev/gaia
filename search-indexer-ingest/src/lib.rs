//! # Search Indexer Ingest
//!
//! This crate provides the ingest components for consuming entity events
//! from Kafka and indexing them into OpenSearch.
//!
//! ## Architecture
//!
//! The ingest follows the Consumer-Processor-Loader pattern:
//!
//! 1. **Consumer**: Receives events from Kafka
//! 2. **Processor**: Transforms events into search documents
//! 3. **Loader**: Indexes documents into OpenSearch
//! 4. **Orchestrator**: Coordinates the ingest flow

pub mod consumer;
pub mod errors;
pub mod loader;
pub mod orchestrator;
pub mod processor;

pub use errors::IngestError;

