/**
 * Search route handler.
 *
 * Provides HTTP endpoints for full-text search across the Knowledge Graph.
 */

import { Hono } from "hono";
import type { SearchClient, SearchScope } from "../services/search";

/**
 * Valid search scope values.
 */
const VALID_SCOPES: Set<SearchScope> = new Set([
  "GLOBAL",
  "GLOBAL_BY_SPACE_SCORE",
  "SPACE_SINGLE",
  "SPACE",
]);

/**
 * UUID regex pattern for validation.
 */
const UUID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/**
 * Create the search router with dependency-injected search client.
 *
 * @param searchClient - The search client to use for queries
 * @returns Configured Hono router
 *
 * @example
 * ```typescript
 * import { createSearchRouter } from "./src/search";
 * import { OpenSearchClient } from "./src/services/search";
 *
 * const searchClient = new OpenSearchClient("http://localhost:9200");
 * app.route("/search", createSearchRouter(searchClient));
 * ```
 */
export function createSearchRouter(searchClient: SearchClient) {
  const router = new Hono();

  /**
   * GET /search
   *
   * Search for entities across the Knowledge Graph.
   *
   * Query Parameters:
   * - query or q: Search query string (required, min 2 characters)
   * - scope: Search scope (optional, default: GLOBAL)
   * - space_id: Space ID for space-scoped searches (required for SPACE_* scopes)
   * - limit: Maximum results (optional, default: 20, max: 100)
   * - offset: Pagination offset (optional, default: 0)
   *
   * Response:
   * - 200: SearchResponse with results
   * - 400: Invalid request parameters
   * - 500: Search failed
   */
  router.get("/", async (c) => {
    // Extract query parameters (accepts "query" first or "q" second)
    const query = c.req.query("query") ?? c.req.query("q");
    const scopeParam = c.req.query("scope") ?? "GLOBAL";
    const spaceId = c.req.query("space_id");
    const limitParam = c.req.query("limit");
    const offsetParam = c.req.query("offset");

    // Validate query
    if (!query || query.trim().length === 0) {
      return c.json(
        {
          error: "Missing required parameter",
          message: "Query parameter 'query' or 'q' is required",
        },
        400
      );
    }

    // if (query.trim().length < 2) {
    //   return c.json(
    //     {
    //       error: "Invalid parameter",
    //       message: "Query must be at least 2 characters",
    //     },
    //     400
    //   );
    // }

    // Validate scope
    if (!VALID_SCOPES.has(scopeParam as SearchScope)) {
      return c.json(
        {
          error: "Invalid parameter",
          message: `Invalid scope '${scopeParam}'. Valid values: ${Array.from(VALID_SCOPES).join(", ")}`,
        },
        400
      );
    }
    const scope = scopeParam as SearchScope;

    // Validate space_id for space-scoped searches
    if (scope === "SPACE_SINGLE" || scope === "SPACE") {
      if (!spaceId) {
        return c.json(
          {
            error: "Missing required parameter",
            message: `space_id is required for ${scope} scope`,
          },
          400
        );
      }

      if (!UUID_PATTERN.test(spaceId)) {
        return c.json(
          {
            error: "Invalid parameter",
            message: "space_id must be a valid UUID",
          },
          400
        );
      }
    }

    // Parse and validate limit
    let limit = 20;
    if (limitParam) {
      const parsedLimit = parseInt(limitParam, 10);
      if (isNaN(parsedLimit) || parsedLimit < 1) {
        return c.json(
          {
            error: "Invalid parameter",
            message: "limit must be a positive integer",
          },
          400
        );
      }
      limit = Math.min(parsedLimit, 100);
    }

    // Parse and validate offset
    let offset = 0;
    if (offsetParam) {
      const parsedOffset = parseInt(offsetParam, 10);
      if (isNaN(parsedOffset) || parsedOffset < 0) {
        return c.json(
          {
            error: "Invalid parameter",
            message: "offset must be a non-negative integer",
          },
          400
        );
      }
      offset = parsedOffset;
    }

    // Execute search
    try {
      const trimmedQuery = query.trim();

      const response = await searchClient.search({
        query: trimmedQuery,
        scope,
        space_id: spaceId,
        limit,
        offset,
      });

      return c.json(response);
    } catch (error) {
      console.error("[SEARCH] Search error:", error);
      return c.json(
        {
          error: "Search failed",
          message:
            error instanceof Error ? error.message : "An unexpected error occurred",
        },
        500
      );
    }
  });

  /**
   * GET /search/health
   *
   * Check the health of the search service.
   *
   * Response:
   * - 200: { status: "healthy" }
   * - 503: { status: "unhealthy" }
   */
  router.get("/health", async (c) => {
    try {
      const healthy = await searchClient.healthCheck();
      if (healthy) {
        return c.json({ status: "healthy" });
      }
      return c.json({ status: "unhealthy" }, 503);
    } catch (error) {
      console.error("[SEARCH] Health check error:", error);
      return c.json({ status: "unhealthy" }, 503);
    }
  });

  return router;
}

