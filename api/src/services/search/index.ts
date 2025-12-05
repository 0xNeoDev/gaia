/**
 * Search service module.
 *
 * Re-exports all search service components.
 */

export type { SearchClient } from "./client";
export { OpenSearchClient } from "./opensearch";
export type {
  SearchQuery,
  SearchResponse,
  SearchResult,
  SearchScope,
  SearchErrorType,
} from "./types";
export { SearchError, isSearchError } from "./types";

