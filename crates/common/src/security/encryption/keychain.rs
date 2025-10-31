//! Generic keychain provider for secure credential storage
//!
//! This module exposes a thin wrapper over the platform keychain for storing
//! arbitrary secrets and encryption keys across macOS (Keychain Access),
//! Windows (Credential Manager), and Linux (Secret Service API).
//!
//! ## Features
//!
//! - **Secret Storage**: Persist arbitrary strings in the platform keychain
//! - **Encryption Keys**: Manage encryption keys with automatic generation
//! - **Cross-platform**: Works on macOS, Windows, and Linux
//! - **Backward Compatible**: Maintains existing keychain entry structure
//!
//! ## Module Relationships
//!
//! This module provides the **generic** keychain provider. Domain-specific
//! helpers are built on top:
//!
//! - This module (`security::encryption::keychain`): Generic secret storage
//! - `auth::keychain`: OAuth token-specific storage helpers
//! - `security::keychain`: Convenience re-export of this module
//!
//! ## Usage
//!
//! ```no_run
//! use pulsearc_common::security::keychain::KeychainProvider;
//!
//! let keychain = KeychainProvider::new("PulseArc.encryption");
//! keychain.set_secret("service_account", "super-secret")?;
//! let secret = keychain.get_secret("service_account")?;
//! assert_eq!(secret, "super-secret");
//! # Ok::<(), pulsearc_common::security::KeychainError>(())
//! ```

use keyring::Entry;
use thiserror::Error;
use tracing::debug;

/// Generic keychain provider for secure credential storage
///
/// Maintains compatibility with existing key naming conventions while
/// allowing callers to persist arbitrary secrets and encryption keys.
pub struct KeychainProvider {
    service_name: String,
}

impl KeychainProvider {
    /// Create a new keychain provider for a specific service
    ///
    /// # Arguments
    /// * `service_name` - Service identifier (e.g., "PulseArc.calendar",
    ///   "PulseArc.sap")
    ///
    /// # Examples
    /// ```
    /// use pulsearc_common::security::keychain::KeychainProvider;
    ///
    /// let keychain = KeychainProvider::new("PulseArc.calendar");
    /// ```
    pub fn new(service_name: impl Into<String>) -> Self {
        Self { service_name: service_name.into() }
    }

    /// Store a secret value in the platform keychain
    ///
    /// # Arguments
    /// * `key` - Logical key (e.g., "access.user@example.com")
    /// * `value` - Secret value to persist
    ///
    /// # Errors
    /// Returns `KeychainError::AccessFailed` if keychain access fails
    pub fn set_secret(&self, key: &str, value: &str) -> Result<(), KeychainError> {
        debug!(
            service = %self.service_name,
            key = %key,
            "Storing secret in keychain"
        );

        let entry = self.create_entry(key)?;
        entry.set_password(value).map_err(|e| {
            KeychainError::AccessFailed(format!("Failed to store secret for {}: {}", key, e))
        })?;

        debug!(service = %self.service_name, key = %key, "Secret stored successfully");

        Ok(())
    }

    /// Retrieve a secret value from the platform keychain
    ///
    /// # Errors
    /// Returns `KeychainError::NotFound` if secret doesn't exist
    /// Returns `KeychainError::AccessFailed` if keychain access fails
    pub fn get_secret(&self, key: &str) -> Result<String, KeychainError> {
        debug!(
            service = %self.service_name,
            key = %key,
            "Retrieving secret from keychain"
        );

        let entry = self.create_entry(key)?;
        let secret = entry.get_password().map_err(|e| {
            if matches!(e, keyring::Error::NoEntry) {
                KeychainError::NotFound
            } else {
                KeychainError::AccessFailed(format!("Failed to retrieve secret for {}: {}", key, e))
            }
        })?;

        debug!(service = %self.service_name, key = %key, "Secret retrieved successfully");

        Ok(secret)
    }

    /// Delete a secret from the platform keychain (idempotent)
    pub fn delete_secret(&self, key: &str) -> Result<(), KeychainError> {
        debug!(
            service = %self.service_name,
            key = %key,
            "Deleting secret from keychain"
        );

        match self.create_entry(key) {
            Ok(entry) => {
                if let Err(e) = entry.delete_credential() {
                    if !matches!(e, keyring::Error::NoEntry) {
                        return Err(KeychainError::AccessFailed(format!(
                            "Failed to delete secret for {}: {}",
                            key, e
                        )));
                    }
                }
            }
            Err(e) => return Err(e),
        }

        debug!(service = %self.service_name, key = %key, "Secret deleted successfully");

        Ok(())
    }

    /// Check if a secret exists in the keychain
    #[must_use]
    pub fn secret_exists(&self, key: &str) -> bool {
        match self.create_entry(key) {
            Ok(entry) => entry.get_password().is_ok(),
            Err(_) => false,
        }
    }

