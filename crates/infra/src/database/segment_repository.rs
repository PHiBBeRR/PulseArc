//! SQLCipher-backed implementation of the `SegmentRepository` port.
//!
//! Provides synchronous accessors that operate directly on the shared
//! SQLCipher pool managed by `DbManager`. Queries use half-open
//! `[start, end)` predicates to preserve index usage and to avoid
//! implicit UTC conversions performed by SQLite date functions.

use std::sync::Arc;

use chrono::{Duration, NaiveDate, NaiveTime};
use pulsearc_common::error::{CommonError, CommonResult};
use pulsearc_common::storage::error::{StorageError, StorageResult};
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_core::tracking::ports::SegmentRepository as SegmentRepositoryPort;
use pulsearc_domain::types::database::ActivitySegment;
use pulsearc_domain::PulseArcError;
use rusqlite::{Row, ToSql};
use serde_json;
use tracing::warn;

use super::manager::DbManager;

/// SQLCipher-backed repository for persisting and querying activity segments.
pub struct SqlCipherSegmentRepository {
    db: Arc<DbManager>,
}

impl SqlCipherSegmentRepository {
    /// Create a repository backed by the shared SQLCipher pool.
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

impl SegmentRepositoryPort for SqlCipherSegmentRepository {
    fn save_segment(&self, segment: &ActivitySegment) -> CommonResult<()> {
        let conn = self
            .db
            .get_connection()
            .map_err(|err| map_connection_error("segment.save.connection", err))?;

        let snapshot_ids = serde_json::to_string(&segment.snapshot_ids)
            .map_err(|err| map_serialization_error("segment.save.snapshot_ids", err))?;

        let processed = bool_to_int(segment.processed);
        let params: [&dyn ToSql; 16] = [
            &segment.id,
            &segment.start_ts,
            &segment.end_ts,
            &segment.primary_app,
            &segment.normalized_label,
            &segment.sample_count,
            &segment.dictionary_keys,
            &segment.created_at,
            &processed,
            &snapshot_ids,
            &segment.work_type,
            &segment.activity_category,
            &segment.detected_activity,
            &segment.idle_time_secs,
            &segment.active_time_secs,
            &segment.user_action,
        ];

        conn.execute(SEGMENT_INSERT_SQL, params.as_slice())
            .map_err(StorageError::from)
            .map_err(|err| map_storage_error("segment.save.execute", err))?;

        Ok(())
    }

    fn find_segments_by_date(&self, date: NaiveDate) -> CommonResult<Vec<ActivitySegment>> {
        let conn = self
            .db
            .get_connection()
            .map_err(|err| map_connection_error("segment.find_by_date.connection", err))?;

        let (start, end) = day_bounds(date);
        query_segments(&conn, SEGMENT_BY_DATE_QUERY, &[&start, &end])
            .map_err(|err| map_storage_error("segment.find_by_date.query", err))
    }

    fn find_unprocessed_segments(&self, limit: usize) -> CommonResult<Vec<ActivitySegment>> {
        let conn = self
            .db
            .get_connection()
            .map_err(|err| map_connection_error("segment.find_unprocessed.connection", err))?;

        let limit = usize_to_i64(limit);
        query_segments(&conn, SEGMENT_UNPROCESSED_QUERY, &[&limit])
            .map_err(|err| map_storage_error("segment.find_unprocessed.query", err))
    }

    fn mark_processed(&self, segment_id: &str) -> CommonResult<()> {
        let conn = self
            .db
            .get_connection()
            .map_err(|err| map_connection_error("segment.mark_processed.connection", err))?;

        let params: [&dyn ToSql; 1] = [&segment_id];
        conn.execute(SEGMENT_MARK_PROCESSED_SQL, params.as_slice())
            .map_err(StorageError::from)
            .map_err(|err| map_storage_error("segment.mark_processed.execute", err))?;

        Ok(())
    }
}

const SEGMENT_INSERT_SQL: &str = "INSERT OR REPLACE INTO activity_segments (
        id, start_ts, end_ts, primary_app, normalized_label, sample_count, dictionary_keys,
        created_at, processed, snapshot_ids, work_type, activity_category, detected_activity,
        idle_time_secs, active_time_secs, user_action
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)";

const SEGMENT_BY_DATE_QUERY: &str = "SELECT id, start_ts, end_ts, primary_app, normalized_label,
        sample_count, dictionary_keys, created_at, processed, snapshot_ids, work_type,
        activity_category, detected_activity, idle_time_secs, active_time_secs, user_action
    FROM activity_segments
    WHERE start_ts >= ?1 AND start_ts < ?2
    ORDER BY start_ts";

const SEGMENT_UNPROCESSED_QUERY: &str =
    "SELECT id, start_ts, end_ts, primary_app, normalized_label,
        sample_count, dictionary_keys, created_at, processed, snapshot_ids, work_type,
        activity_category, detected_activity, idle_time_secs, active_time_secs, user_action
    FROM activity_segments
    WHERE processed = 0
    ORDER BY start_ts
    LIMIT ?1";

const SEGMENT_MARK_PROCESSED_SQL: &str = "UPDATE activity_segments SET processed = 1 WHERE id = ?1";

fn query_segments(
    conn: &SqlCipherConnection,
    sql: &str,
    params: &[&dyn ToSql],
) -> StorageResult<Vec<ActivitySegment>> {
    let mut stmt = conn.prepare(sql)?;
    stmt.query_map(params, map_segment_row)
}

