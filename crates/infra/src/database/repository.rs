//! Repository implementations for core domain

use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use pulsearc_core::TimeEntryRepository;
use pulsearc_domain::{
    OutboxStatus, PulseArcError, Result, TimeEntry, TimeEntryOutbox, TimeEntryParams,
};
use rusqlite::{params, Row};
use tracing::warn;
use uuid::Uuid;

use super::manager::DbManager;

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

            conn.inner().execute(
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
            let mut stmt = conn.inner().prepare("SELECT id, start_time, end_time, duration_seconds, description, project_id, wbs_code FROM time_entries WHERE start_time BETWEEN ?1 AND ?2")
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            let mut rows = stmt
                .query((start.timestamp(), end.timestamp()))
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            let mut entries = Vec::new();

            while let Some(row) = rows
                .next()
                .map_err(|e| PulseArcError::Database(e.to_string()))?
            {
                let raw_id: String =
                    row.get(0).map_err(|e| PulseArcError::Database(e.to_string()))?;
                let id = Uuid::parse_str(&raw_id).map_err(|e| {
                    PulseArcError::Database(format!("invalid time entry id '{}': {}", raw_id, e))
                })?;

                let start_time_ts: i64 =
                    row.get(1).map_err(|e| PulseArcError::Database(e.to_string()))?;
                let start_time = DateTime::from_timestamp(start_time_ts, 0).ok_or_else(|| {
                    PulseArcError::Database(format!(
                        "invalid start timestamp '{}' for time entry {}",
                        start_time_ts, id
                    ))
                })?;

                let end_time = match row
                    .get::<_, Option<i64>>(2)
                    .map_err(|e| PulseArcError::Database(e.to_string()))?
                {
                    Some(ts) => Some(
                        DateTime::from_timestamp(ts, 0).ok_or_else(|| {
                            PulseArcError::Database(format!(
                                "invalid end timestamp '{}' for time entry {}",
                                ts, id
                            ))
                        })?,
                    ),
                    None => None,
                };

                let duration_seconds = row
                    .get::<_, Option<i64>>(3)
                    .map_err(|e| PulseArcError::Database(e.to_string()))?;
                let description: String =
                    row.get(4).map_err(|e| PulseArcError::Database(e.to_string()))?;
                let project_id = row
                    .get::<_, Option<String>>(5)
                    .map_err(|e| PulseArcError::Database(e.to_string()))?;
                let wbs_code = row
                    .get::<_, Option<String>>(6)
                    .map_err(|e| PulseArcError::Database(e.to_string()))?;

                entries.push(TimeEntry::new(TimeEntryParams {
                    id,
                    start_time,
                    end_time,
                    duration_seconds,
                    description,
                    project_id,
                    wbs_code,
                }));
            }

            Ok(entries)
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    async fn update_entry(&self, entry: TimeEntry) -> Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            conn.inner().execute(
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

            conn.inner()
                .execute("DELETE FROM time_entries WHERE id = ?1", [id.to_string()])
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

            conn.inner().execute(
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
                .inner()
                .prepare(OUTBOX_PENDING_QUERY)
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            let entries = stmt
                .query_map(params![as_of_timestamp], map_outbox_row)
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
                .inner()
                .prepare(OUTBOX_ALL_QUERY)
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            let entries = stmt
                .query_map(rusqlite::params![], map_outbox_row)
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
