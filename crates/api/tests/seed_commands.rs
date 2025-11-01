//! Integration tests for seed commands (debug builds only)
//!
//! These tests verify the SnapshotRepository can handle batch inserts
//! and retrieval, which is the core functionality used by
//! seed_activity_snapshots.

#![cfg(debug_assertions)]

use chrono::Utc;
use pulsearc_common::testing::TempDir;
use pulsearc_domain::{ActivitySnapshot, Config, DatabaseConfig};
use pulsearc_lib::context::AppContext;

const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Helper to create test context with temporary database
async fn create_test_context() -> Result<(AppContext, TempDir), Box<dyn std::error::Error>> {
    std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", TEST_KEY);

    let temp_dir = TempDir::new("pulsearc-seed-test")?;
    let test_db_path = temp_dir.path().join("pulsearc.db");
    let lock_dir = temp_dir.create_dir("lock")?;

    let config = Config {
        database: DatabaseConfig {
            path: test_db_path.to_string_lossy().to_string(),
            pool_size: 5,
            encryption_key: None,
        },
        ..Config::default()
    };

    let ctx = AppContext::new_with_config_in_lock_dir(config, lock_dir).await?;
    Ok((ctx, temp_dir))
}

/// Helper to create test snapshots
fn create_test_snapshots(count: usize, base_timestamp: i64) -> Vec<ActivitySnapshot> {
    let mut snapshots = Vec::new();
    for i in 0..count {
        let timestamp = base_timestamp + (i as i64 * 60);
        let activity_context = serde_json::json!({
            "active_app": {
                "app_name": format!("TestApp{}", i % 3),
                "window_title": format!("Test Window {}", i),
                "bundle_id": format!("com.test.app{}", i % 3),
            },
            "keyboard_activity": { "keystrokes": 50 + i * 10 },
            "mouse_activity": { "clicks": 20 + i * 5 },
        });

        snapshots.push(ActivitySnapshot {
            id: format!("snapshot_{}_{}", timestamp, i),
            timestamp,
            activity_context_json: serde_json::to_string(&activity_context).unwrap(),
            detected_activity: if i % 2 == 0 { "coding" } else { "communication" }.to_string(),
            work_type: Some("deep_work".to_string()),
            activity_category: Some("development".to_string()),
            primary_app: format!("TestApp{}", i % 3),
            processed: false,
            batch_id: None,
            created_at: timestamp,
            processed_at: None,
            is_idle: false,
            idle_duration_secs: None,
        });
    }
    snapshots
}

#[tokio::test(flavor = "multi_thread")]
async fn snapshot_repository_saves_and_retrieves() {
    let (context, _temp_dir) = create_test_context().await.unwrap();

    // Create test snapshots
    let now_ts = Utc::now().timestamp();
    let snapshots = create_test_snapshots(3, now_ts);

    // Save via repository
    context.snapshots.store_snapshots_batch(&snapshots).expect("batch save succeeds");

    // Retrieve and verify
    let start = chrono::DateTime::from_timestamp(now_ts - 10, 0).unwrap();
    let end = chrono::DateTime::from_timestamp(now_ts + 200, 0).unwrap();
    let retrieved =
        context.snapshots.find_snapshots_by_time_range(start, end).expect("query succeeds");

    assert_eq!(retrieved.len(), 3);
    assert_eq!(retrieved[0].id, snapshots[0].id);
    assert_eq!(retrieved[1].id, snapshots[1].id);
    assert_eq!(retrieved[2].id, snapshots[2].id);
}

#[tokio::test(flavor = "multi_thread")]
async fn snapshot_repository_handles_large_batch() {
    let (context, _temp_dir) = create_test_context().await.unwrap();

    // Create larger batch
    let now_ts = Utc::now().timestamp();
    let snapshots = create_test_snapshots(100, now_ts);

    // Save via repository
    context.snapshots.store_snapshots_batch(&snapshots).expect("large batch save succeeds");

    // Retrieve and verify count
    let start = chrono::DateTime::from_timestamp(now_ts - 10, 0).unwrap();
    let end = chrono::DateTime::from_timestamp(now_ts + 6000, 0).unwrap();
    let retrieved =
        context.snapshots.find_snapshots_by_time_range(start, end).expect("query succeeds");

    assert_eq!(retrieved.len(), 100);
}

#[tokio::test(flavor = "multi_thread")]
async fn snapshot_repository_is_idempotent() {
    let (context, _temp_dir) = create_test_context().await.unwrap();

    // Create test snapshots
    let now_ts = Utc::now().timestamp();
    let snapshots = create_test_snapshots(5, now_ts);

    // Save twice (INSERT OR REPLACE should handle duplicates)
    context.snapshots.store_snapshots_batch(&snapshots).expect("first save succeeds");

    context.snapshots.store_snapshots_batch(&snapshots).expect("second save succeeds");

    // Retrieve and verify no duplicates
    let start = chrono::DateTime::from_timestamp(now_ts - 10, 0).unwrap();
    let end = chrono::DateTime::from_timestamp(now_ts + 400, 0).unwrap();
    let retrieved =
        context.snapshots.find_snapshots_by_time_range(start, end).expect("query succeeds");

    assert_eq!(retrieved.len(), 5); // Should not have duplicates
}

#[tokio::test(flavor = "multi_thread")]
async fn snapshot_repository_preserves_data() {
    let (context, _temp_dir) = create_test_context().await.unwrap();

    // Create snapshot with specific data
    let now_ts = Utc::now().timestamp();
    let mut snapshots = create_test_snapshots(1, now_ts);
    snapshots[0].detected_activity = "testing".to_string();
    snapshots[0].work_type = Some("validation".to_string());
    snapshots[0].is_idle = false;

    // Save and retrieve
    context.snapshots.store_snapshots_batch(&snapshots).expect("save succeeds");

    let start = chrono::DateTime::from_timestamp(now_ts - 10, 0).unwrap();
    let end = chrono::DateTime::from_timestamp(now_ts + 100, 0).unwrap();
    let retrieved =
        context.snapshots.find_snapshots_by_time_range(start, end).expect("query succeeds");

    // Verify all fields preserved
    assert_eq!(retrieved.len(), 1);
    let snapshot = &retrieved[0];
    assert_eq!(snapshot.detected_activity, "testing");
    assert_eq!(snapshot.work_type.as_deref(), Some("validation"));
    assert!(!snapshot.is_idle);
    assert!(!snapshot.processed);
}

#[tokio::test(flavor = "multi_thread")]
async fn snapshot_repository_filters_by_time_range() {
    let (context, _temp_dir) = create_test_context().await.unwrap();

    // Create snapshots spread over time
    let base_ts = Utc::now().timestamp();
    let snapshots = create_test_snapshots(10, base_ts);

    context.snapshots.store_snapshots_batch(&snapshots).expect("save succeeds");

    // Query narrow time range (first 5 snapshots = 0-240s)
    let start = chrono::DateTime::from_timestamp(base_ts - 10, 0).unwrap();
    let end = chrono::DateTime::from_timestamp(base_ts + 250, 0).unwrap();
    let retrieved =
        context.snapshots.find_snapshots_by_time_range(start, end).expect("query succeeds");

    assert_eq!(retrieved.len(), 5);
}

#[tokio::test(flavor = "multi_thread")]
async fn snapshot_repository_handles_empty_batch() {
    let (context, _temp_dir) = create_test_context().await.unwrap();

    // Save empty batch (should not error)
    let empty: Vec<ActivitySnapshot> = vec![];
    let result = context.snapshots.store_snapshots_batch(&empty);

    assert!(result.is_ok());
}
