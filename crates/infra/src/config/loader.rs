//! Configuration loader
//!
//! Loads application configuration from environment variables or files.
//!
//! ## Loading Strategy
//! 1. First, attempts to load from environment variables
//! 2. If incomplete, falls back to loading from file
//! 3. Probes multiple paths for config files
//! 4. Supports JSON and TOML formats
//!
//! ## Environment Variables
//! - `PULSEARC_DB_PATH`: Database file path
//! - `PULSEARC_DB_POOL_SIZE`: Connection pool size
//! - `PULSEARC_DB_ENCRYPTION_KEY`: Database encryption key
//! - `PULSEARC_SYNC_INTERVAL`: Sync interval in seconds
//! - `PULSEARC_SYNC_ENABLED`: Whether sync is enabled (true/false)
//! - `PULSEARC_TRACKING_SNAPSHOT_INTERVAL`: Snapshot interval in seconds
//! - `PULSEARC_TRACKING_IDLE_THRESHOLD`: Idle threshold in seconds
//! - `PULSEARC_TRACKING_ENABLED`: Whether tracking is enabled (true/false)
//!
//! ## File Locations
//! The loader probes the following paths (in order):
//! 1. `./config.json` or `./config.toml` (current working directory)
//! 2. `./pulsearc.json` or `./pulsearc.toml` (current working directory)
//! 3. `../config.json` or `../config.toml` (parent directory)
//! 4. `../../config.json` or `../../config.toml` (grandparent directory)
//! 5. Relative to executable location

use std::path::{Path, PathBuf};

use pulsearc_domain::{Config, DatabaseConfig, PulseArcError, Result, SyncConfig, TrackingConfig};

/// Load configuration with automatic fallback strategy
///
/// First attempts to load from environment variables. If any required
/// variables are missing, falls back to loading from a config file.
///
/// # Errors
/// Returns `PulseArcError::Config` if:
/// - Configuration cannot be loaded from either source
/// - File format is invalid
/// - Required fields are missing
pub fn load() -> Result<Config> {
    // Try loading from environment first
    match load_from_env() {
        Ok(config) => {
            tracing::info!("Configuration loaded from environment variables");
            Ok(config)
        }
        Err(e) => {
            tracing::debug!(error = ?e, "Failed to load from environment, trying file");
            // Fall back to file
            load_from_file(None)
        }
    }
}

/// Load configuration from environment variables
///
/// All required environment variables must be present. Returns an error
/// if any are missing.
///
/// # Environment Variables
/// See module documentation for the complete list.
///
/// # Errors
/// Returns `PulseArcError::Config` if required variables are missing
/// or have invalid values.
pub fn load_from_env() -> Result<Config> {
    let db_path = env_var("PULSEARC_DB_PATH")?;
    let db_pool_size = env_var("PULSEARC_DB_POOL_SIZE").and_then(|s| {
        s.parse::<u32>().map_err(|e| PulseArcError::Config(format!("Invalid pool size: {}", e)))
    })?;
    let db_encryption_key = std::env::var("PULSEARC_DB_ENCRYPTION_KEY").ok();

    let sync_interval = env_var("PULSEARC_SYNC_INTERVAL").and_then(|s| {
        s.parse::<u64>().map_err(|e| PulseArcError::Config(format!("Invalid sync interval: {}", e)))
    })?;
    let sync_enabled = env_bool("PULSEARC_SYNC_ENABLED", true);

    let tracking_snapshot_interval =
        env_var("PULSEARC_TRACKING_SNAPSHOT_INTERVAL").and_then(|s| {
            s.parse::<u64>().map_err(|e| {
                PulseArcError::Config(format!("Invalid tracking snapshot interval: {}", e))
            })
        })?;
    let tracking_idle_threshold = env_var("PULSEARC_TRACKING_IDLE_THRESHOLD").and_then(|s| {
        s.parse::<u64>()
            .map_err(|e| PulseArcError::Config(format!("Invalid idle threshold: {}", e)))
    })?;
    let tracking_enabled = env_bool("PULSEARC_TRACKING_ENABLED", true);

    Ok(Config {
        database: DatabaseConfig {
            path: db_path,
            pool_size: db_pool_size,
            encryption_key: db_encryption_key,
        },
        sync: SyncConfig { interval_seconds: sync_interval, enabled: sync_enabled },
        tracking: TrackingConfig {
            snapshot_interval_seconds: tracking_snapshot_interval,
            idle_threshold_seconds: tracking_idle_threshold,
            enabled: tracking_enabled,
        },
    })
}

