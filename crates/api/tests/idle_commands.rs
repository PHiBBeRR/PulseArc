//! Integration tests for idle period commands (Phase 4B.3 + 4C.2)
//!
//! These tests verify that the idle period infrastructure (ports, repositories)
//! and telemetry commands work correctly.

use std::sync::Arc;

use chrono::Utc;
use pulsearc_common::testing::TempDir;
use pulsearc_domain::types::database::ActivitySnapshot;
use pulsearc_domain::{Config, DatabaseConfig, IdlePeriod};
use pulsearc_lib::AppContext;

const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Helper to create a test context with a unique database
async fn create_test_context() -> (Arc<AppContext>, TempDir) {
    // Set test encryption key to avoid keychain access
    std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", TEST_KEY);

    // Create temporary database directory with auto-cleanup
    let temp_dir =
        TempDir::new("pulsearc-idle-test").expect("failed to create temporary test directory");

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

/// Helper to create a test idle period
fn create_test_idle_period(id: &str, start_offset: i64, duration: i32) -> IdlePeriod {
    let now = Utc::now().timestamp();
    IdlePeriod {
        id: id.to_string(),
        start_ts: now + start_offset,
        end_ts: now + start_offset + i64::from(duration),
        duration_secs: duration,
        system_trigger: "threshold".to_string(),
        user_action: None,
        threshold_secs: 300,
        created_at: now,
        reviewed_at: None,
        notes: None,
    }
}

/// Helper to create a test activity snapshot
fn create_test_snapshot(id: &str, timestamp: i64, is_idle: bool) -> ActivitySnapshot {
    ActivitySnapshot {
        id: id.to_string(),
        timestamp,
        activity_context_json: serde_json::json!({"app": "TestApp"}).to_string(),
        detected_activity: "working".to_string(),
        work_type: None,
        activity_category: None,
        primary_app: "TestApp".to_string(),
        processed: false,
        batch_id: None,
        created_at: timestamp,
        processed_at: None,
        is_idle,
        idle_duration_secs: if is_idle { Some(0) } else { None },
    }
}

// =============================================================================
// IdlePeriodsRepository Port Tests
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_idle_periods_get_empty_range() {
    let (ctx, _temp_dir) = create_test_context().await;
    let now = Utc::now().timestamp();

    // Query empty database
    let periods = ctx
        .idle_periods
        .get_idle_periods_in_range(now - 1000, now)
        .await
        .expect("get_idle_periods_in_range failed");

    assert_eq!(periods.len(), 0, "expected no idle periods");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_idle_periods_save_and_retrieve() {
    let (ctx, _temp_dir) = create_test_context().await;
    let period = create_test_idle_period("idle-1", -600, 300);

    // Save idle period
    ctx.idle_periods.save_idle_period(period.clone()).await.expect("save_idle_period failed");

    // Retrieve by range
    let periods = ctx
        .idle_periods
        .get_idle_periods_in_range(period.start_ts - 100, period.end_ts + 100)
        .await
        .expect("get_idle_periods_in_range failed");

    assert_eq!(periods.len(), 1, "expected 1 idle period");
    assert_eq!(periods[0].id, period.id);
    assert_eq!(periods[0].duration_secs, 300);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_idle_periods_multiple_results_sorted() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create 3 idle periods at different times
    let period1 = create_test_idle_period("idle-1", -1000, 200);
    let period2 = create_test_idle_period("idle-2", -500, 100);
    let period3 = create_test_idle_period("idle-3", -200, 150);

    ctx.idle_periods.save_idle_period(period1.clone()).await.expect("save period1 failed");
    ctx.idle_periods.save_idle_period(period2.clone()).await.expect("save period2 failed");
    ctx.idle_periods.save_idle_period(period3.clone()).await.expect("save period3 failed");

    // Retrieve all
    let now = Utc::now().timestamp();
    let periods = ctx
        .idle_periods
        .get_idle_periods_in_range(now - 1200, now)
        .await
        .expect("get_idle_periods_in_range failed");

    assert_eq!(periods.len(), 3, "expected 3 idle periods");

    // Verify ascending order by start_ts
    assert_eq!(periods[0].id, "idle-1");
    assert_eq!(periods[1].id, "idle-2");
    assert_eq!(periods[2].id, "idle-3");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_update_idle_period_action_success() {
    let (ctx, _temp_dir) = create_test_context().await;
    let period = create_test_idle_period("idle-1", -600, 300);

    // Save period
    ctx.idle_periods.save_idle_period(period.clone()).await.expect("save failed");

    // Update action
    ctx.idle_periods
        .update_idle_period_action(&period.id, "kept", Some("User decided to keep".to_string()))
        .await
        .expect("update_idle_period_action failed");

    // Retrieve and verify
    let retrieved = ctx
        .idle_periods
        .get_idle_period(&period.id)
        .await
        .expect("get_idle_period failed")
        .expect("period not found");

    assert_eq!(retrieved.user_action, Some("kept".to_string()));
    assert_eq!(retrieved.notes, Some("User decided to keep".to_string()));
    assert!(retrieved.reviewed_at.is_some(), "reviewed_at not set");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_pending_idle_periods() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create periods with different actions
    let mut pending = create_test_idle_period("idle-pending", -600, 300);
    pending.user_action = None;

    let mut kept = create_test_idle_period("idle-kept", -400, 200);
    kept.user_action = Some("kept".to_string());

    let mut pending2 = create_test_idle_period("idle-pending2", -200, 100);
    pending2.user_action = Some("pending".to_string());

    ctx.idle_periods.save_idle_period(pending.clone()).await.expect("save pending failed");
    ctx.idle_periods.save_idle_period(kept.clone()).await.expect("save kept failed");
    ctx.idle_periods.save_idle_period(pending2.clone()).await.expect("save pending2 failed");

    // Get pending periods
    let pending_periods =
        ctx.idle_periods.get_pending_idle_periods().await.expect("get_pending_idle_periods failed");

    assert_eq!(pending_periods.len(), 2, "expected 2 pending periods (NULL and 'pending')");

    let ids: Vec<String> = pending_periods.iter().map(|p| p.id.clone()).collect();
    assert!(ids.contains(&"idle-pending".to_string()));
    assert!(ids.contains(&"idle-pending2".to_string()));
    assert!(!ids.contains(&"idle-kept".to_string()));
}

// =============================================================================
// IdleSummary Tests
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_get_idle_summary_empty_day() {
    let (ctx, _temp_dir) = create_test_context().await;
    let now = Utc::now().timestamp();

    // Query summary for empty day
    let summary =
        ctx.idle_periods.get_idle_summary(now - 3600, now).await.expect("get_idle_summary failed");

    assert_eq!(summary.total_active_secs, 0);
    assert_eq!(summary.total_idle_secs, 0);
    assert_eq!(summary.idle_periods_count, 0);
    assert_eq!(summary.idle_kept_secs, 0);
    assert_eq!(summary.idle_discarded_secs, 0);
    assert_eq!(summary.idle_pending_secs, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_get_idle_summary_with_data() {
    let (ctx, _temp_dir) = create_test_context().await;
    let now = Utc::now().timestamp();
    let start_ts = now - 3600;
    let end_ts = now;

    // Create some active snapshots (30 sec each)
    for i in 0..10 {
        let snapshot = create_test_snapshot(&format!("snap-{}", i), start_ts + (i * 30), false);
        ctx.snapshots.store_snapshot(&snapshot).expect("store_snapshot failed");
    }

    // Create idle periods with different actions
    let mut idle1 = create_test_idle_period("idle-1", -3400, 300); // 5 min
    idle1.user_action = Some("kept".to_string());
    idle1.start_ts = start_ts + 500;
    idle1.end_ts = start_ts + 800;

    let mut idle2 = create_test_idle_period("idle-2", -3000, 600); // 10 min
    idle2.user_action = Some("discarded".to_string());
    idle2.start_ts = start_ts + 1000;
    idle2.end_ts = start_ts + 1600;

    let mut idle3 = create_test_idle_period("idle-3", -2000, 200); // 3.33 min
    idle3.user_action = None; // pending
    idle3.start_ts = start_ts + 2000;
    idle3.end_ts = start_ts + 2200;

    ctx.idle_periods.save_idle_period(idle1.clone()).await.expect("save idle1 failed");
    ctx.idle_periods.save_idle_period(idle2.clone()).await.expect("save idle2 failed");
    ctx.idle_periods.save_idle_period(idle3.clone()).await.expect("save idle3 failed");

    // Get summary
    let summary =
        ctx.idle_periods.get_idle_summary(start_ts, end_ts).await.expect("get_idle_summary failed");

    assert_eq!(summary.total_active_secs, 10 * 30); // 10 snapshots * 30 sec
    assert_eq!(summary.total_idle_secs, 300 + 600 + 200); // 1100 seconds
    assert_eq!(summary.idle_periods_count, 3);
    assert_eq!(summary.idle_kept_secs, 300);
    assert_eq!(summary.idle_discarded_secs, 600);
    assert_eq!(summary.idle_pending_secs, 200);
}

// =============================================================================
// IdleSyncMetrics Tests
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_idle_sync_metrics_record_idle_detection() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Record some idle detections
    ctx.idle_sync_metrics.record_idle_detection(100);
    ctx.idle_sync_metrics.record_idle_detection(150);
    ctx.idle_sync_metrics.record_idle_detection(200);

    // Verify counters incremented (would check metrics in real impl)
    // For now, just verify no panics
}

#[tokio::test(flavor = "multi_thread")]
async fn test_idle_sync_metrics_record_activity_wake() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Record wake events
    ctx.idle_sync_metrics.record_activity_wake("mouse".to_string());
    ctx.idle_sync_metrics.record_activity_wake("keyboard".to_string());

    // Verify no panics
}

#[tokio::test(flavor = "multi_thread")]
async fn test_idle_sync_metrics_record_timer_events() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Record timer emissions
    ctx.idle_sync_metrics.record_timer_event_emission(1000, true);
    ctx.idle_sync_metrics.record_timer_event_emission(2000, false);

    // Record timer receptions
    ctx.idle_sync_metrics.record_timer_event_reception(50);

    // Verify no panics
}

#[tokio::test(flavor = "multi_thread")]
async fn test_idle_sync_metrics_record_invalid_payload() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Record invalid payloads
    ctx.idle_sync_metrics.record_invalid_payload();
    ctx.idle_sync_metrics.record_invalid_payload();

    // Verify no panics
}

#[tokio::test(flavor = "multi_thread")]
async fn test_idle_sync_metrics_record_state_transition() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Record state transitions
    ctx.idle_sync_metrics.record_state_transition("idle", "active", 100);
    ctx.idle_sync_metrics.record_state_transition("active", "idle", 50);

    // Verify no panics
}

#[tokio::test(flavor = "multi_thread")]
async fn test_idle_sync_metrics_record_auto_start_tracker_rule() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Record rule validations
    ctx.idle_sync_metrics.record_auto_start_tracker_rule(
        1, "running", true, true, // correct
    );

    ctx.idle_sync_metrics.record_auto_start_tracker_rule(
        2, "stopped", false, false, // incorrect
    );

    // Verify no panics
}
