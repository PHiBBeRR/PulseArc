//! Platform keychain integration for encryption keys
//!
//! Provides secure key storage using the common keychain provider.
//! Supports multiple key sources for flexibility in different environments.
//!
//! # Platform Support
//! - ✅ macOS: Keychain Access
//! - ✅ Windows: Credential Manager
//! - ✅ Linux: Secret Service API
//!
//! Compatible with existing macos/db/manager.rs implementation.

use tracing::{info, instrument, warn};

use super::keychain::KeychainProvider;
use super::SecureString;
use crate::storage::config::KeySource;
use crate::storage::error::{StorageError, StorageResult};

/// Get or create encryption key from the configured key source
///
/// This is the main entry point for key management. It supports multiple
/// key sources for flexibility in different environments.
///
/// Returns a `SecureString` that automatically zeroes memory on drop.
///
/// # Security
/// This function accesses sensitive credentials and should be audited.
///
/// # Arguments
/// * `key_source` - Source to load/generate the encryption key from
///
/// # Errors
/// Returns an error if:
/// - Keychain access fails (permission denied, not available)
/// - Environment variable not found
/// - Key generation fails
#[instrument(skip_all, fields(key_source_type = ?std::mem::discriminant(key_source)))]
pub fn get_or_create_key(key_source: &KeySource) -> StorageResult<SecureString> {
    match key_source {
        KeySource::Keychain { service, username } => {
            info!("Retrieving encryption key from platform keychain");
            get_or_create_keychain_key(service, username)
        }
        KeySource::Environment { var_name } => {
            info!("Retrieving encryption key from environment variable");
            std::env::var(var_name).map(SecureString::new).map_err(|e| {
                warn!("Environment variable {} not found", var_name);
                StorageError::Keychain(format!(
                    "Environment variable {} not found: {}",
                    var_name, e
                ))
            })
        }
        KeySource::Direct { key } => {
            warn!("Using direct key source (not recommended for production)");
            if key.len() < 32 {
                return Err(StorageError::Encryption(
                    "Direct key must be at least 32 characters".to_string(),
                ));
            }
            Ok(SecureString::new(key.clone()))
        }
    }
}

/// Get or create encryption key from platform keychain
///
/// Uses the common KeychainProvider for secure key storage.
///
/// Compatible with macos/db/manager.rs implementation:
/// - Service: "PulseArc" (or custom service name)
/// - Username: "db_encryption_key" (or custom key ID)
///
/// # Behavior
/// 1. Try to retrieve existing key from keychain
/// 2. If not found, generate new 64-character key
/// 3. Store new key in keychain
/// 4. Return key as SecureString
///
/// # Source
/// Based on macos/db/manager.rs lines 158-201, now using common
/// KeychainProvider
#[instrument(fields(service = service, username = username))]
fn get_or_create_keychain_key(service: &str, username: &str) -> StorageResult<SecureString> {
    info!("Accessing keychain for encryption key");

    let keychain = KeychainProvider::new(service);

    // Try to get existing key, or create new one if not found
    match keychain.get_or_create_key(username, 64) {
        Ok(key) => {
            info!("Encryption key retrieved/created successfully from keychain");
            Ok(SecureString::new(key))
        }
        Err(e) => {
            warn!("Failed to get/create keychain key: {}", e);
            Err(StorageError::Keychain(format!("Failed to get or create encryption key: {}", e)))
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for security::encryption::key_provider.
    use super::*;

    /// Validates `KeySource::Environment` behavior for the environment key
    /// source scenario.
    ///
    /// Assertions:
    /// - Confirms `key.len()` equals `65`.
    #[test]
    fn test_environment_key_source() {
        std::env::set_var(
            "TEST_DB_KEY",
            "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );

        let key_source = KeySource::Environment { var_name: "TEST_DB_KEY".to_string() };

        let key = get_or_create_key(&key_source).unwrap();
        assert_eq!(key.len(), 65);

        std::env::remove_var("TEST_DB_KEY");
    }

    /// Validates `KeySource::Direct` behavior for the direct key source
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `key.len()` equals `65`.
    #[test]
    fn test_direct_key_source() {
        let key_source = KeySource::Direct {
            key: "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        };

        let key = get_or_create_key(&key_source).unwrap();
        assert_eq!(key.len(), 65);
    }

    /// Validates `KeySource::Direct` behavior for the direct key too short
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_direct_key_too_short() {
        let key_source = KeySource::Direct { key: "short".to_string() };

        let result = get_or_create_key(&key_source);
        assert!(result.is_err());
    }

    /// Validates `KeySource::Environment` behavior for the environment key not
    /// found scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_environment_key_not_found() {
        let key_source = KeySource::Environment { var_name: "NONEXISTENT_KEY_VAR".to_string() };

        let result = get_or_create_key(&key_source);
        assert!(result.is_err());
    }
}
