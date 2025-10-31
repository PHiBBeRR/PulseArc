//! Integration tests for security module
//!
//! These tests verify end-to-end security workflows including:
//! - Encryption key management and rotation
//! - RBAC with dynamic policies
//! - Keychain integration
//! - Cache behavior across operations
//! - Security event handling

#![cfg(feature = "platform")]
#![allow(clippy::doc_lazy_continuation)]

use std::time::Duration;

use pulsearc_common::security::encryption::{
    clear_cache_on_security_event, generate_encryption_key, get_cache_stats,
    get_or_create_key_cached, is_cached, KeyRotationSchedule, SecureString,
};
use pulsearc_common::security::rbac::{
    Permission, PolicyCondition, PolicyEffect, RBACManager, RBACPolicy, UserContext,
};
use pulsearc_common::security::traits::{
    AccessControl, DenyAllAccessControl, NoOpAccessControl, Permission as TraitPermission,
    UserContext as TraitUserContext,
};
use pulsearc_common::storage::config::KeySource;

mod fixtures;

// All fixture types are re-exported from fixtures::mod for convenience
use fixtures::*;

// ============================================================================
// Encryption Integration Tests
// ============================================================================

/// Validates `SecureString::new` behavior for the encryption key lifecycle
/// scenario.
///
/// Assertions:
/// - Confirms `key1.len()` equals `64`.
/// - Ensures `!debug_str.contains(key1.expose())` evaluates to true.
/// - Confirms `debug_str` equals `"SecureString(***)"`.
/// - Confirms `key1.expose()` differs from `key2.expose()`.
/// - Ensures `!key1.constant_time_eq(&key2)` evaluates to true.
/// - Ensures `key1.constant_time_eq(&key1_copy)` evaluates to true.
#[test]
fn test_encryption_key_lifecycle() {
    // 1. Generate a new key
    let key1 = generate_encryption_key();
    assert_eq!(key1.len(), 64);

    // 2. Verify secure string doesn't leak secrets
    let debug_str = format!("{:?}", key1);
    assert!(!debug_str.contains(key1.expose()));
    assert_eq!(debug_str, "SecureString(***)");

    // 3. Generate second key (should be different)
    let key2 = generate_encryption_key();
    assert_ne!(key1.expose(), key2.expose());

    // 4. Test constant-time comparison
    assert!(!key1.constant_time_eq(&key2));
    let key1_copy = SecureString::new(key1.expose().to_string());
    assert!(key1.constant_time_eq(&key1_copy));
}

/// Validates `KeySource::Direct` behavior for the key caching workflow
/// scenario.
///
/// Assertions:
/// - Ensures `is_cached()` evaluates to true.
/// - Ensures `std::ptr::eq(key1, key2)` evaluates to true.
/// - Ensures `stats.is_cached` evaluates to true.
/// - Ensures `stats.key_length.is_some()` evaluates to true.
#[test]
fn test_key_caching_workflow() {
    let key_source = KeySource::Direct {
        key: "integration_test_key_32_chars_long_aaaaaaaaaaaaaaaaa".to_string(),
    };

    // 1. Initial cache state
    let initial_stats = get_cache_stats();
    println!("Initial cache state: {:?}", initial_stats);

    // 2. First access should cache the key
    let key1 = get_or_create_key_cached(&key_source).unwrap();
    assert!(is_cached());

    // 3. Second access should return same reference
    let key2 = get_or_create_key_cached(&key_source).unwrap();
    assert!(std::ptr::eq(key1, key2));

    // 4. Check cache statistics
    let stats = get_cache_stats();
    assert!(stats.is_cached);
    assert!(stats.key_length.is_some());
}

