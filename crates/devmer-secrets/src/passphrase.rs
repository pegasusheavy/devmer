//! Passphrase-based encryption provider

use crate::error::{Result, SecretsError};
use crate::provider::SecretsProvider;
use crate::types::{EncryptedValue, EncryptionContext};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use async_trait::async_trait;
use base64::Engine;
use rand::RngCore;
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroize;

/// Current version of the passphrase encryption scheme
const SCHEME_VERSION: u32 = 1;

/// Salt length in bytes
const SALT_LEN: usize = 32;

/// Nonce length in bytes (96 bits for AES-GCM)
const NONCE_LEN: usize = 12;

/// Derived key length
const KEY_LEN: usize = 32;

/// Passphrase-based secrets provider
pub struct PassphraseProvider {
    passphrase: SecretString,
}

impl PassphraseProvider {
    /// Create a new passphrase provider
    pub fn new(passphrase: impl Into<String>) -> Self {
        Self {
            passphrase: SecretString::from(passphrase.into()),
        }
    }

    /// Derive an encryption key from the passphrase and salt
    fn derive_key(&self, salt: &[u8]) -> Result<[u8; KEY_LEN]> {
        let params = Params::new(65536, 3, 4, Some(KEY_LEN))
            .map_err(|e| SecretsError::KeyDerivationFailed(e.to_string()))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let mut key = [0u8; KEY_LEN];
        argon2
            .hash_password_into(
                self.passphrase.expose_secret().as_bytes(),
                salt,
                &mut key,
            )
            .map_err(|e| SecretsError::KeyDerivationFailed(e.to_string()))?;

        Ok(key)
    }

    /// Generate a random salt
    fn generate_salt() -> [u8; SALT_LEN] {
        let mut salt = [0u8; SALT_LEN];
        rand::thread_rng().fill_bytes(&mut salt);
        salt
    }

    /// Generate a random nonce
    fn generate_nonce() -> [u8; NONCE_LEN] {
        let mut nonce = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce);
        nonce
    }
}

#[async_trait]
impl SecretsProvider for PassphraseProvider {
    fn name(&self) -> &str {
        "passphrase"
    }

    async fn encrypt(
        &self,
        plaintext: &[u8],
        context: &EncryptionContext,
    ) -> Result<EncryptedValue> {
        // Generate salt and nonce
        let salt = Self::generate_salt();
        let nonce_bytes = Self::generate_nonce();

        // Derive key
        let mut key = self.derive_key(&salt)?;

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| SecretsError::encryption_failed(e.to_string()))?;

        // Use context as additional authenticated data
        let aad = context.to_aad();

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, aes_gcm::aead::Payload {
                msg: plaintext,
                aad: aad.as_bytes(),
            })
            .map_err(|e| SecretsError::encryption_failed(e.to_string()))?;

        // Zeroize key
        key.zeroize();

        // Encode to base64
        let b64 = base64::engine::general_purpose::STANDARD;

        Ok(EncryptedValue::new("passphrase", b64.encode(&ciphertext), SCHEME_VERSION)
            .with_nonce(b64.encode(nonce_bytes))
            .with_salt(b64.encode(salt))
            .with_aad(aad))
    }

    async fn decrypt(
        &self,
        encrypted: &EncryptedValue,
        context: &EncryptionContext,
    ) -> Result<Vec<u8>> {
        let b64 = base64::engine::general_purpose::STANDARD;

        // Decode components
        let ciphertext = b64
            .decode(&encrypted.ciphertext)
            .map_err(|e| SecretsError::InvalidCiphertext(e.to_string()))?;

        let salt = encrypted
            .salt
            .as_ref()
            .ok_or_else(|| SecretsError::InvalidCiphertext("Missing salt".into()))?;
        let salt = b64
            .decode(salt)
            .map_err(|e| SecretsError::InvalidCiphertext(e.to_string()))?;

        let nonce_str = encrypted
            .nonce
            .as_ref()
            .ok_or_else(|| SecretsError::InvalidCiphertext("Missing nonce".into()))?;
        let nonce_bytes = b64
            .decode(nonce_str)
            .map_err(|e| SecretsError::InvalidCiphertext(e.to_string()))?;

        // Derive key
        let mut key = self.derive_key(&salt)?;

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| SecretsError::decryption_failed(e.to_string()))?;

        // Use stored AAD or generate from context
        let aad = encrypted
            .aad
            .clone()
            .unwrap_or_else(|| context.to_aad());

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, aes_gcm::aead::Payload {
                msg: &ciphertext,
                aad: aad.as_bytes(),
            })
            .map_err(|e| SecretsError::decryption_failed(e.to_string()))?;

        // Zeroize key
        key.zeroize();

        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encrypt_decrypt() {
        let provider = PassphraseProvider::new("test-passphrase-123");
        let context = EncryptionContext::new().with_stack("test");

        let plaintext = b"Hello, World!";

        let encrypted = provider.encrypt(plaintext, &context).await.unwrap();
        assert_eq!(encrypted.provider, "passphrase");
        assert!(encrypted.salt.is_some());
        assert!(encrypted.nonce.is_some());

        let decrypted = provider.decrypt(&encrypted, &context).await.unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn test_wrong_passphrase() {
        let provider1 = PassphraseProvider::new("correct-passphrase");
        let provider2 = PassphraseProvider::new("wrong-passphrase");
        let context = EncryptionContext::new();

        let encrypted = provider1.encrypt(b"secret data", &context).await.unwrap();
        let result = provider2.decrypt(&encrypted, &context).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_aad_is_stored() {
        let provider = PassphraseProvider::new("test-passphrase");
        let context1 = EncryptionContext::new().with_stack("stack1");
        let context2 = EncryptionContext::new().with_stack("stack2");

        let encrypted = provider.encrypt(b"secret data", &context1).await.unwrap();

        // AAD is stored with ciphertext, so decryption uses stored AAD
        // Different context parameter doesn't affect decryption
        let result = provider.decrypt(&encrypted, &context2).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"secret data");
        
        // Verify the AAD was stored
        assert!(encrypted.aad.is_some());
        assert!(encrypted.aad.as_ref().unwrap().contains("stack1"));
    }
}
