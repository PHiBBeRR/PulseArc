//! End-to-end database integration coverage for core SQLCipher repositories.
//!
//! These tests exercise critical repository workflows against the real
//! workspace schema to ensure serialization, migrations, and business rules
//! remain aligned. Each test operates on an isolated SQLCipher database with
//! migrations applied and uses UUIDv7 identifiers to match production ID
//! semantics.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};
use pulsearc_core::batch::ports::BatchRepository;
use pulsearc_core::sync::ports::{IdMappingRepository, OutboxQueue};
use pulsearc_core::tracking::ports::{
    ActivityRepository, IdlePeriodsRepository, SegmentRepository, SnapshotRepository,
};
use pulsearc_domain::types::database::{
    ActivitySegment, ActivitySnapshot, BatchQueue, IdMapping, TimeEntryOutbox,
};
use pulsearc_domain::{BatchStatus, IdlePeriod, OutboxStatus};
use pulsearc_infra::database::{
    DbManager, SqlCipherActivityRepository, SqlCipherBatchRepository, SqlCipherIdMappingRepository,
    SqlCipherIdlePeriodsRepository, SqlCipherOutboxRepository, SqlCipherSegmentRepository,
};
use rusqlite::ToSql;
use serde_json::json;
use tempfile::TempDir;
use tokio::task;
use uuid::Uuid;

const TEST_DB_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

struct DbHarness {
    #[allow(dead_code)]
    temp_dir: TempDir,
    manager: Arc<DbManager>,
}

impl DbHarness {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("temporary directory should be created");
        let db_path = temp_dir.path().join("infra-integration.db");

        let manager = Arc::new(
            DbManager::new(&db_path, 4, Some(TEST_DB_KEY))
                .expect("database manager should initialise"),
        );
        manager.run_migrations().expect("schema migrations should apply");

