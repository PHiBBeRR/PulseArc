//! Integration tests for user profile commands infrastructure (Phase 4A.2)
//!
//! These tests verify that the user profile infrastructure (ports,
//! repositories) work correctly. A subset also exercises the Tauri command
//! wrappers to validate feature flag routing.

use std::sync::Arc;

use chrono::Utc;
use pulsearc_common::testing::TempDir;
use pulsearc_domain::{Config, DatabaseConfig, UserProfile};
use pulsearc_lib::AppContext;

const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Helper to create a test context with a unique database
///
/// Uses `TempDir` from common testing utilities for automatic cleanup.
/// Returns both the context and temp directory to keep temp_dir alive.
async fn create_test_context() -> (Arc<AppContext>, TempDir) {
    // Set test encryption key to avoid keychain access
    std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", TEST_KEY);

    // Create temporary database directory with auto-cleanup
    let temp_dir =
        TempDir::new("pulsearc-test").expect("failed to create temporary test directory");

    let test_db_path = temp_dir.path().join("pulsearc.db");
    let lock_dir = temp_dir.create_dir("lock").expect("failed to create lock directory");

    // Create custom config with test database path
    let config = Config {
        database: DatabaseConfig {
            path: test_db_path.to_string_lossy().to_string(),
            pool_size: 5,
            encryption_key: None, // Use TEST_DATABASE_ENCRYPTION_KEY env var
        },
        ..Config::default()
    };

    let ctx = AppContext::new_with_config_in_lock_dir(config, lock_dir)
        .await
        .expect("failed to create test context");

    (Arc::new(ctx), temp_dir)
}

/// Helper to create a test profile
fn create_test_profile() -> UserProfile {
    let now = Utc::now().timestamp();
    UserProfile {
        id: "test-profile-123".to_string(),
        auth0_id: "auth0|test123".to_string(),
        email: "test@pulsearc.com".to_string(),
        name: Some("Test User".to_string()),
        first_name: Some("Test".to_string()),
        last_name: Some("User".to_string()),
        display_name: Some("Test U.".to_string()),
        avatar_url: Some("https://example.com/avatar.jpg".to_string()),
        phone_number: Some("+1234567890".to_string()),
        title: Some("Engineer".to_string()),
        department: Some("Engineering".to_string()),
        location: Some("San Francisco".to_string()),
        bio: Some("Test bio".to_string()),
        timezone: "America/Los_Angeles".to_string(),
        language: "en".to_string(),
        locale: "en-US".to_string(),
        date_format: "YYYY-MM-DD".to_string(),
        is_active: true,
        email_verified: true,
        two_factor_enabled: false,
        last_login_at: now,
        last_synced_at: now,
        created_at: now,
        updated_at: now,
    }
}

