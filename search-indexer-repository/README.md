# Search Indexer Repository

Repository interfaces and implementations for the search indexer system.

## Overview

This crate provides:

- **SearchEngineClient trait**: Abstract interface for search engine operations
- **OpenSearchClient**: Concrete implementation using OpenSearch

## Architecture

The crate uses a trait-based design for dependency injection, allowing:

- Easy testing with mock implementations
- Swappable search backends
- Clean separation of concerns

```
┌─────────────────────────────────────┐
│       SearchEngineClient            │  (trait)
│  - search()                         │
│  - index_document()                 │
│  - bulk_index()                     │
│  - update_document()                │
│  - delete_document()                │
│  - ensure_index_exists()            │
│  - health_check()                   │
└─────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│        OpenSearchClient             │  (implementation)
│  - Uses opensearch crate            │
│  - Configurable index settings      │
│  - Fuzzy matching & autocomplete    │
└─────────────────────────────────────┘
```

## Usage

```rust
use search_indexer_repository::{OpenSearchClient, SearchEngineClient};
use search_indexer_shared::{EntityDocument, SearchQuery};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let client = OpenSearchClient::new("http://localhost:9200").await?;
    
    // Ensure index exists
    client.ensure_index_exists().await?;
    
    // Index a document
    let doc = EntityDocument::new(
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        "My Entity".to_string(),
        Some("Description".to_string()),
    );
    client.index_document(&doc).await?;
    
    // Search
    let query = SearchQuery::global("entity");
    let results = client.search(&query).await?;
    
    println!("Found {} results", results.total);
    Ok(())
}
```

## Index Configuration

The OpenSearch index is configured with:

- **Autocomplete analyzer**: Edge n-gram tokenizer for search-as-you-type
- **Fuzzy matching**: Allows for typo tolerance
- **Custom field boosting**: Name field boosted 2x over description
- **Rank feature for scores**: Future support for score-based ranking

## Error Handling

All operations return `Result<T, SearchError>` with specific error types:

- `ConnectionError`: Failed to connect to OpenSearch
- `QueryError`: Search query execution failed
- `IndexError`: Document indexing failed
- `BulkIndexError`: Bulk operation partially failed
- `UpdateError`: Document update failed
- `DeleteError`: Document deletion failed