        Self { temp_dir, manager }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn activity_and_segment_repositories_workflow() {
    let harness = DbHarness::new();
    let manager = Arc::clone(&harness.manager);

    let activity_repo = SqlCipherActivityRepository::new(Arc::clone(&manager));
    let segment_repo = SqlCipherSegmentRepository::new(manager);

    let base =
        Utc.with_ymd_and_hms(2025, 1, 2, 9, 0, 0).single().expect("base timestamp should be valid");

    let mut snapshot_ids = Vec::new();

    let active_a = make_snapshot(base, "com.pulsearc.dev", false);
    snapshot_ids.push(active_a.id.clone());
    activity_repo.save_snapshot(active_a.clone()).await.expect("active snapshot should persist");

    let idle_snapshot =
        make_snapshot(base + ChronoDuration::seconds(30), "com.pulsearc.idle", true);
    snapshot_ids.push(idle_snapshot.id.clone());
    activity_repo.save_snapshot(idle_snapshot.clone()).await.expect("idle snapshot should persist");

    let active_b = make_snapshot(base + ChronoDuration::seconds(60), "com.pulsearc.dev", false);
    snapshot_ids.push(active_b.id.clone());
    activity_repo
        .save_snapshot(active_b.clone())
        .await
        .expect("second active snapshot should persist");

    let extra_snapshots = vec![
        make_snapshot(base + ChronoDuration::seconds(90), "com.pulsearc.browser", false),
        make_snapshot(base + ChronoDuration::seconds(120), "com.pulsearc.terminal", false),
    ];
    activity_repo.store_snapshots_batch(&extra_snapshots).expect("batch insert should succeed");

    let start_range = base - ChronoDuration::seconds(30);
    let end_range = base + ChronoDuration::seconds(180);

    let retrieved = activity_repo
        .find_snapshots_by_time_range(start_range, end_range)
        .expect("snapshot query should succeed");
    assert_eq!(retrieved.len(), 5, "all stored snapshots should be returned");
    assert!(
        retrieved.iter().any(|s| s.id == extra_snapshots[0].id),
        "batch-inserted snapshot should be present"
    );
    assert!(retrieved.iter().any(|s| s.is_idle), "idle snapshot should be preserved");

    let active_count = activity_repo
        .count_active_snapshots(start_range, end_range)
        .expect("active snapshot count should execute");
    assert_eq!(active_count, 4, "four snapshots in range are marked active");

    let segment = make_segment(base, "com.pulsearc.dev", snapshot_ids.clone());
    segment_repo.save_segment(&segment).expect("segment should persist");

    let date = base.date_naive();
    let segments_for_day =
        segment_repo.find_segments_by_date(date).expect("segment query should succeed");
    assert_eq!(segments_for_day.len(), 1, "one segment expected for the day");
    let stored_segment = &segments_for_day[0];
    assert_eq!(stored_segment.id, segment.id);
    assert_eq!(
        stored_segment.snapshot_ids.len(),
        snapshot_ids.len(),
        "segment should retain snapshot linkage"
    );

    let unprocessed =
        segment_repo.find_unprocessed_segments(10).expect("unprocessed query should succeed");
    assert_eq!(unprocessed.len(), 1, "one segment should require processing");

    segment_repo.mark_processed(&segment.id).expect("segment processed flag should update");

    let remaining = segment_repo
        .find_unprocessed_segments(10)
        .expect("unprocessed query should succeed after update");
    assert!(remaining.is_empty(), "processed segment should no longer appear in the queue");
}

#[tokio::test(flavor = "multi_thread")]
async fn idle_period_repository_tracks_summary_and_actions() {
    let harness = DbHarness::new();
    let manager = Arc::clone(&harness.manager);

    let activity_repo = SqlCipherActivityRepository::new(Arc::clone(&manager));
    let idle_repo = SqlCipherIdlePeriodsRepository::new(manager);

    let base = Utc
        .with_ymd_and_hms(2025, 3, 10, 8, 0, 0)
        .single()
        .expect("base timestamp should be valid");

    // Ensure active activity exists so summary can compute active_seconds.
    let active_a = make_snapshot(base, "com.pulsearc.dev", false);
    let active_b = make_snapshot(base + ChronoDuration::seconds(60), "com.pulsearc.browser", false);
    activity_repo.save_snapshot(active_a).await.expect("first active snapshot should persist");
    activity_repo.save_snapshot(active_b).await.expect("second active snapshot should persist");

    let kept = make_idle_period(base + ChronoDuration::minutes(15), 300, Some("pending"));
    let discarded = make_idle_period(base + ChronoDuration::minutes(16), 600, Some("pending"));
    let pending = make_idle_period(base + ChronoDuration::minutes(18), 150, None);

    idle_repo.save_idle_period(kept.clone()).await.expect("kept period should persist");
    idle_repo.save_idle_period(discarded.clone()).await.expect("discarded period should persist");
    idle_repo.save_idle_period(pending.clone()).await.expect("pending period should persist");

    idle_repo
        .update_idle_period_action(&kept.id, "kept", Some("kept for audit trail".into()))
        .await
        .expect("kept period update should succeed");
    idle_repo
        .update_idle_period_action(&discarded.id, "discarded", None)
        .await
        .expect("discarded period update should succeed");

    let fetched = idle_repo
        .get_idle_period(&kept.id)
        .await
        .expect("kept period fetch should succeed")
        .expect("kept period should exist");
    assert_eq!(fetched.user_action.as_deref(), Some("kept"));
    assert!(fetched.reviewed_at.is_some(), "review timestamp should be recorded");
    assert_eq!(fetched.notes.as_deref(), Some("kept for audit trail"), "notes should persist");

    let range_start = base.timestamp();
    let range_end = (base + ChronoDuration::hours(1)).timestamp();

    let in_range = idle_repo
        .get_idle_periods_in_range(range_start, range_end)
        .await
        .expect("range query should succeed");
    assert_eq!(in_range.len(), 3, "all inserted periods fall inside the window");

    let pending_periods =
        idle_repo.get_pending_idle_periods().await.expect("pending query should succeed");
    assert_eq!(pending_periods.len(), 1, "only the untouched period should remain pending");
    assert_eq!(pending_periods[0].id, pending.id);

    let summary = idle_repo
        .get_idle_summary(range_start, range_end)
        .await
        .expect("summary query should succeed");
    assert_eq!(summary.total_active_secs, 60);
    assert_eq!(summary.total_idle_secs, 1050);
    assert_eq!(summary.idle_periods_count, 3);
    assert_eq!(summary.idle_kept_secs, 300);
    assert_eq!(summary.idle_discarded_secs, 600);
    assert_eq!(summary.idle_pending_secs, 150);

    let deleted = idle_repo
        .delete_idle_periods_before(range_start - 1)
        .await
        .expect("delete before range should succeed");
    assert_eq!(deleted, 0, "no periods fall before the cutoff");
}

#[tokio::test(flavor = "multi_thread")]
async fn batch_outbox_and_id_mapping_end_to_end() {
    let harness = DbHarness::new();
    let manager = Arc::clone(&harness.manager);

    let batch_repo = SqlCipherBatchRepository::new(Arc::clone(&manager));
    let outbox_repo = SqlCipherOutboxRepository::new(Arc::clone(&manager));
    let id_repo = SqlCipherIdMappingRepository::new(manager);

    let base = Utc
        .with_ymd_and_hms(2025, 5, 20, 10, 30, 0)
        .single()
        .expect("base timestamp should be valid");

    // Batch lifecycle
    // ---------------------------------------------------------------------------
    let batch = make_batch(12, base);
    batch_repo.save_batch(&batch).await.expect("batch insert should succeed");

    let mut retrieved =
        batch_repo.get_batch(&batch.batch_id).await.expect("batch fetch should succeed");
    assert_eq!(retrieved.activity_count, 12);
    assert_eq!(retrieved.status, BatchStatus::Pending);

    let lease = Duration::from_secs(300);
    batch_repo
        .acquire_batch_lease(&batch.batch_id, "worker-alpha", lease)
        .await
        .expect("lease acquisition should succeed");

    retrieved = batch_repo
        .get_batch(&batch.batch_id)
        .await
        .expect("batch fetch after lease should succeed");
    let initial_lease = retrieved.lease_expires_at.expect("lease expiry should be populated");
    assert_eq!(retrieved.worker_id.as_deref(), Some("worker-alpha"));

    tokio::time::sleep(Duration::from_secs(1)).await;

    batch_repo
        .renew_batch_lease(&batch.batch_id, "worker-alpha", lease)
        .await
        .expect("lease renewal should succeed");
    let renewed = batch_repo
        .get_batch(&batch.batch_id)
        .await
        .expect("batch fetch after renewal should succeed");
    let renewed_lease = renewed.lease_expires_at.expect("renewed lease should exist");
    assert!(renewed_lease > initial_lease, "renewal should extend the lease expiry");

    batch_repo
        .update_batch_status(&batch.batch_id, BatchStatus::Processing)
        .await
        .expect("status update should succeed");
    let processing = batch_repo
        .get_batch(&batch.batch_id)
        .await
        .expect("batch fetch after status update should succeed");
    assert_eq!(processing.status, BatchStatus::Processing);

    batch_repo
        .mark_batch_failed(&batch.batch_id, "openai timeout")
        .await
        .expect("mark failed should succeed");
    let failed = batch_repo
        .get_batch(&batch.batch_id)
        .await
        .expect("batch fetch after failure should succeed");
    assert_eq!(failed.status, BatchStatus::Failed);
    assert_eq!(failed.error_message.as_deref(), Some("openai timeout"));

    let failed_batches = batch_repo
        .get_batches_by_status(BatchStatus::Failed)
        .await
        .expect("failed batch query should succeed");
    assert!(
        failed_batches.iter().any(|item| item.batch_id == batch.batch_id),
        "failed listing should include the test batch"
    );

    // Outbox failure + success paths
    // -------------------------------------------------------------
    let entry = make_outbox_entry("user-123", base);
    outbox_repo.enqueue(&entry).await.expect("enqueue should succeed");

    let pending_count = outbox_repo.pending_count().await.expect("pending count should succeed");
    assert_eq!(pending_count, 1);

    let dequeued = outbox_repo.dequeue_batch(5).await.expect("initial dequeue should succeed");
    assert_eq!(dequeued.len(), 1);
    assert_eq!(dequeued[0].id, entry.id);

    outbox_repo
        .mark_failed(&entry.id, "network timeout")
        .await
        .expect("first failure should succeed");

    let after_failure =
        outbox_repo.dequeue_batch(5).await.expect("dequeue after failure should succeed");
    assert!(
        after_failure.is_empty(),
        "retry_after should delay the entry from being dequeued immediately"
    );

    for attempt in 2..=5 {
        let message = format!("transient failure attempt {attempt}");
        outbox_repo
            .mark_failed(&entry.id, &message)
            .await
            .expect("subsequent failure should succeed");
    }

    let (status, attempts) = load_outbox_state(Arc::clone(&harness.manager), &entry.id).await;
    assert_eq!(status, OutboxStatus::Failed);
    assert_eq!(attempts, 5);

    let pending_after_failures =
        outbox_repo.pending_count().await.expect("pending count after failures should succeed");
    assert_eq!(pending_after_failures, 0);

    let sent_entry = make_outbox_entry("user-123", base + ChronoDuration::minutes(1));
    outbox_repo.enqueue(&sent_entry).await.expect("enqueue for sent flow should succeed");

    let ready = outbox_repo.dequeue_batch(1).await.expect("dequeue for sent flow should succeed");
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, sent_entry.id);

