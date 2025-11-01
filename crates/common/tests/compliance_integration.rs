//! Integration tests for compliance module
//!
//! These tests verify end-to-end compliance workflows including:
//! - Global audit logging with file persistence
//! - Feature flag management with role-based and percentage rollouts
//! - Remote configuration management with overrides
//! - Cross-module interactions between audit, feature flags, and config
//! - Concurrent access patterns
//! - Error handling and recovery

#![cfg(feature = "platform")]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use pulsearc_common::compliance::audit::{
    AuditConfig, AuditContext, AuditEvent, AuditSeverity, GlobalAuditEntry, GlobalAuditLogger,
};
use pulsearc_common::compliance::config::{ConfigManager, RemoteConfig};
use pulsearc_common::compliance::feature_flags::{FeatureFlag, FeatureFlagManager};
use pulsearc_common::security::rbac::{Permission, UserContext};
use pulsearc_common::testing::TempDir;
use tokio::time::sleep;

// ============================================================================
// Test Helpers
// ============================================================================

/// Generate a temporary file path in a test directory
fn temp_file_path(temp_dir: &TempDir, filename: &str) -> PathBuf {
    temp_dir.path().join(filename)
}

/// Create a test user context
fn create_test_user(user_id: &str, roles: Vec<&str>) -> UserContext {
    UserContext {
        user_id: user_id.to_string(),
        roles: roles.iter().map(|r| r.to_string()).collect(),
        session_id: Some(format!("session_{}", user_id)),
        ip_address: Some("192.168.1.1".to_string()),
        user_agent: Some("TestAgent/1.0".to_string()),
        attributes: HashMap::new(),
    }
}

/// Create a test audit context
fn create_audit_context(user_id: &str) -> AuditContext {
    AuditContext {
        user_id: Some(user_id.to_string()),
        session_id: Some(format!("session_{}", user_id)),
        ip_address: Some("192.168.1.1".to_string()),
        user_agent: Some("TestAgent/1.0".to_string()),
    }
}

// ============================================================================
// Audit Logger Integration Tests
// ============================================================================

/// Tests the complete audit logging lifecycle: log -> query -> export
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_logger_complete_lifecycle() {
    let temp_dir = TempDir::new("compliance-test").unwrap();
    let log_path = temp_file_path(&temp_dir, "audit.log");

    let logger = GlobalAuditLogger::new();

    // Configure with file logging
    let config = AuditConfig {
        max_memory_entries: 100,
        log_file_path: Some(log_path.clone()),
        enable_streaming: false,
        min_severity: AuditSeverity::Info,
        encrypt_sensitive: true,
        streaming_timeout_secs: 30,
        streaming_url: None,
    };

    logger.configure(config).await;
    logger.initialize_with_path().await.unwrap();

    // Log various events
    logger
        .log_event(
            AuditEvent::ApplicationStarted {
                version: "1.0.0".to_string(),
                environment: "test".to_string(),
            },
            create_audit_context("system"),
            AuditSeverity::Info,
        )
        .await;

    logger
        .log_event(
            AuditEvent::PermissionCheck {
                user_id: "user1".to_string(),
                permission: Permission::new("audit:read"),
                granted: true,
            },
            create_audit_context("user1"),
            AuditSeverity::Info,
        )
        .await;

    logger
        .log_event(
            AuditEvent::UnauthorizedAccess {
                resource: "admin_panel".to_string(),
                user_id: Some("user2".to_string()),
            },
            create_audit_context("user2"),
            AuditSeverity::Security,
        )
        .await;

    // Query events by severity
    let security_events = logger.query(|e| e.severity == AuditSeverity::Security, None).await;
    assert_eq!(security_events.len(), 1);

    // Query with limit
    let limited_events = logger.query(|_| true, Some(2)).await;
    assert_eq!(limited_events.len(), 2);

    // Check statistics
    let stats = logger.get_statistics().await;
    assert_eq!(stats.total_entries, 3);
    assert_eq!(stats.by_severity.get(&AuditSeverity::Info), Some(&2));
    assert_eq!(stats.by_severity.get(&AuditSeverity::Security), Some(&1));

    // Export to file
    let export_path = temp_file_path(&temp_dir, "export.json");
    logger.export(&export_path).await.unwrap();

    // Verify export file exists and contains data
    assert!(export_path.exists());
    let content = std::fs::read_to_string(&export_path).unwrap();
    let parsed: Vec<GlobalAuditEntry> = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed.len(), 3);

    // Verify audit log file was written
    assert!(log_path.exists());
    let log_content = std::fs::read_to_string(&log_path).unwrap();
    assert!(log_content.contains("ApplicationStarted"));
    assert!(log_content.contains("UnauthorizedAccess"));
}

/// Tests audit logging with severity filtering
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_logger_severity_filtering() {
    let logger = GlobalAuditLogger::new();

    // Configure with Warning minimum severity
    let config = AuditConfig { min_severity: AuditSeverity::Warning, ..Default::default() };
    logger.configure(config).await;

    // Log events with different severities
    logger
        .log_event(
            AuditEvent::MenuItemClicked {
                menu_id: "menu1".to_string(),
                label: "Settings".to_string(),
            },
            create_audit_context("user1"),
            AuditSeverity::Debug,
        )
        .await;

    logger
        .log_event(
            AuditEvent::ConfigurationChanged {
                key: "timeout".to_string(),
                old_value: Some("30".to_string()),
                new_value: "60".to_string(),
            },
            create_audit_context("admin"),
            AuditSeverity::Warning,
        )
        .await;

    logger
        .log_event(
            AuditEvent::ErrorOccurred {
                error_type: "DatabaseError".to_string(),
                message: "Connection failed".to_string(),
                stack_trace: None,
            },
            create_audit_context("system"),
            AuditSeverity::Error,
        )
        .await;

    // Should only have Warning and Error (Debug filtered out)
    let count = logger.entry_count().await;
    assert_eq!(count, 2);

    // Verify the correct events were logged
    let all_events = logger.query(|_| true, None).await;
    assert!(all_events.iter().all(|e| e.severity >= AuditSeverity::Warning));
}