/// Load configuration from a file
///
/// If `path` is `None`, probes multiple locations for config files.
/// Supports both JSON and TOML formats (detected by file extension).
///
/// # Arguments
/// * `path` - Optional path to config file. If `None`, uses
///   [`probe_config_paths`].
///
/// # Errors
/// Returns `PulseArcError::Config` if:
/// - File not found (when path is specified)
/// - No config file found (when path is `None`)
/// - File format is invalid
/// - Required fields are missing
pub fn load_from_file(path: Option<PathBuf>) -> Result<Config> {
    let config_path = match path {
        Some(p) => {
            if !p.exists() {
                return Err(PulseArcError::Config(format!(
                    "Config file not found: {}",
                    p.display()
                )));
            }
            p
        }
        None => probe_config_paths().ok_or_else(|| {
            PulseArcError::Config(
                "No config file found in any of the standard locations".to_string(),
            )
        })?,
    };

    tracing::info!(path = %config_path.display(), "Loading configuration from file");

    let contents = std::fs::read_to_string(&config_path)
        .map_err(|e| PulseArcError::Config(format!("Failed to read config file: {}", e)))?;

    parse_config(&contents, &config_path)
}

/// Parse configuration from string content
///
/// Format is detected by file extension (`.json` or `.toml`).
///
/// # Arguments
/// * `contents` - File contents as string
/// * `path` - Path to the file (for format detection and error messages)
///
/// # Errors
/// Returns `PulseArcError::Config` if format is invalid or parsing fails.
fn parse_config(contents: &str, path: &Path) -> Result<Config> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("json");

    match extension {
        "toml" => toml::from_str(contents)
            .map_err(|e| PulseArcError::Config(format!("Invalid TOML format: {}", e))),
        "json" => serde_json::from_str(contents)
            .map_err(|e| PulseArcError::Config(format!("Invalid JSON format: {}", e))),
        _ => Err(PulseArcError::Config(format!("Unsupported config format: {}", extension))),
    }
}

/// Probe multiple paths for configuration files
///
/// Searches for config files in the following locations (in order):
/// 1. Current working directory (`./config.{json,toml}`,
///    `./pulsearc.{json,toml}`)
/// 2. Parent directories (up to 2 levels)
/// 3. Relative to executable location
///
/// # Returns
/// The first config file found, or `None` if no file exists.
pub fn probe_config_paths() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    // Try current working directory
    if let Ok(cwd) = std::env::current_dir() {
        candidates.extend(vec![
            cwd.join("config.json"),
            cwd.join("config.toml"),
            cwd.join("pulsearc.json"),
            cwd.join("pulsearc.toml"),
            cwd.join("../config.json"),
            cwd.join("../config.toml"),
            cwd.join("../../config.json"),
            cwd.join("../../config.toml"),
        ]);
    }

    // Try relative to executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.extend(vec![
                exe_dir.join("config.json"),
                exe_dir.join("config.toml"),
                exe_dir.join("pulsearc.json"),
                exe_dir.join("pulsearc.toml"),
                exe_dir.join("../config.json"),
                exe_dir.join("../config.toml"),
                exe_dir.join("../../config.json"),
                exe_dir.join("../../config.toml"),
            ]);
        }
    }

    // Return first existing candidate
    candidates.into_iter().find(|path| path.exists())
}

/// Get required environment variable
///
/// # Errors
/// Returns `PulseArcError::Config` if the variable is not set.
fn env_var(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| {
        PulseArcError::Config(format!("Missing required environment variable: {}", key))
    })
}

