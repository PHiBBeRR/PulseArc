//! SQLCipher-backed implementation of the outbox queue port.
//!
//! Provides the async adapter used by the sync layer for enqueueing,
//! dequeuing and updating outbox entries with retry bookkeeping. The
//! implementation mirrors the legacy SQLite behaviour while adopting the
//! new SQLCipher connection manager introduced in Phase 3A.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use pulsearc_common::storage::error::StorageError;
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_core::OutboxQueue as OutboxQueuePort;
use pulsearc_domain::{OutboxStatus, PulseArcError, Result as DomainResult, TimeEntryOutbox};
use rusqlite::{Row, ToSql};
use tokio::task;
use tracing::warn;

use super::manager::DbManager;
use crate::errors::InfraError;

const MAX_RETRY_ATTEMPTS: i32 = 5;
const BASE_RETRY_DELAY_SECS: i64 = 60;
const MAX_BACKOFF_EXPONENT: u32 = 4;

/// SQLCipher-backed outbox repository.
pub struct SqlCipherOutboxRepository {
    db: Arc<DbManager>,
}

impl SqlCipherOutboxRepository {
    /// Construct a repository backed by the shared SQLCipher manager.
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }

    fn insert_entry(conn: &SqlCipherConnection, entry: &TimeEntryOutbox) -> DomainResult<()> {
        let auto_applied = bool_to_int(entry.auto_applied);
        let params: [&dyn ToSql; 25] = [
            &entry.id,
            &entry.idempotency_key,
            &entry.user_id,
            &entry.payload_json,
            &entry.backend_cuid,
            &entry.status.to_string(),
            &entry.attempts,
            &entry.last_error,
            &entry.retry_after,
            &entry.created_at,
            &entry.sent_at,
            &entry.correlation_id,
            &entry.local_status,
            &entry.remote_status,
            &entry.sap_entry_id,
            &entry.next_attempt_at,
            &entry.error_code,
            &entry.last_forwarded_at,
            &entry.wbs_code,
            &entry.target,
            &entry.description,
            &auto_applied,
            &entry.version,
            &entry.last_modified_by,
            &entry.last_modified_at,
        ];

        conn.execute(OUTBOX_INSERT_SQL, params.as_slice())
            .map_err(StorageError::from)
            .map(|_| ())
            .map_err(map_storage_error)
    }

    fn fetch_pending_ready(
        conn: &SqlCipherConnection,
        limit: usize,
        as_of_timestamp: i64,
    ) -> DomainResult<Vec<TimeEntryOutbox>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let limit = usize_to_i64(limit);
        let mut stmt = conn.prepare(OUTBOX_DEQUEUE_SQL).map_err(map_storage_error)?;
        let params: [&dyn ToSql; 2] = [&as_of_timestamp, &limit];
        stmt.query_map(params.as_slice(), map_outbox_row).map_err(map_storage_error)
    }

    fn set_entry_sent(conn: &SqlCipherConnection, id: &str) -> DomainResult<()> {
        let now = now_timestamp();
        let status = OutboxStatus::Sent.to_string();
        let params: [&dyn ToSql; 4] = [&status, &now, &now, &id];

        let updated = conn
            .execute(OUTBOX_MARK_SENT_SQL, params.as_slice())
            .map_err(StorageError::from)
            .map_err(map_storage_error)?;

        if updated == 0 {
            Err(PulseArcError::InvalidInput(format!("outbox entry {id} not found or already sent")))
        } else {
            Ok(())
        }
    }

    fn register_failure(conn: &SqlCipherConnection, id: &str, error: &str) -> DomainResult<()> {
        let attempts: i32 = conn
            .query_row(OUTBOX_SELECT_ATTEMPTS_SQL, &[&id as &dyn ToSql], |row| row.get(0))
            .map_err(map_storage_error)?;

        let new_attempts = attempts.saturating_add(1);
        let now = now_timestamp();
        let (status, retry_after) = failure_transition(new_attempts, now);

        let status_str = status.to_string();
        let error_str = error.to_owned();
        let retry_after_value = retry_after;
        let next_attempt_value = retry_after;
        let params: [&dyn ToSql; 7] = [
            &status_str,
            &new_attempts,
            &error_str,
            &retry_after_value,
            &next_attempt_value,
            &now,
            &id,
        ];

        let updated = conn
            .execute(OUTBOX_MARK_FAILED_SQL, params.as_slice())
            .map_err(StorageError::from)
            .map_err(map_storage_error)?;

        if updated == 0 {
            Err(PulseArcError::InvalidInput(format!("outbox entry {id} not found")))
        } else {
            Ok(())
        }
    }

    /// Return the number of entries currently queued with `pending` status.
    pub async fn pending_count(&self) -> DomainResult<i64> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<i64> {
            let conn = db.get_connection()?;
            conn.query_row(OUTBOX_PENDING_COUNT_SQL, &[], |row| row.get(0))
                .map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }
}

