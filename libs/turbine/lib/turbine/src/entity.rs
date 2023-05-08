use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
};
use core::fmt;

use hashbrown::HashMap;
use serde::{
    de::{value::StrDeserializer, Error},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::Value;
use time::OffsetDateTime;
use type_system::url::VersionedUrl;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityId {
    pub owned_by_id: Uuid,
    pub entity_uuid: Uuid,
}

impl fmt::Display for EntityId {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}%{}", self.owned_by_id, self.entity_uuid)
    }
}

impl<'de> Deserialize<'de> for EntityId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // We can be more efficient than this, we know the byte sizes of all the elements
        let as_string = String::deserialize(deserializer)?;
        let mut parts = as_string.split('%');

        Ok(Self {
            owned_by_id: Uuid::deserialize(StrDeserializer::new(parts.next().ok_or_else(
                || D::Error::custom("failed to find second component of `%` delimited string"),
            )?))?,
            entity_uuid: Uuid::deserialize(StrDeserializer::new(parts.next().ok_or_else(
                || D::Error::custom("failed to find second component of `%` delimited string"),
            )?))?,
        })
    }
}

impl Serialize for EntityId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProvenanceMetadata {
    pub record_created_by_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntityTemporalMetadata {
    // too lazy c:
    pub decision_time: Value,
    pub transaction_time: Value,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityRecordId {
    pub entity_id: EntityId,
    pub edition_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EntityLinkOrder {
    #[serde(default, rename = "leftToRightOrder")]
    pub left_to_right: Option<i32>,
    #[serde(default, rename = "rightToLeftOrder")]
    pub right_to_left: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LinkData {
    pub left_entity_id: EntityId,
    pub right_entity_id: EntityId,
    #[serde(flatten)]
    pub order: EntityLinkOrder,
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
pub struct EntityProperties(pub HashMap<String, Value>);

impl EntityProperties {
    #[must_use]
    pub const fn properties(&self) -> &HashMap<String, Value> {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EntityMetadata {
    pub record_id: EntityRecordId,
    temporal_versioning: EntityTemporalMetadata,
    pub entity_type_id: VersionedUrl,
    provenance: ProvenanceMetadata,
    archived: bool,
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Entity {
    pub properties: EntityProperties,
    #[serde(default)]
    pub link_data: Option<LinkData>,
    pub metadata: EntityMetadata,
}

// TODO: versions and such, todo: parsing
// TODO: filter from all entities (and their types) -> output of graph
// TODO: important is also the serialization!
pub struct EntityVertex<T>(BTreeMap<OffsetDateTime, T>);

impl<T> EntityVertex<T> {
    fn latest(&self) -> &T {
        self.0
            .last_key_value()
            .expect("should have at least one entry")
            .1
    }
}
