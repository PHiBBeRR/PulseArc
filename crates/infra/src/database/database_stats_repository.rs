//! SQLCipher-backed database statistics repository.
//!
//! Provides read-only introspection of database state via PRAGMA queries
//! and maintenance operations (VACUUM). All operations use spawn_blocking
//! to avoid blocking the async runtime.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use pulsearc_common::storage::error::StorageError;
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_common::storage::types::Connection as ConnectionTrait;
use pulsearc_core::database_stats_ports::DatabaseStatsPort;
use pulsearc_domain::types::{DatabaseSize, HealthStatus, TableStats};
use pulsearc_domain::{PulseArcError, Result as DomainResult};
use rusqlite::types::{Type, ValueRef};
use tokio::task;

use super::manager::DbManager;
use crate::errors::InfraError;

/// Database statistics repository backed by SQLCipher.
pub struct SqlCipherDatabaseStatsRepository {
    db: Arc<DbManager>,
}

impl SqlCipherDatabaseStatsRepository {
    /// Construct a repository backed by the shared database manager.
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl DatabaseStatsPort for SqlCipherDatabaseStatsRepository {
    async fn get_database_size(&self) -> DomainResult<DatabaseSize> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<DatabaseSize> {
            let conn = db.get_connection()?;

            // Read-only PRAGMA introspection - no parameterization needed. Some PRAGMAs
            // return results as TEXT, so parse via String before converting to
            // integers.
            let page_count = read_pragma_u64(&conn, "page_count")?;
            let page_size = read_pragma_u64(&conn, "page_size")?;
            let freelist_count = read_pragma_u64(&conn, "freelist_count")?;

            // Prefer filesystem-reported size; fall back to logical estimate
            let size_bytes = std::fs::metadata(db.path())
                .map(|meta| meta.len())
                .unwrap_or_else(|_| page_count.saturating_mul(page_size));

            Ok(DatabaseSize { size_bytes, page_count, page_size, freelist_count })
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_table_stats(&self) -> DomainResult<Vec<TableStats>> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Vec<TableStats>> {
            let conn = db.get_connection()?;

            // Query all table names from sqlite_master
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
                .map_err(map_storage_error)?;

            let table_names =
                stmt.query_map(&[], |row| row.get::<_, String>(0)).map_err(map_storage_error)?;

            let mut stats = Vec::new();

            for name in table_names {
                // Skip internal SQLite tables
                if name.starts_with("sqlite_") {
                    continue;
                }

                // Count rows in each table
                // Note: SQLite doesn't support parameterising identifiers, so we sanitize and
                // splice
                let quoted = name.replace('"', "\"\"");
                let count_query = format!("SELECT COUNT(*) FROM \"{}\"", quoted);
                let row_count_i64: i64 = conn
                    .query_row(&count_query, &[], |row| row.get(0))
                    .map_err(map_storage_error)?;
                let row_count = row_count_i64.max(0) as u64;

                stats.push(TableStats { name, row_count });
            }

            Ok(stats)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_unprocessed_count(&self) -> DomainResult<i64> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<i64> {
            let conn = db.get_connection()?;

            // Count snapshots that haven't been processed yet
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM activity_snapshots WHERE processed = 0",
                    &[],
                    |row| row.get(0),
                )
                .map_err(map_storage_error)?;

            Ok(count)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn vacuum_database(&self) -> DomainResult<()> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;

            // VACUUM rebuilds the database to reclaim space
            ConnectionTrait::execute(&conn, "VACUUM", &[]).map_err(map_storage_error)?;

            Ok(())
        })
        .await
        .map_err(map_join_error)?
    }

    async fn check_database_health(&self) -> DomainResult<HealthStatus> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<HealthStatus> {
            let start = Instant::now();
            let conn_result = db.get_connection();

            match conn_result {
                Ok(conn) => {
                    // Simple connectivity test
                    match conn.query_row("SELECT 1", &[], |row| row.get::<_, i32>(0)) {
                        Ok(1) => {
                            let elapsed = start.elapsed().as_millis() as u64;
                            Ok(HealthStatus {
                                is_healthy: true,
                                message: "Database is healthy".to_string(),
                                response_time_ms: elapsed,
                            })
                        }
                        Ok(_) => Ok(HealthStatus {
                            is_healthy: false,
                            message: "Unexpected query result".to_string(),
                            response_time_ms: start.elapsed().as_millis() as u64,
                        }),
                        Err(err) => Ok(HealthStatus {
                            is_healthy: false,
                            message: format!("Query failed: {}", err),
                            response_time_ms: start.elapsed().as_millis() as u64,
                        }),
                    }
                }
                Err(err) => Ok(HealthStatus {
                    is_healthy: false,
                    message: format!("Connection failed: {}", err),
                    response_time_ms: start.elapsed().as_millis() as u64,
                }),
            }
        })
        .await
        .map_err(map_join_error)?
    }
}

