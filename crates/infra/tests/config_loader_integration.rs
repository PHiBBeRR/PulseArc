//! Integration tests for configuration loader
//!
//! Tests the end-to-end behavior of loading configuration from files.

use std::io::Write;

use pulsearc_infra::config;
use tempfile::NamedTempFile;

#[test]
fn test_load_config_from_json_file() {
    // Create a temporary JSON config file
    let json_content = r#"{
        "database": {
            "path": "/tmp/integration_test.db",
            "pool_size": 10,
            "encryption_key": "test-encryption-key-123"
        },
        "sync": {
            "interval_seconds": 30,
            "enabled": true
        },
        "tracking": {
            "snapshot_interval_seconds": 60,
            "idle_threshold_seconds": 300,
            "enabled": true
        }
    }"#;

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(json_content.as_bytes()).expect("Failed to write to temp file");

    let path = temp_file.path().with_extension("json");
    std::fs::copy(temp_file.path(), &path).expect("Failed to copy file");

    // Load configuration from the file
    let result = config::load_from_file(Some(path.clone()));
    assert!(result.is_ok(), "Failed to load config from JSON file");

    let config = result.unwrap();

    // Verify database configuration
    assert_eq!(config.database.path, "/tmp/integration_test.db");
    assert_eq!(config.database.pool_size, 10);
    assert_eq!(config.database.encryption_key, Some("test-encryption-key-123".to_string()));

    // Verify sync configuration
    assert_eq!(config.sync.interval_seconds, 30);
    assert!(config.sync.enabled);

    // Verify tracking configuration
    assert_eq!(config.tracking.snapshot_interval_seconds, 60);
    assert_eq!(config.tracking.idle_threshold_seconds, 300);
    assert!(config.tracking.enabled);

    // Cleanup
    std::fs::remove_file(path).ok();
}

#[test]
fn test_load_config_from_toml_file() {
    // Create a temporary TOML config file
    let toml_content = r#"
[database]
path = "/tmp/integration_test_toml.db"
pool_size = 8
encryption_key = "toml-key-456"

[sync]
interval_seconds = 20
enabled = false

[tracking]
snapshot_interval_seconds = 45
idle_threshold_seconds = 600
enabled = false
"#;

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(toml_content.as_bytes()).expect("Failed to write to temp file");

    let path = temp_file.path().with_extension("toml");
    std::fs::copy(temp_file.path(), &path).expect("Failed to copy file");

    // Load configuration from the file
    let result = config::load_from_file(Some(path.clone()));
    assert!(result.is_ok(), "Failed to load config from TOML file");

    let config = result.unwrap();

    // Verify database configuration
    assert_eq!(config.database.path, "/tmp/integration_test_toml.db");
    assert_eq!(config.database.pool_size, 8);
    assert_eq!(config.database.encryption_key, Some("toml-key-456".to_string()));

    // Verify sync configuration
    assert_eq!(config.sync.interval_seconds, 20);
    assert!(!config.sync.enabled);

    // Verify tracking configuration
    assert_eq!(config.tracking.snapshot_interval_seconds, 45);
    assert_eq!(config.tracking.idle_threshold_seconds, 600);
    assert!(!config.tracking.enabled);

    // Cleanup
    std::fs::remove_file(path).ok();
}

#[test]
fn test_load_config_with_minimal_fields() {
    // Create a config file with only required fields
    let json_content = r#"{
        "database": {
            "path": "minimal.db",
            "pool_size": 5
        },
        "sync": {
            "interval_seconds": 10,
            "enabled": true
        },
        "tracking": {
            "snapshot_interval_seconds": 30,
            "idle_threshold_seconds": 120,
            "enabled": true
        }
    }"#;

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(json_content.as_bytes()).expect("Failed to write to temp file");

    let path = temp_file.path().with_extension("json");
    std::fs::copy(temp_file.path(), &path).expect("Failed to copy file");

    // Load configuration from the file
    let result = config::load_from_file(Some(path.clone()));
    assert!(result.is_ok(), "Failed to load config with minimal fields");

    let config = result.unwrap();

    // Verify encryption_key is None when not provided
    assert_eq!(config.database.encryption_key, None);

    // Cleanup
    std::fs::remove_file(path).ok();
}

#[test]
fn test_load_config_from_nonexistent_file() {
    let result = config::load_from_file(Some("/nonexistent/path/config.json".into()));
    assert!(result.is_err(), "Should fail when file doesn't exist");

    match result {
        Err(pulsearc_domain::PulseArcError::Config(msg)) => {
            assert!(msg.contains("not found"), "Error message should mention 'not found'");
        }
        _ => panic!("Expected Config error"),
    }
}

#[test]
fn test_load_config_with_invalid_format() {
    // Create a file with invalid JSON
    let invalid_content = r#"{ "this is": "not valid" "#;

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(invalid_content.as_bytes()).expect("Failed to write to temp file");

    let path = temp_file.path().with_extension("json");
    std::fs::copy(temp_file.path(), &path).expect("Failed to copy file");

    // Attempt to load configuration
    let result = config::load_from_file(Some(path.clone()));
    assert!(result.is_err(), "Should fail with invalid JSON");

    match result {
        Err(pulsearc_domain::PulseArcError::Config(msg)) => {
            assert!(msg.contains("Invalid JSON"), "Error message should mention invalid JSON");
        }
        _ => panic!("Expected Config error"),
    }

    // Cleanup
    std::fs::remove_file(path).ok();
}
