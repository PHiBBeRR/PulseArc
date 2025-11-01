//! SQLCipher test database helpers.
//!
//! Provides lightweight utilities for creating encrypted SQLite databases
//! backed by SQLCipher for use in integration tests. The helpers keep database
//! lifetimes tied to a temporary directory so clean-up happens automatically
//! when the test completes.

use std::fs;
use std::path::{Path, PathBuf};

use rand::distributions::Alphanumeric;
use rand::Rng;

use crate::error::CommonError;
use crate::storage::sqlcipher::{SqlCipherConnection, SqlCipherPool, SqlCipherPoolConfig};

/// Result type for test database operations
pub type TestDbResult<T> = Result<T, CommonError>;

/// Manage the lifetime of a temporary SQLCipher database for tests.
///
/// The database file is created inside a temporary directory and removed when
/// the struct is dropped. A dedicated encryption key is generated
/// automatically.
#[derive(Debug)]
pub struct SqlCipherTestDatabase {
    /// Temporary directory that owns the database file.
    /// Kept here to ensure RAII cleanup when the struct is dropped.
    #[allow(dead_code)]
    temp_dir: super::TempDir,
    db_path: PathBuf,
    encryption_key: String,
    pool: SqlCipherPool,
}

impl SqlCipherTestDatabase {
    /// Create a new on-disk SQLCipher database using the default pool config.
    pub fn new() -> TestDbResult<Self> {
        Self::with_pool_config(SqlCipherPoolConfig::default())
    }

    /// Create a new on-disk SQLCipher database with a custom pool config.
    pub fn with_pool_config(config: SqlCipherPoolConfig) -> TestDbResult<Self> {
        let temp_dir = super::TempDir::new("sqlcipher-test").map_err(|e| {
            CommonError::persistence_op(
                "create_temp_dir",
                format!("Failed to create temporary directory: {e}"),
            )
        })?;
        let db_path = temp_dir.path().join("sqlcipher.db");
        let key = generate_key();

        let pool = SqlCipherPool::new(&db_path, key.clone(), config).map_err(|err| {
            CommonError::persistence_op(
                "create_pool",
                format!("failed to create SQLCipher pool: {err}"),
            )
        })?;

        Ok(Self { temp_dir, db_path, encryption_key: key, pool })
    }

    /// Acquire a pooled SQLCipher connection.
    pub fn connection(&self) -> TestDbResult<SqlCipherConnection> {
        self.pool.get_sqlcipher_connection().map_err(|err| {
            CommonError::persistence_op("get_connection", format!("acquire connection: {err}"))
        })
    }

    /// Execute a SQL script (potentially multiple statements) against the
    /// database.
    pub fn run_script(&self, sql: &str) -> TestDbResult<()> {
        let conn = self.connection()?;
        conn.execute_batch(sql).map_err(|err| {
            CommonError::persistence_op(
                "execute_batch",
                format!("execute SQL script failed: {err}"),
            )
        })?;
        Ok(())
    }

    /// Apply all `.sql` files (sorted lexicographically) found in `dir`.
    ///
    /// Returns the number of applied migration files.
    pub fn run_migrations_from_dir(&self, dir: &Path) -> TestDbResult<usize> {
        if !dir.exists() {
            return Err(CommonError::not_found(format!("migrations directory: {}", dir.display())));
        }

        let mut files: Vec<PathBuf> = fs::read_dir(dir)
            .map_err(|e| {
                CommonError::persistence_op(
                    "read_migrations_dir",
                    format!("read migrations directory {}: {}", dir.display(), e),
                )
            })?
            .filter_map(|entry| match entry {
                Ok(e) => {
                    let path = e.path();
                    if path.extension().and_then(|ext| ext.to_str()) == Some("sql") {
                        Some(path)
                    } else {
                        None
                    }
                }
                Err(_) => None,
            })
            .collect();

        files.sort();

        for path in &files {
            let script = fs::read_to_string(path).map_err(|e| {
                CommonError::persistence_op(
                    "read_migration_file",
                    format!("read SQL migration {}: {}", path.display(), e),
                )
            })?;
            self.run_script(&script).map_err(|e| {
                CommonError::persistence_op(
                    "apply_migration",
                    format!("apply SQL migration {}: {}", path.display(), e),
                )
            })?;
        }

        Ok(files.len())
    }

    /// Return the path of the database file on disk.
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Return the SQLCipher encryption key associated with this database.
    pub fn encryption_key(&self) -> &str {
        &self.encryption_key
    }
}

fn generate_key() -> String {
    rand::thread_rng().sample_iter(&Alphanumeric).map(char::from).take(64).collect()
}