/// Validates `KeyRotationSchedule::new` behavior for the key rotation schedule
/// workflow scenario.
///
/// Assertions:
/// - Confirms `schedule.rotation_days` equals `30`.
/// - Ensures `!schedule.should_rotate()` evaluates to true.
/// - Ensures `schedule.last_rotation().is_some()` evaluates to true.
/// - Confirms `days` equals `Some(0)`.
/// - Ensures `!schedule.should_rotate()` evaluates to true.
/// - Confirms `schedule.rotation_days` equals `90`.
/// Validates complete rotation schedule workflow within the security
/// integration workflow.
#[test]
fn test_key_rotation_schedule_workflow() {
    // 1. Create schedule with 30-day rotation
    let mut schedule = KeyRotationSchedule::new(30);
    assert_eq!(schedule.rotation_days, 30);

    // 2. Initially should not need rotation (never rotated before)
    assert!(!schedule.should_rotate());

    // 3. Record a rotation
    schedule.record_rotation();
    assert!(schedule.last_rotation().is_some());

    // 4. Check days since rotation
    let days = schedule.days_since_last_rotation();
    assert_eq!(days, Some(0));

    // 5. Immediately after rotation, should not need another
    assert!(!schedule.should_rotate());

    // 6. Adjust rotation period
    schedule.set_rotation_days(90);
    assert_eq!(schedule.rotation_days, 90);
}

/// Validates `KeySource::Direct` behavior for the security event cache clear
/// scenario.
///
/// Assertions:
/// - Ensures `is_cached()` evaluates to true.
/// - Ensures `is_cached()` evaluates to true.
/// - Ensures `stats.is_cached` evaluates to true.
#[test]
fn test_security_event_cache_clear() {
    let key_source =
        KeySource::Direct { key: "security_event_test_key_32_chars_aaaaaaaaaaaaaa".to_string() };

    // 1. Prime the cache
    let _key = get_or_create_key_cached(&key_source).unwrap();
    assert!(is_cached());

    // 2. Trigger security event
    clear_cache_on_security_event("integration_test_suspected_compromise");

    // 3. Cache should still be present (OnceLock limitation)
    // but the event is logged for audit
    assert!(is_cached());

    // 4. Verify cache stats still available
    let stats = get_cache_stats();
    assert!(stats.is_cached);
}

// ============================================================================
// RBAC Integration Tests
// ============================================================================

/// Validates `RBACManager::new` behavior for the rbac basic workflow scenario.
///
/// Assertions:
/// - Ensures `rbac.check_permission(&admin, &view_menu).await` evaluates to
///   true.
/// - Ensures `rbac.check_permission(&admin, &delete_audit).await` evaluates to
///   true.
/// - Ensures `rbac.check_permission(&user, &view_menu).await` evaluates to
///   true.
/// - Ensures `!rbac.check_permission(&user, &delete_audit).await` evaluates to
///   true.
/// Validates complete RBAC workflow with default roles within the security
/// integration workflow.
#[tokio::test]
async fn test_rbac_basic_workflow() {
    let mut rbac = RBACManager::new();
    rbac.initialize().unwrap();

    // 1. Create users with different roles
    let admin = UserContext {
        user_id: "admin_user".to_string(),
        roles: vec!["admin".to_string()],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: std::collections::HashMap::new(),
    };

    let user = UserContext {
        user_id: "regular_user".to_string(),
        roles: vec!["user".to_string()],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: std::collections::HashMap::new(),
    };

    // 2. Create permissions to check
    let view_menu = Permission::new("menu:view");
    let delete_audit = Permission::new("audit:delete");

    // 3. Admin should have all permissions (has system:*)
    assert!(rbac.check_permission(&admin, &view_menu).await);
    assert!(rbac.check_permission(&admin, &delete_audit).await);

    // 4. Regular user should have limited permissions
    assert!(rbac.check_permission(&user, &view_menu).await);
    assert!(!rbac.check_permission(&user, &delete_audit).await);
}

