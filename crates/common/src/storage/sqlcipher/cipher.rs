//! SQLCipher configuration
//!
//! Provides SQLCipher pragma configuration for database encryption.
//! Based on macos/db/manager.rs lines 51-66.

use rusqlite::Connection;
use tracing::{debug, error};

use crate::security::encryption::SecureString;
use crate::storage::error::{StorageError, StorageResult};

/// SQLCipher configuration
#[derive(Clone)]
pub struct SqlCipherConfig {
    /// Encryption key (secured with automatic zeroization)
    pub key: SecureString,

    /// Cipher compatibility version (default: 4 for SQLCipher 4.x)
    pub cipher_compatibility: i32,

    /// KDF iterations for key derivation (default: 256000)
    pub kdf_iter: i32,

    /// Enable cipher memory security (default: true)
    pub cipher_memory_security: bool,
}

// Custom Debug impl to avoid exposing the key
impl std::fmt::Debug for SqlCipherConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqlCipherConfig")
            .field("key", &"SecureString(***)")
            .field("cipher_compatibility", &self.cipher_compatibility)
            .field("kdf_iter", &self.kdf_iter)
            .field("cipher_memory_security", &self.cipher_memory_security)
            .finish()
    }
}

impl SqlCipherConfig {
    /// Create default configuration with the given key
    pub fn new(key: String) -> Self {
        Self {
            key: SecureString::new(key),
            cipher_compatibility: 4,
            kdf_iter: 256000,
            cipher_memory_security: true,
        }
    }

    /// Create configuration from a SecureString key
    pub fn from_secure_key(key: SecureString) -> Self {
        Self { key, cipher_compatibility: 4, kdf_iter: 256000, cipher_memory_security: true }
    }

    /// Set cipher compatibility version
    pub fn with_cipher_compatibility(mut self, version: i32) -> Self {
        self.cipher_compatibility = version;
        self
    }

    /// Set KDF iterations
    pub fn with_kdf_iter(mut self, iterations: i32) -> Self {
        self.kdf_iter = iterations;
        self
    }

    /// Disable cipher memory security (not recommended)
    pub fn without_memory_security(mut self) -> Self {
        self.cipher_memory_security = false;
        self
    }
}

/// Configure SQLCipher for a connection
///
/// Applies encryption pragmas to enable SQLCipher encryption.
/// Must be called immediately after opening the connection.
///
/// # Critical SQLCipher pragmas
/// ```sql
/// PRAGMA key = '<encryption_key>';
/// PRAGMA cipher_compatibility = 4;
/// PRAGMA kdf_iter = 256000;
/// PRAGMA cipher_memory_security = ON;
/// ```
///
/// # Source
/// Based on macos/db/manager.rs lines 51-66
///
/// # Errors
/// Returns an error if any pragma fails to apply
pub fn configure_sqlcipher(conn: &Connection, config: &SqlCipherConfig) -> StorageResult<()> {
    let start = std::time::Instant::now();

    // Apply encryption key (must be first)
    // Use expose() to access the underlying key value
    let result = conn.pragma_update(None, "key", config.key.expose()).map_err(|e| {
        // Check if it's an encryption key error
        let err_str = e.to_string().to_lowercase();
        if err_str.contains("file is not a database")
            || err_str.contains("file is encrypted")
            || err_str.contains("database disk image is malformed")
        {
            StorageError::WrongKeyOrNotEncrypted
        } else {
            StorageError::Encryption(format!("Failed to set encryption key: {}", e))
        }
    });

    if let Err(ref e) = result {
        error!(error = %e, "SQLCipher key setup failed");
        return result;
    }

    // Set cipher compatibility version (SQLCipher 4.x)
    conn.pragma_update(None, "cipher_compatibility", config.cipher_compatibility).map_err(|e| {
        error!(error = %e, "Failed to set cipher_compatibility");
        StorageError::Encryption(format!("Failed to set cipher_compatibility: {}", e))
    })?;

    // Set KDF iterations (key derivation function)
    conn.pragma_update(None, "kdf_iter", config.kdf_iter).map_err(|e| {
        error!(error = %e, "Failed to set kdf_iter");
        StorageError::Encryption(format!("Failed to set kdf_iter: {}", e))
    })?;

    // Enable cipher memory security
    let memory_security = if config.cipher_memory_security { "ON" } else { "OFF" };
    conn.pragma_update(None, "cipher_memory_security", memory_security).map_err(|e| {
        error!(error = %e, "Failed to set cipher_memory_security");
        StorageError::Encryption(format!("Failed to set cipher_memory_security: {}", e))
    })?;

    // Log successful configuration
    let duration = start.elapsed();
    debug!(duration_ms = duration.as_millis(), "SQLCipher configuration successful");

    Ok(())
}

