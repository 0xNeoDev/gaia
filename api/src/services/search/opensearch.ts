/**
 * OpenSearch client implementation.
 *
 * This module provides the concrete implementation of the SearchClient
 * interface using OpenSearch as the backend.
 */

import { Client } from "@opensearch-project/opensearch";
import type { SearchClient } from "./client";
import type {
  SearchQuery,
  SearchResponse,
  SearchResult,
  EntityDocument,
  SearchScope,
} from "./types";

/**
 * UUID regex pattern for detecting ID-based queries.
 */
const UUID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/**
 * OpenSearch client implementation.
 *
 * @example
 * ```typescript
 * const client = new OpenSearchClient("http://localhost:9200");
 * await client.healthCheck();
 *
 * const results = await client.search({
 *   query: "blockchain",
 *   scope: "GLOBAL",
 * });
 * ```
 */
export class OpenSearchClient implements SearchClient {
  private client: Client;
  private indexName: string;

  /**
   * Create a new OpenSearch client.
   *
   * @param nodeUrl - The OpenSearch server URL
   * @param indexName - The index name to use (default: "entities")
   */
  constructor(nodeUrl: string, indexName: string = "entities") {
    this.client = new Client({ node: nodeUrl });
    this.indexName = indexName;
  }

  /**
   * Execute a search query against the index.
   */
  async search(query: SearchQuery): Promise<SearchResponse> {
    const searchBody = this.buildSearchBody(query);

    const response = await this.client.search({
      index: this.indexName,
      body: searchBody,
      from: query.offset ?? 0,
      size: Math.min(query.limit ?? 20, 100),
    });

    const body = response.body;
    const hits = body.hits.hits as Array<{
      _source: Record<string, unknown>;
      _score: number;
    }>;

    const results: SearchResult[] = hits.map((hit) => ({
      entityId: hit._source.entity_id as string,
      spaceId: hit._source.space_id as string,
      name: hit._source.name as string,
      description: hit._source.description as string | undefined,
      avatar: hit._source.avatar as string | undefined,
      cover: hit._source.cover as string | undefined,
      entityGlobalScore: hit._source.entity_global_score as number | undefined,
      spaceScore: hit._source.space_score as number | undefined,
      entitySpaceScore: hit._source.entity_space_score as number | undefined,
      relevanceScore: hit._score,
    }));

    return {
      results,
      total:
        typeof body.hits.total === "number"
          ? body.hits.total
          : body.hits.total.value,
      tookMs: body.took,
    };
  }

  /**
   * Index a single document.
   */
  async indexDocument(document: EntityDocument): Promise<void> {
    const docId = `${document.entityId}_${document.spaceId}`;

    await this.client.index({
      index: this.indexName,
      id: docId,
      body: {
        entity_id: document.entityId,
        space_id: document.spaceId,
        name: document.name,
        description: document.description,
        avatar: document.avatar,
        cover: document.cover,
        entity_global_score: document.entityGlobalScore,
        space_score: document.spaceScore,
        entity_space_score: document.entitySpaceScore,
        indexed_at: new Date().toISOString(),
      },
    });
  }

  /**
   * Check if the search engine is healthy.
   */
  async healthCheck(): Promise<boolean> {
    try {
      const health = await this.client.cluster.health({});
      return health.statusCode === 200;
    } catch {
      return false;
    }
  }

  /**
   * Build the OpenSearch query body based on search parameters.
   */
  private buildSearchBody(query: SearchQuery): object {
    // Check if the query is a UUID for direct ID lookup
    if (UUID_PATTERN.test(query.query)) {
      return this.buildUuidQuery(query.query, query.scope, query.spaceIds);
    }

    // Build base text search query
    const baseTextQuery = this.buildBaseTextQuery(query.query);

    // Apply scope-specific query building
    switch (query.scope) {
      case "GLOBAL":
        return this.buildGlobalQuery(baseTextQuery);

      case "GLOBAL_BY_SPACE_SCORE":
        return this.buildGlobalBySpaceScoreQuery(baseTextQuery);

      case "SPACE_SINGLE":
        if (query.spaceIds && query.spaceIds.length > 0) {
          return this.buildSingleSpaceQuery(baseTextQuery, query.spaceIds[0]);
        }
        return this.buildGlobalQuery(baseTextQuery);

      case "SPACE_AND_ALL_SUBSPACES":
        if (query.spaceIds && query.spaceIds.length > 0) {
          return this.buildMultiSpaceQuery(baseTextQuery, query.spaceIds);
        }
        return this.buildGlobalQuery(baseTextQuery);

      default:
        return this.buildGlobalQuery(baseTextQuery);
    }
  }

