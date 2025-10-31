//! Repository implementations for core domain

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use log::warn;
use pulsearc_common::error::{CommonError, CommonResult};
use pulsearc_core::tracking::ports::{
    SegmentRepository as SegmentRepositoryPort, SnapshotRepository as SnapshotRepositoryPort,
};
use pulsearc_core::{ActivityRepository, TimeEntryRepository};
use pulsearc_domain::{
    ActivitySegment, ActivitySnapshot, OutboxStatus, PulseArcError, Result, TimeEntry,
    TimeEntryOutbox, TimeEntryParams,
};
use rusqlite::{params, Row};
use uuid::Uuid;

use super::manager::DbManager;

/// SQLite implementation of ActivityRepository
pub struct SqliteActivityRepository {
    db: Arc<DbManager>,
}

impl SqliteActivityRepository {
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ActivityRepository for SqliteActivityRepository {
    async fn save_snapshot(&self, snapshot: ActivitySnapshot) -> Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            conn.execute(
                "INSERT INTO activity_snapshots (id, timestamp, activity_context_json, detected_activity, primary_app, processed, created_at, is_idle) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    &snapshot.id,
                    snapshot.timestamp,
                    &snapshot.activity_context_json,
                    &snapshot.detected_activity,
                    &snapshot.primary_app,
                    snapshot.processed,
                    snapshot.created_at,
                    snapshot.is_idle,
                ),
            )
            .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    async fn get_snapshots(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ActivitySnapshot>> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;
            let mut stmt = conn
                .prepare("SELECT id, timestamp, activity_context_json, detected_activity, work_type, activity_category, primary_app, processed, batch_id, created_at, processed_at, is_idle, idle_duration_secs FROM activity_snapshots WHERE timestamp BETWEEN ?1 AND ?2")
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            let snapshots = stmt
                .query_map((start.timestamp(), end.timestamp()), |row| {
                    Ok(ActivitySnapshot {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        activity_context_json: row.get(2)?,
                        detected_activity: row.get(3)?,
                        work_type: row.get(4)?,
                        activity_category: row.get(5)?,
                        primary_app: row.get(6)?,
                        processed: row.get(7)?,
                        batch_id: row.get(8)?,
                        created_at: row.get(9)?,
                        processed_at: row.get(10)?,
                        is_idle: row.get(11)?,
                        idle_duration_secs: row.get(12)?,
                    })
                })
                .map_err(|e| PulseArcError::Database(e.to_string()))?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(snapshots)
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    async fn delete_old_snapshots(&self, before: DateTime<Utc>) -> Result<usize> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;
            let deleted = conn
                .execute(
                    "DELETE FROM activity_snapshots WHERE timestamp < ?1",
                    [before.timestamp()],
                )
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(deleted)
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }
}

/// SQLite implementation of TimeEntryRepository
pub struct SqliteTimeEntryRepository {
    db: Arc<DbManager>,
}

