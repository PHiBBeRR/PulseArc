//! Regression test for Issue #4: Date Query Index Bypass
//!
//! **Bug**: Legacy code used `date(column, 'unixepoch')` which bypasses indexes
//! **Impact**: Full table scans, O(n) query time instead of O(log n)
//! **Reference**: docs/issues/PHASE-3-PRE-MIGRATION-FIXES.md

#![allow(dead_code)]

#[path = "support.rs"]
mod support;

use std::time::{Duration as StdDuration, Instant};

use chrono::{Duration, NaiveDate};
use pulsearc_core::tracking::ports::{SegmentRepository, SnapshotRepository};
use pulsearc_infra::database::{SqlCipherActivityRepository, SqlCipherSegmentRepository};
use rusqlite::params;

#[tokio::test]
async fn test_segment_repository_uses_index_friendly_date_queries() {
    let db = support::setup_segment_db();
    let repo = SqlCipherSegmentRepository::new(db.manager.clone());

    let date = NaiveDate::from_ymd_opt(2025, 11, 1).expect("valid date");
    let (day_start, day_end) = day_bounds(date);

    {
        let conn = db.manager.get_connection().expect("connection");
        insert_segment(&conn, "seg-start", day_start, day_start + 300, false)
            .expect("insert start");
        insert_segment(&conn, "seg-end", day_end - 120, day_end - 30, false).expect("insert end");
        insert_segment(&conn, "seg-before", day_start - 3600, day_start - 1800, false)
            .expect("insert before");
        insert_segment(&conn, "seg-after", day_end + 60, day_end + 120, false)
            .expect("insert after");
    }

    let results = repo.find_segments_by_date(date).expect("query should succeed");

    let ids: Vec<_> = results.into_iter().map(|segment| segment.id).collect();
    assert_eq!(ids, vec!["seg-start", "seg-end"], "Only records for the target day are returned");

    let conn = db.manager.get_connection().expect("connection");
    let mut stmt = rusqlite::Connection::prepare(
        &conn,
        "EXPLAIN QUERY PLAN SELECT id FROM activity_segments WHERE start_ts >= ?1 AND start_ts < ?2",
    )
    .expect("prepare explain");
    let plan: Vec<String> = stmt
        .query_map(params![day_start, day_end], |row| row.get(3))
        .expect("explain query")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect explain");

    assert!(
        plan.iter()
            .any(|line| line.contains("SEARCH") && line.contains("idx_activity_segments_start_ts")),
        "Query should use the start_ts index: {plan:?}"
    );
    assert!(
        plan.iter().all(|line| !line.contains("SCAN TABLE activity_segments")),
        "Query should avoid full table scans: {plan:?}"
    );
}

#[tokio::test]
#[ignore] // Remove this once SnapshotRepository is implemented
async fn test_snapshot_repository_uses_index_friendly_date_queries() {
    let db = support::setup_snapshot_db();
    let repo = SqlCipherActivityRepository::new(db.manager.clone());

    let date = NaiveDate::from_ymd_opt(2025, 11, 1).expect("valid date");
    let (day_start, day_end) = day_bounds(date);

    {
        let conn = db.manager.get_connection().expect("connection");
        insert_snapshot(&conn, "snap-start", day_start).expect("insert start");
        insert_snapshot(&conn, "snap-end", day_end - 1).expect("insert end");
        insert_snapshot(&conn, "snap-before", day_start - 1).expect("insert before");
    }

    let count = repo.count_snapshots_by_date(date).expect("count should succeed");
    assert_eq!(count, 2, "Only snapshots within the target day are counted");

    let conn = db.manager.get_connection().expect("connection");
    let mut stmt = rusqlite::Connection::prepare(
        &conn,
        "EXPLAIN QUERY PLAN SELECT COUNT(*) FROM activity_snapshots WHERE timestamp >= ?1 AND timestamp < ?2",
    )
    .expect("prepare explain");
    let plan: Vec<String> = stmt
        .query_map(params![day_start, day_end], |row| row.get(3))
        .expect("explain query")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect explain");

    assert!(
        plan.iter().any(
            |line| line.contains("SEARCH") && line.contains("idx_activity_snapshots_timestamp")
        ),
        "Snapshot query should use the timestamp index: {plan:?}"
    );
    assert!(
        plan.iter().all(|line| !line.contains("SCAN TABLE activity_snapshots")),
        "Snapshot query should avoid full table scans: {plan:?}"
    );
}

