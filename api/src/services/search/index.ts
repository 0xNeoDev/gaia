/**
 * Search service module.
 *
 * Re-exports all search service components.
 */

export type { SearchClient } from "./client";
export { createSearchClient } from "./client";
export { OpenSearchClient } from "./opensearch";
export type {
  SearchQuery,
  SearchResponse,
  SearchResult,
  SearchScope,
  EntityDocument,
} from "./types";

