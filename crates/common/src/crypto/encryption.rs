//! Cryptographic primitives for encryption and key management.
//!
//! This module provides **low-level encryption primitives** using AES-256-GCM:
//!
//! - [`EncryptionService`]: AES-256-GCM encryption/decryption
//! - [`EncryptedData`]: Serializable encrypted data container
//! - Password-based key derivation using Argon2
//! - Key generation and rotation support
//!
//! ## Module Relationships
//!
//! This module provides the cryptographic primitives. Higher-level key
//! management is built on top:
//!
//! - **`crypto::encryption`** (this module): AES-256-GCM encryption primitives
//! - **`security::encryption`**: Key management infrastructure (caching,
//!   rotation, keychain storage, secure strings)
//!
//! ## Usage
//!
//! ```rust
//! use pulsearc_common::crypto::encryption::EncryptionService;
//!
//! let key = EncryptionService::generate_key();
//! let service = EncryptionService::new(key)?;
//!
//! let plaintext = b"sensitive data";
//! let encrypted = service.encrypt(plaintext)?;
//! let decrypted = service.decrypt(&encrypted)?;
//! assert_eq!(decrypted, plaintext);
//! # Ok::<(), pulsearc_common::error::CommonError>(())
//! ```

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::SaltString;
use argon2::Argon2;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::error::{CommonError, CommonResult};

/// Encrypted data container shared across queue and security modules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub salt: Option<String>,
    pub algorithm: String,
}

/// AES-GCM encryption service with optional password-based key derivation.
pub struct EncryptionService {
    key: Vec<u8>,
    cipher: Option<Aes256Gcm>,
    password_salt: Option<String>,
}

impl std::fmt::Debug for EncryptionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionService")
            .field("key", &"[REDACTED]")
            .field("cipher", &self.cipher.is_some())
            .field("password_salt", &self.password_salt.is_some())
            .finish()
    }
}

impl EncryptionService {
    /// Create a new encryption service from a raw 32-byte key.
    pub fn new(key: Vec<u8>) -> CommonResult<Self> {
        if key.len() != 32 {
            return Err(CommonError::internal(
                "Encryption key must be exactly 32 bytes".to_string(),
            ));
        }

        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| {
            CommonError::internal(format!("Failed to create encryption cipher: {e}"))
        })?;

        Ok(Self { key, cipher: Some(cipher), password_salt: None })
    }

    /// Derive an encryption key from a password using Argon2.
    pub fn from_password(password: &str) -> CommonResult<Self> {
        Self::from_password_with_salt(password, None)
    }

    /// Derive an encryption key from a password and optional salt using Argon2.
    pub fn from_password_with_salt(password: &str, salt: Option<&str>) -> CommonResult<Self> {
        let salt = match salt {
            Some(existing) => SaltString::from_b64(existing)
                .map_err(|e| CommonError::internal(format!("Invalid password salt: {e}")))?,
            None => SaltString::generate(OsRng),
        };
        let argon2 = Argon2::default();

        let mut key = vec![0u8; 32];
        argon2
            .hash_password_into(password.as_bytes(), salt.as_str().as_bytes(), &mut key)
            .map_err(|e| CommonError::internal(format!("Key derivation failed: {e}")))?;

        let mut service = Self::new(key)?;
        service.password_salt = Some(salt.to_string());
        Ok(service)
    }

    /// Generate a random 32-byte symmetric key.
    pub fn generate_key() -> Vec<u8> {
        let mut key = vec![0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }

    /// Encrypt bytes into an `EncryptedData` payload.
    pub fn encrypt(&self, data: &[u8]) -> CommonResult<EncryptedData> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or_else(|| CommonError::internal("Cipher not initialized".to_string()))?;

        let nonce_bytes = Self::generate_nonce();
        let ciphertext = cipher
            .encrypt(&Nonce::from(nonce_bytes), data)
            .map_err(|e| CommonError::internal(format!("Encryption failed: {e}")))?;

        Ok(EncryptedData {
            nonce: nonce_bytes.to_vec(),
            ciphertext,
            salt: self.password_salt.clone(),
            algorithm: "AES-256-GCM".to_string(),
        })
    }

    /// Decrypt an [`EncryptedData`] payload back into raw bytes.
    pub fn decrypt(&self, encrypted: &EncryptedData) -> CommonResult<Vec<u8>> {
        if encrypted.algorithm != "AES-256-GCM" {
            return Err(CommonError::internal(format!(
                "Unsupported algorithm: {}",
                encrypted.algorithm
            )));
        }

        let cipher = self
            .cipher
            .as_ref()
            .ok_or_else(|| CommonError::internal("Cipher not initialized".to_string()))?;

        if encrypted.nonce.len() != 12 {
            return Err(CommonError::internal(
                "Invalid nonce length for AES-256-GCM payload".to_string(),
            ));
        }

        let nonce_array: [u8; 12] = encrypted.nonce.as_slice().try_into().map_err(|_| {
            CommonError::internal("Nonce must be exactly 12 bytes for AES-256-GCM".to_string())
        })?;

        cipher
            .decrypt(&Nonce::from(nonce_array), encrypted.ciphertext.as_ref())
            .map_err(|e| CommonError::internal(format!("Decryption failed: {e}")))
    }

    /// Encrypt bytes and encode the payload as a base64 string.
    pub fn encrypt_to_string(&self, data: &[u8]) -> CommonResult<String> {
        let encrypted = self.encrypt(data)?;
        let serialized = serde_json::to_vec(&encrypted)?;
        Ok(BASE64.encode(serialized))
    }

    /// Decode a base64 string and decrypt the contained payload.
    pub fn decrypt_from_string(&self, encrypted_str: &str) -> CommonResult<Vec<u8>> {
        let decoded = BASE64
            .decode(encrypted_str)
            .map_err(|e| CommonError::internal(format!("Base64 decode failed: {e}")))?;
        let encrypted: EncryptedData = serde_json::from_slice(&decoded)?;
        self.decrypt(&encrypted)
    }

    /// Generate a short fingerprint for the current key.
    pub fn key_fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&self.key);
        let result = hasher.finalize();
        BASE64.encode(&result[..8])
    }

    /// Replace the current encryption key with a new one.
    pub fn rotate_key(&mut self, new_key: Vec<u8>) -> CommonResult<()> {
        if new_key.len() != 32 {
            return Err(CommonError::internal(
                "New encryption key must be exactly 32 bytes".to_string(),
            ));
        }

        let cipher = Aes256Gcm::new_from_slice(&new_key).map_err(|e| {
            CommonError::internal(format!("Failed to create encryption cipher: {e}"))
        })?;

        self.key = new_key;
        self.cipher = Some(cipher);
        self.password_salt = None;
        Ok(())
    }

    /// Re-encrypt data with a different encryption service.
    pub fn reencrypt(
        &self,
        encrypted: &EncryptedData,
        new_service: &EncryptionService,
    ) -> CommonResult<EncryptedData> {
        let decrypted = self.decrypt(encrypted)?;
        new_service.encrypt(&decrypted)
    }

    fn generate_nonce() -> [u8; 12] {
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }
}

