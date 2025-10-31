//! SQLCipher connection pool configuration

use std::time::Duration;

use crate::storage::config::StorageConfig;

/// SQLCipher pool configuration
///
/// Wraps StorageConfig with r2d2-specific settings
#[derive(Debug, Clone)]
pub struct SqlCipherPoolConfig {
    /// Maximum number of connections in the pool
    pub max_size: u32,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Busy timeout for SQLite operations
    pub busy_timeout: Duration,

    /// Enable WAL journal mode
    pub enable_wal: bool,

    /// Enable foreign key constraints
    pub enable_foreign_keys: bool,
}

impl From<&StorageConfig> for SqlCipherPoolConfig {
    fn from(config: &StorageConfig) -> Self {
        Self {
            max_size: config.pool_size,
            connection_timeout: Duration::from_secs(config.connection_timeout_secs),
            busy_timeout: Duration::from_millis(config.busy_timeout_ms),
            enable_wal: config.enable_wal,
            enable_foreign_keys: config.enable_foreign_keys,
        }
    }
}

impl Default for SqlCipherPoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10,
            connection_timeout: Duration::from_secs(5),
            busy_timeout: Duration::from_millis(5000),
            enable_wal: true,
            enable_foreign_keys: true,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for storage::sqlcipher::config.
    use super::*;

    /// Tests the default configuration values for SQLCipher pool.
    ///
    /// Verifies all default settings match expected production values.
    #[test]
    fn test_default_config() {
        let config = SqlCipherPoolConfig::default();

        assert_eq!(config.max_size, 10, "Default pool size should be 10");
        assert_eq!(
            config.connection_timeout,
            Duration::from_secs(5),
            "Default connection timeout should be 5 seconds"
        );
        assert_eq!(
            config.busy_timeout,
            Duration::from_millis(5000),
            "Default busy timeout should be 5000ms"
        );
        assert!(config.enable_wal, "WAL mode should be enabled by default");
        assert!(config.enable_foreign_keys, "Foreign keys should be enabled by default");
    }

    /// Tests conversion from StorageConfig to SqlCipherPoolConfig.
    ///
    /// Verifies that all configuration fields are properly mapped.
    #[test]
    fn test_from_storage_config() {
        let temp_path = std::env::temp_dir().join("pulsearc-test.db");
        let storage_config = StorageConfig {
            path: temp_path,
            pool_size: 20,
            connection_timeout_secs: 10,
            busy_timeout_ms: 10000,
            enable_wal: false,
            enable_foreign_keys: false,
            key_source: crate::storage::config::KeySource::direct(
                "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            ),
        };

        let pool_config = SqlCipherPoolConfig::from(&storage_config);

        assert_eq!(pool_config.max_size, 20, "Pool config should use storage config's pool_size");
        assert_eq!(
            pool_config.connection_timeout,
            Duration::from_secs(10),
            "Pool config should use storage config's connection timeout"
        );
        assert_eq!(
            pool_config.busy_timeout,
            Duration::from_millis(10000),
            "Pool config should use storage config's busy timeout"
        );
        assert!(!pool_config.enable_wal, "Pool config should respect storage config's WAL setting");
        assert!(
            !pool_config.enable_foreign_keys,
            "Pool config should respect storage config's foreign keys setting"
        );
    }

    /// Tests that SqlCipherPoolConfig implements Clone correctly.
    ///
    /// Verifies that cloned config has identical field values.
    #[test]
    fn test_clone() {
        let config1 = SqlCipherPoolConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.max_size, config2.max_size, "Cloned config should have same max_size");
        assert_eq!(
            config1.connection_timeout, config2.connection_timeout,
            "Cloned config should have same connection timeout"
        );
        assert_eq!(
            config1.busy_timeout, config2.busy_timeout,
            "Cloned config should have same busy timeout"
        );
    }

    /// Tests that Debug formatting includes relevant information.
    ///
    /// Verifies struct name and field names appear in debug output.
    #[test]
    fn test_debug_format() {
        let config = SqlCipherPoolConfig::default();
        let debug_str = format!("{:?}", config);

        assert!(
            debug_str.contains("SqlCipherPoolConfig"),
            "Debug format should include struct name"
        );
        assert!(debug_str.contains("max_size"), "Debug format should include field names");
    }
}
