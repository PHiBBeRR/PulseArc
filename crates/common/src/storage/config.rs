//! Storage configuration
//!
//! Provides configuration types for the storage layer, including connection
//! pool settings, encryption key sources, and SQLite pragmas.

use std::path::PathBuf;
use std::time::Duration;

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Database file path
    pub path: PathBuf,

    /// Connection pool size (default: 10)
    pub pool_size: u32,

    /// Connection timeout in seconds (default: 5)
    pub connection_timeout_secs: u64,

    /// Busy timeout in milliseconds (default: 5000)
    pub busy_timeout_ms: u64,

    /// Enable WAL mode (default: true)
    pub enable_wal: bool,

    /// Enable foreign keys (default: true)
    pub enable_foreign_keys: bool,

    /// Encryption key source
    pub key_source: KeySource,
}

impl Default for StorageConfig {
    fn default() -> Self {
        // Check for DB_ENCRYPTION_KEY environment variable first (for testing)
        // This is safe in production since the env var won't be set there
        let key_source = if std::env::var("DB_ENCRYPTION_KEY").is_ok() {
            KeySource::Environment { var_name: "DB_ENCRYPTION_KEY".to_string() }
        } else {
            // Fall back to platform keychain for production
            KeySource::Keychain {
                service: "PulseArc".to_string(),
                username: "db_encryption_key".to_string(),
            }
        };

        Self {
            path: PathBuf::from("data/app.db"),
            pool_size: 10,
            connection_timeout_secs: 5,
            busy_timeout_ms: 5000,
            enable_wal: true,
            enable_foreign_keys: true,
            key_source,
        }
    }
}

impl StorageConfig {
    /// Create a new configuration with the given path
    pub fn new(path: PathBuf) -> Self {
        Self { path, ..Default::default() }
    }

    /// Validate the configuration
    ///
    /// Ensures all configuration values are within acceptable ranges.
    ///
    /// # Errors
    /// Returns an error if any configuration value is invalid.
    pub fn validate(&self) -> Result<(), super::error::StorageError> {
        use super::error::StorageError;

        // Validate pool size
        if self.pool_size == 0 {
            return Err(StorageError::InvalidConfig(
                "pool_size must be greater than 0".to_string(),
            ));
        }
        if self.pool_size > 100 {
            return Err(StorageError::InvalidConfig("pool_size too large (max: 100)".to_string()));
        }

        // Validate timeouts
        if self.connection_timeout_secs == 0 {
            return Err(StorageError::InvalidConfig(
                "connection_timeout_secs must be greater than 0".to_string(),
            ));
        }
        if self.busy_timeout_ms == 0 {
            return Err(StorageError::InvalidConfig(
                "busy_timeout_ms must be greater than 0".to_string(),
            ));
        }

        // Validate database path
        if self.path.as_os_str().is_empty() {
            return Err(StorageError::InvalidConfig("database path cannot be empty".to_string()));
        }

        Ok(())
    }

    /// Set the connection pool size
    pub fn with_pool_size(mut self, size: u32) -> Self {
        self.pool_size = size;
        self
    }

    /// Set the connection timeout
    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout_secs = timeout.as_secs();
        self
    }

    /// Set the busy timeout
    pub fn with_busy_timeout(mut self, timeout: Duration) -> Self {
        self.busy_timeout_ms = timeout.as_millis() as u64;
        self
    }

    /// Set the encryption key source
    pub fn with_key_source(mut self, source: KeySource) -> Self {
        self.key_source = source;
        self
    }

    /// Disable WAL mode (not recommended for production)
    pub fn without_wal(mut self) -> Self {
        self.enable_wal = false;
        self
    }

    /// Disable foreign key constraints (not recommended for production)
    pub fn without_foreign_keys(mut self) -> Self {
        self.enable_foreign_keys = false;
        self
    }

    /// Create a builder for more complex configurations
    pub fn builder(path: PathBuf) -> StorageConfigBuilder {
        StorageConfigBuilder::new(path)
    }
}

/// Builder for StorageConfig with validation
///
/// Follows enterprise builder pattern from agent/common/resilience.
#[derive(Debug)]
pub struct StorageConfigBuilder {
    config: StorageConfig,
}

impl StorageConfigBuilder {
    /// Create a new builder
    pub fn new(path: PathBuf) -> Self {
        Self { config: StorageConfig::new(path) }
    }

    /// Set pool size
    pub fn pool_size(mut self, size: u32) -> Self {
        self.config.pool_size = size;
        self
    }

    /// Set connection timeout
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.config.connection_timeout_secs = timeout.as_secs();
        self
    }

    /// Set busy timeout
    pub fn busy_timeout(mut self, timeout: Duration) -> Self {
        self.config.busy_timeout_ms = timeout.as_millis() as u64;
        self
    }

    /// Set key source
    pub fn key_source(mut self, source: KeySource) -> Self {
        self.config.key_source = source;
        self
    }

    /// Disable WAL mode
    pub fn disable_wal(mut self) -> Self {
        self.config.enable_wal = false;
        self
    }

    /// Disable foreign keys
    pub fn disable_foreign_keys(mut self) -> Self {
        self.config.enable_foreign_keys = false;
        self
    }

    /// Build and validate the configuration
    pub fn build(self) -> Result<StorageConfig, crate::storage::error::StorageError> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Source for encryption keys
