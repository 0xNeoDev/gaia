//! Kafka consumer implementation for the search indexer.
//!
//! Consumes entity events from Kafka topics and forwards them to the pipeline.

use prost::Message;
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::Message as KafkaMessage,
    TopicPartitionList,
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::consumer::messages::{EntityEvent, EntityEventType, StreamMessage};
use crate::errors::PipelineError;

use hermes_schema::pb::knowledge::HermesEdit;
use indexer_utils::id::transform_id_bytes;
use wire::pb::grc20::op::Payload;

/// The Kafka topic for knowledge edits.
const KNOWLEDGE_EDITS_TOPIC: &str = "knowledge.edits";

/// Well-known property IDs for name and description.
/// These are the standard GRC-20 property IDs.
const NAME_PROPERTY_ID: &str = "A7NJa8pVBZPLEv4ufZ2rCr"; // Name property
const DESCRIPTION_PROPERTY_ID: &str = "LA1DjwzfW2omgW7k6xQTo3"; // Description property

/// Kafka consumer for entity events.
pub struct KafkaConsumer {
    consumer: StreamConsumer,
    topics: Vec<String>,
}

impl KafkaConsumer {
    /// Create a new Kafka consumer.
    ///
    /// # Arguments
    ///
    /// * `brokers` - Kafka broker addresses (comma-separated)
    /// * `group_id` - Consumer group ID
    ///
    /// # Returns
    ///
    /// * `Ok(KafkaConsumer)` - A new consumer instance
    /// * `Err(PipelineError)` - If consumer creation fails
    pub fn new(brokers: &str, group_id: &str) -> Result<Self, PipelineError> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("session.timeout.ms", "6000")
            .create()
            .map_err(|e| PipelineError::kafka(e.to_string()))?;

        info!(brokers = %brokers, group_id = %group_id, "Created Kafka consumer");

