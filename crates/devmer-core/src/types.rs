//! Core value types for Devmer

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A property value that can be stored in resource inputs/outputs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PropertyValue {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value (64-bit signed)
    Int(i64),
    /// Floating point value (64-bit)
    Float(f64),
    /// String value
    String(String),
    /// Array of property values
    Array(Vec<PropertyValue>),
    /// Object/map of property values
    Object(HashMap<String, PropertyValue>),
    /// Secret value (encrypted at rest)
    Secret(Box<PropertyValue>),
    /// Output reference to another resource's output
    OutputRef(OutputReference),
}

/// Reference to another resource's output
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputReference {
    /// URN of the resource
    pub urn: String,
    /// Name of the output property
    pub property: String,
}

/// A collection of property values
pub type PropertyValues = HashMap<String, PropertyValue>;

impl PropertyValue {
    /// Create a null value
    pub fn null() -> Self {
        Self::Null
    }

    /// Create a boolean value
    pub fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    /// Create an integer value
    pub fn int(value: i64) -> Self {
        Self::Int(value)
    }

    /// Create a float value
    pub fn float(value: f64) -> Self {
        Self::Float(value)
    }

    /// Create a string value
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    /// Create an array value
    pub fn array(values: Vec<PropertyValue>) -> Self {
        Self::Array(values)
    }

    /// Create an object value
    pub fn object(values: HashMap<String, PropertyValue>) -> Self {
        Self::Object(values)
    }

    /// Create a secret value
    pub fn secret(inner: PropertyValue) -> Self {
        Self::Secret(Box::new(inner))
    }

    /// Create an output reference
    pub fn output_ref(urn: impl Into<String>, property: impl Into<String>) -> Self {
        Self::OutputRef(OutputReference {
            urn: urn.into(),
            property: property.into(),
        })
    }

    /// Check if this value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Check if this value is a secret
    pub fn is_secret(&self) -> bool {
        matches!(self, Self::Secret(_))
    }

    /// Check if this value is an output reference
    pub fn is_output_ref(&self) -> bool {
        matches!(self, Self::OutputRef(_))
    }

    /// Try to get as a string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as an integer
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as a boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as an object
    pub fn as_object(&self) -> Option<&HashMap<String, PropertyValue>> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Try to get as an array
    pub fn as_array(&self) -> Option<&Vec<PropertyValue>> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Unwrap a secret value
    pub fn unwrap_secret(&self) -> Option<&PropertyValue> {
        match self {
            Self::Secret(inner) => Some(inner),
            _ => None,
        }
    }
}

impl From<bool> for PropertyValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for PropertyValue {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<i32> for PropertyValue {
    fn from(value: i32) -> Self {
        Self::Int(value as i64)
    }
}

impl From<f64> for PropertyValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<String> for PropertyValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for PropertyValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl<T: Into<PropertyValue>> From<Vec<T>> for PropertyValue {
    fn from(value: Vec<T>) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<PropertyValue>> From<HashMap<String, T>> for PropertyValue {
    fn from(value: HashMap<String, T>) -> Self {
        Self::Object(value.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_value_types() {
        assert!(PropertyValue::null().is_null());
        assert!(PropertyValue::secret(PropertyValue::string("password")).is_secret());
        assert!(PropertyValue::output_ref("urn:devmer:test", "id").is_output_ref());
    }

    #[test]
    fn test_property_value_conversions() {
        let s: PropertyValue = "hello".into();
        assert_eq!(s.as_str(), Some("hello"));

        let i: PropertyValue = 42i64.into();
        assert_eq!(i.as_int(), Some(42));

        let b: PropertyValue = true.into();
        assert_eq!(b.as_bool(), Some(true));
    }
}