/// Validates `RBACManager::new` behavior for the rbac wildcard permissions
/// scenario.
///
/// Assertions:
/// - Ensures `rbac.check_permission(&admin, &menu_view).await` evaluates to
///   true.
/// - Ensures `rbac.check_permission(&admin, &menu_edit).await` evaluates to
///   true.
/// - Ensures `rbac.check_permission(&admin, &menu_delete).await` evaluates to
///   true.
/// - Ensures `rbac.check_permission(&admin, &config_write).await` evaluates to
///   true.
/// - Ensures `rbac.check_permission(&admin, &audit_delete).await` evaluates to
///   true.
/// Validates wildcard permission matching with admin role within the security
/// integration workflow.
#[tokio::test]
async fn test_rbac_wildcard_permissions() {
    let rbac = RBACManager::new();

    // 1. Admin has global wildcard (system:*)
    let admin = UserContext {
        user_id: "admin_user".to_string(),
        roles: vec!["admin".to_string()],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: std::collections::HashMap::new(),
    };

    // 2. Test various permissions - admin should have ALL
    let menu_view = Permission::new("menu:view");
    let menu_edit = Permission::new("menu:edit");
    let menu_delete = Permission::new("menu:delete");
    let config_write = Permission::new("config:write");
    let audit_delete = Permission::new("audit:delete");

    // Admin has system:* which matches everything
    assert!(rbac.check_permission(&admin, &menu_view).await);
    assert!(rbac.check_permission(&admin, &menu_edit).await);
    assert!(rbac.check_permission(&admin, &menu_delete).await);
    assert!(rbac.check_permission(&admin, &config_write).await);
    assert!(rbac.check_permission(&admin, &audit_delete).await);
}

/// Validates `RBACManager::new` behavior for the rbac dynamic policies
/// scenario.
///
/// Assertions:
/// - Ensures `!rbac.check_permission(&admin, &audit_delete).await` evaluates to
///   true.
/// - Ensures `rbac.check_permission(&admin, &audit_read).await` evaluates to
///   true.
#[tokio::test]
async fn test_rbac_dynamic_policies() {
    let rbac = RBACManager::new();

    // 1. Create admin user
    let mut admin_attrs = std::collections::HashMap::new();
    admin_attrs.insert("department".to_string(), "engineering".to_string());

    let admin = UserContext {
        user_id: "admin_user".to_string(),
        roles: vec!["admin".to_string()],
        session_id: None,
        ip_address: Some("192.168.1.100".to_string()),
        user_agent: None,
        attributes: admin_attrs,
    };

    // 2. Create policy: Deny audit:delete for everyone
    let deny_audit_delete = RBACPolicy {
        id: "deny_audit_delete".to_string(),
        name: "Deny Audit Deletion".to_string(),
        condition: PolicyCondition::Always,
        effect: PolicyEffect::Deny,
        permissions: vec!["audit:delete".to_string()],
    };

    let _ = rbac.add_policy(deny_audit_delete).await;

    // 3. Even admin should be denied due to policy
    let audit_delete = Permission::new("audit:delete");
    assert!(!rbac.check_permission(&admin, &audit_delete).await);

    // 4. But other permissions should still work
    let audit_read = Permission::new("audit:read");
    assert!(rbac.check_permission(&admin, &audit_read).await);
}

/// Validates `RBACManager::new` behavior for the rbac permission caching
/// scenario.
///
/// Assertions:
/// - Confirms `result1` equals `result2`.
#[tokio::test]
async fn test_rbac_permission_caching() {
    let rbac = RBACManager::new();

    let admin = UserContext {
        user_id: "cache_test_admin".to_string(),
        roles: vec!["admin".to_string()],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: std::collections::HashMap::new(),
    };

    let perm = Permission::new("menu:view");

    // 1. First check (cache miss)
    let start1 = std::time::Instant::now();
    let result1 = rbac.check_permission(&admin, &perm).await;
    let duration1 = start1.elapsed();

    // 2. Second check (cache hit)
    let start2 = std::time::Instant::now();
    let result2 = rbac.check_permission(&admin, &perm).await;
    let duration2 = start2.elapsed();

    // 3. Results should be the same
    assert_eq!(result1, result2);

    println!("First check: {:?}, Second check: {:?}", duration1, duration2);
}

