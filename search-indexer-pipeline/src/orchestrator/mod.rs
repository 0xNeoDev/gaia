//! Orchestrator module for the search indexer pipeline.
//!
//! Coordinates the consumer, processor, and loader components.

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, instrument, warn};

use crate::consumer::{KafkaConsumer, StreamMessage};
use crate::errors::PipelineError;
use crate::loader::SearchLoader;
use crate::processor::EntityProcessor;

/// Configuration for the orchestrator.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Size of the message channel buffer.
    pub channel_buffer_size: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            channel_buffer_size: 1000,
        }
    }
}

/// Orchestrator that coordinates the pipeline components.
///
/// The orchestrator:
/// - Manages the lifecycle of pipeline components
/// - Routes messages between components
/// - Handles shutdown signals
/// - Monitors pipeline health
pub struct Orchestrator {
    consumer: Arc<KafkaConsumer>,
    processor: EntityProcessor,
    loader: SearchLoader,
    config: OrchestratorConfig,
    shutdown_tx: broadcast::Sender<()>,
}

impl Orchestrator {
    /// Create a new orchestrator with the given components.
    pub fn new(
        consumer: KafkaConsumer,
        processor: EntityProcessor,
        loader: SearchLoader,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            consumer: Arc::new(consumer),
            processor,
            loader,
            config: OrchestratorConfig::default(),
            shutdown_tx,
        }
    }

    /// Create a new orchestrator with custom configuration.
    pub fn with_config(
        consumer: KafkaConsumer,
        processor: EntityProcessor,
        loader: SearchLoader,
        config: OrchestratorConfig,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            consumer: Arc::new(consumer),
            processor,
            loader,
            config,
            shutdown_tx,
        }
    }

    /// Run the orchestrator.
    ///
    /// This method starts all pipeline components and coordinates message flow.
    /// It blocks until a shutdown signal is received or an error occurs.
    #[instrument(skip(self))]
    pub async fn run(&mut self) -> Result<(), PipelineError> {
        info!("Starting search indexer orchestrator");

        // Ensure the search index exists
        self.loader.ensure_index().await?;

        // Subscribe to Kafka topics
        self.consumer.subscribe()?;

        // Create message channel
        let (tx, mut rx) = mpsc::channel::<StreamMessage>(self.config.channel_buffer_size);

        // Start consumer in background
        let consumer = self.consumer.clone();
        let shutdown_rx = self.shutdown_tx.subscribe();

        let consumer_handle = tokio::spawn(async move {
            if let Err(e) = consumer.run(tx, shutdown_rx).await {
                error!(error = %e, "Consumer error");
            }
        });

        // Process messages
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Some(StreamMessage::Events(events)) => {
                            if let Err(e) = self.process_events(events).await {
                                error!(error = %e, "Failed to process events");
                            }
                        }
                        Some(StreamMessage::Error(e)) => {
                            error!(error = %e, "Received error from consumer");
                        }
                        Some(StreamMessage::End) | None => {
                            info!("Consumer stream ended");
                            break;
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal");
                    let _ = self.shutdown_tx.send(());
                    break;
                }
            }
        }

        // Flush any remaining documents
        if let Err(e) = self.loader.flush().await {
            warn!(error = %e, "Failed to flush remaining documents");
        }

        // Wait for consumer to finish
        let _ = consumer_handle.await;

        info!("Orchestrator shutdown complete");
        Ok(())
    }

    /// Process a batch of events through the pipeline.
    async fn process_events(
        &mut self,
        events: Vec<crate::consumer::EntityEvent>,
    ) -> Result<(), PipelineError> {
        // Transform events to documents
        let processed = self.processor.process_batch(events)?;

        if processed.is_empty() {
            return Ok(());
        }

        // Load into search index
        self.loader.load(processed).await?;

        Ok(())
    }

    /// Trigger a graceful shutdown.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

