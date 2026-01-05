//! Secure memory handling

use secrecy::{ExposeSecret, SecretBox, SecretString};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A secure byte buffer that is zeroized on drop
#[derive(Clone, ZeroizeOnDrop)]
pub struct SecureBuffer {
    data: Vec<u8>,
}

impl SecureBuffer {
    /// Create a new secure buffer
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Create from a string
    pub fn from_string(s: &str) -> Self {
        Self::new(s.as_bytes().to_vec())
    }

    /// Get the data (use carefully)
    pub fn expose(&self) -> &[u8] {
        &self.data
    }

    /// Get the length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl From<Vec<u8>> for SecureBuffer {
    fn from(data: Vec<u8>) -> Self {
        Self::new(data)
    }
}

impl From<&[u8]> for SecureBuffer {
    fn from(data: &[u8]) -> Self {
        Self::new(data.to_vec())
    }
}

/// A wrapper for sensitive string data
pub struct SensitiveString(SecretString);

impl SensitiveString {
    /// Create a new sensitive string
    pub fn new(s: impl Into<String>) -> Self {
        Self(SecretString::from(s.into()))
    }

    /// Expose the inner string (use carefully)
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl From<String> for SensitiveString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SensitiveString {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// A wrapper for sensitive byte data
pub struct SensitiveBytes(SecretBox<[u8]>);

impl SensitiveBytes {
    /// Create new sensitive bytes
    pub fn new(data: Vec<u8>) -> Self {
        Self(SecretBox::new(data.into_boxed_slice()))
    }

    /// Expose the inner bytes (use carefully)
    pub fn expose(&self) -> &[u8] {
        self.0.expose_secret()
    }

    /// Get the length
    pub fn len(&self) -> usize {
        self.0.expose_secret().len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.0.expose_secret().is_empty()
    }
}

impl From<Vec<u8>> for SensitiveBytes {
    fn from(data: Vec<u8>) -> Self {
        Self::new(data)
    }
}

/// Securely compare two byte slices in constant time
pub fn secure_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Securely zero a mutable slice
pub fn secure_zero(data: &mut [u8]) {
    data.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_buffer() {
        let buffer = SecureBuffer::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(buffer.expose(), &[1, 2, 3, 4, 5]);
        assert_eq!(buffer.len(), 5);
    }

    #[test]
    fn test_sensitive_string() {
        let s = SensitiveString::new("secret");
        assert_eq!(s.expose(), "secret");
    }

    #[test]
    fn test_secure_compare() {
        assert!(secure_compare(b"hello", b"hello"));
        assert!(!secure_compare(b"hello", b"world"));
        assert!(!secure_compare(b"hello", b"hell"));
    }

    #[test]
    fn test_secure_zero() {
        let mut data = vec![1, 2, 3, 4, 5];
        secure_zero(&mut data);
        assert_eq!(data, vec![0, 0, 0, 0, 0]);
    }
}
