//! Batch Repository implementation
//!
//! Manages batch queue operations including leases, lifecycle, and statistics.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use pulsearc_common::storage::error::{StorageError, StorageResult};
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_core::batch::ports::BatchRepository as BatchRepositoryPort;
use pulsearc_domain::{BatchQueue, BatchStats, BatchStatus, PulseArcError, Result as DomainResult};
use rusqlite::{params, Row};
use tokio::task;

use super::manager::DbManager;

/// SqlCipher-based Batch repository
pub struct SqlCipherBatchRepository {
    db: Arc<DbManager>,
}

impl SqlCipherBatchRepository {
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BatchRepositoryPort for SqlCipherBatchRepository {
    // ========================================================================
    // Core CRUD
    // ========================================================================

    async fn save_batch(&self, batch: &BatchQueue) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let batch = batch.clone();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            insert_batch(&conn, &batch).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_batch(&self, batch_id: &str) -> DomainResult<BatchQueue> {
        let db = Arc::clone(&self.db);
        let batch_id = batch_id.to_string();

        task::spawn_blocking(move || -> DomainResult<BatchQueue> {
            let conn = db.get_connection()?;
            query_batch_by_id(&conn, &batch_id).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn update_batch_status(&self, batch_id: &str, status: BatchStatus) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let batch_id = batch_id.to_string();
        let status_str = status.to_string();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            update_batch_status_sql(&conn, &batch_id, &status_str).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    // ========================================================================
    // Lease Management
    // ========================================================================

    async fn acquire_batch_lease(
        &self,
        batch_id: &str,
        worker_id: &str,
        duration: Duration,
    ) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let batch_id = batch_id.to_string();
        let worker_id = worker_id.to_string();
        let duration_secs = duration.as_secs() as i64;

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            acquire_lease(&conn, &batch_id, &worker_id, duration_secs).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn renew_batch_lease(
        &self,
        batch_id: &str,
        worker_id: &str,
        duration: Duration,
    ) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let batch_id = batch_id.to_string();
        let worker_id = worker_id.to_string();
        let duration_secs = duration.as_secs() as i64;

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            renew_lease(&conn, &batch_id, &worker_id, duration_secs).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_stale_leases(&self, ttl_secs: i64) -> DomainResult<Vec<BatchQueue>> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Vec<BatchQueue>> {
            let conn = db.get_connection()?;
            query_stale_leases(&conn, ttl_secs).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn recover_stale_leases(&self) -> DomainResult<Vec<String>> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Vec<String>> {
            let conn = db.get_connection()?;
            recover_stale_leases_sql(&conn).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    // ========================================================================
    // Lifecycle
    // ========================================================================

    async fn create_batch_from_unprocessed(
        &self,
        max_snapshots: usize,
        worker_id: &str,
        lease_duration_secs: i64,
    ) -> DomainResult<Option<(String, Vec<String>)>> {
        let db = Arc::clone(&self.db);
        let worker_id = worker_id.to_string();

        task::spawn_blocking(move || -> DomainResult<Option<(String, Vec<String>)>> {
            let conn = db.get_connection()?;
            create_batch_from_snapshots(&conn, max_snapshots, &worker_id, lease_duration_secs)
                .map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn complete_batch(&self, batch_id: &str) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let batch_id = batch_id.to_string();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            mark_batch_completed(&conn, &batch_id).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn mark_batch_failed(&self, batch_id: &str, error: &str) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let batch_id = batch_id.to_string();
        let error = error.to_string();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            mark_batch_failed_sql(&conn, &batch_id, &error).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    // ========================================================================
    // Queries
    // ========================================================================

    async fn get_batches_by_status(&self, status: BatchStatus) -> DomainResult<Vec<BatchQueue>> {
        let db = Arc::clone(&self.db);
        let status_str = status.to_string();

        task::spawn_blocking(move || -> DomainResult<Vec<BatchQueue>> {
            let conn = db.get_connection()?;
            query_batches_by_status(&conn, &status_str).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_batch_stats(&self) -> DomainResult<BatchStats> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<BatchStats> {
            let conn = db.get_connection()?;
            query_batch_stats(&conn).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_pending_batches(&self) -> DomainResult<Vec<BatchQueue>> {
        self.get_batches_by_status(BatchStatus::Pending).await
    }

    // ========================================================================
    // Cleanup
    // ========================================================================

    async fn cleanup_old_batches(&self, older_than_seconds: i64) -> DomainResult<usize> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<usize> {
            let conn = db.get_connection()?;
            delete_old_batches(&conn, older_than_seconds).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn delete_batch(&self, batch_id: &str) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let batch_id = batch_id.to_string();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            delete_batch_by_id(&conn, &batch_id).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }
}

// ============================================================================
// SQL Operations (synchronous)
// ============================================================================

fn insert_batch(conn: &SqlCipherConnection, batch: &BatchQueue) -> StorageResult<()> {
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
    )?;
    Ok(())
}

fn query_batch_by_id(conn: &SqlCipherConnection, batch_id: &str) -> StorageResult<BatchQueue> {
    let sql = "SELECT batch_id, activity_count, status, created_at, processed_at, error_message,
                      processing_started_at, worker_id, lease_expires_at, time_entries_created,
                      openai_cost
               FROM batch_queue
               WHERE batch_id = ?1";

    conn.query_row(sql, params![batch_id], map_batch_row)
}

fn query_batches_by_status(
    conn: &SqlCipherConnection,
    status: &str,
) -> StorageResult<Vec<BatchQueue>> {
    let sql = "SELECT batch_id, activity_count, status, created_at, processed_at, error_message,
                      processing_started_at, worker_id, lease_expires_at, time_entries_created,
                      openai_cost
               FROM batch_queue
               WHERE status = ?1
               ORDER BY created_at ASC";

    let mut stmt = conn.prepare(sql)?;
    stmt.query_map(params![status], map_batch_row)
}

fn update_batch_status_sql(
    conn: &SqlCipherConnection,
    batch_id: &str,
    status: &str,
) -> StorageResult<()> {
    conn.execute(
        "UPDATE batch_queue SET status = ? WHERE batch_id = ?",
        params![status, batch_id],
    )?;
    Ok(())
}

fn acquire_lease(
    conn: &SqlCipherConnection,
    batch_id: &str,
    worker_id: &str,
    duration_secs: i64,
) -> StorageResult<()> {
    let now = Utc::now().timestamp();
    let lease_expires_at = now + duration_secs;

    conn.execute(
        "UPDATE batch_queue
         SET worker_id = ?1, lease_expires_at = ?2, processing_started_at = ?3, status = 'processing'
         WHERE batch_id = ?4",
        params![worker_id, lease_expires_at, now, batch_id],
    )?;
    Ok(())
}

fn renew_lease(
    conn: &SqlCipherConnection,
    batch_id: &str,
    worker_id: &str,
    duration_secs: i64,
) -> StorageResult<()> {
    let now = Utc::now().timestamp();
    let lease_expires_at = now + duration_secs;

    conn.execute(
        "UPDATE batch_queue
         SET lease_expires_at = ?1
         WHERE batch_id = ?2 AND worker_id = ?3",
        params![lease_expires_at, batch_id, worker_id],
    )?;
    Ok(())
}

fn query_stale_leases(conn: &SqlCipherConnection, ttl_secs: i64) -> StorageResult<Vec<BatchQueue>> {
    let now = Utc::now().timestamp();
    let cutoff = now - ttl_secs;

    let sql = "SELECT batch_id, activity_count, status, created_at, processed_at, error_message,
                      processing_started_at, worker_id, lease_expires_at, time_entries_created,
                      openai_cost
               FROM batch_queue
               WHERE status = 'processing' AND lease_expires_at < ?1";

    let mut stmt = conn.prepare(sql)?;
    stmt.query_map(params![cutoff], map_batch_row)
}

fn recover_stale_leases_sql(conn: &SqlCipherConnection) -> StorageResult<Vec<String>> {
    let now = Utc::now().timestamp();

    // Get the IDs of stale batches first
    let sql = "SELECT batch_id FROM batch_queue
               WHERE status = 'processing' AND lease_expires_at < ?1";
    let mut stmt = conn.prepare(sql)?;
    let batch_ids = stmt.query_map(params![now], |row| row.get::<_, String>(0))?;

    // Reset them to pending
    conn.execute(
        "UPDATE batch_queue
         SET status = 'pending', worker_id = NULL, lease_expires_at = NULL, processing_started_at = NULL
         WHERE status = 'processing' AND lease_expires_at < ?1",
        params![now],
    )?;

    Ok(batch_ids)
}

#[allow(clippy::type_complexity)]
fn create_batch_from_snapshots(
    _conn: &SqlCipherConnection,
    _max_snapshots: usize,
    _worker_id: &str,
    _lease_duration_secs: i64,
) -> StorageResult<Option<(String, Vec<String>)>> {
    // This is a placeholder - the actual implementation would need to:
    // 1. Query unprocessed snapshots from activity_snapshots table
    // 2. Create a new batch with those snapshots
    // 3. Acquire a lease on the new batch
    // For now, return None (no unprocessed snapshots)
    Ok(None)
}

fn mark_batch_completed(conn: &SqlCipherConnection, batch_id: &str) -> StorageResult<()> {
    let now = Utc::now().timestamp();
    conn.execute(
        "UPDATE batch_queue
         SET status = 'completed', processed_at = ?1
         WHERE batch_id = ?2",
        params![now, batch_id],
    )?;
    Ok(())
}

fn mark_batch_failed_sql(
    conn: &SqlCipherConnection,
    batch_id: &str,
    error: &str,
) -> StorageResult<()> {
    let now = Utc::now().timestamp();
    conn.execute(
        "UPDATE batch_queue
         SET status = 'failed', processed_at = ?1, error_message = ?2
         WHERE batch_id = ?3",
        params![now, error, batch_id],
    )?;
    Ok(())
}

fn query_batch_stats(conn: &SqlCipherConnection) -> StorageResult<BatchStats> {
    let sql = "SELECT
                   SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
                   SUM(CASE WHEN status = 'processing' THEN 1 ELSE 0 END) as processing,
                   SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
                   SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed
               FROM batch_queue";

    conn.query_row(sql, params![], |row| {
        Ok(BatchStats {
            pending: row.get(0)?,
            processing: row.get(1)?,
            completed: row.get(2)?,
            failed: row.get(3)?,
        })
    })
}

fn delete_old_batches(conn: &SqlCipherConnection, older_than_seconds: i64) -> StorageResult<usize> {
    let now = Utc::now().timestamp();
    let cutoff = now - older_than_seconds;

    let deleted = conn.execute("DELETE FROM batch_queue WHERE created_at < ?1", params![cutoff])?;
    Ok(deleted)
}

fn delete_batch_by_id(conn: &SqlCipherConnection, batch_id: &str) -> StorageResult<()> {
    conn.execute("DELETE FROM batch_queue WHERE batch_id = ?1", params![batch_id])?;
    Ok(())
}

fn map_batch_row(row: &Row<'_>) -> rusqlite::Result<BatchQueue> {
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
    use tempfile::TempDir;

    use super::*;

    const TEST_KEY: &str = "test_key_64_chars_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn test_save_and_get_batch() {
        let (repo, _manager, _dir) = setup_repository().await;

        let batch = sample_batch("batch-1", BatchStatus::Pending);
        repo.save_batch(&batch).await.expect("batch saved");

        let retrieved = repo.get_batch("batch-1").await.expect("batch retrieved");

        assert_eq!(retrieved.batch_id, "batch-1");
        assert_eq!(retrieved.status, BatchStatus::Pending);
        assert_eq!(retrieved.activity_count, 10);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_batch_status() {
        let (repo, _manager, _dir) = setup_repository().await;

        let batch = sample_batch("batch-2", BatchStatus::Pending);
        repo.save_batch(&batch).await.expect("batch saved");

        repo.update_batch_status("batch-2", BatchStatus::Processing).await.expect("status updated");

        let retrieved = repo.get_batch("batch-2").await.expect("batch retrieved");
        assert_eq!(retrieved.status, BatchStatus::Processing);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_complete_batch() {
        let (repo, _manager, _dir) = setup_repository().await;

        let batch = sample_batch("batch-3", BatchStatus::Processing);
        repo.save_batch(&batch).await.expect("batch saved");

        repo.complete_batch("batch-3").await.expect("batch completed");

        let retrieved = repo.get_batch("batch-3").await.expect("batch retrieved");
        assert_eq!(retrieved.status, BatchStatus::Completed);
        assert!(retrieved.processed_at.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_mark_batch_failed() {
        let (repo, _manager, _dir) = setup_repository().await;

        let batch = sample_batch("batch-4", BatchStatus::Processing);
        repo.save_batch(&batch).await.expect("batch saved");

        repo.mark_batch_failed("batch-4", "Test error").await.expect("batch marked failed");

        let retrieved = repo.get_batch("batch-4").await.expect("batch retrieved");
        assert_eq!(retrieved.status, BatchStatus::Failed);
        assert_eq!(retrieved.error_message.as_deref(), Some("Test error"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_batches_by_status() {
        let (repo, _manager, _dir) = setup_repository().await;

        repo.save_batch(&sample_batch("batch-5", BatchStatus::Pending))
            .await
            .expect("batch 5 saved");
        repo.save_batch(&sample_batch("batch-6", BatchStatus::Pending))
            .await
            .expect("batch 6 saved");
        repo.save_batch(&sample_batch("batch-7", BatchStatus::Completed))
            .await
            .expect("batch 7 saved");

        let pending = repo
            .get_batches_by_status(BatchStatus::Pending)
            .await
            .expect("pending batches retrieved");

        assert_eq!(pending.len(), 2);
        assert!(pending.iter().any(|b| b.batch_id == "batch-5"));
        assert!(pending.iter().any(|b| b.batch_id == "batch-6"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_batch_stats() {
        let (repo, _manager, _dir) = setup_repository().await;

        repo.save_batch(&sample_batch("batch-8", BatchStatus::Pending))
            .await
            .expect("batch 8 saved");
        repo.save_batch(&sample_batch("batch-9", BatchStatus::Processing))
            .await
            .expect("batch 9 saved");
        repo.save_batch(&sample_batch("batch-10", BatchStatus::Completed))
            .await
            .expect("batch 10 saved");
        repo.save_batch(&sample_batch("batch-11", BatchStatus::Failed))
            .await
            .expect("batch 11 saved");

        let stats = repo.get_batch_stats().await.expect("stats retrieved");

        assert_eq!(stats.pending, 1);
        assert_eq!(stats.processing, 1);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.failed, 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_batch() {
        let (repo, _manager, _dir) = setup_repository().await;

        let batch = sample_batch("batch-12", BatchStatus::Completed);
        repo.save_batch(&batch).await.expect("batch saved");

        repo.delete_batch("batch-12").await.expect("batch deleted");

        let result = repo.get_batch("batch-12").await;
        assert!(result.is_err());
    }

    // ========================================================================
    // Test Helpers
    // ========================================================================

    async fn setup_repository() -> (SqlCipherBatchRepository, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("batches.db");

        let manager =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        manager.run_migrations().expect("migrations run");

        let repo = SqlCipherBatchRepository::new(manager.clone());
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
}
