//! Key rotation management for SQLCipher databases
//!
//! Provides automatic key rotation with SQLCipher rekey support,
//! integrating with the core encryption infrastructure.

use std::time::SystemTime;

use rusqlite::Connection;
use tracing::{debug, error, info, instrument, warn};

use crate::security::encryption::{KeyRotationSchedule, SecureString};
use crate::storage::error::{StorageError, StorageResult};

/// Storage-specific key manager with SQLCipher integration
///
/// Manages encryption key lifecycle including rotation and rekeying
/// of SQLCipher databases. Integrates with `KeyRotationSchedule` from
/// core encryption module.
pub struct StorageKeyManager {
    current_key: SecureString,
    rotation_schedule: KeyRotationSchedule,
    last_rotation: SystemTime,
}

impl StorageKeyManager {
    /// Create a new key manager with an initial key
    ///
    /// # Arguments
    /// * `initial_key` - The initial encryption key (SecureString for security)
    ///
    /// # Example
    /// ```no_run
    /// use pulsearc_common::security::encryption::keys::generate_encryption_key;
    /// use pulsearc_common::security::encryption::rotation::StorageKeyManager;
    ///
    /// let key = generate_encryption_key();
    /// let manager = StorageKeyManager::new(key);
    /// ```
    pub fn new(initial_key: SecureString) -> Self {
        Self {
            current_key: initial_key,
            rotation_schedule: KeyRotationSchedule::default(),
            last_rotation: SystemTime::now(),
        }
    }

    /// Create a key manager with a custom rotation schedule
    pub fn with_schedule(initial_key: SecureString, schedule: KeyRotationSchedule) -> Self {
        Self {
            current_key: initial_key,
            rotation_schedule: schedule,
            last_rotation: SystemTime::now(),
        }
    }

    /// Check if rotation is needed based on schedule
    ///
    /// Returns `true` if the key should be rotated according to the
    /// configured rotation schedule.
    pub fn should_rotate(&self) -> bool {
        self.rotation_schedule.should_rotate()
    }