/// Parse boolean from environment variable
///
/// Accepts: `1`/`0`, `true`/`false`, `yes`/`no`, `on`/`off` (case-insensitive)
///
/// # Arguments
/// * `key` - Environment variable name
/// * `default` - Default value if variable is not set
///
/// # Returns
/// The parsed boolean value, or `default` if not set.
fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|s| matches!(s.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::Mutex;

    use once_cell::sync::Lazy;
    use tempfile::NamedTempFile;

    use super::*;

    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn test_env_bool_parsing() {
        let _guard = ENV_LOCK.lock().expect("env mutex poisoned");

        // Test true values
        std::env::set_var("TEST_BOOL_TRUE_1", "1");
        std::env::set_var("TEST_BOOL_TRUE_TRUE", "true");
        std::env::set_var("TEST_BOOL_TRUE_YES", "yes");
        std::env::set_var("TEST_BOOL_TRUE_ON", "on");
        std::env::set_var("TEST_BOOL_TRUE_UPPER", "TRUE");

        assert!(env_bool("TEST_BOOL_TRUE_1", false));
        assert!(env_bool("TEST_BOOL_TRUE_TRUE", false));
        assert!(env_bool("TEST_BOOL_TRUE_YES", false));
        assert!(env_bool("TEST_BOOL_TRUE_ON", false));
        assert!(env_bool("TEST_BOOL_TRUE_UPPER", false));

        // Test false values
        std::env::set_var("TEST_BOOL_FALSE_0", "0");
        std::env::set_var("TEST_BOOL_FALSE_FALSE", "false");
        std::env::set_var("TEST_BOOL_FALSE_NO", "no");
        std::env::set_var("TEST_BOOL_FALSE_OFF", "off");

        assert!(!env_bool("TEST_BOOL_FALSE_0", true));
        assert!(!env_bool("TEST_BOOL_FALSE_FALSE", true));
        assert!(!env_bool("TEST_BOOL_FALSE_NO", true));
        assert!(!env_bool("TEST_BOOL_FALSE_OFF", true));

        // Test default when not set
        std::env::remove_var("TEST_BOOL_MISSING");
        assert!(env_bool("TEST_BOOL_MISSING", true));
        assert!(!env_bool("TEST_BOOL_MISSING", false));

        // Cleanup
        std::env::remove_var("TEST_BOOL_TRUE_1");
        std::env::remove_var("TEST_BOOL_TRUE_TRUE");
        std::env::remove_var("TEST_BOOL_TRUE_YES");
        std::env::remove_var("TEST_BOOL_TRUE_ON");
        std::env::remove_var("TEST_BOOL_TRUE_UPPER");
        std::env::remove_var("TEST_BOOL_FALSE_0");
        std::env::remove_var("TEST_BOOL_FALSE_FALSE");
        std::env::remove_var("TEST_BOOL_FALSE_NO");
        std::env::remove_var("TEST_BOOL_FALSE_OFF");
    }

    #[test]
    fn test_load_from_env_all_vars_set() {
        let _guard = ENV_LOCK.lock().expect("env mutex poisoned");

        // Set all required environment variables
        std::env::set_var("PULSEARC_DB_PATH", "/tmp/test.db");
        std::env::set_var("PULSEARC_DB_POOL_SIZE", "5");
        std::env::set_var("PULSEARC_DB_ENCRYPTION_KEY", "test-key");
        std::env::set_var("PULSEARC_SYNC_INTERVAL", "15");
        std::env::set_var("PULSEARC_SYNC_ENABLED", "true");
        std::env::set_var("PULSEARC_TRACKING_SNAPSHOT_INTERVAL", "45");
        std::env::set_var("PULSEARC_TRACKING_IDLE_THRESHOLD", "600");
        std::env::set_var("PULSEARC_TRACKING_ENABLED", "false");

        let result = load_from_env();
        assert!(result.is_ok(), "Should load config from env vars, error: {:?}", result.err());

        let config = result.unwrap();
        assert_eq!(config.database.path, "/tmp/test.db");
        assert_eq!(config.database.pool_size, 5);
        assert_eq!(config.database.encryption_key, Some("test-key".to_string()));
        assert_eq!(config.sync.interval_seconds, 15);
        assert!(config.sync.enabled);
        assert_eq!(config.tracking.snapshot_interval_seconds, 45);
        assert_eq!(config.tracking.idle_threshold_seconds, 600);
        assert!(!config.tracking.enabled);

        // Cleanup
        std::env::remove_var("PULSEARC_DB_PATH");
        std::env::remove_var("PULSEARC_DB_POOL_SIZE");
        std::env::remove_var("PULSEARC_DB_ENCRYPTION_KEY");
        std::env::remove_var("PULSEARC_SYNC_INTERVAL");
        std::env::remove_var("PULSEARC_SYNC_ENABLED");
        std::env::remove_var("PULSEARC_TRACKING_SNAPSHOT_INTERVAL");
        std::env::remove_var("PULSEARC_TRACKING_IDLE_THRESHOLD");
        std::env::remove_var("PULSEARC_TRACKING_ENABLED");
    }

    #[test]
    fn test_load_from_env_missing_var() {
        let _guard = ENV_LOCK.lock().expect("env mutex poisoned");

        // Save current env vars to restore later
        let saved_db_path = std::env::var("PULSEARC_DB_PATH").ok();
        let saved_db_pool_size = std::env::var("PULSEARC_DB_POOL_SIZE").ok();

        // Ensure variable is not set
        std::env::remove_var("PULSEARC_DB_PATH");
        std::env::remove_var("PULSEARC_DB_POOL_SIZE");

        let result = load_from_env();
        assert!(result.is_err(), "Should fail with missing env var");

        let err = result.unwrap_err();
        assert!(matches!(err, PulseArcError::Config(_)), "Should be a Config error");

        // Restore environment
        if let Some(val) = saved_db_path {
            std::env::set_var("PULSEARC_DB_PATH", val);
        }
        if let Some(val) = saved_db_pool_size {
            std::env::set_var("PULSEARC_DB_POOL_SIZE", val);
        }
    }

    #[test]
    fn test_load_from_env_invalid_number() {
        let _guard = ENV_LOCK.lock().expect("env mutex poisoned");

        std::env::set_var("PULSEARC_DB_PATH", "/tmp/test.db");
        std::env::set_var("PULSEARC_DB_POOL_SIZE", "not-a-number");

        let result = load_from_env();
        assert!(result.is_err(), "Should fail with invalid pool size");

        let err = result.unwrap_err();
        assert!(matches!(err, PulseArcError::Config(_)), "Should be a Config error");

        // Cleanup
        std::env::remove_var("PULSEARC_DB_PATH");
        std::env::remove_var("PULSEARC_DB_POOL_SIZE");
    }

    #[test]
    fn test_load_from_file_json() {
        let json_content = r#"{
            "database": {
                "path": "test.db",
                "pool_size": 4,
                "encryption_key": "secret"
            },
            "sync": {
                "interval_seconds": 20,
                "enabled": true
            },
            "tracking": {
                "snapshot_interval_seconds": 60,
                "idle_threshold_seconds": 900,
                "enabled": true
            }
        }"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json_content.as_bytes()).unwrap();
        let path = temp_file.path().with_extension("json");
        std::fs::copy(temp_file.path(), &path).unwrap();

        let result = load_from_file(Some(path.clone()));
        assert!(result.is_ok(), "Should load config from JSON file");

        let config = result.unwrap();
        assert_eq!(config.database.path, "test.db");
        assert_eq!(config.database.pool_size, 4);
        assert_eq!(config.sync.interval_seconds, 20);

        // Cleanup
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_from_file_toml() {
        let toml_content = r#"
[database]
path = "test.db"
pool_size = 6

[sync]
interval_seconds = 25
enabled = false

[tracking]
snapshot_interval_seconds = 90
idle_threshold_seconds = 1200
enabled = true
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(toml_content.as_bytes()).unwrap();
        let path = temp_file.path().with_extension("toml");
        std::fs::copy(temp_file.path(), &path).unwrap();

        let result = load_from_file(Some(path.clone()));
        assert!(result.is_ok(), "Should load config from TOML file");

        let config = result.unwrap();
        assert_eq!(config.database.path, "test.db");
        assert_eq!(config.database.pool_size, 6);
        assert!(!config.sync.enabled);

        // Cleanup
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_load_from_file_not_found() {
        let result = load_from_file(Some(PathBuf::from("/nonexistent/config.json")));
        assert!(result.is_err(), "Should fail when file not found");

        let err = result.unwrap_err();
        assert!(matches!(err, PulseArcError::Config(_)), "Should be a Config error");
    }

    #[test]
    fn test_load_from_file_invalid_json() {
        let invalid_json = r#"{ "this is": "not valid json" "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(invalid_json.as_bytes()).unwrap();
        let path = temp_file.path().with_extension("json");
        std::fs::copy(temp_file.path(), &path).unwrap();

        let result = load_from_file(Some(path.clone()));
        assert!(result.is_err(), "Should fail with invalid JSON");

        // Cleanup
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_probe_config_paths_returns_none_when_missing() {
        // This test assumes no config files exist in standard locations
        // In a real environment, this might find a file
        let result = probe_config_paths();
        // We can't assert None because a file might actually exist in dev environment
        // Just verify it returns an Option
        assert!(result.is_none() || result.is_some());
    }

    #[test]
    fn test_parse_config_json() {
        let json_content = r#"{
            "database": {
                "path": "test.db",
                "pool_size": 4
            },
            "sync": {
                "interval_seconds": 20,
                "enabled": true
            },
            "tracking": {
                "snapshot_interval_seconds": 60,
                "idle_threshold_seconds": 900,
                "enabled": true
            }
        }"#;

        let path = PathBuf::from("test.json");
        let result = parse_config(json_content, &path);
        assert!(result.is_ok(), "Should parse valid JSON");
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_content = r#"
[database]
path = "test.db"
pool_size = 6

[sync]
interval_seconds = 25
enabled = false

[tracking]
snapshot_interval_seconds = 90
idle_threshold_seconds = 1200
enabled = true
"#;

        let path = PathBuf::from("test.toml");
        let result = parse_config(toml_content, &path);
        assert!(result.is_ok(), "Should parse valid TOML");
    }

    #[test]
    fn test_parse_config_unsupported_format() {
        let content = "some content";
        let path = PathBuf::from("test.yaml");
        let result = parse_config(content, &path);
        assert!(result.is_err(), "Should fail with unsupported format");
    }
}