/// Validates `RBACManager::new` behavior for the rbac user role assignment
/// scenario.
///
/// Assertions:
/// - Ensures `initial_roles.is_empty()` evaluates to true.
/// - Confirms `roles.len()` equals `1`.
/// - Ensures `roles.iter().any(|r| r.id == "user")` evaluates to true.
/// - Confirms `roles.len()` equals `2`.
/// - Confirms `roles.len()` equals `1`.
/// - Ensures `roles.iter().any(|r| r.id == "power_user")` evaluates to true.
#[tokio::test]
async fn test_rbac_user_role_assignment() {
    let rbac = RBACManager::new();

    let user_id = "dynamic_user";

    // 1. Initially no roles
    let initial_roles = rbac.get_user_roles(user_id).await;
    assert!(initial_roles.is_empty());

    // 2. Assign user role
    rbac.assign_role(user_id, "user").await.unwrap();
    let roles = rbac.get_user_roles(user_id).await;
    assert_eq!(roles.len(), 1);
    assert!(roles.iter().any(|r| r.id == "user"));

    // 3. Assign additional role
    rbac.assign_role(user_id, "power_user").await.unwrap();
    let roles = rbac.get_user_roles(user_id).await;
    assert_eq!(roles.len(), 2);

    // 4. Revoke a role
    rbac.revoke_role(user_id, "user").await.unwrap();
    let roles = rbac.get_user_roles(user_id).await;
    assert_eq!(roles.len(), 1);
    assert!(roles.iter().any(|r| r.id == "power_user"));
}

// ============================================================================
// Cross-Module Integration Tests
// ============================================================================

/// Validates `RBACManager::new` behavior for the security with multiple users
/// scenario.
///
/// Assertions:
/// - Ensures `rbac.check_permission(&admin, &system_perm).await` evaluates to
///   true.
/// - Ensures `!rbac.check_permission(&power_user, &system_perm).await`
///   evaluates to true.
/// - Ensures `!rbac.check_permission(&regular_user, &system_perm).await`
///   evaluates to true.
/// - Ensures `!rbac.check_permission(&guest, &system_perm).await` evaluates to
///   true.
/// - Ensures `rbac.check_permission(&admin, &menu_perm).await` evaluates to
///   true.
/// - Ensures `rbac.check_permission(&power_user, &menu_perm).await` evaluates
///   to true.
/// - Ensures `rbac.check_permission(&regular_user, &menu_perm).await` evaluates
///   to true.
/// - Ensures `!rbac.check_permission(&guest, &menu_perm).await` evaluates to
///   true.
/// Validates complete security workflow with multiple users within the security
/// integration workflow.
#[tokio::test]
async fn test_security_with_multiple_users() {
    let mut rbac = RBACManager::new();
    rbac.initialize().unwrap();

    // 1. Create diverse user base
    let admin = UserContext {
        user_id: "admin1".to_string(),
        roles: vec!["admin".to_string()],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: std::collections::HashMap::new(),
    };

    let power_user = UserContext {
        user_id: "power1".to_string(),
        roles: vec!["power_user".to_string()],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: std::collections::HashMap::new(),
    };

    let regular_user = UserContext {
        user_id: "user1".to_string(),
        roles: vec!["user".to_string()],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: std::collections::HashMap::new(),
    };

    let guest = UserContext {
        user_id: "guest".to_string(),
        roles: vec!["guest".to_string()],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: std::collections::HashMap::new(),
    };

    // 2. Test permission hierarchy
    let system_perm = Permission::new("system:config");

    assert!(rbac.check_permission(&admin, &system_perm).await);
    assert!(!rbac.check_permission(&power_user, &system_perm).await);
    assert!(!rbac.check_permission(&regular_user, &system_perm).await);
    assert!(!rbac.check_permission(&guest, &system_perm).await);

    // 3. Test resource-specific permissions
    let menu_perm = Permission::new("menu:view");

    assert!(rbac.check_permission(&admin, &menu_perm).await);
    assert!(rbac.check_permission(&power_user, &menu_perm).await);
    assert!(rbac.check_permission(&regular_user, &menu_perm).await);
    assert!(!rbac.check_permission(&guest, &menu_perm).await);
}