#[async_trait]
impl OutboxQueuePort for SqlCipherOutboxRepository {
    async fn enqueue(&self, entry: &TimeEntryOutbox) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let to_insert = entry.clone();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            Self::insert_entry(&conn, &to_insert)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn dequeue_batch(&self, limit: usize) -> DomainResult<Vec<TimeEntryOutbox>> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Vec<TimeEntryOutbox>> {
            let conn = db.get_connection()?;
            let as_of = now_timestamp();
            Self::fetch_pending_ready(&conn, limit, as_of)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn mark_sent(&self, id: &str) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let id = id.to_owned();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            Self::set_entry_sent(&conn, &id)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn mark_failed(&self, id: &str, error: &str) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let id = id.to_owned();
        let error = error.to_owned();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            Self::register_failure(&conn, &id, &error)
        })
        .await
        .map_err(map_join_error)?
    }
}

const OUTBOX_INSERT_SQL: &str = "INSERT INTO time_entry_outbox (
        id, idempotency_key, user_id, payload_json, backend_cuid, status, attempts, last_error,
        retry_after, created_at, sent_at, correlation_id, local_status, remote_status, sap_entry_id,
        next_attempt_at, error_code, last_forwarded_at, wbs_code, target, description, auto_applied,
        version, last_modified_by, last_modified_at
    ) VALUES (
        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
        ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25
    )";

const OUTBOX_DEQUEUE_SQL: &str = "SELECT
        id, idempotency_key, user_id, payload_json, backend_cuid, status, attempts, last_error,
        retry_after, created_at, sent_at, correlation_id, local_status, remote_status, sap_entry_id,
        next_attempt_at, error_code, last_forwarded_at, wbs_code, target, description, auto_applied,
        version, last_modified_by, last_modified_at
    FROM time_entry_outbox
    WHERE status = 'pending'
      AND (retry_after IS NULL OR retry_after <= ?1)
    ORDER BY created_at ASC, id ASC
    LIMIT ?2";

const OUTBOX_MARK_SENT_SQL: &str = "UPDATE time_entry_outbox
    SET status = ?1,
        sent_at = ?2,
        retry_after = NULL,
        next_attempt_at = NULL,
        last_error = NULL,
        last_modified_at = ?3
    WHERE id = ?4 AND status != 'sent'";

const OUTBOX_MARK_FAILED_SQL: &str = "UPDATE time_entry_outbox
    SET status = ?1,
        attempts = ?2,
        last_error = ?3,
        retry_after = ?4,
        next_attempt_at = ?5,
        sent_at = NULL,
        last_modified_at = ?6
    WHERE id = ?7";

const OUTBOX_SELECT_ATTEMPTS_SQL: &str = "SELECT attempts FROM time_entry_outbox WHERE id = ?1";

const OUTBOX_PENDING_COUNT_SQL: &str =
    "SELECT COUNT(*) FROM time_entry_outbox WHERE status = 'pending'";

fn map_outbox_row(row: &Row<'_>) -> rusqlite::Result<TimeEntryOutbox> {
    let id: String = row.get(0)?;
    let status_raw: String = row.get(5)?;
    let status = parse_status(&id, &status_raw);

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
        auto_applied: int_to_bool(row.get(21)?),
        version: row.get(22)?,
        last_modified_by: row.get(23)?,
        last_modified_at: row.get(24)?,
    })
}