// =============================================================================
// UserProfileRepository Tests
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_user_profile_port_get_by_id_returns_none_when_empty() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Test get_by_id on empty database
    let result = ctx.user_profile.get_by_id("nonexistent").await;

    assert!(result.is_ok(), "get_by_id failed: {:?}", result);
    assert!(result.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_user_profile_port_create_and_get_by_id() {
    let (ctx, _temp_dir) = create_test_context().await;
    let profile = create_test_profile();

    // Create profile
    let result = ctx.user_profile.create(profile.clone()).await;
    assert!(result.is_ok(), "create failed: {:?}", result);

    // Get by ID
    let result = ctx.user_profile.get_by_id(&profile.id).await;
    assert!(result.is_ok(), "get_by_id failed: {:?}", result);

    let retrieved = result.unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, profile.id);
    assert_eq!(retrieved.email, profile.email);
    assert_eq!(retrieved.name, profile.name);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_user_profile_port_get_by_auth0_id() {
    let (ctx, _temp_dir) = create_test_context().await;
    let profile = create_test_profile();

    // Create profile
    ctx.user_profile.create(profile.clone()).await.expect("create failed");

    // Get by Auth0 ID
    let result = ctx.user_profile.get_by_auth0_id(&profile.auth0_id).await;
    assert!(result.is_ok(), "get_by_auth0_id failed: {:?}", result);

    let retrieved = result.unwrap().unwrap();
    assert_eq!(retrieved.auth0_id, profile.auth0_id);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_user_profile_port_get_by_email() {
    let (ctx, _temp_dir) = create_test_context().await;
    let profile = create_test_profile();

    // Create profile
    ctx.user_profile.create(profile.clone()).await.expect("create failed");

    // Get by email
    let result = ctx.user_profile.get_by_email(&profile.email).await;
    assert!(result.is_ok(), "get_by_email failed: {:?}", result);

    let retrieved = result.unwrap().unwrap();
    assert_eq!(retrieved.email, profile.email);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_user_profile_port_update() {
    let (ctx, _temp_dir) = create_test_context().await;
    let mut profile = create_test_profile();

    // Create profile
    ctx.user_profile.create(profile.clone()).await.expect("create failed");

    // Update profile
    profile.name = Some("Updated Name".to_string());
    profile.email = "updated@pulsearc.com".to_string();
    profile.updated_at = Utc::now().timestamp();

    let result = ctx.user_profile.update(profile.clone()).await;
    assert!(result.is_ok(), "update failed: {:?}", result);

    // Verify update
    let retrieved = ctx
        .user_profile
        .get_by_id(&profile.id)
        .await
        .expect("get_by_id failed")
        .expect("profile not found");

    assert_eq!(retrieved.name, Some("Updated Name".to_string()));
    assert_eq!(retrieved.email, "updated@pulsearc.com");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_user_profile_port_delete() {
    let (ctx, _temp_dir) = create_test_context().await;
    let profile = create_test_profile();

    // Create profile
    ctx.user_profile.create(profile.clone()).await.expect("create failed");

    // Delete profile
    let result = ctx.user_profile.delete(&profile.id).await;
    assert!(result.is_ok(), "delete failed: {:?}", result);

    // Verify deletion
    let retrieved = ctx.user_profile.get_by_id(&profile.id).await.expect("get_by_id failed");
    assert!(retrieved.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_user_profile_persistence_all_fields() {
    let (ctx, _temp_dir) = create_test_context().await;
    let profile = create_test_profile();

    // Create profile
    ctx.user_profile.create(profile.clone()).await.expect("create failed");

    // Retrieve and verify all fields
    let result = ctx
        .user_profile
        .get_by_id(&profile.id)
        .await
        .expect("get_by_id failed")
        .expect("profile not found");

    assert_eq!(result.id, profile.id);
    assert_eq!(result.auth0_id, profile.auth0_id);
    assert_eq!(result.email, profile.email);
    assert_eq!(result.name, profile.name);
    assert_eq!(result.first_name, profile.first_name);
    assert_eq!(result.last_name, profile.last_name);
    assert_eq!(result.display_name, profile.display_name);
    assert_eq!(result.avatar_url, profile.avatar_url);
    assert_eq!(result.phone_number, profile.phone_number);
    assert_eq!(result.title, profile.title);
    assert_eq!(result.department, profile.department);
    assert_eq!(result.location, profile.location);
    assert_eq!(result.bio, profile.bio);
    assert_eq!(result.timezone, profile.timezone);
    assert_eq!(result.language, profile.language);
    assert_eq!(result.locale, profile.locale);
    assert_eq!(result.date_format, profile.date_format);
    assert_eq!(result.is_active, profile.is_active);
    assert_eq!(result.email_verified, profile.email_verified);
    assert_eq!(result.two_factor_enabled, profile.two_factor_enabled);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_user_profile_boolean_fields() {
    let (ctx, _temp_dir) = create_test_context().await;
    let mut profile = create_test_profile();

    // Test with different boolean combinations
    profile.is_active = false;
    profile.email_verified = false;
    profile.two_factor_enabled = true;

    ctx.user_profile.create(profile.clone()).await.expect("create failed");

    let retrieved =
        ctx.user_profile.get_by_id(&profile.id).await.expect("get_by_id failed").unwrap();

    assert!(!retrieved.is_active);
    assert!(!retrieved.email_verified);
    assert!(retrieved.two_factor_enabled);
}

// =============================================================================
// New Port Methods Tests (Phase 4A.2 corrections)
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_get_current_profile_returns_none_when_empty() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Test get_current_profile on empty database
    let result = ctx.user_profile.get_current_profile().await;

    assert!(result.is_ok(), "get_current_profile failed: {:?}", result);
    assert!(result.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_current_profile_ordering_semantics() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create three profiles with different created_at timestamps
    let mut profile1 = create_test_profile();
    profile1.id = "profile-1".to_string();
    profile1.auth0_id = "auth0|user1".to_string();
    profile1.email = "user1@pulsearc.com".to_string();
    profile1.created_at = Utc::now().timestamp() - 100; // Oldest

    let mut profile2 = create_test_profile();
    profile2.id = "profile-2".to_string();
    profile2.auth0_id = "auth0|user2".to_string();
    profile2.email = "user2@pulsearc.com".to_string();
    profile2.created_at = Utc::now().timestamp() - 50; // Middle

    let mut profile3 = create_test_profile();
    profile3.id = "profile-3".to_string();
    profile3.auth0_id = "auth0|user3".to_string();
    profile3.email = "user3@pulsearc.com".to_string();
    profile3.created_at = Utc::now().timestamp(); // Newest

    // Create in non-chronological order to verify ordering
    ctx.user_profile.create(profile2.clone()).await.expect("create profile2 failed");
    ctx.user_profile.create(profile1.clone()).await.expect("create profile1 failed");
    ctx.user_profile.create(profile3.clone()).await.expect("create profile3 failed");

    // get_current_profile should return the OLDEST profile (first created)
    let result = ctx.user_profile.get_current_profile().await.expect("get_current_profile failed");

    assert!(result.is_some(), "get_current_profile returned None");
    let current = result.unwrap();
    assert_eq!(
        current.id, profile1.id,
        "get_current_profile should return oldest profile (first by created_at)"
    );
    assert_eq!(current.email, "user1@pulsearc.com");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_upsert_insert_path_new_auth0_id() {
    let (ctx, _temp_dir) = create_test_context().await;
    let profile = create_test_profile();

    // Upsert into empty database (insert path)
    let result = ctx.user_profile.upsert(profile.clone()).await;
    assert!(result.is_ok(), "upsert (insert) failed: {:?}", result);

    // Verify profile was created
    let retrieved = ctx
        .user_profile
        .get_by_auth0_id(&profile.auth0_id)
        .await
        .expect("get_by_auth0_id failed")
        .expect("profile not found");

    assert_eq!(retrieved.id, profile.id);
    assert_eq!(retrieved.email, profile.email);
    assert_eq!(retrieved.name, profile.name);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_upsert_update_path_same_auth0_id() {
    let (ctx, _temp_dir) = create_test_context().await;
    let mut profile = create_test_profile();

    // Initial upsert (insert)
    ctx.user_profile.upsert(profile.clone()).await.expect("first upsert failed");

    // Verify initial state
    let initial = ctx
        .user_profile
        .get_by_auth0_id(&profile.auth0_id)
        .await
        .expect("get_by_auth0_id failed")
        .expect("profile not found");
    assert_eq!(initial.name, Some("Test User".to_string()));

    // Second upsert with SAME auth0_id but different data (update path)
    profile.name = Some("Updated via Upsert".to_string());
    profile.email = "updated@pulsearc.com".to_string();
    profile.location = Some("New York".to_string());
    profile.updated_at = Utc::now().timestamp();

    let result = ctx.user_profile.upsert(profile.clone()).await;
    assert!(result.is_ok(), "upsert (update) failed: {:?}", result);

    // Verify profile was UPDATED, not duplicated
    let updated = ctx
        .user_profile
        .get_by_auth0_id(&profile.auth0_id)
        .await
        .expect("get_by_auth0_id failed")
        .expect("profile not found");

    assert_eq!(updated.id, profile.id, "ID should remain the same");
    assert_eq!(updated.auth0_id, profile.auth0_id);
    assert_eq!(updated.name, Some("Updated via Upsert".to_string()));
    assert_eq!(updated.email, "updated@pulsearc.com");
    assert_eq!(updated.location, Some("New York".to_string()));

    // Verify only ONE profile exists with this auth0_id
    let all_profiles_count =
        ctx.user_profile.get_by_auth0_id(&profile.auth0_id).await.expect("get_by_auth0_id failed");
    assert!(all_profiles_count.is_some(), "Profile should exist after upsert");
}

// =============================================================================
// Command-Level Tests (Feature Flag Routing)
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_get_user_profile_command_new_path_returns_profile() {
    let (ctx, _temp_dir) = create_test_context().await;
    let profile = create_test_profile();

    // Seed profile via repository
    ctx.user_profile.create(profile.clone()).await.expect("create failed");

    // Enable new command path
    ctx.feature_flags
        .set_enabled("new_user_profile_commands", true)
        .await
        .expect("failed to enable feature flag");

    let result = pulsearc_lib::commands::user_profile::new_get_user_profile(ctx.as_ref()).await;
    assert!(result.is_ok(), "get_user_profile command failed: {:?}", result);

    let retrieved = result.unwrap();
    assert!(retrieved.is_some(), "expected profile to be returned");
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, profile.id);
    assert_eq!(retrieved.email, profile.email);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_user_profile_command_legacy_path_returns_profile() {
    let (ctx, _temp_dir) = create_test_context().await;
    let profile = create_test_profile();

    ctx.user_profile.create(profile.clone()).await.expect("create failed");

    // Explicitly disable flag to force legacy path
    ctx.feature_flags
        .set_enabled("new_user_profile_commands", false)
        .await
        .expect("failed to disable feature flag");

    let result = pulsearc_lib::commands::user_profile::legacy_get_user_profile(ctx.as_ref()).await;
    assert!(result.is_ok(), "get_user_profile command failed: {:?}", result);

    let retrieved = result.unwrap();
    assert!(retrieved.is_some(), "expected profile to be returned");
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, profile.id);
    assert_eq!(retrieved.email, profile.email);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_upsert_user_profile_command_new_path_updates_in_place() {
    let (ctx, _temp_dir) = create_test_context().await;
    let mut profile = create_test_profile();

    ctx.feature_flags
        .set_enabled("new_user_profile_commands", true)
        .await
        .expect("failed to enable feature flag");

    // Insert via command
    let insert_result = pulsearc_lib::commands::user_profile::new_upsert_user_profile(
        ctx.as_ref(),
        profile.clone(),
    )
    .await;
    assert!(insert_result.is_ok(), "upsert_user_profile (insert) failed: {:?}", insert_result);

    // Update via command
    profile.name = Some("Updated via command".to_string());
    profile.email = "updated-command@pulsearc.com".to_string();
    let update_result = pulsearc_lib::commands::user_profile::new_upsert_user_profile(
        ctx.as_ref(),
        profile.clone(),
    )
    .await;
    assert!(update_result.is_ok(), "upsert_user_profile (update) failed: {:?}", update_result);

    let stored = ctx
        .user_profile
        .get_by_auth0_id(&profile.auth0_id)
        .await
        .expect("get_by_auth0_id failed")
        .expect("expected profile after upsert");
    assert_eq!(stored.name, Some("Updated via command".to_string()));
    assert_eq!(stored.email, "updated-command@pulsearc.com");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_upsert_user_profile_command_legacy_path_updates_in_place() {
    let (ctx, _temp_dir) = create_test_context().await;
    let mut profile = create_test_profile();

    ctx.feature_flags
        .set_enabled("new_user_profile_commands", false)
        .await
        .expect("failed to disable feature flag");

    // Insert via command (legacy path)
    let insert_result = pulsearc_lib::commands::user_profile::legacy_upsert_user_profile(
        ctx.as_ref(),
        profile.clone(),
    )
    .await;
    assert!(
        insert_result.is_ok(),
        "upsert_user_profile (legacy insert) failed: {:?}",
        insert_result
    );

    // Update via command (legacy path)
    profile.name = Some("Updated via legacy command".to_string());
    profile.email = "updated-legacy-command@pulsearc.com".to_string();
    let update_result = pulsearc_lib::commands::user_profile::legacy_upsert_user_profile(
        ctx.as_ref(),
        profile.clone(),
    )
    .await;
    assert!(
        update_result.is_ok(),
        "upsert_user_profile (legacy update) failed: {:?}",
        update_result
    );

    let stored = ctx
        .user_profile
        .get_by_auth0_id(&profile.auth0_id)
        .await
        .expect("get_by_auth0_id failed")
        .expect("expected profile after legacy upsert");
    assert_eq!(stored.name, Some("Updated via legacy command".to_string()));
    assert_eq!(stored.email, "updated-legacy-command@pulsearc.com");
}