/// Validates `RBACManager::new` behavior for the encryption with rbac scenario.
///
/// Assertions:
/// - Ensures `rbac.check_permission(&admin, &key_access).await` evaluates to
///   true.
/// - Ensures `!rbac.check_permission(&user, &key_access).await` evaluates to
///   true.
/// Validates encryption keys with RBAC-controlled access within the security
/// integration workflow.
#[tokio::test]
async fn test_encryption_with_rbac() {
    let rbac = RBACManager::new();

    // 1. Define permission for key access
    let key_access = Permission::new("encryption:access_key");

    // 2. Only admins should access encryption keys
    let admin = UserContext {
        user_id: "admin".to_string(),
        roles: vec!["admin".to_string()],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: std::collections::HashMap::new(),
    };

    let user = UserContext {
        user_id: "user".to_string(),
        roles: vec!["user".to_string()],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: std::collections::HashMap::new(),
    };

    assert!(rbac.check_permission(&admin, &key_access).await);
    assert!(!rbac.check_permission(&user, &key_access).await);

    // 3. If authorized, access encryption key
    if rbac.check_permission(&admin, &key_access).await {
        let key_source =
            KeySource::Direct { key: "rbac_controlled_key_32_chars_aaaaaaaaaaaa".to_string() };
        let _key = get_or_create_key_cached(&key_source).unwrap();
        // Admin successfully accessed key
    }
}

// ============================================================================
// Trait Implementation Tests
// ============================================================================

/// Validates `TraitUserContext::new` behavior for the noop access control
/// scenario.
///
/// Assertions:
/// - Ensures `ac.check_permission(&user, &perm).await` evaluates to true.
/// - Confirms `ac.get_user_permissions(&user).await.len()` equals `0`.
/// Validates no-op implementation (allows everything) within the security
/// integration workflow.
#[tokio::test]
async fn test_noop_access_control() {
    let ac = NoOpAccessControl;
    let user = TraitUserContext::new("test_user", vec![]);
    let perm = TraitPermission::new("any:action", "any", "action");

    // Should always allow
    assert!(ac.check_permission(&user, &perm).await);
    assert_eq!(ac.get_user_permissions(&user).await.len(), 0);
}

/// Validates `TraitUserContext::admin` behavior for the deny all access control
/// scenario.
///
/// Assertions:
/// - Ensures `!ac.check_permission(&admin, &perm).await` evaluates to true.
/// - Confirms `ac.get_user_permissions(&admin).await.len()` equals `0`.
/// Validates deny-all implementation (denies everything) within the security
/// integration workflow.
#[tokio::test]
async fn test_deny_all_access_control() {
    let ac = DenyAllAccessControl;
    let admin = TraitUserContext::admin("admin");
    let perm = TraitPermission::new("system:admin", "system", "admin");

    // Should always deny
    assert!(!ac.check_permission(&admin, &perm).await);
    assert_eq!(ac.get_user_permissions(&admin).await.len(), 0);
}

// ============================================================================
// Performance and Edge Case Tests
// ============================================================================

/// Validates `RBACManager::new` behavior for the rbac performance many
/// permissions scenario.
///
/// Assertions:
/// - Ensures `duration < Duration::from_secs(1)` evaluates to true.
/// Validates RBAC performance with many permission checks within the security
/// integration workflow.
#[tokio::test]
async fn test_rbac_performance_many_permissions() {
    let rbac = RBACManager::new();
    let admin = UserContext {
        user_id: "perf_admin".to_string(),
        roles: vec!["admin".to_string()],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: std::collections::HashMap::new(),
    };

    let start = std::time::Instant::now();

    // Check 100 different permissions
    for i in 0..100 {
        let perm = Permission::new(&format!("resource{}:action", i));
        let _result = rbac.check_permission(&admin, &perm).await;
    }

    let duration = start.elapsed();
    println!("100 permission checks took: {:?}", duration);

    // Should complete in reasonable time (< 1 second)
    assert!(duration < Duration::from_secs(1));
}

