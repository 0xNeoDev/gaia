# Search Indexer Repository Architecture

## Sequence Diagram

```mermaid
sequenceDiagram
    participant App as Application Code
    participant Client as SearchIndexClient
    participant Provider as SearchIndexProvider (Trait)
    participant OpenSearch as OpenSearchClient
    participant Backend as OpenSearch Cluster

    App->>Client: update(UpdateEntityRequest)
    Client->>Client: validate_uuid(entity_id)
    Client->>Client: validate_uuid(space_id)
    Client->>Provider: update_document(request)
    Provider->>OpenSearch: update_document(request)
    OpenSearch->>Backend: HTTP via opensearch crate (/index/_update/{id} with doc_as_upsert)
    Backend-->>OpenSearch: Response (200 OK)
    OpenSearch-->>Provider: Ok(())
    Provider-->>Client: Ok(())
    Client-->>App: Ok(())

    Note over App,Backend: Error Flow (Validation Error)
    App->>Client: update(invalid_request)
    Client->>Client: validate_uuid() fails
    Client-->>App: Err(SearchIndexError::ValidationError)

    Note over App,Backend: Error Flow (Backend Error)
    App->>Client: update(request)
    Client->>Provider: update_document(request)
    Provider->>OpenSearch: update_document(request)
    OpenSearch->>Backend: HTTP via opensearch crate (/index/_update/{id})
    Backend-->>OpenSearch: Response (500 Error)
    OpenSearch-->>Provider: Err(SearchIndexError::UpdateError)
    Provider-->>Client: Err(SearchIndexError::UpdateError)
    Client-->>App: Err(SearchIndexError::UpdateError)
```

## Layered Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Application Code                         │
│  - Uses SearchIndexClient for all operations                    │
│  - Handles SearchIndexError                                     │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                    SearchIndexClient                            │
│  - Validates input (UUIDs, batch sizes)                         │
│  - Converts requests to EntityDocument                          │
│  - Delegates operations to SearchIndexProvider                  │
│  - Returns SearchIndexError                                     │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                SearchIndexProvider (Trait)                      │
│  - Abstract backend interface                                   │
│  - Methods: update_document (upsert), delete_document          │
│    + bulk_update, bulk_delete                                  │
│  - Returns SearchIndexError                                     │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             │ Implementation
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                  OpenSearchClient                               │
│  - Implements SearchIndexProvider                               │
│  - Makes calls to an OpenSearch cluster using the opensearch    │
│    Rust crate for all REST calls                                │
│  - Handles index configuration and OpenSearch-specific logic    │
│  - Converts errors from opensearch crate to SearchIndexError    │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                   OpenSearch Cluster                            │
│  - OpenSearch server                                            │
│  - Stores and indexes documents                                 │
│  - Returns HTTP responses                                       │
└─────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

### SearchIndexClient
- **Input validation**: UUID format, required fields, batch size limits
- **Request handling**: UpdateEntityRequest (upsert: creates or updates)
- **Error handling**: All errors are SearchIndexError
- **Configuration**: Batch size limits, etc.

### SearchIndexProvider (Trait)
- **Abstract interface**: Defines contract for all backend implementations
- **Operation methods**: CRUD and bulk operations
- **Error type**: Returns SearchIndexError for all operations

### OpenSearchClient
- **Implements SearchIndexProvider**: Concrete backend implementation
- **HTTP communication**: All calls to OpenSearch cluster are performed using the [opensearch Rust crate](https://docs.rs/opensearch/)
- **Error conversion**: Translates OpenSearch errors into SearchIndexError
- **Index management**: Handles index creation, aliases, etc.

## Error Flow

All errors propagate as `SearchIndexError` through each layer:

```
OpenSearch Cluster Error
    ↓
OpenSearchClient (converts to SearchIndexError)
    ↓
SearchIndexProvider (passes through)
    ↓
SearchIndexClient (passes through)
    ↓
Application Code (handles SearchIndexError)
```

## Example Data Flow: Updating/Creating a Document

```
1. Application: update(UpdateEntityRequest { entity_id: "123", ... })
   ↓
2. SearchIndexClient: Validates UUIDs
   ↓
3. SearchIndexProvider: update_document(&UpdateEntityRequest)
   ↓
4. OpenSearchClient: Makes HTTP update request with doc_as_upsert=true
   ↓
5. OpenSearch: Creates document if missing, updates if exists, returns 200 OK
   ↓
6. Response flows back: Ok(()) → Application
```

