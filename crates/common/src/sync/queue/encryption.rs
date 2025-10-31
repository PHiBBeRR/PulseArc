use std::fmt;

pub use crate::crypto::encryption::{
    key_storage as shared_storage, EncryptedData, EncryptionService as SharedEncryptionService,
};
use crate::sync::queue::errors::QueueResult;

/// Queue-specific wrapper around the shared encryption service.
pub struct EncryptionService {
    inner: SharedEncryptionService,
}

impl fmt::Debug for EncryptionService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[allow(dead_code)]
impl EncryptionService {
    /// Create new encryption service with a raw 32-byte key.
    pub fn new(key: Vec<u8>) -> QueueResult<Self> {
        Ok(Self { inner: SharedEncryptionService::new(key)? })
    }

    /// Create encryption service from user-supplied password.
    pub fn from_password(password: &str) -> QueueResult<Self> {
        Ok(Self { inner: SharedEncryptionService::from_password(password)? })
    }

    /// Generate a random encryption key suitable for AES-256-GCM.
    pub fn generate_key() -> Vec<u8> {
        SharedEncryptionService::generate_key()
    }

    /// Encrypt arbitrary bytes into an [`EncryptedData`] payload.
    pub fn encrypt(&self, data: &[u8]) -> QueueResult<EncryptedData> {
        Ok(self.inner.encrypt(data)?)
    }

    /// Decrypt an [`EncryptedData`] payload back into bytes.
    pub fn decrypt(&self, encrypted: &EncryptedData) -> QueueResult<Vec<u8>> {
        Ok(self.inner.decrypt(encrypted)?)
    }

    /// Encrypt bytes and encode the payload as a base64 string.
    pub fn encrypt_to_string(&self, data: &[u8]) -> QueueResult<String> {
        Ok(self.inner.encrypt_to_string(data)?)
    }

    /// Decode a base64 string and decrypt the contained payload.
    pub fn decrypt_from_string(&self, encrypted_str: &str) -> QueueResult<Vec<u8>> {
        Ok(self.inner.decrypt_from_string(encrypted_str)?)
    }

    /// Get a short fingerprint for verification/telemetry purposes.
    pub fn key_fingerprint(&self) -> String {
        self.inner.key_fingerprint()
    }

    /// Replace the current encryption key with a new one.
    pub fn rotate_key(&mut self, new_key: Vec<u8>) -> QueueResult<()> {
        self.inner.rotate_key(new_key)?;
        Ok(())
    }

    /// Re-encrypt data with a new service instance.
    pub fn reencrypt(
        &self,
        encrypted: &EncryptedData,
        new_service: &EncryptionService,
    ) -> QueueResult<EncryptedData> {
        Ok(self.inner.reencrypt(encrypted, &new_service.inner)?)
    }
}

/// Secure key storage utilities for queue persistence.
pub mod key_storage {
    use std::path::Path;

    use super::*;

    #[allow(dead_code)]
    pub fn save_key(key: &[u8], path: &Path, master_password: &str) -> QueueResult<()> {
        shared_storage::save_key(key, path, master_password)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn load_key(path: &Path, master_password: &str) -> QueueResult<Vec<u8>> {
        Ok(shared_storage::load_key(path, master_password)?)
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for sync::queue::encryption.
    use super::*;

    /// Validates `EncryptionService::generate_key` behavior for the generate
    /// key has correct length scenario.
    ///
    /// Assertions:
    /// - Confirms `key.len()` equals `32`.
    #[test]
    fn generate_key_has_correct_length() {
        let key = EncryptionService::generate_key();
        assert_eq!(key.len(), 32);
    }

    /// Validates `EncryptionService::new` behavior for the new service rejects
    /// invalid key size scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn new_service_rejects_invalid_key_size() {
        let result = EncryptionService::new(vec![0; 16]);
        assert!(result.is_err());
    }

    /// Validates `EncryptionService::generate_key` behavior for the encrypt and
    /// decrypt round trip scenario.
    ///
    /// Assertions:
    /// - Confirms `decrypted` equals `plaintext`.
    #[test]
    fn encrypt_and_decrypt_round_trip() {
        let key = EncryptionService::generate_key();
        let service = EncryptionService::new(key).unwrap();

        let plaintext = b"hello queue";
        let encrypted = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    /// Validates `EncryptionService::generate_key` behavior for the encrypt to
    /// and from string round trip scenario.
    ///
    /// Assertions:
    /// - Confirms `decoded` equals `plaintext`.
    #[test]
    fn encrypt_to_and_from_string_round_trip() {
        let key = EncryptionService::generate_key();
        let service = EncryptionService::new(key).unwrap();

        let plaintext = b"queue payload";
        let encoded = service.encrypt_to_string(plaintext).unwrap();
        let decoded = service.decrypt_from_string(&encoded).unwrap();

        assert_eq!(decoded, plaintext);
    }

    /// Validates `EncryptionService::generate_key` behavior for the reencrypt
    /// uses new service scenario.
    ///
    /// Assertions:
    /// - Confirms `decrypted` equals `plaintext`.
    #[test]
    fn reencrypt_uses_new_service() {
        let key1 = EncryptionService::generate_key();
        let key2 = EncryptionService::generate_key();
        let service1 = EncryptionService::new(key1).unwrap();
        let service2 = EncryptionService::new(key2).unwrap();

        let plaintext = b"rotate me";
        let encrypted = service1.encrypt(plaintext).unwrap();
        let reencrypted = service1.reencrypt(&encrypted, &service2).unwrap();

        let decrypted = service2.decrypt(&reencrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