/// Verify that encryption is working by attempting to query the database
///
/// This catches encryption errors early before the pool is fully initialized.
/// Uses PRAGMA user_version which forces SQLCipher to actually decrypt pages.
///
/// # Errors
/// Returns `WrongKeyOrNotEncrypted` if the key is wrong or database isn't
/// encrypted
pub fn verify_encryption(conn: &Connection) -> StorageResult<()> {
    // First, try to read user_version which forces decryption of the database
    // header
    let result = conn
        .query_row("PRAGMA user_version", [], |_| Ok::<(), rusqlite::Error>(()))
        .and_then(|_| {
            // Also verify we can actually query tables - this forces reading encrypted
            // pages
            conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
        })
        .map_err(|e| {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("file is not a database")
                || err_str.contains("file is encrypted")
                || err_str.contains("database disk image is malformed")
                || err_str.contains("notadb")
                || err_str.contains("authentication failed")
                || err_str.contains("unsupported file format")
                || err_str.contains("unable to open database")
            {
                StorageError::WrongKeyOrNotEncrypted
            } else {
                StorageError::from(e)
            }
        });

    match &result {
        Ok(_) => {
            debug!("Encryption verification successful");
        }
        Err(e) => {
            error!(error = %e, "Encryption verification failed");
        }
    }

    result
}

#[cfg(test)]
mod tests {
    //! Unit tests for storage::sqlcipher::cipher.
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::*;

    /// Validates `SqlCipherConfig::new` behavior for the sqlcipher config
    /// defaults scenario.
    ///
    /// Assertions:
    /// - Confirms `config.cipher_compatibility` equals `4`.
    /// - Confirms `config.kdf_iter` equals `256000`.
    /// - Ensures `config.cipher_memory_security` evaluates to true.
    #[test]
    fn test_sqlcipher_config_defaults() {
        let config = SqlCipherConfig::new("test_key".to_string());
        assert_eq!(config.cipher_compatibility, 4);
        assert_eq!(config.kdf_iter, 256000);
        assert!(config.cipher_memory_security);
    }

    /// Validates `SqlCipherConfig::new` behavior for the sqlcipher config
    /// builder scenario.
    ///
    /// Assertions:
    /// - Confirms `config.cipher_compatibility` equals `3`.
    /// - Confirms `config.kdf_iter` equals `100000`.
    /// - Ensures `!config.cipher_memory_security` evaluates to true.
    #[test]
    fn test_sqlcipher_config_builder() {
        let config = SqlCipherConfig::new("test_key".to_string())
            .with_cipher_compatibility(3)
            .with_kdf_iter(100000)
            .without_memory_security();

        assert_eq!(config.cipher_compatibility, 3);
        assert_eq!(config.kdf_iter, 100000);
        assert!(!config.cipher_memory_security);
    }

    /// Validates `TempDir::new` behavior for the configure sqlcipher scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_configure_sqlcipher() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let conn = Connection::open(&db_path).unwrap();
        let config = SqlCipherConfig::new(
            "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        );

        // Should succeed with correct key
        configure_sqlcipher(&conn, &config).unwrap();
        verify_encryption(&conn).unwrap();
    }

    /// Validates `TempDir::new` behavior for the wrong encryption key scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(result, Err(StorageError::WrongKeyOrNotEncrypted))`
    ///   evaluates to true.
    #[test]
    fn test_wrong_encryption_key() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create database with one key
        {
            let conn = Connection::open(&db_path).unwrap();
            let config = SqlCipherConfig::new(
                "correct_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            );
            configure_sqlcipher(&conn, &config).unwrap();

            // Create a table to initialize the database
            conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", []).unwrap();
        }

        // Try to open with wrong key
        {
            let conn = Connection::open(&db_path).unwrap();
            let config = SqlCipherConfig::new(
                "wrong_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            );
            configure_sqlcipher(&conn, &config).unwrap();

            // Verification should fail
            let result = verify_encryption(&conn);
            assert!(matches!(result, Err(StorageError::WrongKeyOrNotEncrypted)));
        }
    }
}