    outbox_repo.mark_sent(&sent_entry.id).await.expect("mark sent should succeed");

    let pending_final =
        outbox_repo.pending_count().await.expect("final pending count should succeed");
    assert_eq!(pending_final, 0);

    // ID mapping round trip
    // ---------------------------------------------------------------------
    let mapping = IdMapping {
        local_uuid: new_uuid(),
        backend_cuid: format!("cuid_{}", new_uuid()),
        entity_type: "activity_segment".to_string(),
        created_at: base.timestamp(),
        updated_at: base.timestamp(),
    };

    id_repo.create_id_mapping(&mapping).await.expect("id mapping insert should succeed");

    let by_local = id_repo
        .get_id_mapping_by_local_uuid(&mapping.local_uuid)
        .await
        .expect("fetch by local uuid should succeed")
        .expect("mapping should exist");
    assert_eq!(by_local.backend_cuid, mapping.backend_cuid);

    let backend = id_repo
        .get_backend_cuid_by_local_uuid(&mapping.local_uuid)
        .await
        .expect("fetch backend by local uuid should succeed")
        .expect("backend cuid should exist");
    assert_eq!(backend, mapping.backend_cuid);

    let local = id_repo
        .get_local_uuid_by_backend_cuid(&mapping.backend_cuid)
        .await
        .expect("fetch local uuid by backend cuid should succeed")
        .expect("local uuid should exist");
    assert_eq!(local, mapping.local_uuid);