fn parse_status(id: &str, raw: &str) -> OutboxStatus {
    match raw.parse::<OutboxStatus>() {
        Ok(status) => status,
        Err(err) => {
            warn!(
                entry_id = %id,
                raw_status = %raw,
                error = %err,
                "invalid outbox status returned by SQLCipher – defaulting to pending"
            );
            OutboxStatus::Pending
        }
    }
}

fn map_storage_error(err: StorageError) -> PulseArcError {
    match err {
        StorageError::Rusqlite(sql_err) => PulseArcError::from(InfraError::from(sql_err)),
        other => PulseArcError::Database(other.to_string()),
    }
}

fn map_join_error(err: task::JoinError) -> PulseArcError {
    if err.is_cancelled() {
        PulseArcError::Internal("outbox task cancelled".into())
    } else {
        PulseArcError::Internal(format!("outbox task panic: {err}"))
    }
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

fn failure_transition(new_attempts: i32, now: i64) -> (OutboxStatus, Option<i64>) {
    if new_attempts >= MAX_RETRY_ATTEMPTS {
        (OutboxStatus::Failed, None)
    } else {
        let delay = compute_retry_delay_seconds(new_attempts);
        let retry_after = now.saturating_add(delay);
        (OutboxStatus::Pending, Some(retry_after))
    }
}

fn compute_retry_delay_seconds(attempt: i32) -> i64 {
    if attempt <= 1 {
        return BASE_RETRY_DELAY_SECS;
    }

    let exponent = attempt.saturating_sub(1) as u32;
    let clamped = exponent.min(MAX_BACKOFF_EXPONENT);
    let multiplier = 1_i64.checked_shl(clamped).unwrap_or(i64::MAX);

    BASE_RETRY_DELAY_SECS.saturating_mul(multiplier)
}

fn now_timestamp() -> i64 {
    Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    type SentStatusRow = (String, Option<i64>, Option<i64>, Option<String>);
    type FailureStateRow = (i32, String, Option<i64>, Option<String>);
    type AttemptsStateRow = (i32, String, Option<i64>);

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn enqueue_and_dequeue_pending_entry() {
        let (repo, _manager, _temp_dir) = setup_repository().await;
        let entry = sample_entry("entry-1", 1_700_000_000);

        repo.enqueue(&entry).await.expect("enqueue succeeds");

        let pending = repo.pending_count().await.expect("pending count succeeds");
        assert_eq!(pending, 1);

        let entries = repo.dequeue_batch(5).await.expect("dequeue succeeds");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, entry.id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dequeue_with_zero_limit_returns_empty() {
        let (repo, _manager, _temp_dir) = setup_repository().await;

        let entries = repo.dequeue_batch(0).await.expect("dequeue succeeds");
        assert!(entries.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dequeue_respects_retry_after() {
        let (repo, manager, _temp_dir) = setup_repository().await;
        let mut entry = sample_entry("entry-1", 1_700_000_000);
        entry.retry_after = Some(now_timestamp().saturating_add(3_600));

        repo.enqueue(&entry).await.expect("enqueue succeeds");

        let entries = repo.dequeue_batch(10).await.expect("dequeue succeeds");
        assert!(entries.is_empty(), "entries with future retry_after must not be dequeued");

        // Clear retry_after directly and verify it becomes visible again
        let conn = manager.get_connection().expect("connection");
        conn.execute(
            "UPDATE time_entry_outbox SET retry_after = NULL WHERE id = ?1",
            [&entry.id as &dyn ToSql],
        )
        .expect("clear retry_after");

        let entries = repo.dequeue_batch(10).await.expect("dequeue succeeds");
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mark_sent_updates_status() {
        let (repo, manager, _temp_dir) = setup_repository().await;
        let entry = sample_entry("entry-1", now_timestamp());
        repo.enqueue(&entry).await.expect("enqueue succeeds");

        repo.mark_sent(&entry.id).await.expect("mark_sent succeeds");

        let conn = manager.get_connection().expect("connection");
        let (status, sent_at, retry_after, last_error): SentStatusRow = conn
            .query_row(
                "SELECT status, sent_at, retry_after, last_error FROM time_entry_outbox WHERE id = ?1",
                &[&entry.id as &dyn ToSql],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("row should exist");

        assert_eq!(status, OutboxStatus::Sent.to_string());
        assert!(sent_at.is_some(), "sent_at should be recorded");
        assert!(retry_after.is_none(), "retry_after cleared after success");
        assert!(last_error.is_none(), "last_error cleared after success");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mark_failed_tracks_retry_information() {
        let (repo, manager, _temp_dir) = setup_repository().await;
        let entry = sample_entry("entry-1", now_timestamp());
        repo.enqueue(&entry).await.expect("enqueue succeeds");

        let before = now_timestamp();
        repo.mark_failed(&entry.id, "network error").await.expect("mark_failed succeeds");

        let conn = manager.get_connection().expect("connection");
        let (attempts, status, retry_after, last_error): FailureStateRow = conn
            .query_row(
                "SELECT attempts, status, retry_after, last_error FROM time_entry_outbox WHERE id = ?1",
                &[&entry.id as &dyn ToSql],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("row should exist");

        assert_eq!(attempts, 1);
        assert_eq!(status, OutboxStatus::Pending.to_string());
        let retry_after = retry_after.expect("retry_after should be set");
        assert!(
            retry_after >= before.saturating_add(BASE_RETRY_DELAY_SECS),
            "retry_after should be at least base delay"
        );
        assert_eq!(last_error.as_deref(), Some("network error"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mark_failed_transitions_to_failed_status() {
        let (repo, manager, _temp_dir) = setup_repository().await;
        let mut entry = sample_entry("entry-1", now_timestamp());
        entry.attempts = MAX_RETRY_ATTEMPTS - 1;
        repo.enqueue(&entry).await.expect("enqueue succeeds");

        repo.mark_failed(&entry.id, "permanent failure").await.expect("mark_failed succeeds");

        let conn = manager.get_connection().expect("connection");
        let (attempts, status, retry_after): AttemptsStateRow = conn
            .query_row(
                "SELECT attempts, status, retry_after FROM time_entry_outbox WHERE id = ?1",
                &[&entry.id as &dyn ToSql],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("row should exist");

        assert_eq!(attempts, MAX_RETRY_ATTEMPTS);
        assert_eq!(status, OutboxStatus::Failed.to_string());
        assert!(retry_after.is_none(), "retry_after cleared for terminal failures");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pending_count_reflects_current_queue() {
        let (repo, _manager, _temp_dir) = setup_repository().await;

        let pending = sample_entry("pending-1", now_timestamp());
        repo.enqueue(&pending).await.expect("insert pending succeeds");

        let mut sent = sample_entry("sent-1", now_timestamp());
        sent.status = OutboxStatus::Sent;
        sent.sent_at = Some(now_timestamp());
        sent.local_status = Some("sent".into());
        repo.enqueue(&sent).await.expect("insert sent succeeds");

        let count = repo.pending_count().await.expect("pending count succeeds");
        assert_eq!(count, 1);
    }

    async fn setup_repository() -> (SqlCipherOutboxRepository, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("test.db");

        let manager = DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("manager created");
        manager.run_migrations().expect("migrations applied");
        let manager = Arc::new(manager);
        let repo = SqlCipherOutboxRepository::new(Arc::clone(&manager));

        (repo, manager, temp_dir)
    }

    fn sample_entry(id: &str, timestamp: i64) -> TimeEntryOutbox {
        TimeEntryOutbox {
            id: id.to_string(),
            idempotency_key: format!("{id}-idem"),
            user_id: "user-123".into(),
            payload_json: "{}".into(),
            backend_cuid: None,
            status: OutboxStatus::Pending,
            attempts: 0,
            last_error: None,
            retry_after: None,
            created_at: timestamp,
            sent_at: None,
            correlation_id: Some(String::new()),
            local_status: Some("pending".into()),
            remote_status: None,
            sap_entry_id: None,
            next_attempt_at: None,
            error_code: None,
            last_forwarded_at: None,
            wbs_code: None,
            target: "sap".into(),
            description: None,
            auto_applied: false,
            version: 1,
            last_modified_by: "system".into(),
            last_modified_at: Some(timestamp),
        }
    }
}
