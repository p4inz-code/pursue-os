//! Structured log records.

use pursue_core::{Error, Result};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;

use super::Level;
use super::timestamp::rfc3339_from_unix;

/// The on-wire representation of a [`Record`], with fields as a JSON object.
#[derive(Serialize, Deserialize)]
struct RecordRepr {
    /// RFC 3339 UTC timestamp (human-readable).
    timestamp: String,
    /// Unix timestamp (seconds).
    timestamp_unix: u64,
    /// Severity level.
    level: Level,
    /// Component that emitted the record.
    component: String,
    /// Human-readable message.
    message: String,
    /// Structured key/value fields.
    fields: serde_json::Map<String, JsonValue>,
}

/// A single structured log event.
///
/// Serialized as a JSON object with `timestamp`, `timestamp_unix`, `level`,
/// `component`, `message`, and `fields` (a JSON object). Duplicate field keys
/// are resolved at serialization time: the last value wins.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    timestamp_unix: u64,
    level: Level,
    component: String,
    message: String,
    fields: Vec<(String, JsonValue)>,
}

impl Record {
    /// Creates a record; `component` must be non-empty after trimming.
    pub fn new(
        timestamp_unix: u64,
        level: Level,
        component: &str,
        message: &str,
        fields: Vec<(String, JsonValue)>,
    ) -> Result<Self> {
        let component = component.trim();
        if component.is_empty() {
            return Err(Error::InvalidInput(
                "log component must not be empty".into(),
            ));
        }
        Ok(Self {
            timestamp_unix,
            level,
            component: component.to_string(),
            message: message.to_string(),
            fields,
        })
    }

    /// Unix timestamp (seconds) when the event occurred.
    pub fn timestamp_unix(&self) -> u64 {
        self.timestamp_unix
    }

    /// The event timestamp formatted as an RFC 3339 UTC string.
    pub fn timestamp_rfc3339(&self) -> String {
        rfc3339_from_unix(self.timestamp_unix)
    }

    /// The severity level.
    pub fn level(&self) -> Level {
        self.level
    }

    /// The component that emitted this record.
    pub fn component(&self) -> &str {
        &self.component
    }

    /// The human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The structured fields, in insertion order.
    pub fn fields(&self) -> &[(String, JsonValue)] {
        &self.fields
    }

    /// Sets (or replaces) a field and returns the record for chaining.
    pub fn with_field(mut self, key: &str, value: JsonValue) -> Self {
        self.set_field(key, value);
        self
    }

    /// Sets (or replaces) a field in place.
    pub fn set_field(&mut self, key: &str, value: JsonValue) {
        self.fields.retain(|(k, _)| k != key);
        self.fields.push((key.to_string(), value));
    }
}

impl Serialize for Record {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut fields = serde_json::Map::new();
        for (key, value) in &self.fields {
            fields.insert(key.clone(), value.clone());
        }
        let repr = RecordRepr {
            timestamp: self.timestamp_rfc3339(),
            timestamp_unix: self.timestamp_unix,
            level: self.level,
            component: self.component.clone(),
            message: self.message.clone(),
            fields,
        };
        // Serialize through the struct representation for a stable key order.
        let mut state = serializer.serialize_struct("Record", 6)?;
        state.serialize_field("timestamp", &repr.timestamp)?;
        state.serialize_field("timestamp_unix", &repr.timestamp_unix)?;
        state.serialize_field("level", &repr.level)?;
        state.serialize_field("component", &repr.component)?;
        state.serialize_field("message", &repr.message)?;
        state.serialize_field("fields", &repr.fields)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Record {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let repr = RecordRepr::deserialize(deserializer)?;
        Record::new(
            repr.timestamp_unix,
            repr.level,
            &repr.component,
            &repr.message,
            repr.fields.into_iter().collect(),
        )
        .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::Record;
    use crate::log::Level;
    use serde_json::{Value, json};

    fn sample() -> Record {
        Record::new(
            1_700_000_000,
            Level::Info,
            "evidence",
            "stored evidence",
            vec![("count".to_string(), json!(7))],
        )
        .unwrap()
    }

    #[test]
    fn construction_and_accessors() {
        let record = sample();
        assert_eq!(record.timestamp_unix(), 1_700_000_000);
        assert_eq!(record.timestamp_rfc3339(), "2023-11-14T22:13:20Z");
        assert_eq!(record.level(), Level::Info);
        assert_eq!(record.component(), "evidence");
        assert_eq!(record.message(), "stored evidence");
        assert_eq!(record.fields(), &[("count".to_string(), json!(7))]);
    }

    #[test]
    fn empty_component_is_rejected() {
        assert!(Record::new(0, Level::Info, "", "m", Vec::new()).is_err());
        assert!(Record::new(0, Level::Info, "   ", "m", Vec::new()).is_err());
    }

    #[test]
    fn with_field_replaces_and_inserts() {
        let record = sample()
            .with_field("count", json!(99))
            .with_field("new", json!("x"));
        assert_eq!(record.fields().len(), 2);
        assert!(record.fields().contains(&("count".to_string(), json!(99))));
        assert!(record.fields().contains(&("new".to_string(), json!("x"))));
    }

    #[test]
    fn serializes_to_expected_json_shape() {
        let value: Value = serde_json::to_value(sample()).unwrap();
        assert_eq!(value["timestamp"], json!("2023-11-14T22:13:20Z"));
        assert_eq!(value["timestamp_unix"], json!(1_700_000_000));
        assert_eq!(value["level"], json!("info"));
        assert_eq!(value["component"], json!("evidence"));
        assert_eq!(value["message"], json!("stored evidence"));
        assert_eq!(value["fields"], json!({ "count": 7 }));
    }

    #[test]
    fn deserializes_from_wire_shape() {
        let wire = r#"{"timestamp":"2023-11-14T22:13:20Z","timestamp_unix":1700000000,"level":"debug","component":"ipc","message":"hello","fields":{"a":1}}"#;
        let record: Record = serde_json::from_str(wire).unwrap();
        assert_eq!(record.level(), Level::Debug);
        assert_eq!(record.component(), "ipc");
        assert_eq!(record.fields(), &[("a".to_string(), json!(1))]);
    }

    #[test]
    fn roundtrip_preserves_record() {
        let record = sample().with_field("ok", json!(true));
        let json = serde_json::to_string(&record).unwrap();
        let back: Record = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record);
    }
}