impl SqliteTimeEntryRepository {
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TimeEntryRepository for SqliteTimeEntryRepository {
    async fn save_entry(&self, entry: TimeEntry) -> Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            conn.execute(
                "INSERT INTO time_entries (id, start_time, end_time, duration_seconds, description, project_id, wbs_code)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    entry.id.to_string(),
                    entry.start_time.timestamp(),
                    entry.end_time.map(|dt| dt.timestamp()),
                    entry.duration_seconds,
                    entry.description,
                    entry.project_id,
                    entry.wbs_code,
                ),
            )
            .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    async fn get_entries(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<TimeEntry>> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;
            let mut stmt = conn
                .prepare("SELECT id, start_time, end_time, duration_seconds, description, project_id, wbs_code FROM time_entries WHERE start_time BETWEEN ?1 AND ?2")
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            let entries = stmt
                .query_map((start.timestamp(), end.timestamp()), |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                })
                .map_err(|e| PulseArcError::Database(e.to_string()))?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| PulseArcError::Database(e.to_string()))?
                .into_iter()
                .filter_map(|(id, start_time, end_time, duration_seconds, description, project_id, wbs_code)| {
                    let id = Uuid::parse_str(&id).ok()?;
                    let start_time = DateTime::from_timestamp(start_time, 0)?;
                    let end_time = end_time.and_then(|ts| DateTime::from_timestamp(ts, 0));

                    Some(TimeEntry::new(TimeEntryParams {
                        id,
                        start_time,
                        end_time,
                        duration_seconds,
                        description,
                        project_id,
                        wbs_code,
                    }))
                })
                .collect();

            Ok(entries)
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    async fn update_entry(&self, entry: TimeEntry) -> Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            conn.execute(
                "UPDATE time_entries SET start_time = ?2, end_time = ?3, duration_seconds = ?4, description = ?5, project_id = ?6, wbs_code = ?7 WHERE id = ?1",
                (
                    entry.id.to_string(),
                    entry.start_time.timestamp(),
                    entry.end_time.map(|dt| dt.timestamp()),
                    entry.duration_seconds,
                    entry.description,
                    entry.project_id,
                    entry.wbs_code,
                ),
            )
            .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    async fn delete_entry(&self, id: Uuid) -> Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            conn.execute("DELETE FROM time_entries WHERE id = ?1", [id.to_string()])
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }
}

const OUTBOX_PENDING_QUERY: &str = "SELECT id, idempotency_key, user_id, payload_json, backend_cuid, status, attempts, \
    last_error, retry_after, created_at, sent_at, correlation_id, local_status, remote_status, sap_entry_id, \
    next_attempt_at, error_code, last_forwarded_at, wbs_code, target, description, auto_applied, version, \
    last_modified_by, last_modified_at FROM time_entry_outbox \
    WHERE status = 'pending' AND (retry_after IS NULL OR retry_after <= ?1) \
    ORDER BY created_at ASC";

const OUTBOX_ALL_QUERY: &str = "SELECT id, idempotency_key, user_id, payload_json, backend_cuid, status, attempts, \
    last_error, retry_after, created_at, sent_at, correlation_id, local_status, remote_status, sap_entry_id, \
    next_attempt_at, error_code, last_forwarded_at, wbs_code, target, description, auto_applied, version, \
    last_modified_by, last_modified_at FROM time_entry_outbox ORDER BY created_at ASC";

/// SQLite implementation of the outbox repository
pub struct SqliteOutboxRepository {
    db: Arc<DbManager>,
}