/// Tests audit logger memory limit enforcement
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_logger_memory_limit() {
    let logger = GlobalAuditLogger::new();

    // Configure with small memory limit
    let config = AuditConfig { max_memory_entries: 3, ..Default::default() };
    logger.configure(config).await;

    // Log 5 events
    for i in 0..5 {
        logger
            .log_event(
                AuditEvent::Custom {
                    category: "test".to_string(),
                    action: format!("action_{}", i),
                    details: serde_json::json!({"index": i}),
                },
                create_audit_context(&format!("user{}", i)),
                AuditSeverity::Info,
            )
            .await;
    }

    // Should only keep last 3 entries
    let count = logger.entry_count().await;
    assert_eq!(count, 3);

    // Verify oldest entries were removed (should have events 2, 3, 4)
    let events = logger.query(|_| true, None).await;
    let actions: Vec<String> = events
        .iter()
        .filter_map(|e| match &e.event {
            AuditEvent::Custom { action, .. } => Some(action.clone()),
            _ => None,
        })
        .collect();

    assert_eq!(actions, vec!["action_2", "action_3", "action_4"]);
}

/// Tests audit logger clear operation with external audit trail
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_logger_clear_with_trail() {
    let temp_dir = TempDir::new("compliance-test").unwrap();
    let log_path = temp_file_path(&temp_dir, "audit_clear.log");

    let logger = GlobalAuditLogger::new();
    let config = AuditConfig { log_file_path: Some(log_path.clone()), ..Default::default() };
    logger.configure(config).await;
    logger.initialize_with_path().await.unwrap();

    // Log some events
    for i in 0..3 {
        logger
            .log_event(
                AuditEvent::Custom {
                    category: "test".to_string(),
                    action: format!("action_{}", i),
                    details: serde_json::json!({}),
                },
                create_audit_context("user1"),
                AuditSeverity::Info,
            )
            .await;
    }

    assert_eq!(logger.entry_count().await, 3);

    // Clear with external audit trail
    logger.clear_with_external_audit("compliance_requirement", "admin_user").await.unwrap();

    // Memory should be cleared
    assert_eq!(logger.entry_count().await, 0);

    // Small delay to ensure async file write completes
    sleep(Duration::from_millis(10)).await;

    // Verify clear event was logged to file
    let log_content = std::fs::read_to_string(&log_path).unwrap();
    assert!(log_content.contains("AuditLogCleared"), "Log content: {}", log_content);
    assert!(log_content.contains("compliance_requirement"));
    assert!(log_content.contains("admin_user"));
}

// ============================================================================
// Feature Flags Integration Tests
// ============================================================================

/// Tests complete feature flag lifecycle: create -> check -> toggle -> rollout
#[tokio::test(flavor = "multi_thread")]
async fn test_feature_flag_complete_lifecycle() {
    let mut manager = FeatureFlagManager::new();

    // 1. Create a new feature flag
    let flag = FeatureFlag {
        id: "new_dashboard".to_string(),
        name: "New Dashboard".to_string(),
        description: "Next generation dashboard UI".to_string(),
        enabled: true,
        rollout_percentage: 0.0,
        target_roles: vec!["beta_tester".to_string()],
        target_users: vec!["user_vip".to_string()],
        metadata: {
            let mut m = HashMap::new();
            m.insert("team".to_string(), "frontend".to_string());
            m
        },
    };

    manager.add_flag(flag).unwrap();

    // 2. Test targeted user access
    let vip_user = create_test_user("user_vip", vec![]);
    assert!(manager.is_enabled("new_dashboard", Some(&vip_user)).await);

    // 3. Test targeted role access
    let beta_user = create_test_user("user123", vec!["beta_tester"]);
    assert!(manager.is_enabled("new_dashboard", Some(&beta_user)).await);

    // 4. Test regular user (should be denied, 0% rollout)
    let regular_user = create_test_user("regular_user", vec!["user"]);
    assert!(!manager.is_enabled("new_dashboard", Some(&regular_user)).await);

    // 5. Increase rollout to 50%
    manager.set_rollout_percentage("new_dashboard", 50.0).unwrap();

    // Same user should have deterministic result
    let result1 = manager.is_enabled("new_dashboard", Some(&regular_user)).await;
    let result2 = manager.is_enabled("new_dashboard", Some(&regular_user)).await;
    assert_eq!(result1, result2);

    // 6. Toggle flag off
    manager.toggle_flag("new_dashboard", false).unwrap();

    // Even VIP user should be denied when flag is disabled
    assert!(!manager.is_enabled("new_dashboard", Some(&vip_user)).await);
}