        Ok(Self {
            consumer,
            topics: vec![KNOWLEDGE_EDITS_TOPIC.to_string()],
        })
    }

    /// Subscribe to configured topics.
    pub fn subscribe(&self) -> Result<(), PipelineError> {
        let topics: Vec<&str> = self.topics.iter().map(|s| s.as_str()).collect();
        self.consumer
            .subscribe(&topics)
            .map_err(|e| PipelineError::kafka(e.to_string()))?;

        info!(topics = ?self.topics, "Subscribed to Kafka topics");
        Ok(())
    }

    /// Start consuming messages and send them through the channel.
    ///
    /// # Arguments
    ///
    /// * `sender` - Channel to send messages to
    /// * `shutdown` - Shutdown signal receiver
    #[instrument(skip(self, sender, shutdown))]
    pub async fn run(
        &self,
        sender: mpsc::Sender<StreamMessage>,
        mut shutdown: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<(), PipelineError> {
        use futures::StreamExt;

        let mut message_stream = self.consumer.stream();

        loop {
            tokio::select! {
                _ = shutdown.recv() => {
                    info!("Consumer received shutdown signal");
                    let _ = sender.send(StreamMessage::End).await;
                    break;
                }
                message = message_stream.next() => {
                    match message {
                        Some(Ok(msg)) => {
                            if let Err(e) = self.process_message(&msg, &sender).await {
                                error!(error = %e, "Failed to process message");
                            }
                        }
                        Some(Err(e)) => {
                            error!(error = %e, "Kafka error");
                            let _ = sender.send(StreamMessage::Error(e.to_string())).await;
                        }
                        None => {
                            info!("Kafka stream ended");
                            let _ = sender.send(StreamMessage::End).await;
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Process a single Kafka message.
    async fn process_message(
        &self,
        msg: &rdkafka::message::BorrowedMessage<'_>,
        sender: &mpsc::Sender<StreamMessage>,
    ) -> Result<(), PipelineError> {
        let payload = match msg.payload() {
            Some(p) => p,
            None => {
                debug!("Received message with empty payload");
                return Ok(());
            }
        };

        let topic = msg.topic();
        let partition = msg.partition();
        let offset = msg.offset();

        debug!(
            topic = %topic,
            partition = partition,
            offset = offset,
            "Processing message"
        );

        // Parse the message based on topic
        let events = if topic == KNOWLEDGE_EDITS_TOPIC {
            self.parse_edit_message(payload, offset)?
        } else {
            warn!(topic = %topic, "Unknown topic");
            return Ok(());
        };

        if !events.is_empty() {
            sender
                .send(StreamMessage::Events(events))
                .await
                .map_err(|e| PipelineError::ChannelError(e.to_string()))?;
        }

        // Commit offset
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(topic, partition, rdkafka::Offset::Offset(offset + 1))
            .map_err(|e| PipelineError::kafka(e.to_string()))?;

        self.consumer
            .commit(&tpl, rdkafka::consumer::CommitMode::Async)
            .map_err(|e| PipelineError::kafka(e.to_string()))?;

        Ok(())
    }

    /// Parse a HermesEdit message into entity events.
    fn parse_edit_message(
        &self,
        payload: &[u8],
        offset: i64,
    ) -> Result<Vec<EntityEvent>, PipelineError> {
        let edit = HermesEdit::decode(payload)
            .map_err(|e| PipelineError::parse(format!("Failed to decode HermesEdit: {}", e)))?;

        let space_id_str = &edit.space_id;
        let space_id = Uuid::parse_str(space_id_str)
            .map_err(|e| PipelineError::parse(format!("Invalid space_id: {}", e)))?;

        let block_number = edit
            .meta
            .as_ref()
            .map(|m| m.block_number)
            .unwrap_or(0);

        let cursor = edit
            .meta
            .as_ref()
            .map(|m| m.cursor.clone())
            .unwrap_or_else(|| format!("offset_{}", offset));

        let mut events = Vec::new();

        // Process each operation in the edit
        for op in &edit.ops {
            if let Some(payload) = &op.payload {
                match payload {
                    Payload::UpdateEntity(entity) => {
                        if let Some(event) =
                            self.process_update_entity(entity, space_id, block_number, &cursor)
                        {
                            events.push(event);
                        }
                    }
                    Payload::DeleteRelation(relation_id) => {
                        // Handle relation deletions if needed
                        if let Ok(id_bytes) = transform_id_bytes(relation_id.clone()) {
                            let entity_id = Uuid::from_bytes(id_bytes);
                            events.push(EntityEvent::delete(
                                entity_id,
                                space_id,
                                block_number,
                                cursor.clone(),
                            ));
                        }
                    }
                    _ => {
                        // Other operation types don't affect search index
                    }
                }
            }
        }

        Ok(events)
    }

    /// Process an UpdateEntity operation.
    fn process_update_entity(
        &self,
        entity: &wire::pb::grc20::Entity,
        space_id: Uuid,
        block_number: u64,
        cursor: &str,
    ) -> Option<EntityEvent> {
        let entity_id_bytes = transform_id_bytes(entity.id.clone()).ok()?;
        let entity_id = Uuid::from_bytes(entity_id_bytes);

        // Extract name and description from values
        let mut name: Option<String> = None;
        let mut description: Option<String> = None;

        for value in &entity.values {
            let property_id_bytes = match transform_id_bytes(value.property.clone()) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };

            // Convert property ID bytes to base58 or check against known IDs
            let property_id = bs58::encode(&property_id_bytes).into_string();

            if property_id == NAME_PROPERTY_ID {
                name = Some(value.value.clone());
            } else if property_id == DESCRIPTION_PROPERTY_ID {
                description = Some(value.value.clone());
            }
        }

        // Only create an event if we have at least a name
        let name = name?;

        Some(EntityEvent::upsert(
            entity_id,
            space_id,
            name,
            description,
            block_number,
            cursor.to_string(),
        ))
    }
}

