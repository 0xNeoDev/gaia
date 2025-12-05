//! OpenSearch query builders.
//!
//! This module provides functions to build OpenSearch queries based on
//! search parameters and scope.

use serde_json::{json, Value};
use uuid::Uuid;

use search_indexer_shared::{SearchQuery, SearchScope};

/// Build an OpenSearch query from a SearchQuery.
///
/// The query builder handles:
/// - UUID detection for direct ID lookups
/// - Multi-match queries with `search_as_you_type` field boosting
/// - Fuzzy matching for typo tolerance
/// - `match_phrase_prefix` for strong prefix matching
/// - Space filtering for scoped searches (single or multiple space IDs)
/// - `rank_feature` boosts based on search scope
pub fn build_search_query(query: &SearchQuery) -> Value {
    // If the query looks like a UUID, do a direct ID lookup
    if query.is_uuid_query() {
        return build_uuid_query(&query.query);
    }

    // Build the base text query (shared across all scopes)
    let base_text_query = build_base_text_query(&query.query);

    // Build scope-specific query with rank_feature boosts
    match query.scope {
        SearchScope::Global => build_global_query(base_text_query),
        SearchScope::GlobalBySpaceScore => build_global_by_space_score_query(base_text_query),
        SearchScope::SpaceSingle => {
            if let Some(space_ids) = &query.space_ids {
                if let Some(space_id) = space_ids.first() {
                    build_single_space_query(base_text_query, *space_id)
                } else {
                    // Fallback to global if empty list
                    build_global_query(base_text_query)
                }
            } else {
                // Fallback to global if no space_ids provided
                build_global_query(base_text_query)
            }
        }
        SearchScope::Space => {
            if let Some(space_ids) = &query.space_ids {
                if !space_ids.is_empty() {
                    build_multi_space_query(base_text_query, space_ids)
                } else {
                    build_global_query(base_text_query)
                }
            } else {
                build_global_query(base_text_query)
            }
        }
    }
}

/// Build a query for UUID lookups.
///
/// Searches both entity_id and space_id fields for direct matches.
fn build_uuid_query(uuid_str: &str) -> Value {
    json!({
        "query": {
            "bool": {
                "should": [
                    { "term": { "entity_id": uuid_str } },
                    { "term": { "space_id": uuid_str } }
                ],
                "minimum_should_match": 1
            }
        }
    })
}

/// Build the base text query used across all scopes.
///
/// This query uses:
/// - `multi_match` with `bool_prefix` type for autocomplete on `search_as_you_type` fields
/// - Fuzzy `multi_match` for typo tolerance (AUTO fuzziness)
/// - `match_phrase_prefix` for strong prefix matching on name and description
fn build_base_text_query(query_text: &str) -> Value {
    json!({
        "bool": {
            "should": [
                {
                    // Autocomplete-style match over n-grams with higher weight on name
                    "multi_match": {
                        "query": query_text,
                        "type": "bool_prefix",
                        "fields": [
                            "name^1.5",
                            "name._2gram^1.5",
                            "name._3gram^1.5",
                            "description",
                            "description._2gram",
                            "description._3gram"
                        ]
                    }
                },
                {
                    // Fuzzy text match to tolerate minor typos
                    // AUTO fuzziness allows variable edits based on query length:
                    // 1-2 chars: 0 edits, 3-4 chars: 1 edit, 5+ chars: 2 edits
                    "multi_match": {
                        "query": query_text,
                        "fields": ["name", "description"],
                        "fuzziness": "AUTO",
                        "boost": 0.6
                    }
                },
                {
                    // Strongly boost documents where the name starts with the query text
                    "match_phrase_prefix": {
                        "name": {
                            "query": query_text,
                            "boost": 2.0
                        }
                    }
                },
                {
                    // Moderately boost documents where the description starts with the query text
                    "match_phrase_prefix": {
                        "description": {
                            "query": query_text,
                            "boost": 1.5
                        }
                    }
                }
            ],
            // Enforce that at least one of the text-based clauses must match
            "minimum_should_match": 1
        }
    })
}

/// Build a global search query.
///
/// Boosts results by `entity_global_score` using rank_feature.
fn build_global_query(base_text_query: Value) -> Value {
    json!({
        "query": {
            "bool": {
                "must": [base_text_query],
                "should": [
                    {
                        "rank_feature": {
                            "field": "entity_global_score",
                            "boost": 1.3
                        }
                    }
                ]
            }
        }
    })
}

/// Build a global search query ranked by space score.
///
/// Boosts results by `space_score` using rank_feature.
fn build_global_by_space_score_query(base_text_query: Value) -> Value {
    json!({
        "query": {
            "bool": {
                "must": [base_text_query],
                "should": [
                    {
                        "rank_feature": {
                            "field": "space_score",
                            "boost": 1.3
                        }
                    }
                ]
            }
        }
    })
}

/// Build a single-space filtered query.
///
/// Filters by a single space_id and boosts by `entity_space_score` using rank_feature.
fn build_single_space_query(base_text_query: Value, space_id: Uuid) -> Value {
    json!({
        "query": {
            "bool": {
                "must": [base_text_query],
                "filter": [
                    { "term": { "space_id": space_id.to_string() } }
                ],
                "should": [
                    {
                        "rank_feature": {
                            "field": "entity_space_score",
                            "boost": 1.3
                        }
                    }
                ]
            }
        }
    })
}

