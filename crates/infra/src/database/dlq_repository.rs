//! Dead Letter Queue (DLQ) Repository implementation
//!
//! Manages failed batch operations and retry logic.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use pulsearc_common::storage::error::{StorageError, StorageResult};
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_core::batch::ports::DlqRepository as DlqRepositoryPort;
use pulsearc_domain::{BatchQueue, DlqBatch, PulseArcError, Result as DomainResult};
use rusqlite::{params, Row};
use tokio::task;

use super::manager::DbManager;

/// SqlCipher-based DLQ repository
pub struct SqlCipherDlqRepository {
    db: Arc<DbManager>,
}

impl SqlCipherDlqRepository {
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DlqRepositoryPort for SqlCipherDlqRepository {
    async fn move_batch_to_dlq(&self, batch_id: &str, error: &str) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let batch_id = batch_id.to_string();
        let error = error.to_string();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            move_to_dlq(&conn, &batch_id, &error).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_dlq_batches(&self) -> DomainResult<Vec<BatchQueue>> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Vec<BatchQueue>> {
            let conn = db.get_connection()?;
            query_dlq_batches(&conn).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_dlq_batches_with_details(&self) -> DomainResult<Vec<DlqBatch>> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Vec<DlqBatch>> {
            let conn = db.get_connection()?;
            query_dlq_batches_with_details(&conn).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn reset_batch_for_retry(&self, batch_id: &str) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let batch_id = batch_id.to_string();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            reset_batch_to_pending(&conn, &batch_id).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn retry_failed_batch(&self, batch_id: &str) -> DomainResult<()> {
        self.reset_batch_for_retry(batch_id).await
    }
}

// ============================================================================
// SQL Operations (synchronous)
// ============================================================================

fn move_to_dlq(conn: &SqlCipherConnection, batch_id: &str, error: &str) -> StorageResult<()> {
    let now = Utc::now().timestamp();

    // First, get the batch from batch_queue
    let sql = "SELECT batch_id, activity_count, status, created_at
               FROM batch_queue
               WHERE batch_id = ?1";

    let (activity_count, original_status, created_at): (i32, String, i64) =
        conn.query_row(sql, params![batch_id], |row| Ok((row.get(1)?, row.get(2)?, row.get(3)?)))?;

    // Insert into DLQ (or replace if it exists)
    conn.execute(
        "INSERT OR REPLACE INTO batch_dlq (batch_id, activity_count, original_status,
                                            error_message, error_code, created_at, failed_at, attempts)
         VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, 1)",
        params![batch_id, activity_count, original_status, error, created_at, now],
    )?;

    // Mark the batch as failed in batch_queue
    conn.execute(
        "UPDATE batch_queue
         SET status = 'failed', error_message = ?1, processed_at = ?2
         WHERE batch_id = ?3",
        params![error, now, batch_id],
    )?;

    Ok(())
}

fn query_dlq_batches(conn: &SqlCipherConnection) -> StorageResult<Vec<BatchQueue>> {
    let sql = "SELECT bq.batch_id, bq.activity_count, bq.status, bq.created_at, bq.processed_at,
                      bq.error_message, bq.processing_started_at, bq.worker_id, bq.lease_expires_at,
                      bq.time_entries_created, bq.openai_cost
               FROM batch_queue bq
               INNER JOIN batch_dlq dlq ON bq.batch_id = dlq.batch_id
               ORDER BY dlq.failed_at DESC";

    let mut stmt = conn.prepare(sql)?;
    stmt.query_map(params![], map_batch_queue_row)
}

fn query_dlq_batches_with_details(conn: &SqlCipherConnection) -> StorageResult<Vec<DlqBatch>> {
    let sql = "SELECT batch_id, activity_count, error_message, error_code, created_at, failed_at, attempts
               FROM batch_dlq
               ORDER BY failed_at DESC";

    let mut stmt = conn.prepare(sql)?;
    stmt.query_map(params![], map_dlq_batch_row)
}

fn reset_batch_to_pending(conn: &SqlCipherConnection, batch_id: &str) -> StorageResult<()> {
    // Move batch back to pending status
    conn.execute(
        "UPDATE batch_queue
         SET status = 'pending', error_message = NULL, worker_id = NULL,
             lease_expires_at = NULL, processing_started_at = NULL
         WHERE batch_id = ?1",
        params![batch_id],
    )?;

    // Remove from DLQ
    conn.execute("DELETE FROM batch_dlq WHERE batch_id = ?1", params![batch_id])?;

    Ok(())
}

fn map_batch_queue_row(row: &Row<'_>) -> rusqlite::Result<BatchQueue> {
    use std::str::FromStr;

    use pulsearc_domain::BatchStatus;

    let status_str: String = row.get(2)?;
    let status = BatchStatus::from_str(&status_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;

    Ok(BatchQueue {
        batch_id: row.get(0)?,
        activity_count: row.get(1)?,
        status,
        created_at: row.get(3)?,
        processed_at: row.get(4)?,
        error_message: row.get(5)?,
        processing_started_at: row.get(6)?,
        worker_id: row.get(7)?,
        lease_expires_at: row.get(8)?,
        time_entries_created: row.get(9)?,
        openai_cost: row.get(10)?,
    })
}

fn map_dlq_batch_row(row: &Row<'_>) -> rusqlite::Result<DlqBatch> {
    Ok(DlqBatch {
        batch_id: row.get(0)?,
        activity_count: row.get(1)?,
        error_message: row.get(2)?,
        error_code: row.get(3)?,
        created_at: row.get(4)?,
        failed_at: row.get(5)?,
        attempts: row.get(6)?,
    })
}

// ============================================================================
// Error Mapping
// ============================================================================

fn map_storage_error(err: StorageError) -> PulseArcError {
    match err {
        StorageError::WrongKeyOrNotEncrypted => {
            PulseArcError::Security("sqlcipher key rejected or database not encrypted".into())
        }
        StorageError::Timeout(seconds) => {
            PulseArcError::Database(format!("database timeout after {seconds}s"))
        }
        StorageError::Connection(message)
        | StorageError::Query(message)
        | StorageError::DatabaseError(message)
        | StorageError::Encryption(message)
        | StorageError::Migration(message)
        | StorageError::Keychain(message)
        | StorageError::InvalidConfig(message) => PulseArcError::Database(message),
        StorageError::SchemaVersionMismatch { expected, found } => PulseArcError::Database(
            format!("schema version mismatch (expected {expected}, found {found})"),
        ),
        StorageError::PoolExhausted => PulseArcError::Database("connection pool exhausted".into()),
        StorageError::Common(common_err) => PulseArcError::Database(common_err.to_string()),
        StorageError::Io(io_err) => PulseArcError::Database(io_err.to_string()),
        StorageError::Rusqlite(sql_err) => PulseArcError::Database(sql_err.to_string()),
        StorageError::R2d2(r2d2_err) => PulseArcError::Database(r2d2_err.to_string()),
        StorageError::SerdeJson(json_err) => PulseArcError::Database(json_err.to_string()),
    }
}

fn map_join_error(err: task::JoinError) -> PulseArcError {
    if err.is_cancelled() {
        PulseArcError::Internal("blocking task cancelled".into())
    } else {
        PulseArcError::Internal(format!("blocking task failed: {err}"))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use pulsearc_domain::BatchStatus;
    use tempfile::TempDir;

    use super::*;

    const TEST_KEY: &str = "test_key_64_chars_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn test_move_batch_to_dlq() {
        let (repo, manager, _dir) = setup_repository().await;

        // Create a batch first
        let batch = sample_batch("batch-1", BatchStatus::Failed);
        save_batch_directly(&manager, &batch).await;

        repo.move_batch_to_dlq("batch-1", "Test error").await.expect("batch moved to DLQ");

        let dlq_batches = repo.get_dlq_batches_with_details().await.expect("dlq batches retrieved");

        assert_eq!(dlq_batches.len(), 1);
        assert_eq!(dlq_batches[0].batch_id, "batch-1");
        assert_eq!(dlq_batches[0].error_message.as_deref(), Some("Test error"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_dlq_batches() {
        let (repo, manager, _dir) = setup_repository().await;

        let batch1 = sample_batch("batch-2", BatchStatus::Failed);
        save_batch_directly(&manager, &batch1).await;
        repo.move_batch_to_dlq("batch-2", "Error 1").await.expect("batch 2 moved");

        let batch2 = sample_batch("batch-3", BatchStatus::Failed);
        save_batch_directly(&manager, &batch2).await;
        repo.move_batch_to_dlq("batch-3", "Error 2").await.expect("batch 3 moved");

        let dlq_batches = repo.get_dlq_batches().await.expect("batches retrieved");

        assert_eq!(dlq_batches.len(), 2);
        assert!(dlq_batches.iter().any(|b| b.batch_id == "batch-2"));
        assert!(dlq_batches.iter().any(|b| b.batch_id == "batch-3"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_reset_batch_for_retry() {
        let (repo, manager, _dir) = setup_repository().await;

        let batch = sample_batch("batch-4", BatchStatus::Failed);
        save_batch_directly(&manager, &batch).await;
        repo.move_batch_to_dlq("batch-4", "Test error").await.expect("batch moved to DLQ");

        repo.reset_batch_for_retry("batch-4").await.expect("batch reset");

        // Check that batch is back in pending status
        let conn = manager.get_connection().expect("connection");
        let status: String = conn
            .query_row(
                "SELECT status FROM batch_queue WHERE batch_id = ?1",
                params!["batch-4"],
                |row| row.get(0),
            )
            .expect("status query");

        assert_eq!(status, "pending");

        // Check that batch is removed from DLQ
        let dlq_batches = repo.get_dlq_batches_with_details().await.expect("dlq batches retrieved");
        assert!(!dlq_batches.iter().any(|b| b.batch_id == "batch-4"));
    }

    // ========================================================================
    // Test Helpers
    // ========================================================================

    async fn setup_repository() -> (SqlCipherDlqRepository, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("dlq.db");

        let manager =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        manager.run_migrations().expect("migrations run");

        let repo = SqlCipherDlqRepository::new(manager.clone());
        (repo, manager, temp_dir)
    }

    fn sample_batch(batch_id: &str, status: BatchStatus) -> BatchQueue {
        BatchQueue {
            batch_id: batch_id.to_string(),
            activity_count: 10,
            status,
            created_at: 1700000000,
            processed_at: None,
            error_message: None,
            processing_started_at: None,
            worker_id: None,
            lease_expires_at: None,
            time_entries_created: 0,
            openai_cost: 0.0,
        }
    }

    async fn save_batch_directly(manager: &Arc<DbManager>, batch: &BatchQueue) {
        let conn = manager.get_connection().expect("connection");
        let status_str = batch.status.to_string();

        conn.execute(
            "INSERT INTO batch_queue (batch_id, activity_count, status, created_at, processed_at,
                                       error_message, processing_started_at, worker_id, lease_expires_at,
                                       time_entries_created, openai_cost)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                &batch.batch_id,
                batch.activity_count,
                status_str,
                batch.created_at,
                batch.processed_at,
                &batch.error_message,
                batch.processing_started_at,
                &batch.worker_id,
                batch.lease_expires_at,
                batch.time_entries_created,
                batch.openai_cost,
            ],
        )
        .expect("batch inserted");
    }
}