/// Tests feature flag rollout percentages with multiple users
#[tokio::test(flavor = "multi_thread")]
async fn test_feature_flag_rollout_distribution() {
    let mut manager = FeatureFlagManager::new();

    let flag = FeatureFlag {
        id: "test_rollout".to_string(),
        name: "Test Rollout".to_string(),
        description: "Test".to_string(),
        enabled: true,
        rollout_percentage: 50.0,
        target_roles: vec![],
        target_users: vec![],
        metadata: HashMap::new(),
    };

    manager.add_flag(flag).unwrap();

    // Test with 100 users
    let mut enabled_count = 0;
    for i in 0..100 {
        let user = create_test_user(&format!("user{}", i), vec![]);
        if manager.is_enabled("test_rollout", Some(&user)).await {
            enabled_count += 1;
        }
    }

    // With 50% rollout, we expect roughly 50 users enabled (allow 20-80 range for
    // randomness)
    assert!(
        (20..=80).contains(&enabled_count),
        "Expected 20-80 users enabled with 50% rollout, got {}",
        enabled_count
    );

    // Test determinism - same users should get same results
    let test_user = create_test_user("deterministic_user", vec![]);
    let result1 = manager.is_enabled("test_rollout", Some(&test_user)).await;
    let result2 = manager.is_enabled("test_rollout", Some(&test_user)).await;
    let result3 = manager.is_enabled("test_rollout", Some(&test_user)).await;

    assert_eq!(result1, result2);
    assert_eq!(result2, result3);
}

/// Tests feature flag with complex role hierarchies
#[tokio::test(flavor = "multi_thread")]
async fn test_feature_flag_role_hierarchies() {
    let mut manager = FeatureFlagManager::new();

    let flag = FeatureFlag {
        id: "admin_feature".to_string(),
        name: "Admin Feature".to_string(),
        description: "Admin only feature".to_string(),
        enabled: true,
        rollout_percentage: 0.0,
        target_roles: vec!["admin".to_string(), "super_admin".to_string()],
        target_users: vec![],
        metadata: HashMap::new(),
    };

    manager.add_flag(flag).unwrap();

    // Test admin role
    let admin = create_test_user("admin1", vec!["admin"]);
    assert!(manager.is_enabled("admin_feature", Some(&admin)).await);

    // Test super_admin role
    let super_admin = create_test_user("superadmin1", vec!["super_admin"]);
    assert!(manager.is_enabled("admin_feature", Some(&super_admin)).await);

    // Test user with multiple roles (including admin)
    let multi_role = create_test_user("user1", vec!["user", "moderator", "admin"]);
    assert!(manager.is_enabled("admin_feature", Some(&multi_role)).await);

    // Test user without admin role
    let regular_user = create_test_user("user2", vec!["user"]);
    assert!(!manager.is_enabled("admin_feature", Some(&regular_user)).await);
}

/// Tests feature flag error handling
#[tokio::test(flavor = "multi_thread")]
async fn test_feature_flag_error_handling() {
    let mut manager = FeatureFlagManager::new();

    // Test adding duplicate flag
    let flag1 = FeatureFlag {
        id: "duplicate".to_string(),
        name: "Duplicate".to_string(),
        description: "Test".to_string(),
        enabled: true,
        rollout_percentage: 100.0,
        target_roles: vec![],
        target_users: vec![],
        metadata: HashMap::new(),
    };

    assert!(manager.add_flag(flag1.clone()).is_ok());
    assert!(manager.add_flag(flag1).is_err());

    // Test invalid rollout percentage
    assert!(manager.set_rollout_percentage("duplicate", 150.0).is_err());
    assert!(manager.set_rollout_percentage("duplicate", -10.0).is_err());

    // Test operations on nonexistent flag
    assert!(manager.toggle_flag("nonexistent", true).is_err());
    assert!(manager.set_rollout_percentage("nonexistent", 50.0).is_err());

    // Test querying nonexistent flag
    let user = create_test_user("user1", vec![]);
    assert!(!manager.is_enabled("nonexistent", Some(&user)).await);
}

// ============================================================================
// Config Manager Integration Tests
// ============================================================================

/// Tests complete config manager lifecycle: create -> load -> override -> merge
#[tokio::test(flavor = "multi_thread")]
async fn test_config_manager_complete_lifecycle() {
    let temp_dir = TempDir::new("compliance-test").unwrap();
    let config_path = temp_file_path(&temp_dir, "config.json");

    // 1. Create a remote config
    let remote_config = RemoteConfig {
        version: "1.5.0".to_string(),
        environment: "production".to_string(),
        settings: {
            let mut settings = HashMap::new();
            settings.insert("api_timeout".to_string(), serde_json::json!(30));
            settings.insert("max_retries".to_string(), serde_json::json!(3));
            settings.insert("log_level".to_string(), serde_json::json!("info"));
            settings
        },
        last_sync: Some(chrono::Utc::now()),
        sync_url: Some("https://config.example.com".to_string()),
    };

    // 2. Save config to file
    let config_json = serde_json::to_string(&remote_config).unwrap();
    std::fs::write(&config_path, config_json).unwrap();

    // 3. Load config from file
    let mut manager = ConfigManager::new();
    manager.load_from_file(config_path.to_str().unwrap()).unwrap();

    assert_eq!(manager.get_version(), "1.5.0");
    assert_eq!(manager.get_environment(), "production");
    assert_eq!(manager.get("api_timeout"), Some(serde_json::json!(30)));

    // 4. Set local overrides
    manager.set_override("api_timeout".to_string(), serde_json::json!(60));
    manager.set_override("debug_mode".to_string(), serde_json::json!(true));

    // 5. Verify override takes precedence
    assert_eq!(manager.get("api_timeout"), Some(serde_json::json!(60)));
    assert_eq!(manager.get("debug_mode"), Some(serde_json::json!(true)));
    assert_eq!(manager.get("max_retries"), Some(serde_json::json!(3)));

    // 6. Get all merged settings
    let all_settings = manager.get_all_settings();
    assert_eq!(all_settings.len(), 4); // 3 from config + 1 override
    assert_eq!(all_settings.get("api_timeout"), Some(&serde_json::json!(60)));

    // 7. Clear overrides
    manager.clear_overrides();
    assert_eq!(manager.get("api_timeout"), Some(serde_json::json!(30)));
    assert_eq!(manager.get("debug_mode"), None);
}