/// Validates `Instant::now` behavior for the key generation performance
/// scenario.
///
/// Assertions:
/// - Confirms `unique_count` equals `100`.
/// - Ensures `duration < Duration::from_secs(1)` evaluates to true.
#[test]
fn test_key_generation_performance() {
    let start = std::time::Instant::now();

    // Generate 100 keys
    let mut keys = Vec::new();
    for _ in 0..100 {
        keys.push(generate_encryption_key());
    }

    let duration = start.elapsed();
    println!("100 key generations took: {:?}", duration);

    // All keys should be unique
    let unique_count =
        keys.iter().map(|k| k.expose().to_string()).collect::<std::collections::HashSet<_>>().len();
    assert_eq!(unique_count, 100);

    // Should complete in reasonable time
    assert!(duration < Duration::from_secs(1));
}

/// Validates `SecureString::new` behavior for the secure string edge cases
/// scenario.
///
/// Assertions:
/// - Ensures `empty.is_empty()` evaluates to true.
/// - Confirms `empty.len()` equals `0`.
/// - Confirms `long_secure.len()` equals `10000`.
/// - Confirms `long_secure.expose()` equals `long_str`.
/// - Ensures `!unicode.is_empty()` evaluates to true.
/// - Confirms `format!("{:?}", unicode)` equals `"SecureString(***)"`.
#[test]
fn test_secure_string_edge_cases() {
    // Empty string
    let empty = SecureString::new(String::new());
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);

    // Very long string
    let long_str = "a".repeat(10000);
    let long_secure = SecureString::new(long_str.clone());
    assert_eq!(long_secure.len(), 10000);
    assert_eq!(long_secure.expose(), long_str);

    // Unicode string
    let unicode = SecureString::new("🔐🔑🛡️".to_string());
    assert!(!unicode.is_empty());
    assert_eq!(format!("{:?}", unicode), "SecureString(***)");
}

/// Validates `RBACManager::new` behavior for the rbac empty roles scenario.
///
/// Assertions:
/// - Ensures `!rbac.check_permission(&empty_user, &perm).await` evaluates to
///   true.
#[tokio::test]
async fn test_rbac_empty_roles() {
    let rbac = RBACManager::new();
    let empty_user = UserContext {
        user_id: "empty".to_string(),
        roles: vec![],
        session_id: None,
        ip_address: None,
        user_agent: None,
        attributes: std::collections::HashMap::new(),
    };

    let perm = Permission::new("menu:view");

    // User with no roles should have no permissions
    assert!(!rbac.check_permission(&empty_user, &perm).await);
}

// ============================================================================
// Tests Using Fixtures
// ============================================================================

/// Validates `EncryptionKeyFixture::generate` behavior for the encryption with
/// fixtures scenario.
///
/// Assertions:
/// - Confirms `key1.len()` equals `64`.
/// - Confirms `key1.expose()` differs from `key2.expose()`.
/// - Confirms `fixed_key.len()` equals `64`.
/// Using the fixture makes tests much cleaner
#[test]
fn test_encryption_with_fixtures() {
    let key1 = EncryptionKeyFixture::generate();
    let key2 = EncryptionKeyFixture::generate();

    assert_eq!(key1.len(), 64);
    assert_ne!(key1.expose(), key2.expose());

    // Fixed keys for reproducible tests
    let fixed_key = EncryptionKeyFixture::fixed(64);
    assert_eq!(fixed_key.len(), 64);
}

/// Validates `EncryptionKeyFixture::direct_source` behavior for the key source
/// fixtures scenario.
///
/// Assertions:
/// - Ensures `is_cached()` evaluates to true.
/// - Ensures `!key.is_empty()` evaluates to true.
/// Clean, reusable key source creation
#[test]
fn test_key_source_fixtures() {
    let direct = EncryptionKeyFixture::direct_source("test1");
    let _env = EncryptionKeyFixture::env_source("TEST_KEY");
    let _keychain = EncryptionKeyFixture::keychain_source("PulseArc", "db_key");

    // Test with direct source
    let key = get_or_create_key_cached(&direct).unwrap();
    assert!(is_cached());
    assert!(!key.is_empty());
}

