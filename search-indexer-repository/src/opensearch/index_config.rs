//! OpenSearch index configuration and mappings.
//!
//! This module defines the index settings and mappings for the entity search index.

use serde_json::{json, Value};

/// The name of the search index.
pub const INDEX_NAME: &str = "entities";

/// Get the index settings and mappings for the entity search index.
///
/// The configuration includes:
/// - **search_as_you_type**: Built-in field type for autocomplete on name and description
/// - **rank_feature**: Score fields optimized for relevance boosting
/// - **Keyword fields**: For filtering and exact ID lookups
///
/// # Sharding Configuration
///
/// - 3 primary shards for horizontal scaling
/// - 1 replica for redundancy
pub fn get_index_settings() -> Value {
    json!({
        "settings": {
            "number_of_shards": 1,
            "number_of_replicas": 1
        },
        "mappings": {
            "properties": {
                "entity_id": {
                    "type": "keyword"
                },
                "space_id": {
                    "type": "keyword"
                },
                "name": {
                    "type": "search_as_you_type",
                    "fields": {
                        "raw": {
                            "type": "keyword"
                        }
                    }
                },
                "description": {
                    "type": "search_as_you_type"
                },
                "avatar": {
                    "type": "keyword",
                    "index": false
                },
                "cover": {
                    "type": "keyword",
                    "index": false
                },
                "entity_global_score": {
                    "type": "rank_feature"
                },
                "space_score": {
                    "type": "rank_feature"
                },
                "entity_space_score": {
                    "type": "rank_feature"
                },
                "indexed_at": {
                    "type": "date"
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_settings_structure() {
        let settings = get_index_settings();

        // Check settings exist
        assert!(settings["settings"]["number_of_shards"].is_number());
        assert!(settings["settings"]["number_of_replicas"].is_number());

        // Check mappings exist
        assert!(settings["mappings"]["properties"]["entity_id"].is_object());
        assert!(settings["mappings"]["properties"]["name"].is_object());
        assert!(settings["mappings"]["properties"]["description"].is_object());

        // Check search_as_you_type fields
        assert_eq!(
            settings["mappings"]["properties"]["name"]["type"],
            "search_as_you_type"
        );
        assert_eq!(
            settings["mappings"]["properties"]["description"]["type"],
            "search_as_you_type"
        );

        // Check rank_feature fields
        assert_eq!(
            settings["mappings"]["properties"]["entity_global_score"]["type"],
            "rank_feature"
        );
        assert_eq!(
            settings["mappings"]["properties"]["space_score"]["type"],
            "rank_feature"
        );
        assert_eq!(
            settings["mappings"]["properties"]["entity_space_score"]["type"],
            "rank_feature"
        );
    }

    #[test]
    fn test_index_name() {
        assert_eq!(INDEX_NAME, "entities");
    }
}