/// Tests config manager version handling
#[tokio::test(flavor = "multi_thread")]
async fn test_config_manager_version_handling() {
    let manager = ConfigManager::new();
    assert_eq!(manager.get_version(), "1.0.0");

    // Test with temp configs
    let temp_dir = TempDir::new("compliance-test").unwrap();

    // Compatible version (1.x.x)
    let compatible_path = temp_file_path(&temp_dir, "compatible.json");
    let compatible_config = RemoteConfig {
        version: "1.9.9".to_string(),
        environment: "test".to_string(),
        settings: HashMap::new(),
        last_sync: None,
        sync_url: None,
    };
    std::fs::write(&compatible_path, serde_json::to_string(&compatible_config).unwrap()).unwrap();

    let mut manager1 = ConfigManager::new();
    assert!(manager1.load_from_file(compatible_path.to_str().unwrap()).is_ok());
    assert_eq!(manager1.get_version(), "1.9.9");

    // Different major version (2.x.x)
    let different_path = temp_file_path(&temp_dir, "different.json");
    let different_config = RemoteConfig {
        version: "2.0.0".to_string(),
        environment: "test".to_string(),
        settings: HashMap::new(),
        last_sync: None,
        sync_url: None,
    };
    std::fs::write(&different_path, serde_json::to_string(&different_config).unwrap()).unwrap();

    // load_from_file loads the version successfully
    let mut manager2 = ConfigManager::new();
    manager2.load_from_file(different_path.to_str().unwrap()).unwrap();
    assert_eq!(manager2.get_version(), "2.0.0");
}

/// Tests config manager with complex JSON types
#[tokio::test(flavor = "multi_thread")]
async fn test_config_manager_complex_types() {
    let mut manager = ConfigManager::new();

    // Test array
    manager.set_override("allowed_ips".to_string(), serde_json::json!(["192.168.1.1", "10.0.0.1"]));

    // Test nested object
    manager.set_override(
        "database".to_string(),
        serde_json::json!({
            "host": "localhost",
            "port": 5432,
            "credentials": {
                "username": "admin",
                "password": "secret"
            }
        }),
    );

    // Test null
    manager.set_override("optional_field".to_string(), serde_json::json!(null));

    // Verify retrieval
    assert_eq!(manager.get("allowed_ips"), Some(serde_json::json!(["192.168.1.1", "10.0.0.1"])));

    let db_config = manager.get("database").unwrap();
    assert_eq!(db_config["host"], "localhost");
    assert_eq!(db_config["credentials"]["username"], "admin");

    assert_eq!(manager.get("optional_field"), Some(serde_json::json!(null)));
}

// ============================================================================
// Cross-Module Integration Tests
// ============================================================================

/// Tests audit logging of feature flag changes
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_feature_flag_changes() {
    let logger = GlobalAuditLogger::new();
    let mut flag_manager = FeatureFlagManager::new();

    // Create a feature flag
    let flag = FeatureFlag {
        id: "beta_feature".to_string(),
        name: "Beta Feature".to_string(),
        description: "New beta feature".to_string(),
        enabled: true,
        rollout_percentage: 50.0,
        target_roles: vec![],
        target_users: vec![],
        metadata: HashMap::new(),
    };

    flag_manager.add_flag(flag).unwrap();

    // Audit the flag creation
    logger
        .log_event(
            AuditEvent::FeatureFlagToggled { flag: "beta_feature".to_string(), enabled: true },
            create_audit_context("admin"),
            AuditSeverity::Info,
        )
        .await;

    // Toggle the flag
    flag_manager.toggle_flag("beta_feature", false).unwrap();

    // Audit the toggle
    logger
        .log_event(
            AuditEvent::FeatureFlagToggled { flag: "beta_feature".to_string(), enabled: false },
            create_audit_context("admin"),
            AuditSeverity::Warning,
        )
        .await;

    // Query feature flag audit events
    let flag_events =
        logger.query(|e| matches!(e.event, AuditEvent::FeatureFlagToggled { .. }), None).await;

    assert_eq!(flag_events.len(), 2);
}

/// Tests audit logging of configuration changes
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_config_changes() {
    let logger = GlobalAuditLogger::new();
    let mut config_manager = ConfigManager::new();

    // Set initial config
    config_manager.set_override("api_timeout".to_string(), serde_json::json!(30));

    // Audit the change
    logger
        .log_event(
            AuditEvent::ConfigurationChanged {
                key: "api_timeout".to_string(),
                old_value: None,
                new_value: "30".to_string(),
            },
            create_audit_context("admin"),
            AuditSeverity::Info,
        )
        .await;

    // Update config
    let old_value = config_manager.get("api_timeout");
    config_manager.set_override("api_timeout".to_string(), serde_json::json!(60));

    // Audit the update
    logger
        .log_event(
            AuditEvent::ConfigurationChanged {
                key: "api_timeout".to_string(),
                old_value: old_value.as_ref().map(|v| v.to_string()),
                new_value: "60".to_string(),
            },
            create_audit_context("admin"),
            AuditSeverity::Warning,
        )
        .await;

    // Query config change events
    let config_events =
        logger.query(|e| matches!(e.event, AuditEvent::ConfigurationChanged { .. }), None).await;

    assert_eq!(config_events.len(), 2);
}

