use anyhow::{Context, Result};
use opensearch::{
    http::request::JsonBody,
    http::transport::{SingleNodeConnectionPool, TransportBuilder},
    BulkParts, OpenSearch, SearchParts,
};
use serde_json::{json, Value};
use std::time::Instant;
use tracing::{error, info, warn};
use url::Url;

use crate::generators::EntityDocument;

#[derive(Debug, Clone)]
pub struct IndexStatistics {
    pub document_count: u64,
    pub average_doc_size_kb: f64,
    pub total_storage_gb: f64,
    pub primary_shards: u64,
    pub replica_shards: u64,
}

pub struct OpenSearchTestClient {
    client: OpenSearch,
    index_name: String,
}

#[derive(Debug)]
pub struct IndexResult {
    pub success: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct SearchResult {
    pub success: bool,
    pub latency_ms: u64,
    pub result_count: usize,
    pub error: Option<String>,
}

impl OpenSearchTestClient {
    pub async fn new(opensearch_url: &str, index_name: &str) -> Result<Self> {
        info!("Connecting to OpenSearch at: {}", opensearch_url);
        
        let url = Url::parse(opensearch_url)
            .with_context(|| format!("Invalid OpenSearch URL: {}", opensearch_url))?;
        
        let conn_pool = SingleNodeConnectionPool::new(url.clone());
        let transport = TransportBuilder::new(conn_pool)
            .disable_proxy()
            .build()
            .with_context(|| "Failed to create OpenSearch transport")?;
        
        let client = OpenSearch::new(transport);
        
        info!("OpenSearch client created successfully");
        
        Ok(Self {
            client,
            index_name: index_name.to_string(),
        })
    }

    pub async fn bulk_index(&self, documents: &[EntityDocument]) -> IndexResult {
        let start = Instant::now();

        let mut body: Vec<JsonBody<Value>> = Vec::with_capacity(documents.len() * 2);

        for doc in documents {
            let doc_id = format!("{}_{}", doc.entity_id, doc.space_id);
            body.push(json!({"index": {"_index": self.index_name, "_id": doc_id}}).into());
            body.push(
                serde_json::to_value(doc)
                    .unwrap_or_else(|_| json!({}))
                    .into(),
            );
        }

        match self
            .client
            .bulk(BulkParts::Index(&self.index_name))
            .body(body)
            .send()
            .await
        {
            Ok(response) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                let response_body: Value = response.json().await.unwrap_or(json!({}));

                if let Some(errors) = response_body.get("errors").and_then(|e| e.as_bool()) {
                    if errors {
                        let empty_vec = Vec::<Value>::new();
                        let items_array = response_body
                            .get("items")
                            .and_then(|i| i.as_array())
                            .unwrap_or(&empty_vec);
                        let error_items: Vec<&Value> = items_array
                            .iter()
                            .filter(|item| {
                                item.get("index")
                                    .and_then(|i| i.get("error"))
                                    .is_some()
                            })
                            .collect();

                        if !error_items.is_empty() {
                            return IndexResult {
                                success: false,
                                latency_ms,
                                error: Some(format!("Bulk index errors: {} failed", error_items.len())),
                            };
                        }
                    }
                }

                IndexResult {
                    success: true,
                    latency_ms,
                    error: None,
                }
            }
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                error!("Bulk index error: {}", e);
                IndexResult {
                    success: false,
                    latency_ms,
                    error: Some(format!("OpenSearch error: {}", e)),
                }
            }
        }
    }

    pub async fn search(
        &self,
        query: &str,
        _scope: &str,
        limit: usize,
    ) -> SearchResult {
        let start = Instant::now();

        let query_body = json!({
            "query": {
                "multi_match": {
                    "query": query,
                    "fields": ["name^2", "description"],
                    "type": "best_fields",
                    "fuzziness": "AUTO"
                }
            },
            "size": limit
        });

        match self
            .client
            .search(SearchParts::Index(&[&self.index_name]))
            .body(query_body)
            .send()
            .await
        {
            Ok(response) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                let response_body: Value = response.json().await.unwrap_or(json!({}));

                let result_count = response_body
                    .get("hits")
                    .and_then(|h| h.get("hits"))
                    .and_then(|h| h.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);

                SearchResult {
                    success: true,
                    latency_ms,
                    result_count,
                    error: None,
                }
            }
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                error!("Search error: {}", e);
                SearchResult {
                    success: false,
                    latency_ms,
                    result_count: 0,
                    error: Some(format!("OpenSearch error: {}", e)),
                }
            }
        }
    }

    pub async fn health_check(&self) -> Result<bool> {
        info!("Checking OpenSearch health...");
        
        match self
            .client
            .cluster()
            .health(opensearch::cluster::ClusterHealthParts::None)
            .send()
            .await
        {
            Ok(response) => {
                let health: Value = response.json().await.unwrap_or(json!({}));
                let status = health.get("status").and_then(|s| s.as_str()).unwrap_or("unknown");
                
                info!("OpenSearch cluster status: {}", status);
                
                Ok(status == "green" || status == "yellow")
            }
            Err(e) => {
                error!("OpenSearch health check failed: {}", e);
                Err(anyhow::anyhow!("Health check failed: {}", e))
            }
        }
    }

    pub async fn get_index_statistics(&self) -> Result<IndexStatistics> {
        info!("Fetching index statistics for: {}", self.index_name);
        
        // Get index stats (docs, storage)
        let stats_result = self
            .client
            .indices()
            .stats(opensearch::indices::IndicesStatsParts::Index(&[&self.index_name]))
            .send()
            .await;
        
        // Get index settings (shards)
        let settings_result = self
            .client
            .indices()
            .get(opensearch::indices::IndicesGetParts::Index(&[&self.index_name]))
            .send()
            .await;
        
        let (document_count, average_doc_size_kb, total_storage_gb) = match stats_result {
            Ok(response) => {
                let stats: Value = response.json().await.unwrap_or(json!({}));
                
                // Navigate to indices.{index_name}.total
                let index_stats = stats
                    .get("indices")
                    .and_then(|indices| indices.get(&self.index_name))
                    .and_then(|idx| idx.get("total"));
                
                let doc_count = index_stats
                    .and_then(|t| t.get("docs"))
                    .and_then(|d| d.get("count"))
                    .and_then(|c| c.as_u64())
                    .unwrap_or(0);
                
                // Store size in bytes (primary + replica, but we'll use primary)
                let store_size_bytes = index_stats
                    .and_then(|t| t.get("store"))
                    .and_then(|s| s.get("size_in_bytes"))
                    .and_then(|s| s.as_u64())
                    .unwrap_or(0);
                
                // Calculate average document size
                let avg_doc_size_kb = if doc_count > 0 {
                    (store_size_bytes as f64 / doc_count as f64) / 1024.0
                } else {
                    0.0
                };
                
                // Calculate total storage in GB
                let total_storage = store_size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                
                (doc_count, avg_doc_size_kb, total_storage)
            }
            Err(e) => {
                warn!("Failed to get index stats: {}", e);
                (0, 0.0, 0.0)
            }
        };
        
        let (primary_shards, replica_shards) = match settings_result {
            Ok(response) => {
                let settings: Value = response.json().await.unwrap_or(json!({}));
                
                // Navigate to {index_name}.settings.index.number_of_shards and number_of_replicas
                let index_settings = settings
                    .get(&self.index_name)
                    .and_then(|idx| idx.get("settings"))
                    .and_then(|s| s.get("index"));
                
                let primary = index_settings
                    .and_then(|idx| idx.get("number_of_shards"))
                    .and_then(|s| s.as_str())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                
                let replicas = index_settings
                    .and_then(|idx| idx.get("number_of_replicas"))
                    .and_then(|s| s.as_str())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                
                (primary, replicas)
            }
            Err(e) => {
                warn!("Failed to get index settings: {}", e);
                (0, 0)
            }
        };
        
        info!(
            "Index statistics: {} docs, {:.2} kB/doc avg, {:.3} GB total, {} primary shards, {} replica shards",
            document_count, average_doc_size_kb, total_storage_gb, primary_shards, replica_shards
        );
        
        Ok(IndexStatistics {
            document_count,
            average_doc_size_kb,
            total_storage_gb,
            primary_shards,
            replica_shards,
        })
    }
}