    /// Rotate the encryption key and rekey the database
    ///
    /// This performs the following steps:
    /// 1. Checks if rotation is needed
    /// 2. Generates a new encryption key
    /// 3. Rekeys the SQLCipher database with the new key
    /// 4. Updates internal state (old key is automatically zeroized)
    ///
    /// # Arguments
    /// * `conn` - Active database connection to rekey
    ///
    /// # Errors
    /// Returns an error if:
    /// - Key generation fails
    /// - Database rekey operation fails
    /// - Post-rekey verification fails
    ///
    /// # Example
    /// ```no_run
    /// # use pulsearc_common::security::encryption::rotation::StorageKeyManager;
    /// # use rusqlite::Connection;
    /// # fn example(manager: &mut StorageKeyManager, conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    /// if manager.should_rotate() {
    ///     manager.rotate_key(conn)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self, conn))]
    pub fn rotate_key(&mut self, conn: &Connection) -> StorageResult<()> {
        if !self.should_rotate() {
            info!("Key rotation not needed yet");
            return Ok(());
        }

        info!("Starting key rotation for storage encryption");
        let start = std::time::Instant::now();

        // Generate new key
        let new_key = crate::security::encryption::keys::generate_encryption_key();

        // Rekey the database with SQLCipher
        let rekey_result = self.rekey_database(conn, &new_key);

        match rekey_result {
            Ok(_) => {
                // Update internal state (old key is zeroized automatically)
                self.current_key = new_key;
                self.last_rotation = SystemTime::now();
                self.rotation_schedule.record_rotation();

                // Log success
                let duration = start.elapsed();
                info!(duration_ms = duration.as_millis(), "Key rotation completed successfully");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "Key rotation failed");
                Err(e)
            }
        }
    }

    /// Force key rotation regardless of schedule
    ///
    /// Use this when you need to rotate immediately, bypassing the
    /// normal schedule checks (e.g., security incident, compliance
    /// requirement).
    #[instrument(skip(self, conn))]
    pub fn force_rotate(&mut self, conn: &Connection) -> StorageResult<()> {
        info!("Forcing immediate key rotation");
        let start = std::time::Instant::now();

        let new_key = crate::security::encryption::keys::generate_encryption_key();
        let rekey_result = self.rekey_database(conn, &new_key);

        match rekey_result {
            Ok(_) => {
                self.current_key = new_key;
                self.last_rotation = SystemTime::now();
                self.rotation_schedule.record_rotation();

                // Log forced rotation success
                let duration = start.elapsed();
                info!(
                    duration_ms = duration.as_millis(),
                    "Forced key rotation completed successfully"
                );
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "Forced key rotation failed");
                Err(e)
            }
        }
    }

    /// Rekey SQLCipher database with new encryption key
    ///
    /// Uses SQLCipher's PRAGMA rekey to change the encryption key
    /// while preserving all data. This is an atomic operation.
    ///
    /// # Security Note
    /// During rekey, SQLCipher:
    /// 1. Decrypts pages with the old key
    /// 2. Re-encrypts them with the new key
    /// 3. Writes them back to disk
    ///
    /// The operation is atomic - if it fails, the database remains
    /// encrypted with the old key.
    fn rekey_database(&self, conn: &Connection, new_key: &SecureString) -> StorageResult<()> {
        // Use SQLCipher's rekey pragma to change the encryption key
        conn.pragma_update(None, "rekey", new_key.expose()).map_err(|e| {
            error!(error = %e, "Failed to rekey database");
            StorageError::Encryption(format!("Database rekey failed: {}", e))
        })?;

        // Verify the new key works by querying the database
        let verify_result =
            conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(())).map_err(|e| {
                error!(error = %e, "Key verification after rekey failed");
                StorageError::Encryption(format!("Key verification after rekey failed: {}", e))
            });

        if verify_result.is_ok() {
            debug!("Database rekey successful");
        }

        verify_result
    }

    /// Get reference to the current encryption key
    ///
    /// # Security Warning
    /// Use with caution. The returned reference should not be stored
    /// or logged. Use only for immediate encryption operations.
    pub fn get_current_key(&self) -> &SecureString {
        &self.current_key
    }

    /// Set a custom rotation schedule
    ///
    /// # Arguments
    /// * `schedule` - New rotation schedule to use
    ///
    /// # Example
    /// ```no_run
    /// # use pulsearc_common::security::encryption::rotation::StorageKeyManager;
    /// # use pulsearc_common::security::encryption::KeyRotationSchedule;
    /// # fn example(manager: &mut StorageKeyManager) {
    /// let mut schedule = KeyRotationSchedule::default();
    /// schedule.set_rotation_days(30); // Rotate every 30 days
    /// manager.set_rotation_schedule(schedule);
    /// # }
    /// ```
    pub fn set_rotation_schedule(&mut self, schedule: KeyRotationSchedule) {
        self.rotation_schedule = schedule;
    }

    /// Get the current rotation schedule
    pub fn get_rotation_schedule(&self) -> &KeyRotationSchedule {
        &self.rotation_schedule
    }

    /// Get time since last rotation
    ///
    /// # Notes
    /// If the system clock goes backwards, this will default to zero duration
    /// and log a warning.
    pub fn time_since_last_rotation(&self) -> std::time::Duration {
        SystemTime::now().duration_since(self.last_rotation).unwrap_or_else(|e| {
            warn!(
                error = %e,
                "System clock went backwards during rotation time calculation, defaulting to zero"
            );
            std::time::Duration::ZERO
        })
    }

    /// Get days since last rotation
    pub fn days_since_last_rotation(&self) -> u64 {
        self.time_since_last_rotation().as_secs() / (24 * 3600)
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for security::encryption::rotation.
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::*;

    /// Validates `StorageKeyManager::new` behavior for the key manager creation
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `manager.days_since_last_rotation()` equals `0`.
    #[test]
    fn test_key_manager_creation() {
        let key = crate::security::encryption::keys::generate_encryption_key();
        let manager = StorageKeyManager::new(key);

        assert_eq!(manager.days_since_last_rotation(), 0);
    }

    /// Validates `StorageKeyManager::new` behavior for the should rotate
    /// initially false scenario.
    ///
    /// Assertions:
    /// - Ensures `!manager.should_rotate()` evaluates to true.
    #[test]
    fn test_should_rotate_initially_false() {
        let key = crate::security::encryption::keys::generate_encryption_key();
        let manager = StorageKeyManager::new(key);

        // Should not need rotation immediately after creation
        assert!(!manager.should_rotate());
    }

    /// Validates `KeyRotationSchedule::default` behavior for the custom
    /// schedule scenario.
    ///
    /// Assertions:
    /// - Confirms `manager.get_rotation_schedule().rotation_days` equals `30`.
    #[test]
    fn test_custom_schedule() {
        let key = crate::security::encryption::keys::generate_encryption_key();
        let mut schedule = KeyRotationSchedule::default();
        schedule.set_rotation_days(30);

        let manager = StorageKeyManager::with_schedule(key, schedule);
        assert_eq!(manager.get_rotation_schedule().rotation_days, 30);
    }

    /// Validates `TempDir::new` behavior for the key rotation with data
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `value` equals `"sensitive data"`.
    /// - Confirms `key_manager.days_since_last_rotation()` equals `0`.
    #[test]
    fn test_key_rotation_with_data() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create database with initial key
        let initial_key = crate::security::encryption::keys::generate_encryption_key();
        let mut key_manager = StorageKeyManager::new(initial_key.clone());

        let conn = Connection::open(&db_path).unwrap();

        // Configure SQLCipher with initial key
        let config =
            crate::storage::sqlcipher::cipher::SqlCipherConfig::from_secure_key(initial_key);
        crate::storage::sqlcipher::cipher::configure_sqlcipher(&conn, &config).unwrap();

        // Insert test data
        conn.execute("CREATE TABLE test (id INTEGER, value TEXT)", []).unwrap();
        conn.execute("INSERT INTO test VALUES (1, 'sensitive data')", []).unwrap();

        // Force rotation
        key_manager.force_rotate(&conn).unwrap();

        // Verify data still accessible after rotation
        let value: String =
            conn.query_row("SELECT value FROM test WHERE id = 1", [], |row| row.get(0)).unwrap();

        assert_eq!(value, "sensitive data");

        // Verify days since rotation is updated
        assert_eq!(key_manager.days_since_last_rotation(), 0);
    }

    /// Validates `TempDir::new` behavior for the rotation updates timestamp
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `after_rotation > before_rotation` evaluates to true.
    #[test]
    fn test_rotation_updates_timestamp() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let initial_key = crate::security::encryption::keys::generate_encryption_key();
        let mut key_manager = StorageKeyManager::new(initial_key.clone());

        let conn = Connection::open(&db_path).unwrap();
        let config =
            crate::storage::sqlcipher::cipher::SqlCipherConfig::from_secure_key(initial_key);
        crate::storage::sqlcipher::cipher::configure_sqlcipher(&conn, &config).unwrap();

        conn.execute("CREATE TABLE test (id INTEGER)", []).unwrap();

        let before_rotation = key_manager.last_rotation;

        // Force rotation
        key_manager.force_rotate(&conn).unwrap();

        let after_rotation = key_manager.last_rotation;

        assert!(after_rotation > before_rotation);
    }
}