    let by_type = id_repo
        .get_id_mappings_by_entity_type(&mapping.entity_type)
        .await
        .expect("fetch by entity type should succeed");
    assert!(
        by_type.iter().any(|item| item.local_uuid == mapping.local_uuid),
        "entity type listing should include the inserted mapping"
    );
}

fn make_snapshot(ts: DateTime<Utc>, primary_app: &str, is_idle: bool) -> ActivitySnapshot {
    let created_at = ts + ChronoDuration::seconds(5);
    ActivitySnapshot {
        id: new_uuid(),
        timestamp: ts.timestamp(),
        activity_context_json: json!({
            "app": primary_app,
            "window": "Integration Test Window",
            "url": "https://pulsearc.dev/test"
        })
        .to_string(),
        detected_activity: if is_idle { "idle" } else { "active" }.to_string(),
        work_type: Some("development".to_string()),
        activity_category: Some("coding".to_string()),
        primary_app: primary_app.to_string(),
        processed: false,
        batch_id: None,
        created_at: created_at.timestamp(),
        processed_at: None,
        is_idle,
        idle_duration_secs: if is_idle { Some(120) } else { None },
    }
}

fn make_segment(
    start: DateTime<Utc>,
    primary_app: &str,
    snapshot_ids: Vec<String>,
) -> ActivitySegment {
    let end = start + ChronoDuration::minutes(5);
    ActivitySegment {
        id: new_uuid(),
        start_ts: start.timestamp(),
        end_ts: end.timestamp(),
        primary_app: primary_app.to_string(),
        normalized_label: "focus/development".to_string(),
        sample_count: snapshot_ids.len() as i32,
        dictionary_keys: None,
        created_at: start.timestamp(),
        processed: false,
        snapshot_ids,
        work_type: Some("Development".to_string()),
        activity_category: "coding".to_string(),
        detected_activity: "Focused coding session".to_string(),
        extracted_signals_json: None,
        project_match_json: None,
        idle_time_secs: 30,
        active_time_secs: 270,
        user_action: None,
    }
}

