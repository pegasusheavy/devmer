//! Type conversion utilities between Rust types and protobuf types

use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;

/// Convert JSON value to protobuf-compatible format
pub fn json_to_proto(value: &JsonValue) -> ProtoValue {
    match value {
        JsonValue::Null => ProtoValue::Null,
        JsonValue::Bool(b) => ProtoValue::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                ProtoValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                ProtoValue::Float(f)
            } else {
                ProtoValue::String(n.to_string())
            }
        }
        JsonValue::String(s) => ProtoValue::String(s.clone()),
        JsonValue::Array(arr) => {
            ProtoValue::Array(arr.iter().map(json_to_proto).collect())
        }
        JsonValue::Object(obj) => {
            ProtoValue::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), json_to_proto(v)))
                    .collect(),
            )
        }
    }
}

/// Convert protobuf value to JSON
pub fn proto_to_json(value: &ProtoValue) -> JsonValue {
    match value {
        ProtoValue::Null => JsonValue::Null,
        ProtoValue::Bool(b) => JsonValue::Bool(*b),
        ProtoValue::Int(i) => json!(*i),
        ProtoValue::Float(f) => json!(*f),
        ProtoValue::String(s) => JsonValue::String(s.clone()),
        ProtoValue::Secret(s) => json!({ "__secret": s }),
        ProtoValue::Output(s) => json!({ "__output": s }),
        ProtoValue::ResourceRef(s) => json!({ "__resource": s }),
        ProtoValue::Array(arr) => {
            JsonValue::Array(arr.iter().map(proto_to_json).collect())
        }
        ProtoValue::Object(obj) => {
            JsonValue::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), proto_to_json(v)))
                    .collect(),
            )
        }
    }
}

/// Intermediate protobuf value representation
#[derive(Debug, Clone, PartialEq)]
pub enum ProtoValue {
    /// Null value
    Null,
    /// Boolean
    Bool(bool),
    /// Integer
    Int(i64),
    /// Float
    Float(f64),
    /// String
    String(String),
    /// Secret (encrypted)
    Secret(String),
    /// Output reference
    Output(String),
    /// Resource reference
    ResourceRef(String),
    /// Array
    Array(Vec<ProtoValue>),
    /// Object
    Object(HashMap<String, ProtoValue>),
}

impl ProtoValue {
    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, ProtoValue::Null)
    }

    /// Check if value is a secret
    pub fn is_secret(&self) -> bool {
        matches!(self, ProtoValue::Secret(_))
    }

    /// Check if value is an output
    pub fn is_output(&self) -> bool {
        matches!(self, ProtoValue::Output(_))
    }

    /// Get as string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ProtoValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ProtoValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as i64
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ProtoValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as f64
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ProtoValue::Float(f) => Some(*f),
            ProtoValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }
}

impl From<JsonValue> for ProtoValue {
    fn from(value: JsonValue) -> Self {
        json_to_proto(&value)
    }
}

impl From<ProtoValue> for JsonValue {
    fn from(value: ProtoValue) -> Self {
        proto_to_json(&value)
    }
}

impl From<&str> for ProtoValue {
    fn from(s: &str) -> Self {
        ProtoValue::String(s.to_string())
    }
}

impl From<String> for ProtoValue {
    fn from(s: String) -> Self {
        ProtoValue::String(s)
    }
}

impl From<bool> for ProtoValue {
    fn from(b: bool) -> Self {
        ProtoValue::Bool(b)
    }
}

impl From<i64> for ProtoValue {
    fn from(i: i64) -> Self {
        ProtoValue::Int(i)
    }
}

impl From<i32> for ProtoValue {
    fn from(i: i32) -> Self {
        ProtoValue::Int(i as i64)
    }
}

impl From<f64> for ProtoValue {
    fn from(f: f64) -> Self {
        ProtoValue::Float(f)
    }
}

/// Extract secrets from a JSON value
pub fn extract_secrets(value: &JsonValue) -> Vec<String> {
    let mut secrets = Vec::new();
    extract_secrets_recursive(value, &mut secrets);
    secrets
}

fn extract_secrets_recursive(value: &JsonValue, secrets: &mut Vec<String>) {
    match value {
        JsonValue::Object(obj) => {
            if let Some(JsonValue::String(s)) = obj.get("__secret") {
                secrets.push(s.clone());
            } else {
                for v in obj.values() {
                    extract_secrets_recursive(v, secrets);
                }
            }
        }
        JsonValue::Array(arr) => {
            for v in arr {
                extract_secrets_recursive(v, secrets);
            }
        }
        _ => {}
    }
}

/// Check if a JSON value contains any secrets
pub fn contains_secrets(value: &JsonValue) -> bool {
    match value {
        JsonValue::Object(obj) => {
            if obj.contains_key("__secret") {
                return true;
            }
            obj.values().any(contains_secrets)
        }
        JsonValue::Array(arr) => arr.iter().any(contains_secrets),
        _ => false,
    }
}

/// Redact secrets in a JSON value for logging
pub fn redact_secrets(value: &JsonValue) -> JsonValue {
    match value {
        JsonValue::Object(obj) => {
            if obj.contains_key("__secret") {
                json!("[secret]")
            } else {
                JsonValue::Object(
                    obj.iter()
                        .map(|(k, v)| (k.clone(), redact_secrets(v)))
                        .collect(),
                )
            }
        }
        JsonValue::Array(arr) => {
            JsonValue::Array(arr.iter().map(redact_secrets).collect())
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_proto_roundtrip() {
        let original = json!({
            "name": "test",
            "count": 42,
            "enabled": true,
            "tags": ["a", "b"],
            "config": {
                "nested": "value"
            }
        });

        let proto = json_to_proto(&original);
        let back = proto_to_json(&proto);

        assert_eq!(original, back);
    }

    #[test]
    fn test_secret_detection() {
        let value = json!({
            "password": { "__secret": "encrypted123" },
            "username": "admin"
        });

        assert!(contains_secrets(&value));

        let secrets = extract_secrets(&value);
        assert_eq!(secrets.len(), 1);
        assert_eq!(secrets[0], "encrypted123");
    }

    #[test]
    fn test_redact_secrets() {
        let value = json!({
            "password": { "__secret": "encrypted123" },
            "username": "admin"
        });

        let redacted = redact_secrets(&value);
        assert_eq!(redacted["password"], json!("[secret]"));
        assert_eq!(redacted["username"], "admin");
    }

    #[test]
    fn test_proto_value_conversions() {
        let s: ProtoValue = "hello".into();
        assert_eq!(s.as_string(), Some("hello"));

        let b: ProtoValue = true.into();
        assert_eq!(b.as_bool(), Some(true));

        let i: ProtoValue = 42i64.into();
        assert_eq!(i.as_int(), Some(42));
    }
}