    /// Store an encryption key in the keychain
    ///
    /// # Arguments
    /// * `key_id` - Key identifier (e.g., "db_encryption_key")
    /// * `key` - Encryption key string
    ///
    /// # Errors
    /// Returns `KeychainError::AccessFailed` if keychain access fails
    pub fn store_key(&self, key_id: &str, key: &str) -> Result<(), KeychainError> {
        debug!(
            service = %self.service_name,
            key_id = %key_id,
            "Storing encryption key in keychain"
        );

        self.set_secret(key_id, key)
    }

    /// Retrieve an encryption key from the keychain
    ///
    /// # Arguments
    /// * `key_id` - Key identifier
    ///
    /// # Returns
    /// The encryption key string
    ///
    /// # Errors
    /// Returns `KeychainError::NotFound` if key doesn't exist
    /// Returns `KeychainError::AccessFailed` if keychain access fails
    pub fn retrieve_key(&self, key_id: &str) -> Result<String, KeychainError> {
        debug!(
            service = %self.service_name,
            key_id = %key_id,
            "Retrieving encryption key from keychain"
        );

        self.get_secret(key_id)
    }

    /// Get or create an encryption key
    ///
    /// If the key exists in the keychain, it is returned.
    /// If the key doesn't exist, a new random key of the specified size is
    /// generated, stored in the keychain, and returned.
    ///
    /// # Arguments
    /// * `key_id` - Key identifier (e.g., "db_encryption_key")
    /// * `key_size` - Size of the key to generate (in characters) if it doesn't
    ///   exist
    ///
    /// # Returns
    /// The encryption key string
    ///
    /// # Errors
    /// Returns `KeychainError::AccessFailed` if keychain access fails
    pub fn get_or_create_key(
        &self,
        key_id: &str,
        key_size: usize,
    ) -> Result<String, KeychainError> {
        debug!(
            service = %self.service_name,
            key_id = %key_id,
            key_size = %key_size,
            "Getting or creating encryption key"
        );

        match self.retrieve_key(key_id) {
            Ok(key) => {
                debug!(
                    service = %self.service_name,
                    key_id = %key_id,
                    "Existing encryption key found"
                );
                Ok(key)
            }
            Err(KeychainError::NotFound) => {
                debug!(
                    service = %self.service_name,
                    key_id = %key_id,
                    "No existing key found, generating new key"
                );

                // Generate new random key
                use rand::distributions::{Alphanumeric, DistString};
                let key = Alphanumeric.sample_string(&mut rand::thread_rng(), key_size);

                // Store in keychain
                self.store_key(key_id, &key)?;

                debug!(
                    service = %self.service_name,
                    key_id = %key_id,
                    "New encryption key generated and stored"
                );

                Ok(key)
            }
            Err(e) => Err(e),
        }
    }

    /// Create a keyring entry
    ///
    /// # Arguments
    /// * `account` - Account/key identifier
    ///
    /// # Errors
    /// Returns `KeychainError::AccessFailed` if entry creation fails
    fn create_entry(&self, account: &str) -> Result<Entry, KeychainError> {
        Entry::new(&self.service_name, account).map_err(|e| {
            KeychainError::AccessFailed(format!("Failed to create keychain entry: {}", e))
        })
    }
}

/// Keychain error types
#[derive(Debug, Error)]
pub enum KeychainError {
    /// Keychain access failed (permission denied, not available, etc.)
    #[error("Keychain access failed: {0}")]
    AccessFailed(String),

    /// Entry not found in keychain
    #[error("Entry not found")]
    NotFound,

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Underlying keyring library error
    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),
}

// Note: Error conversions (From<KeychainError>) have been moved to consuming
// modules:
// - integrations/calendar/error.rs for CalendarError
// - integrations/sap/error.rs for SapError
// - storage/error.rs for StorageError
// This prevents common/keychain from depending on integration-specific modules.

#[cfg(test)]
mod tests {
    //! Unit tests for security::encryption::keychain.
    use super::*;
    #[cfg(feature = "platform")]
    use crate::testing::MockKeychainProvider;

    /// Create a test service name to avoid conflicts with real keychain entries
    fn test_service_name() -> String {
        format!("PulseArcTest.{}", uuid::Uuid::new_v4())
    }

    /// Validates `KeychainProvider::new` behavior for the keychain provider
    /// creation scenario.
    ///
    /// Assertions:
    /// - Confirms `keychain.service_name` equals `"test-service"`.
    #[test]
    fn test_keychain_provider_creation() {
        let keychain = KeychainProvider::new("test-service");
        assert_eq!(keychain.service_name, "test-service");
    }