/// Build a query to filter by multiple space IDs.
///
/// Used for Space scope when we have the list of subspace IDs.
/// Boosts by `entity_space_score` using rank_feature.
fn build_multi_space_query(base_text_query: Value, space_ids: &[Uuid]) -> Value {
    let space_id_strings: Vec<String> = space_ids.iter().map(|id| id.to_string()).collect();

    json!({
        "query": {
            "bool": {
                "must": [base_text_query],
                "filter": [
                    { "terms": { "space_id": space_id_strings } }
                ],
                "should": [
                    {
                        "rank_feature": {
                            "field": "entity_space_score",
                            "boost": 1.3
                        }
                    }
                ]
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_uuid_query() {
        let query = build_uuid_query("550e8400-e29b-41d4-a716-446655440000");

        assert!(query["query"]["bool"]["should"].is_array());
        let should = query["query"]["bool"]["should"].as_array().unwrap();
        assert_eq!(should.len(), 2);
    }

    #[test]
    fn test_build_base_text_query() {
        let query = build_base_text_query("blockchain");

        // Should have 4 clauses in the should array
        let should = query["bool"]["should"].as_array().unwrap();
        assert_eq!(should.len(), 4);

        // First clause should be bool_prefix multi_match
        assert_eq!(should[0]["multi_match"]["type"], "bool_prefix");

        // Second clause should be fuzzy multi_match
        assert_eq!(should[1]["multi_match"]["fuzziness"], "AUTO");

        // Third and fourth should be match_phrase_prefix
        assert!(should[2]["match_phrase_prefix"]["name"].is_object());
        assert!(should[3]["match_phrase_prefix"]["description"].is_object());
    }

    #[test]
    fn test_build_global_query() {
        let base = build_base_text_query("test");
        let query = build_global_query(base);

        // Should have must and should at the top level
        assert!(query["query"]["bool"]["must"].is_array());
        assert!(query["query"]["bool"]["should"].is_array());

        // Should boost by entity_global_score
        let should = query["query"]["bool"]["should"].as_array().unwrap();
        assert_eq!(should[0]["rank_feature"]["field"], "entity_global_score");
    }

    #[test]
    fn test_build_global_by_space_score_query() {
        let base = build_base_text_query("test");
        let query = build_global_by_space_score_query(base);

        // Should boost by space_score
        let should = query["query"]["bool"]["should"].as_array().unwrap();
        assert_eq!(should[0]["rank_feature"]["field"], "space_score");
    }

    #[test]
    fn test_build_single_space_query() {
        let base = build_base_text_query("test");
        let space_id = Uuid::new_v4();
        let query = build_single_space_query(base, space_id);

        // Should have filter for space_id using term (singular)
        assert!(query["query"]["bool"]["filter"].is_array());
        let filter = query["query"]["bool"]["filter"].as_array().unwrap();
        assert!(filter[0]["term"]["space_id"].is_string());

        // Should boost by entity_space_score
        let should = query["query"]["bool"]["should"].as_array().unwrap();
        assert_eq!(should[0]["rank_feature"]["field"], "entity_space_score");
    }

    #[test]
    fn test_build_multi_space_query() {
        let base = build_base_text_query("test");
        let space_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let query = build_multi_space_query(base, &space_ids);

        // Should have terms filter (plural) for multiple space IDs
        let filter = query["query"]["bool"]["filter"].as_array().unwrap();
        assert!(filter[0]["terms"]["space_id"].is_array());

        let terms_array = filter[0]["terms"]["space_id"].as_array().unwrap();
        assert_eq!(terms_array.len(), 2);

        // Should boost by entity_space_score
        let should = query["query"]["bool"]["should"].as_array().unwrap();
        assert_eq!(should[0]["rank_feature"]["field"], "entity_space_score");
    }

    #[test]
    fn test_build_search_query_global() {
        let query = SearchQuery::global("test");
        let result = build_search_query(&query);

        // Should have rank_feature for entity_global_score
        let should = result["query"]["bool"]["should"].as_array().unwrap();
        assert_eq!(should[0]["rank_feature"]["field"], "entity_global_score");
    }

    #[test]
    fn test_build_search_query_single_space() {
        let space_id = Uuid::new_v4();
        let query = SearchQuery::in_space("test", space_id);
        let result = build_search_query(&query);

        // Should have a term filter for space_id
        assert!(result["query"]["bool"]["filter"].is_array());
        let filter = result["query"]["bool"]["filter"].as_array().unwrap();
        assert!(filter[0]["term"]["space_id"].is_string());

        // Should boost by entity_space_score
        let should = result["query"]["bool"]["should"].as_array().unwrap();
        assert_eq!(should[0]["rank_feature"]["field"], "entity_space_score");
    }

    #[test]
    fn test_build_search_query_multi_space() {
        let space1 = Uuid::new_v4();
        let space2 = Uuid::new_v4();
        let query = SearchQuery::in_spaces("test", vec![space1, space2]);
        let result = build_search_query(&query);

        // Should have a terms filter for multiple space_ids
        assert!(result["query"]["bool"]["filter"].is_array());
        let filter = result["query"]["bool"]["filter"].as_array().unwrap();
        assert!(filter[0]["terms"]["space_id"].is_array());

        // Should boost by entity_space_score
        let should = result["query"]["bool"]["should"].as_array().unwrap();
        assert_eq!(should[0]["rank_feature"]["field"], "entity_space_score");
    }

    #[test]
    fn test_build_search_query_uuid() {
        let query = SearchQuery::global("550e8400-e29b-41d4-a716-446655440000");
        let result = build_search_query(&query);

        // Should be a UUID query with term matches
        assert!(result["query"]["bool"]["should"].is_array());
        let should = result["query"]["bool"]["should"].as_array().unwrap();
        assert!(should[0]["term"]["entity_id"].is_string());
    }
}
