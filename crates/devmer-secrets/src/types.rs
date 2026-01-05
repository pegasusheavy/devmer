//! Secret types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An encrypted value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedValue {
    /// Encryption provider used
    pub provider: String,

    /// Encrypted ciphertext (base64 encoded)
    pub ciphertext: String,

    /// Nonce/IV (base64 encoded)
    pub nonce: Option<String>,

    /// Salt for key derivation (base64 encoded)
    pub salt: Option<String>,

    /// Key ID (for KMS providers)
    pub key_id: Option<String>,

    /// Additional authenticated data
    #[serde(default)]
    pub aad: Option<String>,

    /// Version of the encryption scheme
    pub version: u32,
}

impl EncryptedValue {
    /// Create a new encrypted value
    pub fn new(
        provider: impl Into<String>,
        ciphertext: impl Into<String>,
        version: u32,
    ) -> Self {
        Self {
            provider: provider.into(),
            ciphertext: ciphertext.into(),
            nonce: None,
            salt: None,
            key_id: None,
            aad: None,
            version,
        }
    }

    /// Set the nonce
    pub fn with_nonce(mut self, nonce: impl Into<String>) -> Self {
        self.nonce = Some(nonce.into());
        self
    }

    /// Set the salt
    pub fn with_salt(mut self, salt: impl Into<String>) -> Self {
        self.salt = Some(salt.into());
        self
    }

    /// Set the key ID
    pub fn with_key_id(mut self, key_id: impl Into<String>) -> Self {
        self.key_id = Some(key_id.into());
        self
    }

    /// Set additional authenticated data
    pub fn with_aad(mut self, aad: impl Into<String>) -> Self {
        self.aad = Some(aad.into());
        self
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to base64 prefixed format for storage in config
    pub fn to_prefixed_string(&self) -> Result<String, serde_json::Error> {
        let json = self.to_json()?;
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &json);
        Ok(format!("enc:v{}:{}", self.version, encoded))
    }

    /// Parse from prefixed string format
    pub fn from_prefixed_string(s: &str) -> Option<Self> {
        let rest = s.strip_prefix("enc:")?;
        let (version_part, encoded) = rest.split_once(':')?;
        let _version: u32 = version_part.strip_prefix('v')?.parse().ok()?;

        let json = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded).ok()?;
        let json_str = String::from_utf8(json).ok()?;
        Self::from_json(&json_str).ok()
    }
}

/// Context for encryption operations
#[derive(Debug, Clone, Default)]
pub struct EncryptionContext {
    /// Stack name
    pub stack: Option<String>,

    /// Resource URN
    pub resource_urn: Option<String>,

    /// Property path
    pub property: Option<String>,

    /// Additional context key-value pairs
    pub additional: HashMap<String, String>,
}

impl EncryptionContext {
    /// Create a new encryption context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the stack
    pub fn with_stack(mut self, stack: impl Into<String>) -> Self {
        self.stack = Some(stack.into());
        self
    }

    /// Set the resource URN
    pub fn with_resource(mut self, urn: impl Into<String>) -> Self {
        self.resource_urn = Some(urn.into());
        self
    }

    /// Set the property path
    pub fn with_property(mut self, property: impl Into<String>) -> Self {
        self.property = Some(property.into());
        self
    }

    /// Add additional context
    pub fn with_additional(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional.insert(key.into(), value.into());
        self
    }

    /// Convert to additional authenticated data string
    pub fn to_aad(&self) -> String {
        let mut parts = vec![];

        if let Some(ref stack) = self.stack {
            parts.push(format!("stack:{}", stack));
        }
        if let Some(ref urn) = self.resource_urn {
            parts.push(format!("resource:{}", urn));
        }
        if let Some(ref prop) = self.property {
            parts.push(format!("property:{}", prop));
        }

        for (k, v) in &self.additional {
            parts.push(format!("{}:{}", k, v));
        }

        parts.join(";")
    }
}

/// Metadata about a secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMetadata {
    /// Secret name/path
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// When the secret was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// When the secret was last modified
    pub modified_at: chrono::DateTime<chrono::Utc>,

    /// Version number
    pub version: u32,

    /// Tags
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypted_value_serialization() {
        let value = EncryptedValue::new("passphrase", "encrypted_data", 1)
            .with_nonce("random_nonce")
            .with_salt("random_salt");

        let json = value.to_json().unwrap();
        let parsed = EncryptedValue::from_json(&json).unwrap();

        assert_eq!(parsed.provider, "passphrase");
        assert_eq!(parsed.ciphertext, "encrypted_data");
    }

    #[test]
    fn test_encryption_context() {
        let ctx = EncryptionContext::new()
            .with_stack("dev")
            .with_resource("urn:devmer:dev::aws:s3:Bucket::my-bucket")
            .with_property("secret_key");

        let aad = ctx.to_aad();
        assert!(aad.contains("stack:dev"));
        assert!(aad.contains("resource:"));
        assert!(aad.contains("property:secret_key"));
    }
}