/// Validates `RBACFixture::new` behavior for the rbac with user fixtures
/// scenario.
///
/// Assertions:
/// - Ensures `rbac.manager().check_permission(&admin, &view_perm).await`
///   evaluates to true.
/// - Ensures `rbac.manager().check_permission(&admin, &delete_perm).await`
///   evaluates to true.
/// - Ensures `rbac.manager().check_permission(&user, &view_perm).await`
///   evaluates to true.
/// - Ensures `!rbac.manager().check_permission(&user, &delete_perm).await`
///   evaluates to true.
/// - Ensures `!rbac.manager().check_permission(&empty, &view_perm).await`
///   evaluates to true.
/// Using fixtures makes user creation much cleaner
#[tokio::test]
async fn test_rbac_with_user_fixtures() {
    let rbac = RBACFixture::new();

    let admin = UserContextFixture::admin("admin1");
    let user = UserContextFixture::user("user1");
    let _guest = UserContextFixture::guest();
    let empty = UserContextFixture::empty("empty1");

    // Using permission fixtures
    let view_perm = PermissionFixture::menu_view();
    let delete_perm = PermissionFixture::audit_delete();

    // Clean assertions
    assert!(rbac.manager().check_permission(&admin, &view_perm).await);
    assert!(rbac.manager().check_permission(&admin, &delete_perm).await);

    assert!(rbac.manager().check_permission(&user, &view_perm).await);
    assert!(!rbac.manager().check_permission(&user, &delete_perm).await);

    assert!(!rbac.manager().check_permission(&empty, &view_perm).await);
}

/// Validates `RBACFixture::new` behavior for the rbac with policy fixtures
/// scenario.
///
/// Assertions:
/// - Ensures `!rbac.check_permission(&admin, &audit_perm).await` evaluates to
///   true.
/// Exercises the rbac with policy fixtures integration scenario end-to-end.
#[tokio::test]
async fn test_rbac_with_policy_fixtures() {
    let mut rbac_fixture = RBACFixture::new();
    let rbac = rbac_fixture.manager_mut();

    // Using policy fixtures makes policy creation clean
    let deny_policy = PolicyFixture::deny_audit_delete();
    let _ = rbac.add_policy(deny_policy).await;

    let admin = UserContextFixture::admin("admin");
    let audit_perm = PermissionFixture::audit_delete();

    // Policy should deny even admin
    assert!(!rbac.check_permission(&admin, &audit_perm).await);
}

/// Validates `RBACFixture::new` behavior for the department policy with
/// fixtures scenario.
///
/// Assertion coverage: ensures the routine completes without panicking.
/// Exercises the department policy with fixtures integration scenario
/// end-to-end.
#[tokio::test]
async fn test_department_policy_with_fixtures() {
    let mut rbac_fixture = RBACFixture::new();
    let rbac = rbac_fixture.manager_mut();

    // Create department-specific policy
    let eng_policy =
        PolicyFixture::department_only("engineering", vec!["secure:access".to_string()]);
    let _ = rbac.add_policy(eng_policy).await;

    // Users with/without department attribute
    let eng_user = UserContextFixture::with_department("eng_user", "engineering");
    let sales_user = UserContextFixture::with_department("sales_user", "sales");

    let secure_perm = PermissionFixture::custom("secure", "access");

    // Policy evaluation
    let _eng_result = rbac.check_permission(&eng_user, &secure_perm).await;
    let _sales_result = rbac.check_permission(&sales_user, &secure_perm).await;
}

/// Validates `RBACFixture::new` behavior for the ip based policy with fixtures
/// scenario.
///
/// Assertion coverage: ensures the routine completes without panicking.
/// Exercises the ip based policy with fixtures integration scenario end-to-end.
#[tokio::test]
async fn test_ip_based_policy_with_fixtures() {
    let mut rbac_fixture = RBACFixture::new();
    let rbac = rbac_fixture.manager_mut();

    // IP-restricted policy
    let ip_policy = PolicyFixture::ip_restricted(
        vec!["192.168.1.100".to_string()],
        vec!["admin:access".to_string()],
    );
    let _ = rbac.add_policy(ip_policy).await;

    // Users from different IPs
    let office_user = UserContextFixture::from_ip("user1", "192.168.1.100");
    let remote_user = UserContextFixture::from_ip("user2", "203.0.113.1");

    let admin_perm = PermissionFixture::custom("admin", "access");

    let _office_result = rbac.check_permission(&office_user, &admin_perm).await;
    let _remote_result = rbac.check_permission(&remote_user, &admin_perm).await;
}

