//! Consumer module for the search indexer pipeline.
//!
//! Provides Kafka consumer functionality for receiving entity events.

mod kafka_consumer;
mod messages;

pub use kafka_consumer::KafkaConsumer;
pub use messages::{EntityEvent, EntityEventType, StreamMessage};

