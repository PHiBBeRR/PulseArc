//! Token Usage Repository implementation
//!
//! Tracks AI token usage and costs for classification batches.

use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_common::storage::error::{StorageError, StorageResult};
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_core::sync::ports::TokenUsageRepository as TokenUsageRepositoryPort;
use pulsearc_domain::{PulseArcError, Result as DomainResult, TokenUsage};
use rusqlite::{params, Row};
use tokio::task;
use uuid::Uuid;

use super::manager::DbManager;

/// SqlCipher-based Token Usage repository
pub struct SqlCipherTokenUsageRepository {
    db: Arc<DbManager>,
}

impl SqlCipherTokenUsageRepository {
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TokenUsageRepositoryPort for SqlCipherTokenUsageRepository {
    async fn record_token_usage(&self, usage: &TokenUsage) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let usage = usage.clone();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            insert_token_usage(&conn, &usage, false).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_token_usage_by_batch(&self, batch_id: &str) -> DomainResult<TokenUsage> {
        let db = Arc::clone(&self.db);
        let batch_id = batch_id.to_string();

        task::spawn_blocking(move || -> DomainResult<TokenUsage> {
            let conn = db.get_connection()?;
            query_token_usage_by_batch(&conn, &batch_id).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn record_estimated_usage(&self, usage: &TokenUsage) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let usage = usage.clone();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            insert_token_usage(&conn, &usage, false).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn record_actual_usage(&self, usage: &TokenUsage) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let usage = usage.clone();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            insert_token_usage(&conn, &usage, true).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn delete_token_usage_by_batch(&self, batch_id: &str) -> DomainResult<usize> {
        let db = Arc::clone(&self.db);
        let batch_id = batch_id.to_string();

        task::spawn_blocking(move || -> DomainResult<usize> {
            let conn = db.get_connection()?;
            delete_token_usage(&conn, &batch_id).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }
}

// ============================================================================
// SQL Operations (synchronous)
// ============================================================================

fn insert_token_usage(
    conn: &SqlCipherConnection,
    usage: &TokenUsage,
    is_actual: bool,
) -> StorageResult<()> {
    let id = Uuid::new_v4().to_string();
    let is_actual_flag = if is_actual { 1 } else { 0 };

    conn.execute(
        "INSERT INTO token_usage (id, batch_id, user_id, input_tokens, output_tokens,
                                   estimated_cost_usd, is_actual, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            &usage.batch_id,
            &usage.user_id,
            usage.input_tokens,
            usage.output_tokens,
            usage.estimated_cost_usd,
            is_actual_flag,
            usage.timestamp,
        ],
    )?;
    Ok(())
}

fn query_token_usage_by_batch(
    conn: &SqlCipherConnection,
    batch_id: &str,
) -> StorageResult<TokenUsage> {
    let sql = "SELECT batch_id, user_id, input_tokens, output_tokens, estimated_cost_usd, timestamp
               FROM token_usage
               WHERE batch_id = ?1 AND is_actual = 1
               ORDER BY timestamp DESC
               LIMIT 1";

    conn.query_row(sql, params![batch_id], map_token_usage_row)
}

fn delete_token_usage(conn: &SqlCipherConnection, batch_id: &str) -> StorageResult<usize> {
    let deleted = conn.execute("DELETE FROM token_usage WHERE batch_id = ?1", params![batch_id])?;
    Ok(deleted)
}

fn map_token_usage_row(row: &Row<'_>) -> rusqlite::Result<TokenUsage> {
    Ok(TokenUsage {
        batch_id: row.get(0)?,
        user_id: row.get(1)?,
        input_tokens: row.get(2)?,
        output_tokens: row.get(3)?,
        estimated_cost_usd: row.get(4)?,
        timestamp: row.get(5)?,
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
    async fn test_record_and_get_token_usage() {
        let (repo, _manager, _dir) = setup_repository().await;

        let usage = sample_usage("batch-1", "user-1", 100, 50);
        repo.record_actual_usage(&usage).await.expect("usage recorded");

        let retrieved = repo.get_token_usage_by_batch("batch-1").await.expect("query succeeded");

        assert_eq!(retrieved.batch_id, "batch-1");
        assert_eq!(retrieved.user_id, "user-1");
        assert_eq!(retrieved.input_tokens, 100);
        assert_eq!(retrieved.output_tokens, 50);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_record_estimated_and_actual_usage() {
        let (repo, _manager, _dir) = setup_repository().await;

        // Record estimated usage
        let estimated = sample_usage("batch-2", "user-2", 90, 45);
        repo.record_estimated_usage(&estimated).await.expect("estimated recorded");

        // Record actual usage
        let actual = sample_usage("batch-2", "user-2", 100, 50);
        repo.record_actual_usage(&actual).await.expect("actual recorded");

        // Should get the actual usage (is_actual = 1)
        let retrieved = repo.get_token_usage_by_batch("batch-2").await.expect("query succeeded");

        assert_eq!(retrieved.input_tokens, 100);
        assert_eq!(retrieved.output_tokens, 50);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_token_usage_by_batch() {
        let (repo, _manager, _dir) = setup_repository().await;

        let usage1 = sample_usage("batch-3", "user-3", 100, 50);
        let usage2 = sample_usage("batch-4", "user-3", 200, 100);

        repo.record_actual_usage(&usage1).await.expect("usage 1 recorded");
        repo.record_actual_usage(&usage2).await.expect("usage 2 recorded");

        let deleted = repo.delete_token_usage_by_batch("batch-3").await.expect("delete succeeded");

        assert!(deleted > 0);

        // batch-3 should be gone
        let result = repo.get_token_usage_by_batch("batch-3").await;
        assert!(result.is_err());

        // batch-4 should still exist
        let retrieved = repo.get_token_usage_by_batch("batch-4").await.expect("query succeeded");
        assert_eq!(retrieved.batch_id, "batch-4");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_missing_batch_returns_error() {
        let (repo, _manager, _dir) = setup_repository().await;

        let result = repo.get_token_usage_by_batch("nonexistent-batch").await;

        assert!(result.is_err());
    }

    // ========================================================================
    // Test Helpers
    // ========================================================================

    async fn setup_repository() -> (SqlCipherTokenUsageRepository, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("token_usage.db");

        let manager =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        manager.run_migrations().expect("migrations run");

        let repo = SqlCipherTokenUsageRepository::new(manager.clone());
        (repo, manager, temp_dir)
    }

    fn sample_usage(
        batch_id: &str,
        user_id: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> TokenUsage {
        TokenUsage {
            batch_id: batch_id.to_string(),
            user_id: user_id.to_string(),
            input_tokens,
            output_tokens,
            estimated_cost_usd: (input_tokens as f64 * 0.00001) + (output_tokens as f64 * 0.00003),
            timestamp: 1700000000,
        }
    }
}