impl SqliteOutboxRepository {
    /// Create a new repository backed by the shared DbManager
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }

    /// Insert (or replace) an outbox entry
    pub async fn insert_entry(&self, entry: &TimeEntryOutbox) -> Result<()> {
        let db = self.db.clone();
        let entry = entry.clone();

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            conn.execute(
                "INSERT OR REPLACE INTO time_entry_outbox (id, idempotency_key, user_id, payload_json, \
                    backend_cuid, status, attempts, last_error, retry_after, created_at, sent_at, \
                    correlation_id, local_status, remote_status, sap_entry_id, next_attempt_at, error_code, \
                    last_forwarded_at, wbs_code, target, description, auto_applied, version, \
                    last_modified_by, last_modified_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, \
                    ?20, ?21, ?22, ?23, ?24, ?25)",
                params![
                    entry.id,
                    entry.idempotency_key,
                    entry.user_id,
                    entry.payload_json,
                    entry.backend_cuid,
                    entry.status.to_string(),
                    entry.attempts,
                    entry.last_error,
                    entry.retry_after,
                    entry.created_at,
                    entry.sent_at,
                    entry.correlation_id,
                    entry.local_status,
                    entry.remote_status,
                    entry.sap_entry_id,
                    entry.next_attempt_at,
                    entry.error_code,
                    entry.last_forwarded_at,
                    entry.wbs_code,
                    entry.target,
                    entry.description,
                    if entry.auto_applied { 1 } else { 0 },
                    entry.version,
                    entry.last_modified_by,
                    entry.last_modified_at,
                ],
            )
            .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    /// Fetch pending entries that are ready to be retried as of the provided
    /// timestamp.
    pub async fn get_pending_entries_ready_for_retry(
        &self,
        as_of_timestamp: i64,
    ) -> Result<Vec<TimeEntryOutbox>> {
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            let mut stmt = conn
                .prepare(OUTBOX_PENDING_QUERY)
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            let entries = stmt
                .query_map([as_of_timestamp], |row| map_outbox_row(row))
                .map_err(|e| PulseArcError::Database(e.to_string()))?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(entries)
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    /// Fetch all outbox entries. Primarily used by regression tests to verify
    /// parsing logic.
    pub async fn list_all_entries(&self) -> Result<Vec<TimeEntryOutbox>> {
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            let mut stmt = conn
                .prepare(OUTBOX_ALL_QUERY)
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            let entries = stmt
                .query_map([], |row| map_outbox_row(row))
                .map_err(|e| PulseArcError::Database(e.to_string()))?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(entries)
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }
}

fn map_outbox_row(row: &Row<'_>) -> rusqlite::Result<TimeEntryOutbox> {
    let id: String = row.get(0)?;
    let status_raw: String = row.get(5)?;
    let status = parse_outbox_status(&id, &status_raw);

    Ok(TimeEntryOutbox {
        id,
        idempotency_key: row.get(1)?,
        user_id: row.get(2)?,
        payload_json: row.get(3)?,
        backend_cuid: row.get(4)?,
        status,
        attempts: row.get(6)?,
        last_error: row.get(7)?,
        retry_after: row.get(8)?,
        created_at: row.get(9)?,
        sent_at: row.get(10)?,
        correlation_id: row.get(11)?,
        local_status: row.get(12)?,
        remote_status: row.get(13)?,
        sap_entry_id: row.get(14)?,
        next_attempt_at: row.get(15)?,
        error_code: row.get(16)?,
        last_forwarded_at: row.get(17)?,
        wbs_code: row.get(18)?,
        target: row.get(19)?,
        description: row.get(20)?,
        auto_applied: row.get::<_, i64>(21)? != 0,
        version: row.get(22)?,
        last_modified_by: row.get(23)?,
        last_modified_at: row.get(24)?,
    })
}

fn parse_outbox_status(id: &str, status_raw: &str) -> OutboxStatus {
    match OutboxStatus::from_str(status_raw) {
        Ok(status) => status,
        Err(err) => {
            warn!(
                "Invalid outbox status '{}' for entry {} – defaulting to pending ({})",
                status_raw, id, err
            );
            OutboxStatus::Pending
        }
    }
}

const SEGMENT_BY_DATE_QUERY: &str = "SELECT id, start_ts, end_ts, primary_app, normalized_label, sample_count, \
    dictionary_keys, created_at, processed, snapshot_ids, work_type, activity_category, detected_activity, \
    extracted_signals_json, project_match_json, idle_time_secs, active_time_secs, user_action \
    FROM activity_segments \
    WHERE start_ts >= ?1 AND start_ts < ?2 \
    ORDER BY start_ts";

const SEGMENT_UNPROCESSED_QUERY: &str = "SELECT id, start_ts, end_ts, primary_app, normalized_label, sample_count, \
    dictionary_keys, created_at, processed, snapshot_ids, work_type, activity_category, detected_activity, \
    extracted_signals_json, project_match_json, idle_time_secs, active_time_secs, user_action \
    FROM activity_segments \
    WHERE processed = 0 \
    ORDER BY start_ts \
    LIMIT ?1";

const SNAPSHOT_RANGE_QUERY: &str = "SELECT id, timestamp, activity_context_json, detected_activity, work_type, \
    activity_category, primary_app, processed, batch_id, created_at, processed_at, is_idle, idle_duration_secs \
    FROM activity_snapshots \
    WHERE timestamp >= ?1 AND timestamp < ?2 \
    ORDER BY timestamp";

const SNAPSHOT_COUNT_BY_DATE_QUERY: &str =
    "SELECT COUNT(*) FROM activity_snapshots WHERE timestamp >= ?1 AND timestamp < ?2";

/// SQLite-backed implementation of the SegmentRepository port.
pub struct SqliteSegmentRepository {
    db: Arc<DbManager>,
}

impl SqliteSegmentRepository {
    /// Create a repository backed by the shared database manager.
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

impl SegmentRepositoryPort for SqliteSegmentRepository {
    fn save_segment(&self, segment: &ActivitySegment) -> CommonResult<()> {
        let conn =
            self.db.get_connection().map_err(|e| map_db_error("segment_save_connection", e))?;

        let snapshot_ids = serde_json::to_string(&segment.snapshot_ids)
            .map_err(|e| map_serialization_error("segment_save_snapshot_ids", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO activity_segments (id, start_ts, end_ts, primary_app, \
                normalized_label, sample_count, dictionary_keys, created_at, processed, \
                snapshot_ids, work_type, activity_category, detected_activity, extracted_signals_json, \
                project_match_json, idle_time_secs, active_time_secs, user_action)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                &segment.id,
                segment.start_ts,
                segment.end_ts,
                &segment.primary_app,
                &segment.normalized_label,
                segment.sample_count,
                &segment.dictionary_keys,
                segment.created_at,
                if segment.processed { 1 } else { 0 },
                snapshot_ids,
                &segment.work_type,
                &segment.activity_category,
                &segment.detected_activity,
                &segment.extracted_signals_json,
                &segment.project_match_json,
                segment.idle_time_secs,
                segment.active_time_secs,
                &segment.user_action,
            ],
        )
        .map_err(|e| map_sqlite_error("segment_save_execute", e))?;

        Ok(())
    }

    fn find_segments_by_date(&self, date: NaiveDate) -> CommonResult<Vec<ActivitySegment>> {
        let conn =
            self.db.get_connection().map_err(|e| map_db_error("segment_find_connection", e))?;

        let (day_start, day_end) = day_bounds(date);
        let mut stmt = conn
            .prepare(SEGMENT_BY_DATE_QUERY)
            .map_err(|e| map_sqlite_error("segment_find_prepare", e))?;

        let segments = stmt
            .query_map(params![day_start, day_end], |row| map_activity_segment(row))
            .map_err(|e| map_sqlite_error("segment_find_query", e))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| map_sqlite_error("segment_find_collect", e))?;

        Ok(segments)
    }

    fn find_unprocessed_segments(&self, limit: usize) -> CommonResult<Vec<ActivitySegment>> {
        let conn = self
            .db
            .get_connection()
            .map_err(|e| map_db_error("segment_unprocessed_connection", e))?;

        let mut stmt = conn
            .prepare(SEGMENT_UNPROCESSED_QUERY)
            .map_err(|e| map_sqlite_error("segment_unprocessed_prepare", e))?;

        let segments = stmt
            .query_map([limit as i64], |row| map_activity_segment(row))
            .map_err(|e| map_sqlite_error("segment_unprocessed_query", e))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| map_sqlite_error("segment_unprocessed_collect", e))?;

        Ok(segments)
    }

    fn mark_processed(&self, segment_id: &str) -> CommonResult<()> {
        let conn =
            self.db.get_connection().map_err(|e| map_db_error("segment_mark_connection", e))?;

        conn.execute("UPDATE activity_segments SET processed = 1 WHERE id = ?1", [segment_id])
            .map_err(|e| map_sqlite_error("segment_mark_execute", e))?;

        Ok(())
    }
}