  /**
   * Build a query for UUID-based lookups.
   * Performs a direct lookup on entity_id field with scope filtering applied.
   *
   * Uses `term` query which is the correct query type for exact matches
   * on keyword fields. The entity_id field is indexed as a keyword type
   * in the OpenSearch index mapping.
   */
  private buildUuidQuery(
    uuid: string,
    scope: SearchScope,
    spaceIds?: string[]
  ): object {
    // term query is correct for keyword fields - performs exact match lookup
    const baseUuidQuery = {
      term: { entity_id: uuid },
    };

    // Apply scope-specific filtering
    switch (scope) {
      case "GLOBAL":
        return {
          query: baseUuidQuery,
        };

      case "GLOBAL_BY_SPACE_SCORE":
        return {
          query: {
            bool: {
              must: [baseUuidQuery],
              should: [
                {
                  rank_feature: {
                    field: "space_score",
                    boost: 1.3,
                  },
                },
              ],
            },
          },
        };

      case "SPACE_SINGLE":
        if (spaceIds && spaceIds.length > 0) {
          return {
            query: {
              bool: {
                must: [baseUuidQuery],
                filter: [{ term: { space_id: spaceIds[0] } }],
                should: [
                  {
                    rank_feature: {
                      field: "entity_space_score",
                      boost: 1.3,
                    },
                  },
                ],
              },
            },
          };
        }
        return {
          query: baseUuidQuery,
        };

      case "SPACE_AND_ALL_SUBSPACES":
        if (spaceIds && spaceIds.length > 0) {
          return {
            query: {
              bool: {
                must: [baseUuidQuery],
                filter: [{ terms: { space_id: spaceIds } }],
                should: [
                  {
                    rank_feature: {
                      field: "entity_space_score",
                      boost: 1.3,
                    },
                  },
                ],
              },
            },
          };
        }
        return {
          query: baseUuidQuery,
        };

      default:
        return {
          query: baseUuidQuery,
        };
    }
  }

  /**
   * Build the base text search query used across all scopes.
   *
   * Uses:
   * - multi_match with bool_prefix for autocomplete on search_as_you_type fields
   * - Fuzzy multi_match for typo tolerance
   * - match_phrase_prefix for strong prefix matching on name and description
   */
  private buildBaseTextQuery(queryText: string): object {
    return {
      bool: {
        should: [
          {
            // Autocomplete-style match over n-grams with higher weight on name
            multi_match: {
              query: queryText,
              type: "bool_prefix",
              fields: [
                "name^1.5",
                "name._2gram^1.5",
                "name._3gram^1.5",
                "description",
                "description._2gram",
                "description._3gram",
              ],
            },
          },
          {
            // Fuzzy text match to tolerate minor typos
            // AUTO fuzziness: 1-2 chars: 0 edits, 3-4 chars: 1 edit, 5+ chars: 2 edits
            multi_match: {
              query: queryText,
              fields: ["name", "description"],
              fuzziness: "AUTO",
              boost: 0.6,
            },
          },
          {
            // Strongly boost documents where the name starts with the query text
            match_phrase_prefix: {
              name: {
                query: queryText,
                boost: 2.0,
              },
            },
          },
          {
            // Moderately boost documents where the description starts with the query text
            match_phrase_prefix: {
              description: {
                query: queryText,
                boost: 1.5,
              },
            },
          },
        ],
        minimum_should_match: 1,
      },
    };
  }

  /**
   * Build a global search query.
   * Boosts results by entity_global_score using rank_feature.
   */
  private buildGlobalQuery(baseTextQuery: object): object {
    return {
      query: {
        bool: {
          must: [baseTextQuery],
          should: [
            {
              rank_feature: {
                field: "entity_global_score",
                boost: 1.3,
              },
            },
          ],
        },
      },
    };
  }

  /**
   * Build a global search query ranked by space score.
   * Boosts results by space_score using rank_feature.
   */
  private buildGlobalBySpaceScoreQuery(baseTextQuery: object): object {
    return {
      query: {
        bool: {
          must: [baseTextQuery],
          should: [
            {
              rank_feature: {
                field: "space_score",
                boost: 1.3,
              },
            },
          ],
        },
      },
    };
  }

  /**
   * Build a single-space filtered query.
   * Filters by a single space_id and boosts by entity_space_score.
   */
  private buildSingleSpaceQuery(
    baseTextQuery: object,
    spaceId: string
  ): object {
    return {
      query: {
        bool: {
          must: [baseTextQuery],
          filter: [{ term: { space_id: spaceId } }],
          should: [
            {
              rank_feature: {
                field: "entity_space_score",
                boost: 1.3,
              },
            },
          ],
        },
      },
    };
  }

  /**
   * Build a multi-space filtered query.
   * Filters by multiple space_ids and boosts by entity_space_score.
   */
  private buildMultiSpaceQuery(
    baseTextQuery: object,
    spaceIds: string[]
  ): object {
    return {
      query: {
        bool: {
          must: [baseTextQuery],
          filter: [{ terms: { space_id: spaceIds } }],
          should: [
            {
              rank_feature: {
                field: "entity_space_score",
                boost: 1.3,
              },
            },
          ],
        },
      },
    };
  }
}