#[tokio::test]
async fn test_date_query_performance_under_10ms() {
    let db = support::setup_segment_db();
    let repo = SqlCipherSegmentRepository::new(db.manager.clone());

    let base_date = NaiveDate::from_ymd_opt(2025, 10, 1).expect("valid base date");
    {
        let mut conn = db.manager.get_connection().expect("connection");
        let tx = rusqlite::Connection::transaction(&mut conn).expect("transaction");
        for day_offset in 0..30 {
            let current_date = base_date + Duration::days(day_offset);
            let day_start =
                current_date.and_hms_opt(0, 0, 0).expect("midnight").and_utc().timestamp();

            for idx in 0..333 {
                let start_ts = day_start + (idx * 180 % 86_400) as i64;
                let end_ts = start_ts + 120;
                insert_segment_raw(
                    &tx,
                    &format!("seg-{day_offset}-{idx}"),
                    start_ts,
                    end_ts,
                    false,
                )
                .expect("insert segment");
            }
        }
        tx.commit().expect("commit");
    }

    let target_date = base_date + Duration::days(12);
    let baseline = repo.find_segments_by_date(target_date).expect("baseline query should succeed");
    assert!(!baseline.is_empty(), "dataset should return results");

    let mut durations = Vec::with_capacity(100);
    for _ in 0..100 {
        let start = Instant::now();
        let result = repo.find_segments_by_date(target_date).expect("query should succeed");
        assert_eq!(result.len(), baseline.len(), "consistent result size");
        durations.push(start.elapsed());
    }

    durations.sort();
    let p50 = durations[durations.len() / 2];
    let p95 = durations[(durations.len() as f64 * 0.95).ceil() as usize - 1];
    let p99 = durations[(durations.len() as f64 * 0.99).ceil() as usize - 1];

    assert!(p50 < StdDuration::from_millis(5), "p50 latency should be <5ms: {p50:?}");
    assert!(p95 < StdDuration::from_millis(8), "p95 latency should be <8ms: {p95:?}");
    assert!(p99 < StdDuration::from_millis(10), "p99 latency should be <10ms: {p99:?}");
}

#[tokio::test]
async fn test_date_query_correctness_at_day_boundaries() {
    let db = support::setup_segment_db();
    let repo = SqlCipherSegmentRepository::new(db.manager.clone());

    let date = NaiveDate::from_ymd_opt(2025, 11, 1).expect("valid date");
    let (day_start, day_end) = day_bounds(date);

    {
        let conn = db.manager.get_connection().expect("connection");
        insert_segment(&conn, "seg-start", day_start, day_start + 60, false)
            .expect("insert start boundary");
        insert_segment(&conn, "seg-end", day_end - 1, day_end + 30, false)
            .expect("insert end boundary");
        insert_segment(&conn, "seg-next-day", day_end, day_end + 60, false)
            .expect("insert next day");
    }

    let results = repo.find_segments_by_date(date).expect("query should succeed");

    let ids: Vec<_> = results.into_iter().map(|segment| segment.id).collect();
    assert_eq!(ids, vec!["seg-start", "seg-end"], "Half-open range must exclude next-day entries");
}

#[test]
fn test_date_to_timestamp_range_conversion() {
    let date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();

    // Convert to timestamp range
    let day_start = date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    let day_end = date.succ_opt().unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();

    // Assert range is [2025-11-01 00:00:00, 2025-11-02 00:00:00)
    assert_eq!(day_start, 1761955200); // 2025-11-01 00:00:00 UTC
    assert_eq!(day_end, 1762041600); // 2025-11-02 00:00:00 UTC
    assert_eq!(day_end - day_start, 86400); // 24 hours in seconds
}

fn insert_segment(
    conn: &rusqlite::Connection,
    id: &str,
    start_ts: i64,
    end_ts: i64,
    processed: bool,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO activity_segments (id, start_ts, end_ts, primary_app, normalized_label, sample_count, \
            dictionary_keys, created_at, processed, snapshot_ids, work_type, activity_category, detected_activity, \
            extracted_signals_json, project_match_json, idle_time_secs, active_time_secs, user_action) \
         VALUES (?1, ?2, ?3, 'Safari', 'Browsing', 10, NULL, ?4, ?5, '[]', NULL, 'client_work', 'Browsing', NULL, NULL, 0, 600, NULL)",
        params![id, start_ts, end_ts, start_ts, if processed { 1 } else { 0 }],
    )
    .map(|_| ())
}

fn insert_segment_raw(
    tx: &rusqlite::Transaction<'_>,
    id: &str,
    start_ts: i64,
    end_ts: i64,
    processed: bool,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO activity_segments (id, start_ts, end_ts, primary_app, normalized_label, sample_count, \
            dictionary_keys, created_at, processed, snapshot_ids, work_type, activity_category, detected_activity, \
            extracted_signals_json, project_match_json, idle_time_secs, active_time_secs, user_action) \
         VALUES (?1, ?2, ?3, 'Safari', 'Browsing', 10, NULL, ?4, ?5, '[]', NULL, 'client_work', 'Browsing', NULL, NULL, 0, 600, NULL)",
        params![id, start_ts, end_ts, start_ts, if processed { 1 } else { 0 }],
    )
    .map(|_| ())
}

fn insert_snapshot(conn: &rusqlite::Connection, id: &str, timestamp: i64) -> rusqlite::Result<()> {
    conn.execute(
        r#"INSERT INTO activity_snapshots (
            id,
            timestamp,
            activity_context_json,
            detected_activity,
            work_type,
            activity_category,
            primary_app,
            processed,
            batch_id,
            created_at,
            processed_at,
            is_idle,
            idle_duration_secs
        ) VALUES (
            ?1,
            ?2,
            '{"context":"test"}',
            'Browsing',
            NULL,
            NULL,
            'Safari',
            0,
            NULL,
            ?2,
            NULL,
            0,
            NULL
        )"#,
        params![id, timestamp],
    )
    .map(|_| ())
}

fn day_bounds(date: NaiveDate) -> (i64, i64) {
    let start = date.and_hms_opt(0, 0, 0).expect("midnight").and_utc().timestamp();
    let end = date
        .succ_opt()
        .expect("next day")
        .and_hms_opt(0, 0, 0)
        .expect("midnight")
        .and_utc()
        .timestamp();
    (start, end)
}
