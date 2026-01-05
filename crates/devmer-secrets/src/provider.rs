//! Secrets provider trait

use crate::types::{EncryptedValue, EncryptionContext};
use crate::Result;
use async_trait::async_trait;

/// Trait for secrets encryption providers
#[async_trait]
pub trait SecretsProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Encrypt plaintext
    async fn encrypt(
        &self,
        plaintext: &[u8],
        context: &EncryptionContext,
    ) -> Result<EncryptedValue>;

    /// Decrypt ciphertext
    async fn decrypt(
        &self,
        ciphertext: &EncryptedValue,
        context: &EncryptionContext,
    ) -> Result<Vec<u8>>;

    /// Rotate a secret (re-encrypt with new key/parameters)
    async fn rotate(
        &self,
        ciphertext: &EncryptedValue,
        context: &EncryptionContext,
    ) -> Result<EncryptedValue> {
        // Default implementation: decrypt and re-encrypt
        let plaintext = self.decrypt(ciphertext, context).await?;
        self.encrypt(&plaintext, context).await
    }

    /// Check if this provider can decrypt the given value
    fn can_decrypt(&self, ciphertext: &EncryptedValue) -> bool {
        ciphertext.provider == self.name()
    }
}

/// A provider that combines multiple providers
pub struct MultiProvider {
    providers: Vec<Box<dyn SecretsProvider>>,
    default_provider: String,
}

impl MultiProvider {
    /// Create a new multi-provider
    pub fn new(default_provider: impl Into<String>) -> Self {
        Self {
            providers: vec![],
            default_provider: default_provider.into(),
        }
    }

    /// Add a provider
    pub fn add_provider(mut self, provider: Box<dyn SecretsProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    /// Get a provider by name
    pub fn get_provider(&self, name: &str) -> Option<&dyn SecretsProvider> {
        self.providers
            .iter()
            .find(|p| p.name() == name)
            .map(|p| p.as_ref())
    }

    /// Get the default provider
    pub fn default_provider(&self) -> Option<&dyn SecretsProvider> {
        self.get_provider(&self.default_provider)
    }
}

#[async_trait]
impl SecretsProvider for MultiProvider {
    fn name(&self) -> &str {
        "multi"
    }

    async fn encrypt(
        &self,
        plaintext: &[u8],
        context: &EncryptionContext,
    ) -> Result<EncryptedValue> {
        let provider = self.default_provider().ok_or_else(|| {
            crate::SecretsError::ProviderNotConfigured(self.default_provider.clone())
        })?;
        provider.encrypt(plaintext, context).await
    }

    async fn decrypt(
        &self,
        ciphertext: &EncryptedValue,
        context: &EncryptionContext,
    ) -> Result<Vec<u8>> {
        let provider = self.get_provider(&ciphertext.provider).ok_or_else(|| {
            crate::SecretsError::ProviderNotConfigured(ciphertext.provider.clone())
        })?;
        provider.decrypt(ciphertext, context).await
    }

    fn can_decrypt(&self, ciphertext: &EncryptedValue) -> bool {
        self.providers.iter().any(|p| p.can_decrypt(ciphertext))
    }
}
