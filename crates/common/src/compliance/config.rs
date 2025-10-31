// Remote Configuration Management

use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize};
use tracing::info;

// Type aliases for complex types
type InitResult = Result<(), Box<dyn std::error::Error>>;

/// Remote configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub version: String,
    pub environment: String,
    pub settings: HashMap<String, serde_json::Value>,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_url: Option<String>,
}

/// Configuration manager for remote and local configs
#[derive(Debug)]
pub struct ConfigManager {
    config: RemoteConfig,
    local_overrides: HashMap<String, serde_json::Value>,
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            config: RemoteConfig {
                version: "1.0.0".to_string(),
                environment: "production".to_string(),
                settings: HashMap::new(),
                last_sync: None,
                sync_url: None,
            },
            local_overrides: HashMap::new(),
        }
    }

    pub async fn sync_from_remote(&mut self, url: &str) -> InitResult {
        info!("Syncing configuration from {}", url);

        // Fetch configuration from remote endpoint
        let response = reqwest::get(url).await?;

        if !response.status().is_success() {
            return Err(
                format!("Remote config endpoint returned status: {}", response.status()).into()
            );
        }

        let remote_config: RemoteConfig = response.json().await?;

        // Validate version compatibility
        if !self.is_version_compatible(&remote_config.version) {
            return Err(format!(
                "Incompatible config version: {} (current: {})",
                remote_config.version, self.config.version
            )
            .into());
        }

        // Apply the remote configuration
        self.config = remote_config;
        self.config.sync_url = Some(url.to_string());
        self.config.last_sync = Some(chrono::Utc::now());

        info!("Successfully synced configuration from remote. Version: {}", self.config.version);
        Ok(())
    }

    /// Check if a config version is compatible
    fn is_version_compatible(&self, version: &str) -> bool {
        // Simple major version check (e.g., "1.x.x" is compatible with "1.y.z")
        let current_major = self.config.version.split('.').next().unwrap_or("0");
        let remote_major = version.split('.').next().unwrap_or("0");
        current_major == remote_major
    }

    pub fn load_from_file(&mut self, path: &str) -> InitResult {
        let content = fs::read_to_string(path)?;
        self.config = serde_json::from_str(&content)?;

        info!("Loaded configuration from {}", path);
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.local_overrides.get(key).or_else(|| self.config.settings.get(key)).cloned()
    }

    pub fn set_override(&mut self, key: String, value: serde_json::Value) {
        self.local_overrides.insert(key.clone(), value.clone());
        info!("Set local override for '{}': {:?}", key, value);
    }

    pub fn get_environment(&self) -> &str {
        &self.config.environment
    }

    pub fn get_version(&self) -> &str {
        &self.config.version
    }

    pub fn clear_overrides(&mut self) {
        self.local_overrides.clear();
        info!("Cleared all local overrides");
    }

    pub fn get_all_settings(&self) -> HashMap<String, serde_json::Value> {
        let mut merged = self.config.settings.clone();
        for (key, value) in &self.local_overrides {
            merged.insert(key.clone(), value.clone());
        }
        merged
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for compliance::config.
    use tempfile::TempDir;

    use super::*;

    /// Type alias for test results to reduce complexity
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// Tests that `ConfigManager::new()` creates a manager with default
    /// configuration.
    #[test]
    fn test_config_manager_new() {
        let manager = ConfigManager::new();
        assert_eq!(manager.get_version(), "1.0.0");
        assert_eq!(manager.get_environment(), "production");
        assert!(manager.config.settings.is_empty());
        assert!(manager.local_overrides.is_empty());
    }

    /// Tests that version compatibility checks accept matching major versions.
    #[test]
    fn test_version_compatibility_same_major() {
        let manager = ConfigManager::new();
        assert!(manager.is_version_compatible("1.0.0"));
        assert!(manager.is_version_compatible("1.5.0"));
        assert!(manager.is_version_compatible("1.99.99"));
    }

    /// Tests that version compatibility checks reject different major versions.
    #[test]
    fn test_version_compatibility_different_major() {
        let manager = ConfigManager::new();
        assert!(!manager.is_version_compatible("2.0.0"));
        assert!(!manager.is_version_compatible("0.9.0"));
        assert!(!manager.is_version_compatible("3.1.0"));
    }

    /// Tests that version compatibility handles malformed version strings
    /// gracefully.
    #[test]
    fn test_version_compatibility_malformed() {
        let manager = ConfigManager::new();
        // Missing parts should still compare major version
        assert!(manager.is_version_compatible("1"));
        assert!(manager.is_version_compatible("1.0"));
        assert!(!manager.is_version_compatible("2"));
    }

    /// Tests that empty version strings default to "0" and are incompatible
    /// with "1".
    #[test]
    fn test_version_compatibility_empty() {
        let manager = ConfigManager::new();
        // Empty version should default to "0" which doesn't match "1"
        assert!(!manager.is_version_compatible(""));
    }

    /// Tests that `set_override()` and `get()` work together for local
    /// overrides.
    #[test]
    fn test_get_set_override() {
        let mut manager = ConfigManager::new();

        // Set an override
        manager.set_override("test_key".to_string(), serde_json::json!("test_value"));

        // Get the override
        let value = manager.get("test_key");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), serde_json::json!("test_value"));
    }

    /// Tests that `get()` retrieves values from config settings.
    #[test]
    fn test_get_from_config_settings() {
        let mut manager = ConfigManager::new();

        // Add a value to config settings (not an override)
        manager.config.settings.insert("config_key".to_string(), serde_json::json!("config_value"));

        let value = manager.get("config_key");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), serde_json::json!("config_value"));
    }

    /// Tests that local overrides take precedence over config settings.
    #[test]
    fn test_override_takes_precedence() {
        let mut manager = ConfigManager::new();

        // Add value to config
        manager.config.settings.insert("key".to_string(), serde_json::json!("original"));

        // Add override
        manager.set_override("key".to_string(), serde_json::json!("overridden"));

        // Override should take precedence
        let value = manager.get("key");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), serde_json::json!("overridden"));
    }

    /// Tests that `get()` returns None for nonexistent keys.
    #[test]
    fn test_get_nonexistent_key() {
        let manager = ConfigManager::new();
        let value = manager.get("nonexistent");
        assert!(value.is_none());
    }

    /// Tests that `clear_overrides()` removes all local overrides.
    #[test]
    fn test_clear_overrides() {
        let mut manager = ConfigManager::new();

        // Add some overrides
        manager.set_override("key1".to_string(), serde_json::json!("value1"));
        manager.set_override("key2".to_string(), serde_json::json!("value2"));

        assert_eq!(manager.local_overrides.len(), 2);

        // Clear overrides
        manager.clear_overrides();

        assert_eq!(manager.local_overrides.len(), 0);
        assert!(manager.get("key1").is_none());
    }

    /// Tests that `clear_overrides()` does not affect base config settings.
    #[test]
    fn test_clear_overrides_preserves_config() {
        let mut manager = ConfigManager::new();

        // Add value to config
        manager.config.settings.insert("config_key".to_string(), serde_json::json!("config_value"));

        // Add override
        manager.set_override("override_key".to_string(), serde_json::json!("override_value"));

        // Clear overrides
        manager.clear_overrides();

        // Config value should still be available
        assert!(manager.get("config_key").is_some());
        // Override should be gone
        assert!(manager.get("override_key").is_none());
    }

    /// Tests that `get_all_settings()` merges config and overrides correctly.
    #[test]
    fn test_get_all_settings() {
        let mut manager = ConfigManager::new();

        // Add config settings
        manager.config.settings.insert("config1".to_string(), serde_json::json!("value1"));
        manager.config.settings.insert("config2".to_string(), serde_json::json!("value2"));

        // Add overrides
        manager.set_override("override1".to_string(), serde_json::json!("value3"));
        manager.set_override("config1".to_string(), serde_json::json!("overridden"));

        let all_settings = manager.get_all_settings();

        // Should have 3 keys total (config2, override1, and overridden config1)
        assert_eq!(all_settings.len(), 3);
        assert_eq!(all_settings.get("config1"), Some(&serde_json::json!("overridden")));
        assert_eq!(all_settings.get("config2"), Some(&serde_json::json!("value2")));
        assert_eq!(all_settings.get("override1"), Some(&serde_json::json!("value3")));
    }

    /// Tests that `load_from_file()` successfully loads configuration from JSON
    /// file.
    #[test]
    fn test_load_from_file() -> TestResult {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.json");

        // Create a test config file
        let test_config = RemoteConfig {
            version: "1.2.0".to_string(),
            environment: "staging".to_string(),
            settings: {
                let mut map = HashMap::new();
                map.insert("test_key".to_string(), serde_json::json!("test_value"));
                map
            },
            last_sync: None,
            sync_url: None,
        };

        let config_json = serde_json::to_string(&test_config)?;
        std::fs::write(&config_path, config_json)?;

        // Load the config
        let mut manager = ConfigManager::new();
        let result = manager.load_from_file(config_path.to_str().unwrap());

        assert!(result.is_ok());
        assert_eq!(manager.get_version(), "1.2.0");
        assert_eq!(manager.get_environment(), "staging");
        assert_eq!(manager.get("test_key"), Some(serde_json::json!("test_value")));

        Ok(())
    }

    /// Tests that `load_from_file()` returns an error for nonexistent files.
    #[test]
    fn test_load_from_nonexistent_file() {
        let mut manager = ConfigManager::new();
        let result = manager.load_from_file("/nonexistent/path/config.json");
        assert!(result.is_err());
    }

    /// Tests that `load_from_file()` returns an error for invalid JSON content.
    #[test]
    fn test_load_from_invalid_json() -> TestResult {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("invalid.json");

        // Write invalid JSON
        std::fs::write(&config_path, "{ invalid json }")?;

        let mut manager = ConfigManager::new();
        let result = manager.load_from_file(config_path.to_str().unwrap());

        assert!(result.is_err());

        Ok(())
    }

    /// Tests that `get_environment()` returns the current environment name.
    #[test]
    fn test_get_environment() {
        let mut manager = ConfigManager::new();
        assert_eq!(manager.get_environment(), "production");

        manager.config.environment = "development".to_string();
        assert_eq!(manager.get_environment(), "development");
    }

    /// Tests that `get_version()` returns the current config version.
    #[test]
    fn test_get_version() {
        let mut manager = ConfigManager::new();
        assert_eq!(manager.get_version(), "1.0.0");

        manager.config.version = "2.0.0".to_string();
        assert_eq!(manager.get_version(), "2.0.0");
    }

    /// Tests that `RemoteConfig` can be serialized to and deserialized from
    /// JSON.
    #[test]
    fn test_remote_config_serialization() {
        let config = RemoteConfig {
            version: "1.0.0".to_string(),
            environment: "test".to_string(),
            settings: HashMap::new(),
            last_sync: None,
            sync_url: Some("https://example.com/config".to_string()),
        };

        // Test serialization
        let json = serde_json::to_string(&config);
        assert!(json.is_ok());

        // Test deserialization
        let deserialized: Result<RemoteConfig, _> = serde_json::from_str(&json.unwrap_or_default());
        assert!(deserialized.is_ok());

        let deserialized_config = deserialized.unwrap();
        assert_eq!(deserialized_config.version, "1.0.0");
        assert_eq!(deserialized_config.environment, "test");
    }

    /// Tests that multiple overrides can be set and retrieved independently.
    #[test]
    fn test_multiple_overrides() {
        let mut manager = ConfigManager::new();

        // Set multiple overrides
        manager.set_override("key1".to_string(), serde_json::json!(1));
        manager.set_override("key2".to_string(), serde_json::json!(2));
        manager.set_override("key3".to_string(), serde_json::json!(3));

        assert_eq!(manager.get("key1"), Some(serde_json::json!(1)));
        assert_eq!(manager.get("key2"), Some(serde_json::json!(2)));
        assert_eq!(manager.get("key3"), Some(serde_json::json!(3)));
    }

    /// Tests that overrides can be updated by setting a new value for the same
    /// key.
    #[test]
    fn test_override_update() {
        let mut manager = ConfigManager::new();

        // Set initial value
        manager.set_override("key".to_string(), serde_json::json!("initial"));
        assert_eq!(manager.get("key"), Some(serde_json::json!("initial")));

        // Update the value
        manager.set_override("key".to_string(), serde_json::json!("updated"));
        assert_eq!(manager.get("key"), Some(serde_json::json!("updated")));
    }

    /// Tests that config values support complex JSON types (arrays, objects,
    /// null).
    #[test]
    fn test_config_with_complex_types() {
        let mut manager = ConfigManager::new();

        // Test with array
        manager.set_override("array".to_string(), serde_json::json!([1, 2, 3]));

        // Test with object
        manager.set_override("object".to_string(), serde_json::json!({"nested": "value"}));

        // Test with null
        manager.set_override("null".to_string(), serde_json::json!(null));

        assert_eq!(manager.get("array"), Some(serde_json::json!([1, 2, 3])));
        assert_eq!(manager.get("object"), Some(serde_json::json!({"nested": "value"})));
        assert_eq!(manager.get("null"), Some(serde_json::json!(null)));
    }

    /// Tests that loading config from file preserves existing local overrides.
    #[test]
    fn test_load_preserves_overrides() -> TestResult {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.json");

        // Create a test config file
        let test_config = RemoteConfig {
            version: "1.0.0".to_string(),
            environment: "test".to_string(),
            settings: HashMap::new(),
            last_sync: None,
            sync_url: None,
        };

        std::fs::write(&config_path, serde_json::to_string(&test_config)?)?;

        let mut manager = ConfigManager::new();

        // Set an override before loading
        manager.set_override("override_key".to_string(), serde_json::json!("override_value"));

        // Load config from file
        manager.load_from_file(config_path.to_str().unwrap())?;

        // Override should still exist
        assert_eq!(manager.get("override_key"), Some(serde_json::json!("override_value")));

        Ok(())
    }
}
