//! Error types for the search indexer ingest.

use search_indexer_repository::SearchError;
use thiserror::Error;

/// Errors that can occur in the search indexer ingest.
#[derive(Error, Debug)]
pub enum IngestError {
    /// Error from the consumer component.
    #[error("Consumer error: {0}")]
    ConsumerError(String),

    /// Error from the processor component.
    #[error("Processor error: {0}")]
    ProcessorError(String),

    /// Error from the loader component.
    #[error("Loader error: {0}")]
    LoaderError(String),

    /// Error from the search engine.
    #[error("Search error: {0}")]
    SearchError(#[from] SearchError),

    /// Kafka-related error.
    #[error("Kafka error: {0}")]
    KafkaError(String),

    /// Error parsing or decoding data.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Channel communication error.
    #[error("Channel error: {0}")]
    ChannelError(String),

    /// Ingest was cancelled or interrupted.
    #[error("Ingest cancelled")]
    Cancelled,
}

impl IngestError {
    /// Create a consumer error.
    pub fn consumer(msg: impl Into<String>) -> Self {
        Self::ConsumerError(msg.into())
    }

    /// Create a processor error.
    pub fn processor(msg: impl Into<String>) -> Self {
        Self::ProcessorError(msg.into())
    }

    /// Create a loader error.
    pub fn loader(msg: impl Into<String>) -> Self {
        Self::LoaderError(msg.into())
    }

    /// Create a Kafka error.
    pub fn kafka(msg: impl Into<String>) -> Self {
        Self::KafkaError(msg.into())
    }

    /// Create a parse error.
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::ParseError(msg.into())
    }
}

impl From<rdkafka::error::KafkaError> for IngestError {
    fn from(err: rdkafka::error::KafkaError) -> Self {
        Self::KafkaError(err.to_string())
    }
}

