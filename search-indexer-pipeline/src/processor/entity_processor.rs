//! Entity processor implementation.
//!
//! Transforms entity events into EntityDocument structures for indexing.

use tracing::{debug, instrument};

use crate::consumer::{EntityEvent, EntityEventType};
use crate::errors::PipelineError;
use search_indexer_shared::EntityDocument;

/// Processed result from the entity processor.
#[derive(Debug)]
pub enum ProcessedEvent {
    /// Document to be indexed (create or update).
    Index(EntityDocument),
    /// Document to be deleted.
    Delete {
        entity_id: uuid::Uuid,
        space_id: uuid::Uuid,
    },
}

/// Processor that transforms entity events into search documents.
///
/// The processor is responsible for:
/// - Converting entity events to EntityDocument structures
/// - Filtering out events that shouldn't be indexed
/// - Enriching documents with additional metadata
pub struct EntityProcessor {
    // Could hold configuration or caches in the future
}

impl EntityProcessor {
    /// Create a new entity processor.
    pub fn new() -> Self {
        Self {}
    }

    /// Process a batch of entity events.
    ///
    /// # Arguments
    ///
    /// * `events` - The events to process
    ///
    /// # Returns
    ///
    /// A vector of processed events ready for loading.
    #[instrument(skip(self, events), fields(event_count = events.len()))]
    pub fn process_batch(&self, events: Vec<EntityEvent>) -> Result<Vec<ProcessedEvent>, PipelineError> {
        let mut processed = Vec::with_capacity(events.len());

        for event in events {
            if let Some(result) = self.process_event(event)? {
                processed.push(result);
            }
        }

        debug!(processed_count = processed.len(), "Processed event batch");
        Ok(processed)
    }

    /// Process a single entity event.
    fn process_event(&self, event: EntityEvent) -> Result<Option<ProcessedEvent>, PipelineError> {
        match event.event_type {
            EntityEventType::Upsert => {
                // Need at least a name to index
                let name = match event.name {
                    Some(n) if !n.trim().is_empty() => n,
                    _ => {
                        debug!(
                            entity_id = %event.entity_id,
                            "Skipping entity with no name"
                        );
                        return Ok(None);
                    }
                };

                let mut doc = EntityDocument::new(
                    event.entity_id,
                    event.space_id,
                    name,
                    event.description,
                );

                // Set optional fields
                doc.avatar = event.avatar;
                doc.cover = event.cover;

                Ok(Some(ProcessedEvent::Index(doc)))
            }
            EntityEventType::Delete => {
                Ok(Some(ProcessedEvent::Delete {
                    entity_id: event.entity_id,
                    space_id: event.space_id,
                }))
            }
        }
    }
}

impl Default for EntityProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_process_upsert_event() {
        let processor = EntityProcessor::new();

        let event = EntityEvent::upsert(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Test Entity".to_string(),
            Some("Description".to_string()),
            12345,
            "cursor_123".to_string(),
        );

        let result = processor.process_event(event).unwrap();
        assert!(matches!(result, Some(ProcessedEvent::Index(_))));

        if let Some(ProcessedEvent::Index(doc)) = result {
            assert_eq!(doc.name, "Test Entity");
            assert_eq!(doc.description, Some("Description".to_string()));
        }
    }

    #[test]
    fn test_process_delete_event() {
        let processor = EntityProcessor::new();
        let entity_id = Uuid::new_v4();
        let space_id = Uuid::new_v4();

        let event = EntityEvent::delete(entity_id, space_id, 12345, "cursor_123".to_string());

        let result = processor.process_event(event).unwrap();
        assert!(matches!(result, Some(ProcessedEvent::Delete { .. })));

        if let Some(ProcessedEvent::Delete { entity_id: eid, space_id: sid }) = result {
            assert_eq!(eid, entity_id);
            assert_eq!(sid, space_id);
        }
    }

    #[test]
    fn test_skip_entity_without_name() {
        let processor = EntityProcessor::new();

        let mut event = EntityEvent::upsert(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "".to_string(), // Empty name
            None,
            12345,
            "cursor_123".to_string(),
        );
        event.name = Some("".to_string());

        let result = processor.process_event(event).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_process_batch() {
        let processor = EntityProcessor::new();

        let events = vec![
            EntityEvent::upsert(
                Uuid::new_v4(),
                Uuid::new_v4(),
                "Entity 1".to_string(),
                None,
                1,
                "c1".to_string(),
            ),
            EntityEvent::upsert(
                Uuid::new_v4(),
                Uuid::new_v4(),
                "Entity 2".to_string(),
                Some("Desc".to_string()),
                2,
                "c2".to_string(),
            ),
            EntityEvent::delete(Uuid::new_v4(), Uuid::new_v4(), 3, "c3".to_string()),
        ];

        let results = processor.process_batch(events).unwrap();
        assert_eq!(results.len(), 3);
    }
}