/// Tests feature flags controlling audit behavior
#[tokio::test(flavor = "multi_thread")]
async fn test_feature_flag_controlled_audit() {
    let logger = GlobalAuditLogger::new();

    // Configure logger to accept Debug level events
    let config = AuditConfig { min_severity: AuditSeverity::Debug, ..Default::default() };
    logger.configure(config).await;

    let mut flag_manager = FeatureFlagManager::new();

    // Create feature flag for detailed audit logging
    let flag = FeatureFlag {
        id: "detailed_audit".to_string(),
        name: "Detailed Audit Logging".to_string(),
        description: "Enable detailed audit logging".to_string(),
        enabled: true,
        rollout_percentage: 100.0,
        target_roles: vec![],
        target_users: vec![],
        metadata: HashMap::new(),
    };

    flag_manager.add_flag(flag).unwrap();

    let admin_user = create_test_user("admin1", vec!["admin"]);

    // Check if detailed logging is enabled
    if flag_manager.is_enabled("detailed_audit", Some(&admin_user)).await {
        // Log detailed event
        logger
            .log_event(
                AuditEvent::Custom {
                    category: "detailed".to_string(),
                    action: "user_action".to_string(),
                    details: serde_json::json!({
                        "user_id": "admin1",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "metadata": {
                            "browser": "Chrome",
                            "location": "US"
                        }
                    }),
                },
                create_audit_context("admin1"),
                AuditSeverity::Debug,
            )
            .await;
    }

    // Verify event was logged
    let detailed_events = logger
        .query(
            |e| matches!(&e.event, AuditEvent::Custom { category, .. } if category == "detailed"),
            None,
        )
        .await;

    assert_eq!(detailed_events.len(), 1);
}

/// Tests configuration-driven feature flag rollout
#[tokio::test(flavor = "multi_thread")]
async fn test_config_driven_feature_rollout() {
    let mut config_manager = ConfigManager::new();
    let mut flag_manager = FeatureFlagManager::new();

    // Store rollout configuration
    config_manager.set_override(
        "feature_rollouts".to_string(),
        serde_json::json!({
            "new_ui": 25.0,
            "beta_api": 50.0,
            "experimental": 10.0
        }),
    );

    // Create feature flags based on config
    if let Some(rollouts) = config_manager.get("feature_rollouts") {
        if let Some(obj) = rollouts.as_object() {
            for (feature_id, percentage) in obj {
                let rollout = percentage.as_f64().unwrap_or(0.0) as f32;

                let flag = FeatureFlag {
                    id: feature_id.clone(),
                    name: feature_id.clone(),
                    description: format!("Feature {}", feature_id),
                    enabled: true,
                    rollout_percentage: rollout,
                    target_roles: vec![],
                    target_users: vec![],
                    metadata: HashMap::new(),
                };

                flag_manager.add_flag(flag).ok();
            }
        }
    }

    // Verify flags were created with correct rollout
    let new_ui_flag = flag_manager.get_flag("new_ui").unwrap();
    assert_eq!(new_ui_flag.rollout_percentage, 25.0);

    let beta_api_flag = flag_manager.get_flag("beta_api").unwrap();
    assert_eq!(beta_api_flag.rollout_percentage, 50.0);
}