fn map_segment_row(row: &Row<'_>) -> rusqlite::Result<ActivitySegment> {
    let id: String = row.get(0)?;
    let snapshot_ids_raw: String = row.get(9)?;

    let snapshot_ids =
        serde_json::from_str::<Vec<String>>(&snapshot_ids_raw).unwrap_or_else(|err| {
            warn!(
                segment_id = %id,
                error = %err,
                "failed to parse snapshot_ids – returning empty list"
            );
            Vec::new()
        });

    Ok(ActivitySegment {
        id,
        start_ts: row.get(1)?,
        end_ts: row.get(2)?,
        primary_app: row.get(3)?,
        normalized_label: row.get(4)?,
        sample_count: row.get(5)?,
        dictionary_keys: row.get(6)?,
        created_at: row.get(7)?,
        processed: int_to_bool(row.get(8)?),
        snapshot_ids,
        work_type: row.get(10)?,
        activity_category: row.get(11)?,
        detected_activity: row.get(12)?,
        extracted_signals_json: None,
        project_match_json: None,
        idle_time_secs: row.get(13)?,
        active_time_secs: row.get(14)?,
        user_action: row.get(15)?,
    })
}

fn day_bounds(date: NaiveDate) -> (i64, i64) {
    let start = date.and_time(NaiveTime::MIN).and_utc();
    let end = start.checked_add_signed(Duration::days(1)).unwrap_or(start);
    (start.timestamp(), end.timestamp())
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn int_to_bool(value: i64) -> bool {
    value != 0
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn map_connection_error(operation: &str, err: PulseArcError) -> CommonError {
    CommonError::Storage { message: err.to_string(), operation: Some(operation.to_string()) }
}

fn map_storage_error(operation: &str, err: StorageError) -> CommonError {
    CommonError::from(err.with_operation(operation.to_string()))
}

fn map_serialization_error(operation: &str, err: serde_json::Error) -> CommonError {
    CommonError::Serialization {
        message: format!("{operation}: {err}"),
        format: Some("json".into()),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn save_and_find_segment_by_date() {
        let (repo, _manager, _guard) = setup_repository();
        let segment = sample_segment("seg-1", 1_700_000_000, 1_700_000_300);

        repo.save_segment(&segment).expect("segment saved");

        let date = NaiveDate::from_ymd_opt(2023, 11, 14).unwrap();
        let segments = repo.find_segments_by_date(date).expect("segments fetched");

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].id, "seg-1");
    }

    #[test]
    fn find_unprocessed_segments_respects_limit() {
        let (repo, _manager, _guard) = setup_repository();
        for idx in 0..5 {
            let seg =
                sample_segment(&format!("seg-{idx}"), 1_700_000_000 + idx * 60, 1_700_000_030);
            repo.save_segment(&seg).expect("segment saved");
        }

        let segments = repo.find_unprocessed_segments(2).expect("segments fetched");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].id, "seg-0");
        assert_eq!(segments[1].id, "seg-1");
    }

    #[test]
    fn mark_processed_updates_flag() {
        let (repo, manager, _guard) = setup_repository();
        let segment = sample_segment("seg-processed", 1_700_000_000, 1_700_000_300);
        repo.save_segment(&segment).expect("segment saved");

        repo.mark_processed(&segment.id).expect("mark processed");

        let conn = manager.get_connection().expect("connection");
        let processed: i64 = conn
            .query_row(
                "SELECT processed FROM activity_segments WHERE id = ?1",
                &[&segment.id],
                |row| row.get(0),
            )
            .expect("query processed flag");
        assert_eq!(processed, 1);
    }

    #[test]
    fn find_segments_by_date_excludes_end_timestamp() {
        let (repo, _manager, _guard) = setup_repository();
        let within = sample_segment("seg-in", 1_700_000_000, 1_700_000_060);
        let boundary = sample_segment("seg-out", 1_700_036_800, 1_700_036_860); // exactly next day start
        repo.save_segment(&within).expect("segment saved");
        repo.save_segment(&boundary).expect("segment saved");

        let date = NaiveDate::from_ymd_opt(2023, 11, 14).unwrap();
        let segments = repo.find_segments_by_date(date).expect("segments fetched");

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].id, "seg-in");
    }

    #[test]
    fn find_unprocessed_segments_returns_error_when_table_missing() {
        let (repo, manager, _guard) = setup_repository();

        {
            let conn = manager.get_connection().expect("connection");
            conn.execute("DROP TABLE activity_segments", []).expect("drop table");
        }

        let err = repo.find_unprocessed_segments(1).expect_err("should error");
        assert!(matches!(err, CommonError::Storage { .. }));
    }

    fn setup_repository() -> (SqlCipherSegmentRepository, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_path = temp_dir.path().join("segments.db");
        let manager =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        manager.run_migrations().expect("schema created");

        let repo = SqlCipherSegmentRepository::new(manager.clone());
        (repo, manager, temp_dir)
    }

    fn sample_segment(id: &str, start_ts: i64, end_ts: i64) -> ActivitySegment {
        ActivitySegment {
            id: id.to_string(),
            start_ts,
            end_ts,
            primary_app: "PulseArc".into(),
            normalized_label: "Coding".into(),
            sample_count: 10,
            dictionary_keys: None,
            created_at: start_ts,
            processed: false,
            snapshot_ids: vec!["snap-1".into(), "snap-2".into()],
            work_type: Some("focus".into()),
            activity_category: "development".into(),
            detected_activity: "working".into(),
            extracted_signals_json: None,
            project_match_json: None,
            idle_time_secs: 0,
            active_time_secs: 60,
            user_action: None,
        }
    }
}
