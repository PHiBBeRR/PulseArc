//! Database connection manager backed by the shared SQLCipher pool.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use pulsearc_common::storage::sqlcipher::{
    SqlCipherConnection, SqlCipherPool, SqlCipherPoolConfig,
};
use pulsearc_common::storage::StorageError;
use pulsearc_domain::{PulseArcError, Result};
use rusqlite::params;
use tracing::info;

use super::sqlcipher_pool::create_sqlcipher_pool;
use crate::errors::InfraError;

const SCHEMA_VERSION: i32 = 1;
const SCHEMA_SQL: &str = include_str!("schema.sql");

/// Database manager that wraps an [`SqlCipherPool`].
pub struct DbManager {
    pool: Arc<SqlCipherPool>,
    path: PathBuf,
}

impl DbManager {
    /// Create a new manager with the given pool size and SQLCipher key.
    pub fn new<P: AsRef<Path>>(
        db_path: P,
        pool_size: u32,
        encryption_key: Option<&str>,
    ) -> Result<Self> {
        let key = encryption_key.map(std::borrow::ToOwned::to_owned).ok_or_else(|| {
            PulseArcError::Security("database encryption key not provided".into())
        })?;

        let path = db_path.as_ref().to_path_buf();

        let config =
            SqlCipherPoolConfig { max_size: pool_size.max(1), ..SqlCipherPoolConfig::default() };

        let pool = create_sqlcipher_pool(&path, key, config)?;

        info!(
            db_path = %path.display(),
            max_connections = pool.metrics().max_pool_size(),
            "sqlcipher pool initialised"
        );

        Ok(Self { pool, path })
    }

    /// Borrow the underlying SQLCipher pool.
    pub fn pool(&self) -> &Arc<SqlCipherPool> {
        &self.pool
    }

    /// Acquire a SQLCipher connection from the pool.
    pub fn get_connection(&self) -> Result<SqlCipherConnection> {
        self.pool.get_sqlcipher_connection().map_err(map_storage_error)
    }

    /// Ensure the full schema exists on the current database.
    pub fn run_migrations(&self) -> Result<()> {
        let conn = self.get_connection()?;
        create_schema(&conn)?;
        Ok(())
    }

    /// Return the configured database path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Perform a health check to verify database connectivity.
    ///
    /// This method acquires a connection from the pool and executes a simple
    /// query to verify the database is accessible and responding.
    pub fn health_check(&self) -> Result<()> {
        let conn = self.get_connection()?;
        // Simple query to verify database is responsive
        conn.query_row("SELECT 1", params![], |row| row.get::<_, i32>(0))
            .map_err(map_storage_error)?;
        Ok(())
    }
}

fn create_schema(conn: &SqlCipherConnection) -> Result<()> {
    conn.execute_batch(SCHEMA_SQL).map_err(map_sql_error)?;
    conn.execute(
        "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (?, CAST(strftime('%s','now') AS INTEGER))",
        params![SCHEMA_VERSION],
    )
    .map_err(map_sql_error)?;
    Ok(())
}

fn map_sql_error(err: rusqlite::Error) -> PulseArcError {
    PulseArcError::from(InfraError::from(err))
}

fn map_storage_error(err: StorageError) -> PulseArcError {
    PulseArcError::Database(err.to_string())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn migrations_create_schema_version() {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("test.db");

        let manager = DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("manager created");
        manager.run_migrations().expect("migrations run");

        let conn = manager.get_connection().expect("connection acquired");
        let version: i32 =
            conn.query_row("SELECT version FROM schema_version", &[], |row| row.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn health_check_succeeds_for_valid_database() {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("test.db");

        let manager = DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("manager created");
        manager.run_migrations().expect("migrations run");

        // Health check should succeed
        manager.health_check().expect("health check passed");
    }

    #[test]
    fn health_check_fails_without_encryption_key() {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("test.db");

        // Should fail to create manager without key
        let result = DbManager::new(&db_path, 4, None);
        assert!(result.is_err());
    }
}