pub struct APITestClient {
    base_url: String,
    client: reqwest::Client,
}

impl APITestClient {
    pub fn new(api_url: &str) -> Self {
        info!("Creating API client for: {}", api_url);
        Self {
            base_url: api_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn search(
        &self,
        query: &str,
        scope: &str,
        space_id: Option<&str>,
        limit: usize,
    ) -> SearchResult {
        let start = Instant::now();

        let limit_str = limit.to_string();
        let mut params = vec![
            ("query", query),
            ("scope", scope),
            ("limit", &limit_str),
        ];

        let mut url = format!("{}/search", self.base_url);
        if let Some(space_id) = space_id {
            params.push(("space_id", space_id));
        }

        let query_string: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        url.push_str("?");
        url.push_str(&query_string);

        match self.client.get(&url).send().await {
            Ok(response) => {
                let latency_ms = start.elapsed().as_millis() as u64;

                if !response.status().is_success() {
                    return SearchResult {
                        success: false,
                        latency_ms,
                        result_count: 0,
                        error: Some(format!(
                            "HTTP {}: {}",
                            response.status(),
                            response.status().canonical_reason().unwrap_or("Unknown")
                        )),
                    };
                }

                match response.json::<Value>().await {
                    Ok(data) => {
                        let result_count = data
                            .get("results")
                            .and_then(|r| r.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);

                        SearchResult {
                            success: true,
                            latency_ms,
                            result_count,
                            error: None,
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse API response: {}", e);
                        SearchResult {
                            success: false,
                            latency_ms,
                            result_count: 0,
                            error: Some(format!("Parse error: {}", e)),
                        }
                    }
                }
            }
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                error!("API request failed: {}", e);
                SearchResult {
                    success: false,
                    latency_ms,
                    result_count: 0,
                    error: Some(format!("Request error: {}", e)),
                }
            }
        }
    }

    pub async fn health_check(&self) -> Result<bool> {
        info!("Checking API health at: {}/health", self.base_url);
        
        match self.client.get(&format!("{}/health", self.base_url)).send().await {
            Ok(response) => {
                let healthy = response.status().is_success();
                if healthy {
                    info!("API health check passed");
                } else {
                    warn!("API health check failed with status: {}", response.status());
                }
                Ok(healthy)
            }
            Err(e) => {
                error!("API health check failed: {}", e);
                Err(anyhow::anyhow!("Health check failed: {}", e))
            }
        }
    }
}

