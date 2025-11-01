//! Integration tests for database commands infrastructure (Phase 4A.1)
//!
//! These tests verify that the database infrastructure (ports, repositories,
//! adapters) work correctly. The actual Tauri command wrappers are validated
//! via manual testing with the running application.

use pulsearc_common::testing::TempDir;
use pulsearc_domain::Config;
use pulsearc_lib::adapters::database_stats::build_database_stats;
use pulsearc_lib::context::AppContext;

const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Helper function to create a test AppContext with a temporary database
///
/// Uses `TempDir` from common testing utilities for automatic cleanup.
/// The returned context and temp directory must be kept alive for the duration
/// of the test to prevent premature cleanup.
async fn create_test_context() -> pulsearc_domain::Result<(AppContext, TempDir)> {
    // Set test encryption key to avoid keychain access
    std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", TEST_KEY);

    // Create temporary database directory with auto-cleanup
    let temp_dir =
        TempDir::new("pulsearc-test").expect("failed to create temporary test directory");

    let test_db_path = temp_dir.path().join("pulsearc.db");
    let lock_dir = temp_dir.create_dir("lock").expect("failed to create lock directory");

    // Create custom config with test database path
    let config = Config {
        database: pulsearc_domain::DatabaseConfig {
            path: test_db_path.to_string_lossy().to_string(),
            pool_size: 5,
            encryption_key: None, // Use TEST_DATABASE_ENCRYPTION_KEY env var
        },
        ..Config::default()
    };

    let ctx = AppContext::new_with_config_in_lock_dir(config, lock_dir).await?;

    // Return both context and temp_dir to keep temp_dir alive
    Ok((ctx, temp_dir))
}

// =============================================================================
// DatabaseStatsPort Tests
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_database_stats_port_get_database_size() {
    let (ctx, _temp_dir) = create_test_context().await.expect("Failed to create test context");

    // Test get_database_size via port
    let result = ctx.database_stats.get_database_size().await;

    assert!(result.is_ok(), "get_database_size failed: {:?}", result);

    let size = result.unwrap();
    assert!(size.size_bytes > 0);
    assert!(size.page_count > 0);
    assert!(size.page_size > 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_database_stats_port_get_table_stats() {
    let (ctx, _temp_dir) = create_test_context().await.expect("Failed to create test context");

    // Test get_table_stats via port
    let result = ctx.database_stats.get_table_stats().await;

    assert!(result.is_ok(), "get_table_stats failed: {:?}", result);

    let stats = result.unwrap();
    // Should have multiple tables (migrations create schema)
    assert!(!stats.is_empty(), "Expected multiple tables");

    // Check for key tables
    let has_snapshots = stats.iter().any(|t| t.name == "activity_snapshots");
    let has_segments = stats.iter().any(|t| t.name == "activity_segments");
    assert!(has_snapshots, "Expected activity_snapshots table");
    assert!(has_segments, "Expected activity_segments table");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_database_stats_port_get_unprocessed_count() {
    let (ctx, _temp_dir) = create_test_context().await.expect("Failed to create test context");

    // Test get_unprocessed_count via port (NEW METHOD)
    let result = ctx.database_stats.get_unprocessed_count().await;

    assert!(result.is_ok(), "get_unprocessed_count failed: {:?}", result);

    let count = result.unwrap();
    // New database should have 0 unprocessed snapshots
    assert_eq!(count, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_database_stats_port_vacuum_database() {
    let (ctx, _temp_dir) = create_test_context().await.expect("Failed to create test context");

    // Test vacuum_database via port
    let result = ctx.database_stats.vacuum_database().await;

    assert!(result.is_ok(), "vacuum_database failed: {:?}", result);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_database_stats_port_check_database_health() {
    let (ctx, _temp_dir) = create_test_context().await.expect("Failed to create test context");

    // Test check_database_health via port
    let result = ctx.database_stats.check_database_health().await;

    assert!(result.is_ok(), "check_database_health failed: {:?}", result);

    let health = result.unwrap();
    assert!(health.is_healthy, "Database should be healthy");
    assert_eq!(health.message, "Database is healthy");
    assert!(health.response_time_ms < 1000, "Health check should be fast");
}

// =============================================================================
// Adapter Tests
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_database_stats_adapter_builds_legacy_type() {
    let (ctx, _temp_dir) = create_test_context().await.expect("Failed to create test context");

    // Test adapter: converts new granular port methods → legacy DatabaseStats
    let result = build_database_stats(&ctx.database_stats).await;

    assert!(result.is_ok(), "build_database_stats failed: {:?}", result);

    let stats = result.unwrap();
    // New database should have 0 snapshots/segments
    assert_eq!(stats.snapshot_count, 0);
    assert_eq!(stats.unprocessed_count, 0);
    assert_eq!(stats.segment_count, 0);

    // batch_stats should be empty struct (not used by frontend)
    assert_eq!(stats.batch_stats.pending, 0);
    assert_eq!(stats.batch_stats.processing, 0);
    assert_eq!(stats.batch_stats.completed, 0);
    assert_eq!(stats.batch_stats.failed, 0);
}

// =============================================================================
// Feature Flag Tests
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_feature_flag_can_be_toggled() {
    let (ctx, _temp_dir) = create_test_context().await.expect("Failed to create test context");

    // Test feature flag: initially disabled
    let initial = ctx
        .feature_flags
        .is_enabled("new_database_commands", false)
        .await
        .expect("Failed to check flag");
    assert!(!initial, "Flag should be disabled by default");

    // Enable flag
    ctx.feature_flags
        .set_enabled("new_database_commands", true)
        .await
        .expect("Failed to enable flag");

    // Verify enabled
    let enabled = ctx
        .feature_flags
        .is_enabled("new_database_commands", false)
        .await
        .expect("Failed to check flag");
    assert!(enabled, "Flag should be enabled after set_enabled");

    // Disable flag
    ctx.feature_flags
        .set_enabled("new_database_commands", false)
        .await
        .expect("Failed to disable flag");

    // Verify disabled
    let disabled = ctx
        .feature_flags
        .is_enabled("new_database_commands", false)
        .await
        .expect("Failed to check flag");
    assert!(!disabled, "Flag should be disabled after disabling");
}

// =============================================================================
// Snapshot Repository Tests
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_snapshot_repository_wired_to_context() {
    let (ctx, _temp_dir) = create_test_context().await.expect("Failed to create test context");

    // Verify snapshots repository is wired to AppContext
    // The snapshots field should exist and be accessible
    // (actual repository methods are tested in
    // infra/database/activity_repository.rs)

    // Simple check: verify the field exists by accessing it
    let _snapshots_ref = &ctx.snapshots;

    // If we got here without panic, the wiring is correct
    // (no explicit assertion needed - test passes if it doesn't panic)
}