// ============================================================================
// Error Mapping
// ============================================================================

/// Map `StorageError` to `PulseArcError` for domain layer compatibility.
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
        StorageError::Rusqlite(sql_err) => PulseArcError::from(InfraError::from(sql_err)),
        StorageError::R2d2(r2d2_err) => PulseArcError::Database(r2d2_err.to_string()),
        StorageError::SerdeJson(json_err) => PulseArcError::Database(json_err.to_string()),
    }
}

/// Map `JoinError` to `PulseArcError` for async task failures.
fn map_join_error(err: task::JoinError) -> PulseArcError {
    if err.is_cancelled() {
        PulseArcError::Internal("blocking database stats task cancelled".into())
    } else {
        PulseArcError::Internal(format!("blocking database stats task failed: {err}"))
    }
}

fn read_pragma_u64(conn: &SqlCipherConnection, pragma: &str) -> DomainResult<u64> {
    let sql = format!("PRAGMA {pragma}");
    conn.query_row(&sql, &[], |row| value_ref_to_i64(row.get_ref(0)?))
        .map(|value| value.max(0) as u64)
        .map_err(|err| PulseArcError::Database(format!("failed to read PRAGMA {pragma}: {err}")))
}

fn value_ref_to_i64(value: ValueRef<'_>) -> Result<i64, rusqlite::Error> {
    match value {
        ValueRef::Integer(v) => Ok(v),
        ValueRef::Real(v) => Ok(v as i64),
        ValueRef::Text(bytes) => {
            let text = std::str::from_utf8(bytes).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
            text.parse::<i64>().map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })
        }
        ValueRef::Null => Ok(0),
        ValueRef::Blob(_) => {
            Err(rusqlite::Error::InvalidColumnType(0, Type::Blob.to_string(), Type::Integer))
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_database_size() {
        let (repo, _manager, _temp_dir) = setup_repository().await;

        let size = repo.get_database_size().await.expect("get database size");

        // Database should have pages and a size
        assert!(size.size_bytes > 0, "size_bytes should be > 0");
        assert!(size.page_count > 0, "page_count should be > 0");
        assert_eq!(size.page_size, 4096, "default page_size is 4096");
        // freelist_count can be 0 for a new database
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_table_stats() {
        let (repo, manager, _temp_dir) = setup_repository().await;

        // Insert test data into activity_snapshots table
        {
            let conn = manager.get_connection().expect("connection");
            conn.execute(
                "INSERT INTO activity_snapshots (id, timestamp, activity_context_json, detected_activity, work_type, activity_category, primary_app, processed, created_at, is_idle)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    "snap-1",
                    1_700_000_000_i64,
                    "{}",
                    "working",
                    "focus",
                    "development",
                    "VSCode",
                    0,
                    1_700_000_000_i64,
                    0,
                ],
            )
            .expect("insert snapshot");
        }

        let stats = repo.get_table_stats().await.expect("get table stats");

        // Should have multiple tables (activity_snapshots, segments, etc.)
        assert!(!stats.is_empty(), "should have at least one table");

        // Find activity_snapshots table
        let snapshots_stat = stats
            .iter()
            .find(|s| s.name == "activity_snapshots")
            .expect("activity_snapshots table");

        assert_eq!(snapshots_stat.row_count, 1, "should have 1 row");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_vacuum_database() {
        let (repo, _manager, _temp_dir) = setup_repository().await;

        // VACUUM should succeed without errors
        repo.vacuum_database().await.expect("vacuum succeeds");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_health_check() {
        let (repo, _manager, _temp_dir) = setup_repository().await;

        let health = repo.check_database_health().await.expect("health check");

        assert!(health.is_healthy, "database should be healthy");
        assert_eq!(health.message, "Database is healthy");
        assert!(health.response_time_ms < 1000, "response time should be < 1s");
    }

    async fn setup_repository() -> (SqlCipherDatabaseStatsRepository, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("tempdir created");
        let db_path = temp_dir.path().join("stats_test.db");

        let manager =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        manager.run_migrations().expect("migrations run");

        let repo = SqlCipherDatabaseStatsRepository::new(manager.clone());
        (repo, manager, temp_dir)
    }
}
