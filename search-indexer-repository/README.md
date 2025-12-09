# Search Indexer Repository

Repository interfaces and implementations for the search indexer system.

## Overview

This crate provides:

- **SearchIndexProvider trait**: Abstract interface for search index operations
- **OpenSearchClient**: Concrete implementation using OpenSearch

## Architecture

The crate uses a trait-based design for dependency injection, allowing:

- Easy testing with mock implementations
- Swappable search backends
- Clean separation of concerns

```
┌─────────────────────────────────────┐
│   SearchIndexProvider               │  (trait)
│  - index_document()                 │
│  - update_document()                │
│  - delete_document()                │
│  - bulk_index_documents()           │
│  - bulk_update_documents()          │
│  - bulk_delete_documents()          │
└─────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│        OpenSearchClient             │  (implementation)
│  - Uses opensearch crate            │
│  - Configurable index settings      │
└─────────────────────────────────────┘
```

## Usage

```rust
use search_indexer_repository::{OpenSearchClient, SearchIndexProvider};
use search_indexer_shared::EntityDocument;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    use search_indexer_repository::opensearch::IndexConfig;
    let config = IndexConfig::new("entities", 0);
    let client = OpenSearchClient::new("http://localhost:9200", config).await?;
    
    // Index a document
    let doc = EntityDocument::new(
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        Some("My Entity".to_string()),
        Some("Description".to_string()),
    );
    client.index_document(&doc).await?;
    
    Ok(())
}
```

## Index Configuration

The OpenSearch index is configured with:

- **search_as_you_type fields**: Built-in field type for autocomplete on name and description (uses n-grams internally)
- **rank_feature fields**: Score fields (entity_global_score, space_score, entity_space_score) optimized for relevance boosting

## Error Handling

All operations return `Result<T, SearchIndexError>` with specific error types:

- `ValidationError`: Input validation failed (e.g., invalid UUIDs)
- `ConnectionError`: Failed to connect to OpenSearch
- `IndexError`: Document indexing failed
- `BulkIndexError`: Bulk operation partially failed
- `UpdateError`: Document update failed
- `DeleteError`: Document deletion failed
- `IndexCreationError`: Failed to create the search index
- `ParseError`: Failed to parse response from search index backend
- `SerializationError`: Failed to serialize data for the search index backend
- `DocumentNotFound`: Document not found
- `BatchSizeExceeded`: Batch size exceeds configured maximum
- `Unknown`: Unknown error

