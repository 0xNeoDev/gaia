/**
 * Search service types.
 *
 * These types define the API contract for the search service.
 */

/**
 * Defines the scope of a search query.
 */
export type SearchScope =
  | "GLOBAL"
  | "GLOBAL_BY_SPACE_SCORE"
  | "SPACE_SINGLE"
  | "SPACE_AND_ALL_SUBSPACES";

/**
 * Search query parameters.
 */
export interface SearchQuery {
  /** The search query string. */
  query: string;
  /** The scope of the search. */
  scope: SearchScope;
  /**
   * Space IDs for space-scoped searches.
   * - For SPACE_SINGLE: exactly one space ID
   * - For SPACE_AND_ALL_SUBSPACES: one or more space IDs
   */
  spaceIds?: string[];
  /** Maximum number of results to return (default: 20, max: 100). */
  limit?: number;
  /** Offset for pagination (default: 0). */
  offset?: number;
}

/**
 * A single search result item.
 */
export interface SearchResult {
  /** The entity's unique identifier. */
  entityId: string;
  /** The space this entity belongs to. */
  spaceId: string;
  /** The entity's display name. */
  name: string;
  /** Optional description text. */
  description?: string;
  /** Optional avatar image URL. */
  avatar?: string;
  /** Optional cover image URL. */
  cover?: string;
  /** Global entity score (null until scoring service is implemented). */
  entityGlobalScore?: number;
  /** Space score (null until scoring service is implemented). */
  spaceScore?: number;
  /** Entity-space score (null until scoring service is implemented). */
  entitySpaceScore?: number;
  /** Relevance score from the search engine. */
  relevanceScore: number;
}

/**
 * Complete search response with results and metadata.
 */
export interface SearchResponse {
  /** The list of search results, ordered by relevance. */
  results: SearchResult[];
  /** Total number of matching documents. */
  total: number;
  /** Time taken to execute the search in milliseconds. */
  tookMs: number;
}

/**
 * Entity document for indexing.
 */
export interface EntityDocument {
  /** The entity's unique identifier. */
  entityId: string;
  /** The space this entity belongs to. */
  spaceId: string;
  /** The entity's display name. */
  name: string;
  /** Optional description text. */
  description?: string;
  /** Optional avatar image URL. */
  avatar?: string;
  /** Optional cover image URL. */
  cover?: string;
  /** Global entity score. */
  entityGlobalScore?: number;
  /** Space score. */
  spaceScore?: number;
  /** Entity-space score. */
  entitySpaceScore?: number;
}