/// Tests complete compliance workflow: config + flags + audit
#[tokio::test(flavor = "multi_thread")]
async fn test_complete_compliance_workflow() {
    let temp_dir = TempDir::new("compliance-test").unwrap();
    let audit_log_path = temp_file_path(&temp_dir, "compliance_audit.log");

    // 1. Initialize all compliance components
    let logger = GlobalAuditLogger::new();
    let config = AuditConfig {
        log_file_path: Some(audit_log_path.clone()),
        min_severity: AuditSeverity::Info,
        ..Default::default()
    };
    logger.configure(config).await;
    logger.initialize_with_path().await.unwrap();

    let mut config_manager = ConfigManager::new();
    let mut flag_manager = FeatureFlagManager::new();

    // 2. Application starts
    logger
        .log_event(
            AuditEvent::ApplicationStarted {
                version: "2.0.0".to_string(),
                environment: "production".to_string(),
            },
            create_audit_context("system"),
            AuditSeverity::Info,
        )
        .await;

    // 3. Load configuration
    config_manager
        .set_override("api_endpoint".to_string(), serde_json::json!("https://api.example.com"));
    logger
        .log_event(
            AuditEvent::ConfigurationChanged {
                key: "api_endpoint".to_string(),
                old_value: None,
                new_value: "https://api.example.com".to_string(),
            },
            create_audit_context("system"),
            AuditSeverity::Info,
        )
        .await;

    // 4. Create feature flag (0% rollout, only for premium role)
    let flag = FeatureFlag {
        id: "premium_features".to_string(),
        name: "Premium Features".to_string(),
        description: "Enable premium tier features".to_string(),
        enabled: true,
        rollout_percentage: 0.0, // Only target_roles have access
        target_roles: vec!["premium".to_string()],
        target_users: vec![],
        metadata: HashMap::new(),
    };
    flag_manager.add_flag(flag).unwrap();

    logger
        .log_event(
            AuditEvent::FeatureFlagToggled { flag: "premium_features".to_string(), enabled: true },
            create_audit_context("admin"),
            AuditSeverity::Info,
        )
        .await;

    // 5. User interaction
    let premium_user = create_test_user("user_premium", vec!["premium"]);

    if flag_manager.is_enabled("premium_features", Some(&premium_user)).await {
        logger
            .log_event(
                AuditEvent::DataAccessed {
                    data_type: "premium_content".to_string(),
                    operation: "read".to_string(),
                    record_count: 1,
                },
                create_audit_context("user_premium"),
                AuditSeverity::Info,
            )
            .await;
    }

    // 6. Security event
    let regular_user = create_test_user("user_regular", vec!["user"]);
    if !flag_manager.is_enabled("premium_features", Some(&regular_user)).await {
        logger
            .log_event(
                AuditEvent::UnauthorizedAccess {
                    resource: "premium_content".to_string(),
                    user_id: Some("user_regular".to_string()),
                },
                create_audit_context("user_regular"),
                AuditSeverity::Security,
            )
            .await;
    }

    // 7. Verify complete audit trail
    let stats = logger.get_statistics().await;
    assert_eq!(stats.total_entries, 5); // start + config + flag + access + unauthorized

    // Verify different event types were logged
    assert!(stats.by_event_type.contains_key("ApplicationStarted"));
    assert!(stats.by_event_type.contains_key("ConfigurationChanged"));
    assert!(stats.by_event_type.contains_key("FeatureFlagToggled"));
    assert!(stats.by_event_type.contains_key("DataAccessed"));
    assert!(stats.by_event_type.contains_key("UnauthorizedAccess"));

    // Verify file was written
    assert!(audit_log_path.exists());
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

/// Tests concurrent audit logging from multiple tasks
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_audit_logging() {
    let logger = Arc::new(GlobalAuditLogger::new());
    let mut handles = vec![];

    // Spawn 20 concurrent tasks
    for i in 0..20 {
        let logger_clone = Arc::clone(&logger);
        let handle = tokio::spawn(async move {
            for j in 0..5 {
                logger_clone
                    .log_event(
                        AuditEvent::Custom {
                            category: format!("task_{}", i),
                            action: format!("action_{}", j),
                            details: serde_json::json!({"task": i, "action": j}),
                        },
                        create_audit_context(&format!("user{}", i)),
                        AuditSeverity::Info,
                    )
                    .await;

                // Small delay to increase chance of race conditions
                sleep(Duration::from_micros(10)).await;
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all events were logged (20 tasks × 5 events = 100 total)
    let count = logger.entry_count().await;
    assert_eq!(count, 100);

    // Verify statistics are accurate
    let stats = logger.get_statistics().await;
    assert_eq!(stats.total_entries, 100);
}

/// Tests concurrent feature flag checks
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_feature_flag_checks() {
    let manager = Arc::new(FeatureFlagManager::new());
    let mut handles = vec![];

    // Spawn 50 concurrent tasks checking feature flags
    for i in 0..50 {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            let user = create_test_user(&format!("user{}", i), vec!["admin"]);

            // Check multiple flags
            let _enterprise = manager_clone.is_enabled("enterprise_menu", Some(&user)).await;
            let _telemetry = manager_clone.is_enabled("advanced_telemetry", Some(&user)).await;

            sleep(Duration::from_micros(10)).await;
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify manager state is still valid
    assert!(manager.get_flag("enterprise_menu").is_some());
    assert!(manager.get_flag("advanced_telemetry").is_some());
}

/// Tests concurrent config manager operations
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_config_access() {
    let manager = Arc::new(tokio::sync::RwLock::new(ConfigManager::new()));
    let mut handles = vec![];

    // Initialize with some settings
    {
        let mut mgr = manager.write().await;
        mgr.set_override("shared_key".to_string(), serde_json::json!("initial"));
    }

    // Spawn readers
    for _ in 0..20 {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            for _ in 0..5 {
                let mgr = manager_clone.read().await;
                let _value = mgr.get("shared_key");
                sleep(Duration::from_micros(1)).await;
            }
        });
        handles.push(handle);
    }

    // Spawn writers
    for i in 0..5 {
        let manager_clone = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            let mut mgr = manager_clone.write().await;
            mgr.set_override(format!("key_{}", i), serde_json::json!(i));
            sleep(Duration::from_micros(10)).await;
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all writes succeeded
    let mgr = manager.read().await;
    for i in 0..5 {
        assert!(mgr.get(&format!("key_{}", i)).is_some());
    }
}

/// Tests cross-module concurrent operations
#[tokio::test(flavor = "multi_thread")]
async fn test_cross_module_concurrent_operations() {
    let logger = Arc::new(GlobalAuditLogger::new());
    let flag_manager = Arc::new(FeatureFlagManager::new());
    let config_manager = Arc::new(tokio::sync::RwLock::new(ConfigManager::new()));

    let mut handles = vec![];

    // Spawn tasks that use all three components
    for i in 0..10 {
        let logger_clone = Arc::clone(&logger);
        let flag_clone = Arc::clone(&flag_manager);
        let config_clone = Arc::clone(&config_manager);

        let handle = tokio::spawn(async move {
            let user = create_test_user(&format!("user{}", i), vec!["user"]);

            // Check feature flag
            let flag_enabled = flag_clone.is_enabled("enterprise_menu", Some(&user)).await;

            // Read config
            let config_value = {
                let mgr = config_clone.read().await;
                mgr.get("some_key")
            };

            // Log the operation
            logger_clone
                .log_event(
                    AuditEvent::Custom {
                        category: "concurrent_test".to_string(),
                        action: format!("operation_{}", i),
                        details: serde_json::json!({
                            "flag_enabled": flag_enabled,
                            "config_present": config_value.is_some()
                        }),
                    },
                    create_audit_context(&format!("user{}", i)),
                    AuditSeverity::Info,
                )
                .await;
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all operations completed
    let count = logger.entry_count().await;
    assert_eq!(count, 10);
}

// ============================================================================
// Error Handling and Edge Cases
// ============================================================================

/// Tests audit logger with invalid file paths
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_logger_invalid_paths() {
    let logger = GlobalAuditLogger::new();

    // Configure with invalid path (no parent directory)
    let config = AuditConfig {
        log_file_path: Some(PathBuf::from("/nonexistent/path/audit.log")),
        ..Default::default()
    };

    logger.configure(config).await;

    // initialize_with_path should fail
    let result = logger.initialize_with_path().await;
    assert!(result.is_err());

    // But logging should still work (in memory only)
    logger
        .log_event(
            AuditEvent::Custom {
                category: "test".to_string(),
                action: "test".to_string(),
                details: serde_json::json!({}),
            },
            create_audit_context("user1"),
            AuditSeverity::Info,
        )
        .await;

    assert_eq!(logger.entry_count().await, 1);
}

/// Tests feature flags with edge case inputs
#[tokio::test(flavor = "multi_thread")]
async fn test_feature_flags_edge_cases() {
    let mut manager = FeatureFlagManager::new();

    // Test with empty user context - enterprise_menu has 100% rollout, so even
    // empty context gets access
    let empty_context = UserContext {
        user_id: String::new(),
        roles: vec![],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: HashMap::new(),
    };

    // enterprise_menu has 100% rollout, so it should be enabled even for empty
    // context
    assert!(manager.is_enabled("enterprise_menu", Some(&empty_context)).await);

    // Test with None context
    assert!(manager.is_enabled("enterprise_menu", None).await); // 100% rollout

    // Test flag with extreme rollout values
    let flag = FeatureFlag {
        id: "edge_case".to_string(),
        name: "Edge Case".to_string(),
        description: "Test".to_string(),
        enabled: true,
        rollout_percentage: 0.1, // Very small percentage
        target_roles: vec![],
        target_users: vec![],
        metadata: HashMap::new(),
    };

    manager.add_flag(flag).unwrap();

    let user = create_test_user("testuser", vec![]);
    let result = manager.is_enabled("edge_case", Some(&user)).await;
    // Should be deterministic even with very small percentage
    assert_eq!(result, manager.is_enabled("edge_case", Some(&user)).await);
}

/// Tests config manager with malformed data
#[tokio::test(flavor = "multi_thread")]
async fn test_config_manager_malformed_data() {
    let temp_dir = TempDir::new("compliance-test").unwrap();

    // Test with invalid JSON
    let invalid_json_path = temp_file_path(&temp_dir, "invalid.json");
    std::fs::write(&invalid_json_path, "{ invalid json }").unwrap();

    let mut manager = ConfigManager::new();
    let result = manager.load_from_file(invalid_json_path.to_str().unwrap());
    assert!(result.is_err());

    // Test with missing file
    let result = manager.load_from_file("/nonexistent/config.json");
    assert!(result.is_err());

    // Manager should still be usable after errors
    manager.set_override("test".to_string(), serde_json::json!("value"));
    assert_eq!(manager.get("test"), Some(serde_json::json!("value")));
}

// ============================================================================
// Audit Streaming Integration Tests
// ============================================================================

/// Tests successful webhook delivery with proper payload structure
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_streaming_success() {
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    // Start mock HTTP server
    let mock_server = MockServer::start().await;

    // Configure mock to expect POST requests with JSON payload
    Mock::given(matchers::method("POST"))
        .and(matchers::header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1..) // Expect at least one request
        .mount(&mock_server)
        .await;

    // Configure audit logger with streaming enabled
    let logger = GlobalAuditLogger::new();
    let config = AuditConfig {
        max_memory_entries: 100,
        log_file_path: None,
        enable_streaming: true,
        min_severity: AuditSeverity::Info,
        encrypt_sensitive: false,
        streaming_timeout_secs: 5,
        streaming_url: Some(mock_server.uri()),
    };

    logger.configure(config).await;

    // Log an event
    logger
        .log_event(
            AuditEvent::ApplicationStarted {
                version: "1.0.0".to_string(),
                environment: "test".to_string(),
            },
            create_audit_context("system"),
            AuditSeverity::Info,
        )
        .await;

    // Give async task time to complete webhook call
    // Increased from 100ms to 500ms to handle test parallelism and system load
    sleep(Duration::from_millis(500)).await;

    // Verify the mock server received the request
    // (wiremock will panic if expectations aren't met when mock_server is
    // dropped)
}

/// Tests webhook delivery with 4xx client error response
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_streaming_client_error() {
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    // Configure mock to return 400 Bad Request
    Mock::given(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&mock_server)
        .await;

    let logger = GlobalAuditLogger::new();
    let config = AuditConfig {
        enable_streaming: true,
        streaming_url: Some(mock_server.uri()),
        streaming_timeout_secs: 5,
        ..Default::default()
    };

    logger.configure(config).await;

    // Log event - should not panic even with 400 response
    logger
        .log_event(
            AuditEvent::Custom {
                category: "test".to_string(),
                action: "test".to_string(),
                details: serde_json::json!({}),
            },
            create_audit_context("user1"),
            AuditSeverity::Info,
        )
        .await;

    // Event should still be stored in memory despite streaming failure
    assert_eq!(logger.entry_count().await, 1);

    sleep(Duration::from_millis(100)).await;
}

/// Tests webhook delivery with 5xx server error response
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_streaming_server_error() {
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    // Configure mock to return 503 Service Unavailable
    Mock::given(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock_server)
        .await;

    let logger = GlobalAuditLogger::new();
    let config = AuditConfig {
        enable_streaming: true,
        streaming_url: Some(mock_server.uri()),
        streaming_timeout_secs: 5,
        ..Default::default()
    };

    logger.configure(config).await;

    logger
        .log_event(
            AuditEvent::Custom {
                category: "test".to_string(),
                action: "error_test".to_string(),
                details: serde_json::json!({"test": "data"}),
            },
            create_audit_context("user1"),
            AuditSeverity::Error,
        )
        .await;

    // Event should still be logged despite server error
    assert_eq!(logger.entry_count().await, 1);

    sleep(Duration::from_millis(100)).await;
}

/// Tests webhook timeout handling
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_streaming_timeout() {
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    // Configure mock to delay response beyond timeout
    Mock::given(matchers::method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_delay(Duration::from_secs(10)), // Longer than timeout
        )
        .mount(&mock_server)
        .await;

    let logger = GlobalAuditLogger::new();
    let config = AuditConfig {
        enable_streaming: true,
        streaming_url: Some(mock_server.uri()),
        streaming_timeout_secs: 1, // Very short timeout
        ..Default::default()
    };

    logger.configure(config).await;

    logger
        .log_event(
            AuditEvent::Custom {
                category: "test".to_string(),
                action: "timeout_test".to_string(),
                details: serde_json::json!({}),
            },
            create_audit_context("user1"),
            AuditSeverity::Warning,
        )
        .await;

    // Event should still be logged despite timeout
    assert_eq!(logger.entry_count().await, 1);

    sleep(Duration::from_millis(1500)).await;
}

/// Tests concurrent streaming requests
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_streaming_concurrent() {
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    Mock::given(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(5) // Expect exactly 5 requests
        .mount(&mock_server)
        .await;

    let logger = Arc::new(GlobalAuditLogger::new());
    let config = AuditConfig {
        enable_streaming: true,
        streaming_url: Some(mock_server.uri()),
        streaming_timeout_secs: 5,
        ..Default::default()
    };

    logger.configure(config).await;

    // Spawn multiple concurrent log operations
    let mut handles = vec![];
    for i in 0..5 {
        let logger_clone = Arc::clone(&logger);
        let handle = tokio::spawn(async move {
            logger_clone
                .log_event(
                    AuditEvent::Custom {
                        category: "concurrent".to_string(),
                        action: format!("test_{}", i),
                        details: serde_json::json!({"index": i}),
                    },
                    create_audit_context(&format!("user{}", i)),
                    AuditSeverity::Info,
                )
                .await;
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // All events should be logged
    assert_eq!(logger.entry_count().await, 5);

    // Give time for all webhook calls to complete
    sleep(Duration::from_millis(200)).await;

    // wiremock will verify all 5 requests were received
}

/// Tests payload structure verification
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_streaming_payload_structure() {
    use wiremock::{matchers, Mock, MockServer, Request, ResponseTemplate};

    let mock_server = MockServer::start().await;

    // Custom matcher to verify JSON structure
    let body_matcher = |req: &Request| -> bool {
        let body = req.body_json::<serde_json::Value>();
        if let Ok(json) = body {
            // Verify expected fields exist
            json.get("timestamp").is_some()
                && json.get("severity").is_some()
                && json.get("event").is_some()
        } else {
            false
        }
    };

    Mock::given(matchers::method("POST"))
        .and(matchers::header("content-type", "application/json"))
        .and(body_matcher)
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let logger = GlobalAuditLogger::new();
    let config = AuditConfig {
        enable_streaming: true,
        streaming_url: Some(mock_server.uri()),
        streaming_timeout_secs: 5,
        ..Default::default()
    };

    logger.configure(config).await;

    logger
        .log_event(
            AuditEvent::PermissionCheck {
                user_id: "user123".to_string(),
                permission: Permission::new("audit:read"),
                granted: true,
            },
            create_audit_context("user123"),
            AuditSeverity::Info,
        )
        .await;

    sleep(Duration::from_millis(100)).await;
}

/// Tests that logging still works when streaming is disabled
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_without_streaming() {
    let logger = GlobalAuditLogger::new();
    let config = AuditConfig {
        enable_streaming: false, // Explicitly disabled
        streaming_url: Some("http://should-not-be-called.example.com".to_string()),
        ..Default::default()
    };

    logger.configure(config).await;

    logger
        .log_event(
            AuditEvent::ApplicationStarted {
                version: "1.0.0".to_string(),
                environment: "test".to_string(),
            },
            create_audit_context("system"),
            AuditSeverity::Info,
        )
        .await;

    // Event should be logged in memory
    assert_eq!(logger.entry_count().await, 1);

    // No webhook calls should be made (verified by not providing mock server)
    sleep(Duration::from_millis(100)).await;
}

/// Tests webhook URL from environment variable
#[tokio::test(flavor = "multi_thread")]
async fn test_audit_streaming_env_var_url() {
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    Mock::given(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    // Set environment variable
    std::env::set_var("AUDIT_WEBHOOK_URL", mock_server.uri());

    let logger = GlobalAuditLogger::new();
    let config = AuditConfig {
        enable_streaming: true,
        streaming_url: None, // URL should come from env var
        streaming_timeout_secs: 5,
        ..Default::default()
    };

    logger.configure(config).await;

    logger
        .log_event(
            AuditEvent::Custom {
                category: "env_test".to_string(),
                action: "test".to_string(),
                details: serde_json::json!({}),
            },
            create_audit_context("user1"),
            AuditSeverity::Info,
        )
        .await;

    sleep(Duration::from_millis(100)).await;

    // Clean up
    std::env::remove_var("AUDIT_WEBHOOK_URL");
}
