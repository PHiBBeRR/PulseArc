//! SQLCipher connection wrapper
//!
//! Implements the Connection trait for SQLCipher encrypted databases.

use std::ops::{Deref, DerefMut};

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection as RusqliteConnection, Row, Statement as RusqliteStatement, ToSql};
use tracing::instrument;

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::types::{Connection as ConnectionTrait, Transaction};

/// SQLCipher connection wrapper
///
/// Wraps a pooled rusqlite connection and implements the platform-agnostic
/// Connection trait. The connection is automatically returned to the pool when
/// dropped.
pub struct SqlCipherConnection {
    inner: PooledConnection<SqliteConnectionManager>,
}

impl SqlCipherConnection {
    /// Create a new connection wrapper from a pooled connection
    pub fn new(conn: PooledConnection<SqliteConnectionManager>) -> Self {
        Self { inner: conn }
    }

    /// Get a reference to the inner connection
    ///
    /// PooledConnection derefs to &Connection, so this returns a reference to
    /// the underlying rusqlite connection
    pub fn inner(&self) -> &RusqliteConnection {
        &self.inner
    }
}

impl SqlCipherConnection {
    /// Execute a SQL query that returns a single row
    ///
    /// The callback function is called with the row data.
    #[instrument(skip(self, params, f), fields(sql = %sql))]
    pub fn query_row<T, F>(&self, sql: &str, params: &[&dyn ToSql], f: F) -> StorageResult<T>
    where
        F: FnOnce(&Row<'_>) -> Result<T, rusqlite::Error>,
    {
        self.inner.query_row(sql, params, f).map_err(StorageError::from)
    }

    /// Prepare a SQL statement for efficient repeated execution
    #[instrument(skip(self), fields(sql = %sql))]
    pub fn prepare(&self, sql: &str) -> StorageResult<SqlCipherStatement<'_>> {
        let stmt = self.inner.prepare(sql).map_err(StorageError::from)?;

        Ok(SqlCipherStatement::new(stmt))
    }

    /// Begin a transaction
    #[instrument(skip(self))]
    pub fn transaction(&mut self) -> StorageResult<Transaction<'_>> {
        let tx = self.inner.transaction().map_err(StorageError::from)?;

        Ok(Transaction::new(tx))
    }
}

impl ConnectionTrait for SqlCipherConnection {
    #[instrument(skip(self, params), fields(sql = %sql))]
    fn execute(&self, sql: &str, params: &[&dyn ToSql]) -> StorageResult<usize> {
        self.inner.execute(sql, params).map_err(StorageError::from)
    }

    #[instrument(skip(self), fields(timeout_ms = %timeout_ms))]
    fn busy_timeout(&self, timeout_ms: u64) -> StorageResult<()> {
        self.inner
            .busy_timeout(std::time::Duration::from_millis(timeout_ms))
            .map_err(StorageError::from)
    }
}

// Allow using SqlCipherConnection as RusqliteConnection
impl Deref for SqlCipherConnection {
    type Target = RusqliteConnection;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for SqlCipherConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// SQLCipher prepared statement wrapper
pub struct SqlCipherStatement<'conn> {
    inner: RusqliteStatement<'conn>,
}

impl<'conn> SqlCipherStatement<'conn> {
    /// Create a new statement wrapper
    pub fn new(stmt: RusqliteStatement<'conn>) -> Self {
        Self { inner: stmt }
    }
}

impl<'conn> SqlCipherStatement<'conn> {
    /// Execute the statement with parameters
    pub fn execute(&mut self, params: &[&dyn ToSql]) -> StorageResult<usize> {
        self.inner.execute(params).map_err(StorageError::from)
    }

    /// Query with the statement and map results
    pub fn query_map<T, F>(&mut self, params: &[&dyn ToSql], mut f: F) -> StorageResult<Vec<T>>
    where
        F: FnMut(&Row<'_>) -> Result<T, rusqlite::Error>,
    {
        let rows = self.inner.query_map(params, |row| f(row)).map_err(StorageError::from)?;

        rows.collect::<Result<Vec<_>, _>>().map_err(StorageError::from)
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for storage::sqlcipher::connection.
    use tempfile::TempDir;

    use super::*;
    use crate::storage::sqlcipher::{SqlCipherPool, SqlCipherPoolConfig};

    fn test_key() -> String {
        "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()
    }

    /// Validates `TempDir::new` behavior for the connection execute scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `result.unwrap()` equals `1`.
    #[test]
    fn test_connection_execute() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = SqlCipherPoolConfig::default();
        let pool = SqlCipherPool::new(&db_path, test_key(), config).unwrap();
        let conn = pool.get_sqlcipher_connection().unwrap();

        // Create table
        let result = conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)", &[]);
        assert!(result.is_ok());

        // Insert data
        let name = "Alice";
        let result = conn.execute("INSERT INTO test (name) VALUES (?)", &[&name]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    /// Validates `TempDir::new` behavior for the connection query row scenario.
    ///
    /// Assertions:
    /// - Confirms `result` equals `"Bob"`.
    #[test]
    fn test_connection_query_row() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = SqlCipherPoolConfig::default();
        let pool = SqlCipherPool::new(&db_path, test_key(), config).unwrap();
        let conn = pool.get_sqlcipher_connection().unwrap();

        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)", &[]).unwrap();
        let name = "Bob";
        conn.execute("INSERT INTO test (name) VALUES (?)", &[&name]).unwrap();

        // Query row
        let result: String =
            conn.query_row("SELECT name FROM test WHERE id = ?", &[&1], |row| row.get(0)).unwrap();

        assert_eq!(result, "Bob");
    }

    /// Validates `TempDir::new` behavior for the connection prepare scenario.
    ///
    /// Assertions:
    /// - Confirms `count` equals `2`.
    #[test]
    fn test_connection_prepare() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = SqlCipherPoolConfig::default();
        let pool = SqlCipherPool::new(&db_path, test_key(), config).unwrap();
        let conn = pool.get_sqlcipher_connection().unwrap();

        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)", &[]).unwrap();

        // Prepare statement
        let mut stmt = conn.prepare("INSERT INTO test (name) VALUES (?)").unwrap();

        // Execute multiple times
        let name1 = "Charlie";
        stmt.execute(&[&name1]).unwrap();
        let name2 = "Diana";
        stmt.execute(&[&name2]).unwrap();

        // Verify
        let count: i32 =
            conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
        assert_eq!(count, 2);
    }
}