/// Validates `RBACFixture::new` behavior for the custom user builder scenario.
///
/// Assertions:
/// - Confirms `custom_user.roles.len()` equals `2`.
/// - Confirms `custom_user.attributes.len()` equals `2`.
/// - Ensures `rbac.manager().check_permission(&custom_user, &perm).await`
///   evaluates to true.
/// Exercises the custom user builder integration scenario end-to-end.
#[tokio::test]
async fn test_custom_user_builder() {
    let rbac = RBACFixture::new();

    // Custom user with builder pattern
    let custom_user = UserContextBuilder::new("custom1")
        .with_role("admin")
        .with_role("auditor")
        .with_ip("10.0.0.1")
        .with_attribute("department", "security")
        .with_attribute("clearance", "top_secret")
        .with_session("session_abc123")
        .build();

    assert_eq!(custom_user.roles.len(), 2);
    assert_eq!(custom_user.attributes.len(), 2);

    let perm = PermissionFixture::audit_read();
    assert!(rbac.manager().check_permission(&custom_user, &perm).await);
}

/// Validates `RBACFixture::new` behavior for the performance with fixtures
/// scenario.
///
/// Assertion coverage: ensures the routine completes without panicking.
/// Exercises the performance with fixtures integration scenario end-to-end.
#[tokio::test]
async fn test_performance_with_fixtures() {
    let rbac = RBACFixture::new();
    let admin = UserContextFixture::admin("perf_admin");

    // Generate test data
    let permissions = generate_test_permissions(100);

    // Measure performance
    let perf = PerformanceMeasurement::start("100 permission checks");

    for perm in &permissions {
        let _result = rbac.manager().check_permission(&admin, perm).await;
    }

    perf.assert_below(Duration::from_secs(1));
}

/// Validates the generate test data scenario.
///
/// Assertions:
/// - Confirms `perms.len()` equals `10`.
/// - Confirms `users.len()` equals `5`.
/// - Ensures `users[0].roles.contains(&"admin".to_string())` evaluates to true.
#[test]
fn test_generate_test_data() {
    let perms = generate_test_permissions(10);
    assert_eq!(perms.len(), 10);

    let users = generate_test_users(5, "admin");
    assert_eq!(users.len(), 5);
    assert!(users[0].roles.contains(&"admin".to_string()));
}

/// Validates `RBACFixture::new` behavior for the multiple users with fixtures
/// scenario.
///
/// Assertions:
/// - Ensures `rbac.manager().check_permission(user, &perm).await` evaluates to
///   true.
/// Exercises the multiple users with fixtures integration scenario end-to-end.
#[tokio::test]
async fn test_multiple_users_with_fixtures() {
    let rbac = RBACFixture::new();

    // Generate multiple test users
    let users = generate_test_users(10, "user");
    let perm = PermissionFixture::menu_view();

    // Test all users have the permission
    for user in &users {
        assert!(rbac.manager().check_permission(user, &perm).await);
    }
}

/// Validates `EncryptionKeyFixture::direct_source` behavior for the security
/// event with fixtures scenario.
///
/// Assertions:
/// - Ensures `is_cached()` evaluates to true.
/// - Ensures `stats.is_cached` evaluates to true.
/// Exercises the security event with fixtures integration scenario end-to-end.
#[test]
fn test_security_event_with_fixtures() {
    let key_source = EncryptionKeyFixture::direct_source("security_event");

    let _key = get_or_create_key_cached(&key_source).unwrap();
    assert!(is_cached());

    clear_cache_on_security_event("test_security_event");

    let stats = get_cache_stats();
    assert!(stats.is_cached);
}
