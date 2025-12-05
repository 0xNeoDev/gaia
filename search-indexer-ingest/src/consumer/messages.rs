//! Message types for the consumer.
//!
//! Defines the event structures that flow through the ingest.

use uuid::Uuid;

/// Types of entity events that can be received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityEventType {
    /// Entity was created or updated.
    Upsert,
    /// Entity was deleted.
    Delete,
}

/// An entity event received from Kafka.
#[derive(Debug, Clone)]
pub struct EntityEvent {
    /// The type of event.
    pub event_type: EntityEventType,
    /// The entity's unique identifier.
    pub entity_id: Uuid,
    /// The space this entity belongs to.
    pub space_id: Uuid,
    /// The entity's name (for upsert events).
    pub name: Option<String>,
    /// The entity's description (for upsert events).
    pub description: Option<String>,
    /// Avatar URL (for upsert events).
    pub avatar: Option<String>,
    /// Cover image URL (for upsert events).
    pub cover: Option<String>,
    /// Block number where the event occurred.
    pub block_number: u64,
    /// Cursor for this event (for persistence).
    pub cursor: String,
}

impl EntityEvent {
    /// Create a new upsert event.
    pub fn upsert(
        entity_id: Uuid,
        space_id: Uuid,
        name: Option<String>,
        description: Option<String>,
        block_number: u64,
        cursor: String,
    ) -> Self {
        Self {
            event_type: EntityEventType::Upsert,
            entity_id,
            space_id,
            name,
            description,
            avatar: None,
            cover: None,
            block_number,
            cursor,
        }
    }

    /// Create a new delete event.
    pub fn delete(entity_id: Uuid, space_id: Uuid, block_number: u64, cursor: String) -> Self {
        Self {
            event_type: EntityEventType::Delete,
            entity_id,
            space_id,
            name: None,
            description: None,
            avatar: None,
            cover: None,
            block_number,
            cursor,
        }
    }
}

/// Messages that flow through the ingest.
#[derive(Debug)]
pub enum StreamMessage {
    /// A batch of entity events with associated offsets for acknowledgment.
    Events {
        events: Vec<EntityEvent>,
        offsets: Vec<(String, i32, i64)>,
    },
    /// Acknowledgment that events were successfully processed.
    Acknowledgment {
        offsets: Vec<(String, i32, i64)>,
        success: bool,
        error: Option<String>,
    },
    /// Stream has ended.
    End,
    /// An error occurred.
    Error(String),
}