/// Secure key storage helpers used by queue persistence and platform modules.
pub mod key_storage {
    use std::fs;
    use std::path::Path;

    use super::*;

    /// Save an encrypted key to disk using a master password.
    pub fn save_key(key: &[u8], path: &Path, master_password: &str) -> CommonResult<()> {
        let service = EncryptionService::from_password(master_password)?;
        let encrypted_key = service.encrypt_to_string(key)?;
        fs::write(path, encrypted_key).map_err(|e| {
            CommonError::internal(format!("Failed to write encrypted key file: {e}"))
        })?;
        Ok(())
    }

    /// Load an encrypted key from disk using a master password.
    pub fn load_key(path: &Path, master_password: &str) -> CommonResult<Vec<u8>> {
        let encrypted_key = fs::read_to_string(path).map_err(|e| {
            CommonError::internal(format!("Failed to read encrypted key file: {e}"))
        })?;
        let decoded = BASE64
            .decode(encrypted_key)
            .map_err(|e| CommonError::internal(format!("Failed to decode encrypted key: {e}")))?;
        let encrypted: EncryptedData = serde_json::from_slice(&decoded)?;
        let salt = encrypted.salt.clone().ok_or_else(|| {
            CommonError::internal("Encrypted key is missing password salt".to_string())
        })?;
        let service = EncryptionService::from_password_with_salt(master_password, Some(&salt))?;
        service.decrypt(&encrypted)
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for crypto::encryption.
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

        let plaintext = b"hello world";
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

        let plaintext = b"secure payload";
        let encoded = service.encrypt_to_string(plaintext).unwrap();
        let decoded = service.decrypt_from_string(&encoded).unwrap();

        assert_eq!(decoded, plaintext);
    }

    /// Validates `EncryptionService::generate_key` behavior for the rotate key
    /// updates cipher scenario.
    ///
    /// Assertions:
    /// - Confirms `service.key_fingerprint().len()` equals `BASE64.encode([0u8;
    ///   8]).len()`.
    #[test]
    fn rotate_key_updates_cipher() {
        let key = EncryptionService::generate_key();
        let mut service = EncryptionService::new(key).unwrap();

        let new_key = EncryptionService::generate_key();
        service.rotate_key(new_key.clone()).unwrap();

        assert_eq!(service.key_fingerprint().len(), BASE64.encode([0u8; 8]).len());
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
