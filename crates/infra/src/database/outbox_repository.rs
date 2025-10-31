//! SQLCipher-backed implementation of the outbox queue port.
//!
//! This module provides the initial skeleton for migrating the legacy
//! outbox queue to `SqlCipherConnection`. The current focus is on the
//! enqueue/dequeue path; status transitions and retry bookkeeping remain
//! to be ported in a follow-up task (see
//! docs/issues/PHASE-3-INFRA-TRACKING.md).

use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_common::storage::error::StorageError;
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_core::OutboxQueue as OutboxQueuePort;
use pulsearc_domain::{OutboxStatus, PulseArcError, Result as DomainResult, TimeEntryOutbox};
use rusqlite::{Row, ToSql};
use tokio::task;
use tracing::warn;

use super::manager::DbManager;
use crate::errors::InfraError;

/// SQLCipher-backed outbox repository (partial migration).
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

    fn fetch_pending(
        conn: &SqlCipherConnection,
        limit: usize,
    ) -> DomainResult<Vec<TimeEntryOutbox>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let limit = usize_to_i64(limit);
        let mut stmt = conn.prepare(OUTBOX_DEQUEUE_SQL).map_err(map_storage_error)?;
        let params: [&dyn ToSql; 1] = [&limit];
        stmt.query_map(params.as_slice(), map_outbox_row).map_err(map_storage_error)
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
            Self::fetch_pending(&conn, limit)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn mark_sent(&self, id: &str) -> DomainResult<()> {
        // TODO(phase3-outbox): wire SQLCipher update to transition pending rows to
        // sent.
        Err(PulseArcError::Internal(format!(
            "mark_sent not yet implemented for SqlCipherOutboxRepository ({id})"
        )))
    }

    async fn mark_failed(&self, id: &str, error: &str) -> DomainResult<()> {
        // TODO(phase3-outbox): persist failure details and retry bookkeeping once
        // migrated.
        Err(PulseArcError::Internal(format!(
            "mark_failed not yet implemented for SqlCipherOutboxRepository ({id}): {error}"
        )))
    }
}

const OUTBOX_INSERT_SQL: &str = "INSERT OR REPLACE INTO time_entry_outbox (
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
    ORDER BY created_at ASC
    LIMIT ?1";

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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn enqueue_and_dequeue_pending_entry() {
        let (repo, _manager, _temp_dir) = setup_repository().await;
        let entry = sample_entry("entry-1", 1_700_000_000);

        repo.enqueue(&entry).await.expect("enqueue succeeds");

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
    async fn mark_sent_returns_error_placeholder() {
        let (repo, _manager, _temp_dir) = setup_repository().await;

        let result = repo.mark_sent("missing").await;
        assert!(
            matches!(result, Err(PulseArcError::Internal(message)) if message.contains("not yet implemented"))
        );
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
            correlation_id: None,
            local_status: None,
            remote_status: None,
            sap_entry_id: None,
            next_attempt_at: None,
            error_code: None,
            last_forwarded_at: None,
            wbs_code: None,
            target: "time_entry".into(),
            description: None,
            auto_applied: false,
            version: 1,
            last_modified_by: "system".into(),
            last_modified_at: Some(timestamp),
        }
    }
}