/// SQLite-backed implementation of the SnapshotRepository port.
pub struct SqliteSnapshotRepository {
    db: Arc<DbManager>,
}

impl SqliteSnapshotRepository {
    /// Create a repository backed by the shared database manager.
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

impl SnapshotRepositoryPort for SqliteSnapshotRepository {
    fn find_snapshots_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> CommonResult<Vec<ActivitySnapshot>> {
        let conn =
            self.db.get_connection().map_err(|e| map_db_error("snapshot_range_connection", e))?;

        let mut stmt = conn
            .prepare(SNAPSHOT_RANGE_QUERY)
            .map_err(|e| map_sqlite_error("snapshot_range_prepare", e))?;

        let snapshots = stmt
            .query_map(params![start.timestamp(), end.timestamp()], |row| {
                map_activity_snapshot(row)
            })
            .map_err(|e| map_sqlite_error("snapshot_range_query", e))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| map_sqlite_error("snapshot_range_collect", e))?;

        Ok(snapshots)
    }

    fn count_snapshots_by_date(&self, date: NaiveDate) -> CommonResult<usize> {
        let conn =
            self.db.get_connection().map_err(|e| map_db_error("snapshot_count_connection", e))?;

        let (day_start, day_end) = day_bounds(date);

        let mut stmt = conn
            .prepare(SNAPSHOT_COUNT_BY_DATE_QUERY)
            .map_err(|e| map_sqlite_error("snapshot_count_prepare", e))?;

        let count: usize = stmt
            .query_row(params![day_start, day_end], |row| row.get::<_, i64>(0))
            .map(|value| value as usize)
            .map_err(|e| map_sqlite_error("snapshot_count_query", e))?;

        Ok(count)
    }
}

