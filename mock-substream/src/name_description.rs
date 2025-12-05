//! Name and description entity operation generation.
//!
//! This module provides functionality to create entity operations that set
//! name and description values using the SDK's well-known attribute IDs.

use crate::events::{EntityId, Op, PropertyId, UpdateEntity, Value};
use sdk::core::ids::{DESCRIPTION_ATTRIBUTE, NAME_ATTRIBUTE};
use uuid::Uuid;

#[cfg(feature = "random")]
use rand::Rng;

use crate::events::make_id;

/// Get the name and description property IDs from base58-encoded attribute IDs.
/// Returns (name_property_id, description_property_id) as 16-byte arrays.
/// Falls back to well-known test IDs if decoding fails.
pub fn get_name_description_property_ids() -> (PropertyId, PropertyId) {
    // Decode base58 strings to UUID bytes
    let name_property_id =
        decode_base58_to_property_id(NAME_ATTRIBUTE).unwrap_or_else(|| make_id(0xD1)); // Fallback to PROPERTY_NAME from test_topology

    let description_property_id =
        decode_base58_to_property_id(DESCRIPTION_ATTRIBUTE).unwrap_or_else(|| make_id(0xD2)); // Fallback to PROPERTY_DESCRIPTION from test_topology

    (name_property_id, description_property_id)
}

/// Create an UpdateEntity operation with name and description values.
/// The entity ID is randomly generated, and description is included with 70% probability.
#[cfg(feature = "random")]
pub fn create_name_description_entity_op<R: Rng>(rng: &mut R) -> Op {
    // Generate random entity ID
    let mut entity_id_bytes = [0u8; 16];
    rng.fill(&mut entity_id_bytes);
    let entity_id = entity_id_bytes;

    let (name_property_id, description_property_id) = get_name_description_property_ids();

    let mut values = Vec::new();

    // Always add name value
    values.push(Value {
        property: name_property_id,
        value: format!("Entity {}", rng.gen::<u32>()),
    });

    // Add description value (70% chance)
    if rng.gen_bool(0.7) {
        values.push(Value {
            property: description_property_id,
            value: format!("Description for entity {}", rng.gen::<u32>()),
        });
    }

    Op::UpdateEntity(UpdateEntity {
        id: entity_id,
        values,
    })
}

/// Create an UpdateEntity operation with name and description values (deterministic version).
/// Uses provided entity_id and counter for deterministic values.
pub fn create_name_description_entity_op_deterministic(
    entity_id: EntityId,
    counter: u32,
    include_description: bool,
) -> Op {
    let (name_property_id, description_property_id) = get_name_description_property_ids();

    let mut values = Vec::new();

    // Always add name value
    values.push(Value {
        property: name_property_id,
        value: format!("Entity {}", counter),
    });

    // Add description value if requested
    if include_description {
        values.push(Value {
            property: description_property_id,
            value: format!("Description for entity {}", counter),
        });
    }

    Op::UpdateEntity(UpdateEntity {
        id: entity_id,
        values,
    })
}

/// Decode a base58-encoded string to a PropertyId (16-byte UUID).
/// Returns None if decoding fails or the decoded bytes are not 16 bytes.
///
/// The base58 strings in the SDK constants are base58-encoded UUID bytes.
/// Decoding should yield exactly 16 bytes which can be used directly as a PropertyId.
fn decode_base58_to_property_id(base58_str: &str) -> Option<PropertyId> {
    // Decode base58 to bytes
    let decoded_bytes = bs58::decode(base58_str).into_vec().ok()?;

    // The decoded bytes should be exactly 16 bytes (UUID size)
    if decoded_bytes.len() == 16 {
        // Direct UUID bytes - convert to array
        decoded_bytes.try_into().ok()
    } else {
        // If not exactly 16 bytes, try to parse as UUID string representation
        // (some encodings might store UUID as string)
        String::from_utf8(decoded_bytes)
            .ok()
            .and_then(|s| Uuid::parse_str(&s).ok())
            .map(|uuid| *uuid.as_bytes())
    }
}
