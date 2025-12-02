/**
 * Search client interface.
 *
 * This module defines the abstract interface for search operations,
 * allowing for dependency injection of different implementations.
 */

import type {
  SearchQuery,
  SearchResponse,
  EntityDocument,
} from "./types";

/**
 * Abstract search client interface for dependency injection.
 *
 * Implementations can be swapped for testing or alternative search backends.
 *
 * @example
 * ```typescript
 * // Production
 * const client = new OpenSearchClient("http://localhost:9200");
 *
 * // Testing
 * const client = new MockSearchClient();
 *
 * // Use the same interface
 * const results = await client.search({ query: "test", scope: "GLOBAL" });
 * ```
 */
export interface SearchClient {
  /**
   * Execute a search query against the index.
   *
   * @param query - The search query parameters
   * @returns Promise resolving to search results
   * @throws Error if the search fails
   */
  search(query: SearchQuery): Promise<SearchResponse>;

  /**
   * Index a single document.
   *
   * @param document - The entity document to index
   * @throws Error if indexing fails
   */
  indexDocument(document: EntityDocument): Promise<void>;

  /**
   * Check if the search engine is healthy.
   *
   * @returns Promise resolving to true if healthy
   */
  healthCheck(): Promise<boolean>;
}

/**
 * Create a search client wrapper.
 *
 * This factory function allows for easy dependency injection.
 *
 * @param client - The underlying search client implementation
 * @returns The wrapped search client
 */
export function createSearchClient(client: SearchClient): SearchClient {
  return client;
}

