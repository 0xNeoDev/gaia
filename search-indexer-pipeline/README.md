# Search Indexer Pipeline

Pipeline components for consuming, processing, and loading entity data into the search index.

## Overview

This crate implements the data pipeline for the search indexer using an orchestrator pattern:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Consumer   │ ──▶ │  Processor  │ ──▶ │   Loader    │
│  (Kafka)    │     │ (Transform) │     │ (OpenSearch)│
└─────────────┘     └─────────────┘     └─────────────┘
        │                                      │
        └──────────────────────────────────────┘
                    Orchestrator
```

## Components

### Consumer

Consumes entity events from Kafka topics:
- `knowledge.edits` - Entity create/update events
- `space.creations` - New space events

### Processor

Transforms raw Kafka events into `EntityDocument` structures:
- Extracts entity ID, space ID, name, and description
- Handles different event types (create, update, delete)
- Resolves property values

### Loader

Loads processed documents into OpenSearch:
- Batches documents for efficient bulk indexing
- Handles retries and error recovery
- Manages cursor persistence

### Orchestrator

Coordinates the pipeline components:
- Manages the message flow between components
- Handles shutdown signals
- Monitors pipeline health

## Usage

```rust
use search_indexer_pipeline::{
    consumer::KafkaConsumer,
    processor::EntityProcessor,
    loader::SearchLoader,
    orchestrator::Orchestrator,
};
use search_indexer_repository::OpenSearchClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create components
    let consumer = KafkaConsumer::new("localhost:9092", "search-indexer-group")?;
    let processor = EntityProcessor::new();
    let search_client = OpenSearchClient::new("http://localhost:9200").await?;
    let loader = SearchLoader::new(Box::new(search_client));
    
    // Create and run orchestrator
    let orchestrator = Orchestrator::new(consumer, processor, loader);
    orchestrator.run().await?;
    
    Ok(())
}
```

## Configuration

Environment variables:
- `KAFKA_BROKER` - Kafka broker address (default: localhost:9092)
- `KAFKA_GROUP_ID` - Consumer group ID (default: search-indexer)
- `OPENSEARCH_URL` - OpenSearch URL (default: http://localhost:9200)