fn map_activity_segment(row: &Row<'_>) -> rusqlite::Result<ActivitySegment> {
    let id: String = row.get(0)?;
    let snapshot_ids_raw: String = row.get(9)?;

    let snapshot_ids =
        serde_json::from_str::<Vec<String>>(&snapshot_ids_raw).unwrap_or_else(|err| {
            warn!(
                "Failed to parse snapshot_ids for segment {} – defaulting to empty list ({})",
                id, err
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
        processed: row.get::<_, i64>(8)? != 0,
        snapshot_ids,
        work_type: row.get(10)?,
        activity_category: row.get(11)?,
        detected_activity: row.get(12)?,
        extracted_signals_json: row.get(13)?,
        project_match_json: row.get(14)?,
        idle_time_secs: row.get(15)?,
        active_time_secs: row.get(16)?,
        user_action: row.get(17)?,
    })
}

fn map_activity_snapshot(row: &Row<'_>) -> rusqlite::Result<ActivitySnapshot> {
    Ok(ActivitySnapshot {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        activity_context_json: row.get(2)?,
        detected_activity: row.get(3)?,
        work_type: row.get(4)?,
        activity_category: row.get(5)?,
        primary_app: row.get(6)?,
        processed: row.get::<_, i64>(7)? != 0,
        batch_id: row.get(8)?,
        created_at: row.get(9)?,
        processed_at: row.get(10)?,
        is_idle: row.get::<_, i64>(11)? != 0,
        idle_duration_secs: row.get(12)?,
    })
}

fn day_bounds(date: NaiveDate) -> (i64, i64) {
    let start =
        date.and_hms_opt(0, 0, 0).expect("00:00:00 is always a valid time").and_utc().timestamp();
    let end = date
        .succ_opt()
        .expect("NaiveDate::succ should succeed for valid dates")
        .and_hms_opt(0, 0, 0)
        .expect("00:00:00 is always a valid time")
        .and_utc()
        .timestamp();
    (start, end)
}

fn map_db_error(operation: &str, err: PulseArcError) -> CommonError {
    CommonError::Storage { message: err.to_string(), operation: Some(operation.to_string()) }
}

fn map_sqlite_error(operation: &str, err: rusqlite::Error) -> CommonError {
    CommonError::Storage { message: err.to_string(), operation: Some(operation.to_string()) }
}

fn map_serialization_error(operation: &str, err: serde_json::Error) -> CommonError {
    CommonError::Serialization {
        message: format!("{}: {}", operation, err),
        format: Some("json".to_string()),
    }
}
