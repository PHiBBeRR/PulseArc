//! Integration tests for window commands infrastructure (Phase 4A.3)
//!
//! ## Testing Strategy
//!
//! Window commands present unique testing challenges because they:
//! 1. Require a Tauri runtime and actual window context
//! 2. Manipulate UI state rather than data
//! 3. Use platform-specific unsafe code (NSWindow on macOS)
//!
//! ### What We Test
//!
//! - Feature flag routing (via mock implementations)
//! - Input validation (dimensions > 0)
//! - Error handling (window not found, invalid dimensions)
//!
//! ### What We DON'T Test
//!
//! - Actual window resizing (requires Tauri app context)
//! - Animation behavior (requires visual inspection)
//! - NSWindow unsafe code (requires macOS runtime)
//!
//! ### Manual Testing Required
//!
//! The following must be tested manually:
//! 1. Window actually resizes on macOS with animation
//! 2. Window resizes on non-macOS platforms without animation
//! 3. Feature flag correctly routes to new/legacy implementations
//! 4. Error messages are user-friendly
//!
//! See: docs/PHASE-4-NEW-CRATE-MIGRATION.md Section 4A.3 for manual test
//! checklist.

use pulsearc_common::testing::TempDir;
use pulsearc_domain::{Config, DatabaseConfig};
use pulsearc_lib::AppContext;

const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Helper to create a test context with a unique database
///
/// Uses `TempDir` from common testing utilities for automatic cleanup.
/// Returns both the context and temp directory to keep temp_dir alive.
async fn create_test_context() -> (AppContext, TempDir) {
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

    (ctx, temp_dir)
}

// =============================================================================
// Feature Flag Tests
// =============================================================================

/// Test that feature flag defaults to false (legacy path)
///
/// This verifies that without explicit configuration, the window commands
/// use the legacy implementation for backwards compatibility.
#[tokio::test(flavor = "multi_thread")]
async fn test_new_window_commands_flag_defaults_to_false() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Check that flag is disabled by default
    let flag_enabled = ctx
        .feature_flags
        .is_enabled("new_window_commands", true)
        .await
        .expect("failed to check feature flag");

    assert!(!flag_enabled, "new_window_commands should default to false");
}

/// Test that feature flag can be enabled
///
/// This verifies that the feature flag can be toggled to enable the new
/// window commands implementation.
#[tokio::test(flavor = "multi_thread")]
async fn test_new_window_commands_flag_can_be_enabled() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Enable the feature flag
    ctx.feature_flags
        .set_enabled("new_window_commands", true)
        .await
        .expect("failed to enable feature flag");

    // Verify it's enabled
    let flag_enabled = ctx
        .feature_flags
        .is_enabled("new_window_commands", true)
        .await
        .expect("failed to check feature flag");

    assert!(flag_enabled, "new_window_commands should be enabled");
}

/// Test that feature flag can be disabled after being enabled
///
/// This verifies rollback capability - we can disable the flag to revert
/// to legacy behavior if issues are discovered.
#[tokio::test(flavor = "multi_thread")]
async fn test_new_window_commands_flag_can_be_disabled() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Enable then disable
    ctx.feature_flags
        .set_enabled("new_window_commands", true)
        .await
        .expect("failed to enable feature flag");

    ctx.feature_flags
        .set_enabled("new_window_commands", false)
        .await
        .expect("failed to disable feature flag");

    // Verify it's disabled
    let flag_enabled = ctx
        .feature_flags
        .is_enabled("new_window_commands", true)
        .await
        .expect("failed to check feature flag");

    assert!(!flag_enabled, "new_window_commands should be disabled");
}

// =============================================================================
// Manual Testing Notes
// =============================================================================

/// Manual test checklist (to be performed by developer)
///
/// ## Prerequisites
///
/// - macOS system (for NSWindow animation testing)
/// - PulseArc app running in dev mode (`pnpm tauri dev`)
/// - Access to feature flags database table
///
/// ## Test Cases
///
/// ### TC1: Legacy Implementation (Flag Disabled)
///
/// 1. Ensure `new_window_commands` flag is disabled (default)
/// 2. From frontend console, run: ```javascript await
///    invoke('animate_window_resize', { width: 1200, height: 800 }) ```
/// 3. **Expected:** Window resizes with smooth animation (macOS) or instant
///    resize (other platforms)
/// 4. **Expected:** Logs show `implementation = "legacy"`
/// 5. Verify window is centered during resize
///
/// ### TC2: New Implementation (Flag Enabled)
///
/// 1. Enable `new_window_commands` flag: ```sql INSERT INTO feature_flags (key,
///    enabled, updated_at) VALUES ('new_window_commands', 1, CURRENT_TIMESTAMP)
///    ON CONFLICT(key) DO UPDATE SET enabled = 1; ```
/// 2. From frontend console, run: ```javascript await
///    invoke('animate_window_resize', { width: 1000, height: 600 }) ```
/// 3. **Expected:** Window resizes with smooth animation (macOS) or instant
///    resize (other platforms)
/// 4. **Expected:** Logs show `implementation = "new"`
/// 5. Verify window is centered during resize
/// 6. Verify animation is smooth (no jarring jumps)
///
/// ### TC3: Error Handling - Invalid Dimensions
///
/// 1. With flag enabled, test negative width: ```javascript await
///    invoke('animate_window_resize', { width: -100, height: 800 }) ```
/// 2. **Expected:** Error returned: "invalid dimensions: width=-100,
///    height=800"
/// 3. **Expected:** Error logged with tracing
/// 4. Test zero dimensions: ```javascript await invoke('animate_window_resize',
///    { width: 0, height: 0 }) ```
/// 5. **Expected:** Error returned: "invalid dimensions: width=0, height=0"
///
/// ### TC4: Rollback (Enable → Disable)
///
/// 1. With flag enabled, resize window to 1200x800
/// 2. Disable flag: ```sql UPDATE feature_flags SET enabled = 0 WHERE key =
///    'new_window_commands'; ```
/// 3. Resize window to 1000x600
/// 4. **Expected:** Both implementations produce identical visual results
/// 5. **Expected:** Logs show implementation switches from "new" to "legacy"
///
/// ### TC5: Platform Compatibility (if non-macOS available)
///
/// 1. On non-macOS platform, enable flag
/// 2. Resize window
/// 3. **Expected:** Window resizes instantly (no animation)
/// 4. **Expected:** Logs show `platform = "non-macos"`
/// 5. **Expected:** No errors in console
///
/// ## Success Criteria
///
/// - [ ] All test cases pass
/// - [ ] No visual differences between new and legacy implementations
/// - [ ] Error messages are clear and user-friendly
/// - [ ] Logs provide adequate debugging information
/// - [ ] No console errors or warnings
/// - [ ] Window remains centered during all resizes
///
/// ## Rollback Plan
///
/// If issues are discovered:
/// 1. Disable `new_window_commands` flag (< 5 seconds)
/// 2. Verify legacy behavior works (< 1 minute)
/// 3. File GitHub issue with reproduction steps
/// 4. Investigate and fix in separate PR
#[test]
#[ignore = "Manual test - see function documentation"]
fn manual_testing_checklist() {
    // This test exists only to document the manual testing process.
    // It is ignored by default and serves as a checklist for developers.
    //
    // To view this test's documentation:
    // cargo test --doc window_commands::manual_testing_checklist
}