#[derive(Debug, Clone)]
pub enum KeySource {
    /// Load from platform keychain
    ///
    /// - macOS: Uses Keychain Access
    /// - Windows: Uses Credential Manager
    Keychain { service: String, username: String },

    /// Load from environment variable (test/dev only)
    ///
    /// **Security Warning**: Not recommended for production use.
    Environment { var_name: String },

    /// Use provided key directly (dangerous - use only for testing)
    ///
    /// **Security Warning**: Exposing keys in code is extremely dangerous.
    /// This option should only be used in isolated test environments.
    Direct { key: String },
}

impl Default for KeySource {
    fn default() -> Self {
        Self::Keychain {
            service: "PulseArc".to_string(),
            username: "db_encryption_key".to_string(),
        }
    }
}

impl KeySource {
    /// Create a keychain key source
    pub fn keychain(service: impl Into<String>, username: impl Into<String>) -> Self {
        Self::Keychain { service: service.into(), username: username.into() }
    }

    /// Create an environment variable key source
    pub fn environment(var_name: impl Into<String>) -> Self {
        Self::Environment { var_name: var_name.into() }
    }

    /// Create a direct key source (testing only)
    pub fn direct(key: impl Into<String>) -> Self {
        Self::Direct { key: key.into() }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for storage::config.
    use super::*;

    /// Validates `StorageConfig::default` behavior for the default config
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `config.pool_size` equals `10`.
    /// - Confirms `config.connection_timeout_secs` equals `5`.
    /// - Confirms `config.busy_timeout_ms` equals `5000`.
    /// - Ensures `config.enable_wal` evaluates to true.
    /// - Ensures `config.enable_foreign_keys` evaluates to true.
    #[test]
    fn test_default_config() {
        let config = StorageConfig::default();
        assert_eq!(config.pool_size, 10);
        assert_eq!(config.connection_timeout_secs, 5);
        assert_eq!(config.busy_timeout_ms, 5000);
        assert!(config.enable_wal);
        assert!(config.enable_foreign_keys);
    }

    /// Validates `StorageConfig::new` behavior for the config builder method
    /// chaining scenario.
    ///
    /// Assertions:
    /// - Confirms `config.pool_size` equals `20`.
    /// - Confirms `config.connection_timeout_secs` equals `10`.
    /// - Confirms `config.busy_timeout_ms` equals `10000`.
    /// - Ensures `!config.enable_wal` evaluates to true.
    /// - Ensures `!config.enable_foreign_keys` evaluates to true.
    #[test]
    fn test_config_builder_method_chaining() {
        let temp_path = std::env::temp_dir().join("pulsearc-test.db");
        let config = StorageConfig::new(temp_path)
            .with_pool_size(20)
            .with_connection_timeout(Duration::from_secs(10))
            .with_busy_timeout(Duration::from_millis(10000))
            .without_wal()
            .without_foreign_keys();

        assert_eq!(config.pool_size, 20);
        assert_eq!(config.connection_timeout_secs, 10);
        assert_eq!(config.busy_timeout_ms, 10000);
        assert!(!config.enable_wal);
        assert!(!config.enable_foreign_keys);
    }

    /// Validates `StorageConfig::builder` behavior for the config builder
    /// pattern scenario.
    ///
    /// Assertions:
    /// - Confirms `config.pool_size` equals `15`.
    /// - Confirms `config.connection_timeout_secs` equals `8`.
    #[test]
    fn test_config_builder_pattern() {
        let temp_path = std::env::temp_dir().join("pulsearc-test.db");
        let config = StorageConfig::builder(temp_path)
            .pool_size(15)
            .connection_timeout(Duration::from_secs(8))
            .key_source(KeySource::environment("TEST_KEY"))
            .build()
            .unwrap();

        assert_eq!(config.pool_size, 15);
        assert_eq!(config.connection_timeout_secs, 8);
    }

    /// Validates `StorageConfig::builder` behavior for the config validation
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `config.is_err()` evaluates to true.
    /// - Ensures `config.is_err()` evaluates to true.
    #[test]
    fn test_config_validation() {
        let temp_path = std::env::temp_dir().join("pulsearc-test.db");

        // Pool size too large
        let config = StorageConfig::builder(temp_path.clone()).pool_size(150).build();
        assert!(config.is_err());

        // Pool size zero
        let config = StorageConfig::builder(temp_path).pool_size(0).build();
        assert!(config.is_err());
    }

    /// Validates `KeySource::keychain` behavior for the key source constructors
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(keychain, KeySource::Keychain { .. })` evaluates to
    ///   true.
    /// - Ensures `matches!(env, KeySource::Environment { .. })` evaluates to
    ///   true.
    /// - Ensures `matches!(direct, KeySource::Direct { .. })` evaluates to
    ///   true.
    #[test]
    fn test_key_source_constructors() {
        let keychain = KeySource::keychain("MyApp", "db_key");
        assert!(matches!(keychain, KeySource::Keychain { .. }));

        let env = KeySource::environment("MY_DB_KEY");
        assert!(matches!(env, KeySource::Environment { .. }));

        let direct = KeySource::direct("test_key_123");
        assert!(matches!(direct, KeySource::Direct { .. }));
    }
}