    /// Validates `MockKeychainProvider::new` behavior for the set get and
    /// delete secret scenario.
    ///
    /// Assertions:
    /// - Ensures `keychain.secret_exists(key_id)` evaluates to true.
    /// - Confirms `retrieved` equals `"super-secret"`.
    /// - Ensures `!keychain.secret_exists(key_id)` evaluates to true.
    #[cfg(feature = "platform")]
    #[test]
    fn test_set_get_and_delete_secret() {
        let service = test_service_name();
        let keychain = MockKeychainProvider::new(&service);
        let key_id = "test.secret";

        keychain.set_secret(key_id, "super-secret").unwrap();
        assert!(keychain.secret_exists(key_id));

        let retrieved = keychain.get_secret(key_id).unwrap();
        assert_eq!(retrieved, "super-secret");

        keychain.delete_secret(key_id).unwrap();
        assert!(!keychain.secret_exists(key_id));
    }

    /// Validates `MockKeychainProvider::new` behavior for the delete secret
    /// idempotent scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[cfg(feature = "platform")]
    #[test]
    fn test_delete_secret_idempotent() {
        let service = test_service_name();
        let keychain = MockKeychainProvider::new(&service);
        let key_id = "test.secret.delete";

        keychain.delete_secret(key_id).unwrap();
        keychain.set_secret(key_id, "value").unwrap();
        keychain.delete_secret(key_id).unwrap();
        keychain.delete_secret(key_id).unwrap();
    }

    /// Validates `MockKeychainProvider::new` behavior for the get secret not
    /// found scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(result, Err(KeychainError::NotFound))` evaluates to
    ///   true.
    #[cfg(feature = "platform")]
    #[test]
    fn test_get_secret_not_found() {
        let service = test_service_name();
        let keychain = MockKeychainProvider::new(&service);
        let key_id = "missing.secret";

        let result = keychain.get_secret(key_id);
        assert!(matches!(result, Err(KeychainError::NotFound)));
    }

    /// Validates `MockKeychainProvider::new` behavior for the store and
    /// retrieve key scenario.
    ///
    /// Assertions:
    /// - Confirms `retrieved` equals `key`.
    #[cfg(feature = "platform")]
    #[test]
    fn test_store_and_retrieve_key() {
        let service = test_service_name();
        let keychain = MockKeychainProvider::new(&service);
        let key_id = "test_encryption_key";
        let key = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

        keychain.store_key(key_id, key).unwrap();
        let retrieved = keychain.retrieve_key(key_id).unwrap();
        assert_eq!(retrieved, key);
    }

    /// Validates `MockKeychainProvider::new` behavior for the get or create key
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `key1.len()` equals `64`.
    /// - Confirms `key1` equals `key2`.
    #[cfg(feature = "platform")]
    #[test]
    fn test_get_or_create_key() {
        let service = test_service_name();
        let keychain = MockKeychainProvider::new(&service);
        let key_id = "test_auto_key";

        let key1 = keychain.get_or_create_key(key_id, 64).unwrap();
        assert_eq!(key1.len(), 64);

        let key2 = keychain.get_or_create_key(key_id, 64).unwrap();
        assert_eq!(key1, key2);
    }

    /// Validates `MockKeychainProvider::new` behavior for the retrieve
    /// nonexistent key scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(result, Err(KeychainError::NotFound))` evaluates to
    ///   true.
    #[cfg(feature = "platform")]
    #[test]
    fn test_retrieve_nonexistent_key() {
        let service = test_service_name();
        let keychain = MockKeychainProvider::new(&service);
        let key_id = "nonexistent_key";

        let result = keychain.retrieve_key(key_id);
        assert!(matches!(result, Err(KeychainError::NotFound)));
    }

    /// Validates `MockKeychainProvider::new` behavior for the multiple secrets
    /// same service scenario.
    ///
    /// Assertions:
    /// - Confirms `keychain.get_secret("account1").unwrap()` equals
    ///   `"secret-one"`.
    /// - Confirms `keychain.get_secret("account2").unwrap()` equals
    ///   `"secret-two"`.
    #[cfg(feature = "platform")]
    #[test]
    fn test_multiple_secrets_same_service() {
        let service = test_service_name();
        let keychain = MockKeychainProvider::new(&service);

        keychain.set_secret("account1", "secret-one").unwrap();
        keychain.set_secret("account2", "secret-two").unwrap();

        assert_eq!(keychain.get_secret("account1").unwrap(), "secret-one");
        assert_eq!(keychain.get_secret("account2").unwrap(), "secret-two");
    }

    /// Validates `MockKeychainProvider::new` behavior for the service isolation
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(keychain2.get_secret("account"),
    ///   Err(KeychainError::NotFound))` evaluates to true.
    #[cfg(feature = "platform")]
    #[test]
    fn test_service_isolation() {
        let service1 = test_service_name();
        let service2 = test_service_name();

        let keychain1 = MockKeychainProvider::new(&service1);
        let keychain2 = MockKeychainProvider::new(&service2);

        keychain1.set_secret("account", "secret").unwrap();

        assert!(matches!(keychain2.get_secret("account"), Err(KeychainError::NotFound)));
    }
}
