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
    use search_indexer_repository::opensearch::IndexConfig;
    let config = IndexConfig::new("entities", 0);
    let client = OpenSearchClient::new("http://localhost:9200", config).await?;
    
    // Ensure index exists
    client.ensure_index_exists().await?;
    
    // Index a document
    let doc = EntityDocument::new(
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        Some("My Entity".to_string()),
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

- **search_as_you_type fields**: Built-in field type for autocomplete on name and description (uses n-grams internally)
- **Fuzzy matching**: Allows for typo tolerance with AUTO fuzziness
- **Custom field boosting**: Name field boosted higher than description (via multi-match and match_phrase_prefix queries)
- **rank_feature fields**: Score fields (entity_global_score, space_score, entity_space_score) optimized for relevance boosting

## Error Handling

All operations return `Result<T, SearchError>` with specific error types:

- `ConnectionError`: Failed to connect to OpenSearch
- `QueryError`: Search query execution failed
- `InvalidQuery`: The provided query is invalid
- `IndexError`: Document indexing failed
- `BulkIndexError`: Bulk operation partially failed
- `UpdateError`: Document update failed
- `DeleteError`: Document deletion failed
- `IndexCreationError`: Failed to create the search index
- `ParseError`: Failed to parse response from search engine
- `SerializationError`: Failed to serialize data for the search engine
- `NotFound`: Document not found

