use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::info;

use crate::clients::{APITestClient, OpenSearchTestClient};
use crate::generators::{generate_documents, generate_query, EntityDocument};
use crate::metrics::MetricsCollector;

pub struct IndexLoader {
    client: Arc<OpenSearchTestClient>,
    metrics: Arc<MetricsCollector>,
    batch_size: usize,
    workers: usize,
    duration_seconds: u64,
    running: Arc<std::sync::atomic::AtomicBool>,
    total_indexed: Arc<std::sync::atomic::AtomicUsize>,
    start_time: Arc<std::sync::Mutex<Option<Instant>>>,
}

impl IndexLoader {
    pub fn new(
        client: Arc<OpenSearchTestClient>,
        metrics: Arc<MetricsCollector>,
        batch_size: usize,
        workers: usize,
        duration_seconds: u64,
    ) -> Self {
        Self {
            client,
            metrics,
            batch_size,
            workers,
            duration_seconds,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            total_indexed: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            start_time: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub async fn start(&self) -> Result<(), anyhow::Error> {
        info!(
            "Starting indexing load test with {} workers, batch size {}",
            self.workers, self.batch_size
        );

        self.running
            .store(true, std::sync::atomic::Ordering::Relaxed);
        *self.start_time.lock().unwrap() = Some(Instant::now());

        let mut handles = Vec::new();

        for _i in 0..self.workers {
            let client = Arc::clone(&self.client);
            let metrics = Arc::clone(&self.metrics);
            let batch_size = self.batch_size;
            let running = Arc::clone(&self.running);
            let total_indexed = Arc::clone(&self.total_indexed);
            let start_time = Arc::clone(&self.start_time);
            let duration_seconds = self.duration_seconds;

            let handle = tokio::spawn(async move {
                let end_time =
                    start_time.lock().unwrap().unwrap() + Duration::from_secs(duration_seconds);

                while running.load(std::sync::atomic::Ordering::Relaxed)
                    && Instant::now() < end_time
                {
                    let documents = generate_documents(batch_size, None);
                    let result = client.bulk_index(&documents).await;

                    let indexed_count = if result.success { documents.len() } else { 0 };
                    total_indexed.fetch_add(indexed_count, std::sync::atomic::Ordering::Relaxed);

                    metrics.record_indexing(
                        result.latency_ms,
                        result.success,
                        result.error.as_deref(),
                    );

                    if !result.success {
                        sleep(Duration::from_millis(100)).await;
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await?;
        }

        Ok(())
    }

    pub fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (usize, f64) {
        let start = self.start_time.lock().unwrap();
        if let Some(start_time) = *start {
            let elapsed = (Instant::now() - start_time).as_secs_f64();
            let total = self
                .total_indexed
                .load(std::sync::atomic::Ordering::Relaxed);
            (
                total,
                if elapsed > 0.0 {
                    total as f64 / elapsed
                } else {
                    0.0
                },
            )
        } else {
            (0, 0.0)
        }
    }
}

pub struct QueryLoader {
    opensearch_client: Option<Arc<OpenSearchTestClient>>,
    api_client: Option<Arc<APITestClient>>,
    metrics: Arc<MetricsCollector>,
    workers: usize,
    duration_seconds: u64,
    documents: Vec<EntityDocument>,
    running: Arc<std::sync::atomic::AtomicBool>,
    total_queries: Arc<std::sync::atomic::AtomicUsize>,
    start_time: Arc<std::sync::Mutex<Option<Instant>>>,
}

impl QueryLoader {
    pub fn new(
        opensearch_client: Option<Arc<OpenSearchTestClient>>,
        api_client: Option<Arc<APITestClient>>,
        metrics: Arc<MetricsCollector>,
        workers: usize,
        duration_seconds: u64,
        documents: Vec<EntityDocument>,
    ) -> Self {
        if opensearch_client.is_none() && api_client.is_none() {
            panic!("Either opensearch_client or api_client must be provided");
        }

        // Extract unique space IDs (stored for potential future use)
        let _space_ids: Vec<String> = documents
            .iter()
            .map(|d| d.space_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Self {
            opensearch_client,
            api_client,
            metrics,
            workers,
            duration_seconds,
            documents,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            total_queries: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            start_time: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub async fn start(&self) -> Result<(), anyhow::Error> {
        info!("Starting querying load test with {} workers", self.workers);

        self.running
            .store(true, std::sync::atomic::Ordering::Relaxed);
        *self.start_time.lock().unwrap() = Some(Instant::now());

        let space_ids: Vec<String> = self
            .documents
            .iter()
            .map(|d| d.space_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let mut handles = Vec::new();

        for _ in 0..self.workers {
            let opensearch_client = self.opensearch_client.as_ref().map(Arc::clone);
            let api_client = self.api_client.as_ref().map(Arc::clone);
            let metrics = Arc::clone(&self.metrics);
            let documents = self.documents.clone();
            let space_ids = space_ids.clone();
            let running = Arc::clone(&self.running);
            let total_queries = Arc::clone(&self.total_queries);
            let start_time = Arc::clone(&self.start_time);
            let duration_seconds = self.duration_seconds;

            let handle = tokio::spawn(async move {
                let end_time =
                    start_time.lock().unwrap().unwrap() + Duration::from_secs(duration_seconds);

                while running.load(std::sync::atomic::Ordering::Relaxed)
                    && Instant::now() < end_time
                {
                    let query = generate_query(&documents, &space_ids);

                    let result = if let Some(ref api_client) = api_client {
                        api_client
                            .search(
                                &query.query,
                                &query.scope,
                                query.space_id.as_deref(),
                                query.limit,
                            )
                            .await
                    } else if let Some(ref opensearch_client) = opensearch_client {
                        opensearch_client
                            .search(&query.query, &query.scope, query.limit)
                            .await
                    } else {
                        panic!("No client available");
                    };

                    total_queries.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    metrics.record_querying(
                        result.latency_ms,
                        result.success,
                        result.error.as_deref(),
                    );

                    if !result.success {
                        sleep(Duration::from_millis(100)).await;
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await?;
        }

        Ok(())
    }

    pub fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (usize, f64) {
        let start = self.start_time.lock().unwrap();
        if let Some(start_time) = *start {
            let elapsed = (Instant::now() - start_time).as_secs_f64();
            let total = self
                .total_queries
                .load(std::sync::atomic::Ordering::Relaxed);
            (
                total,
                if elapsed > 0.0 {
                    total as f64 / elapsed
                } else {
                    0.0
                },
            )
        } else {
            (0, 0.0)
        }
    }
}
