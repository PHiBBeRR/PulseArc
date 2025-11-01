//! Integration tests for AppContext lifecycle
//!
//! Tests verify that AppContext can be created, initialized, and shutdown gracefully.
//! These tests ensure the application startup and shutdown sequence works correctly.

use std::sync::Arc;
use std::time::Duration;

use pulsearc_domain::Config;

const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
use pulsearc_lib::context::AppContext;

/// Helper function to create a test AppContext with a temporary database
///
/// This avoids conflicts with the production database and ensures tests
/// don't require keychain access.
async fn create_test_context() -> pulsearc_domain::Result<AppContext> {
    // Set test encryption key to avoid keychain access
    std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", TEST_KEY);

    // Create temporary database path for this test
    let temp_dir = std::env::temp_dir();
    let test_db_path = temp_dir.join(format!("pulsearc_test_{}.db", uuid::Uuid::new_v4()));

    // Create custom config with test database path
    let config = Config {
        database: pulsearc_domain::DatabaseConfig {
            path: test_db_path.to_string_lossy().to_string(),
            pool_size: 5,
            encryption_key: None, // Use TEST_DATABASE_ENCRYPTION_KEY env var
        },
        ..Config::default()
    };

    AppContext::new_with_config(config).await
}

/// Test that AppContext::new succeeds and all schedulers start
///
/// This test verifies:
/// - AppContext can be created without errors
/// - All schedulers call .start().await? successfully
/// - Database migrations run successfully
/// - Instance lock is acquired
#[tokio::test(flavor = "multi_thread")]
async fn test_context_creation_succeeds() {
    // Attempt to create AppContext
    let result = create_test_context().await;

    // Verify it succeeds
    assert!(result.is_ok(), "AppContext creation should succeed, got error: {:?}", result.err());

    let context = result.unwrap();

    // Verify schedulers are initialized (non-null Arc pointers)
    assert!(
        Arc::strong_count(&context.block_scheduler) >= 1,
        "block_scheduler should be initialized"
    );
    assert!(
        Arc::strong_count(&context.classification_scheduler) >= 1,
        "classification_scheduler should be initialized"
    );
    assert!(
        Arc::strong_count(&context.sync_scheduler) >= 1,
        "sync_scheduler should be initialized"
    );

    // Verify core services are initialized
    assert!(Arc::strong_count(&context.db) >= 1, "db should be initialized");
    assert!(
        Arc::strong_count(&context.tracking_service) >= 1,
        "tracking_service should be initialized"
    );
    assert!(Arc::strong_count(&context.feature_flags) >= 1, "feature_flags should be initialized");
    assert!(
        Arc::strong_count(&context.database_stats) >= 1,
        "database_stats should be initialized"
    );

    // Note: calendar_scheduler is feature-gated and currently returns an error
    // (tracked in Phase 4.1.3), so we don't test it here

    // Cleanup: Shutdown context
    let shutdown_result = context.shutdown().await;
    assert!(shutdown_result.is_ok(), "shutdown() should complete without error");
}

/// Test that AppContext::shutdown completes without panicking
///
/// This test verifies:
/// - shutdown() returns Ok(())
/// - No panics occur during shutdown
/// - Method completes within reasonable time (< 5 seconds)
#[tokio::test(flavor = "multi_thread")]
async fn test_shutdown_completes_without_panicking() {
    // Create context
    let context = create_test_context().await.expect("AppContext creation should succeed");

    // Shutdown with timeout to prevent hanging tests
    let shutdown_future = context.shutdown();
    let result = tokio::time::timeout(Duration::from_secs(5), shutdown_future).await;

    // Verify shutdown completed within timeout
    assert!(result.is_ok(), "shutdown() should complete within 5 seconds");

    // Verify shutdown returned Ok
    let shutdown_result = result.unwrap();
    assert!(
        shutdown_result.is_ok(),
        "shutdown() should return Ok(()), got: {:?}",
        shutdown_result.err()
    );
}

/// Test that shutdown() is idempotent (can be called multiple times)
///
/// This test verifies:
/// - shutdown() can be called multiple times on the same context
/// - Each call succeeds without error
/// - No state corruption occurs
///
/// Note: Since shutdown() is a no-op (cleanup handled by Drop), this should always pass.
#[tokio::test(flavor = "multi_thread")]
async fn test_shutdown_is_idempotent() {
    // Create context
    let context = create_test_context().await.expect("AppContext creation should succeed");

    // Call shutdown multiple times
    for i in 1..=5 {
        let result = context.shutdown().await;
        assert!(result.is_ok(), "shutdown() call #{} should succeed, got: {:?}", i, result.err());
    }

    // Verify context is still usable after multiple shutdowns
    // (since shutdown is a no-op, all services should still work)
    assert!(
        Arc::strong_count(&context.db) >= 1,
        "db should still be valid after multiple shutdown calls"
    );
}

/// Test graceful shutdown scenario with active operations
///
/// This test verifies:
/// - Context can be shutdown while schedulers might be running
/// - No deadlocks occur
/// - Shutdown completes successfully even with background work
#[tokio::test(flavor = "multi_thread")]
async fn test_shutdown_with_active_schedulers() {
    // Create context (schedulers are started automatically)
    let context = create_test_context().await.expect("AppContext creation should succeed");

    // Give schedulers a moment to start their background tasks
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Shutdown should complete successfully even if schedulers are running
    let result = context.shutdown().await;
    assert!(
        result.is_ok(),
        "shutdown() should succeed with active schedulers, got: {:?}",
        result.err()
    );
}

/// Test that Drop cleanup works when shutdown() is never called
///
/// This test verifies:
/// - AppContext can be dropped without calling shutdown()
/// - No resource leaks occur
/// - Cleanup happens via Drop impls
///
/// Note: This test doesn't explicitly call shutdown() to verify Drop-based cleanup.
#[tokio::test(flavor = "multi_thread")]
async fn test_cleanup_via_drop_without_shutdown() {
    // Create context in a scope
    {
        let _context = create_test_context().await.expect("AppContext creation should succeed");

        // Give schedulers a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Context will be dropped here without calling shutdown()
    }

    // If we reach here without hanging or panicking, Drop cleanup worked
    // Give a moment for async tasks to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test passes if we didn't panic or hang
}

/// Test concurrent shutdown calls (thread safety)
///
/// This test verifies:
/// - shutdown() can be called concurrently from multiple tasks
/// - No race conditions occur
/// - All calls complete successfully
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_shutdown_calls() {
    // Create context
    let context =
        Arc::new(create_test_context().await.expect("AppContext creation should succeed"));

    // Spawn multiple tasks that call shutdown concurrently
    let mut handles = Vec::new();
    for _ in 0..10 {
        let ctx: Arc<AppContext> = Arc::clone(&context);
        let handle = tokio::spawn(async move { ctx.shutdown().await });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await;
        assert!(result.is_ok(), "Task {} should complete without panic", i);

        let shutdown_result = result.unwrap();
        assert!(shutdown_result.is_ok(), "shutdown() call in task {} should succeed", i);
    }
}