fn make_idle_period(
    start: DateTime<Utc>,
    duration_secs: i32,
    user_action: Option<&str>,
) -> IdlePeriod {
    let end = start + ChronoDuration::seconds(duration_secs.into());
    IdlePeriod {
        id: new_uuid(),
        start_ts: start.timestamp(),
        end_ts: end.timestamp(),
        duration_secs,
        system_trigger: "threshold".to_string(),
        user_action: user_action.map(ToString::to_string),
        threshold_secs: 300,
        created_at: start.timestamp(),
        reviewed_at: None,
        notes: None,
    }
}

fn make_batch(activity_count: i32, created_at: DateTime<Utc>) -> BatchQueue {
    BatchQueue {
        batch_id: new_uuid(),
        activity_count,
        status: BatchStatus::Pending,
        created_at: created_at.timestamp(),
        processed_at: None,
        error_message: None,
        processing_started_at: None,
        worker_id: None,
        lease_expires_at: None,
        time_entries_created: 0,
        openai_cost: 0.0,
    }
}

fn make_outbox_entry(user_id: &str, created_at: DateTime<Utc>) -> TimeEntryOutbox {
    TimeEntryOutbox {
        id: new_uuid(),
        idempotency_key: new_uuid(),
        user_id: user_id.to_string(),
        payload_json: json!({
            "description": "Integration test time entry",
            "duration_secs": 3600,
            "start_time": created_at.timestamp()
        })
        .to_string(),
        backend_cuid: None,
        status: OutboxStatus::Pending,
        attempts: 0,
        last_error: None,
        retry_after: None,
        created_at: created_at.timestamp(),
        sent_at: None,
        correlation_id: Some(new_uuid()),
        local_status: Some("draft".to_string()),
        remote_status: None,
        sap_entry_id: None,
        next_attempt_at: None,
        error_code: None,
        last_forwarded_at: None,
        wbs_code: Some("WBS-001".to_string()),
        target: "sap".to_string(),
        description: Some("Integration test entry".to_string()),
        auto_applied: false,
        version: 1,
        last_modified_by: user_id.to_string(),
        last_modified_at: None,
    }
}

fn new_uuid() -> String {
    Uuid::now_v7().to_string()
}

async fn load_outbox_state(manager: Arc<DbManager>, id: &str) -> (OutboxStatus, i32) {
    let id = id.to_string();
    let (status_str, attempts) = task::spawn_blocking(move || {
        let conn = manager.get_connection().expect("inspection connection should be available");
        let params: [&dyn ToSql; 1] = [&id];
        conn.query_row(
            "SELECT status, attempts FROM time_entry_outbox WHERE id = ?1",
            &params,
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)),
        )
        .expect("outbox entry should exist")
    })
    .await
    .expect("blocking inspection should complete");

    let status =
        OutboxStatus::from_str(&status_str).expect("stored status should map to enum variant");
    (status, attempts)
}
