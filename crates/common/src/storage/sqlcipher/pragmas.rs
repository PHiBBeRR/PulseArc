//! SQLite pragma management
//!
//! Applies per-connection pragmas for optimal performance and safety.
//! Based on macos/db/manager.rs lines 59-66.

use rusqlite::Connection;

use super::config::SqlCipherPoolConfig;
use crate::storage::error::{StorageError, StorageResult};

/// Apply connection-level pragmas
///
/// These pragmas are applied to each connection in the pool:
/// - WAL mode for better concurrency
/// - NORMAL synchronous mode for balanced safety/performance
/// - WAL autocheckpoint for automatic checkpoint management
/// - Foreign key constraints enabled
/// - Busy timeout for handling lock contention
///
/// # Source
/// Based on macos/db/manager.rs lines 59-66
pub fn apply_connection_pragmas(
    conn: &Connection,
    config: &SqlCipherPoolConfig,
) -> StorageResult<()> {
    // Build pragma batch
    let mut pragma_sql = String::new();

    // Journal mode (WAL for concurrency)
    if config.enable_wal {
        pragma_sql.push_str("PRAGMA journal_mode=WAL;\n");
        // WAL autocheckpoint (checkpoint after 1000 pages)
        pragma_sql.push_str("PRAGMA wal_autocheckpoint=1000;\n");
    }

    // Synchronous mode (NORMAL for balance)
    pragma_sql.push_str("PRAGMA synchronous=NORMAL;\n");

    // Foreign keys
    if config.enable_foreign_keys {
        pragma_sql.push_str("PRAGMA foreign_keys=ON;\n");
    }

    // Execute pragma batch
    conn.execute_batch(&pragma_sql)
        .map_err(|e| StorageError::Query(format!("Failed to apply pragmas: {}", e)))?;

    // Set busy timeout (separate call as it takes a parameter)
    conn.busy_timeout(config.busy_timeout)
        .map_err(|e| StorageError::Query(format!("Failed to set busy timeout: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests for storage::sqlcipher::pragmas.
    use tempfile::TempDir;

    use super::*;

    /// Validates `TempDir::new` behavior for the apply pragmas scenario.
    ///
    /// Assertions:
    /// - Confirms `journal_mode.to_lowercase()` equals `"wal"`.
    /// - Confirms `foreign_keys` equals `1`.
    /// - Confirms `synchronous` equals `1`.
    #[test]
    fn test_apply_pragmas() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let conn = Connection::open(db_path).unwrap();
        let config = SqlCipherPoolConfig::default();

        apply_connection_pragmas(&conn, &config).unwrap();

        // Verify WAL mode
        let journal_mode: String =
            conn.pragma_query_value(None, "journal_mode", |row| row.get(0)).unwrap();
        assert_eq!(journal_mode.to_lowercase(), "wal");

        // Verify foreign keys
        let foreign_keys: i32 =
            conn.pragma_query_value(None, "foreign_keys", |row| row.get(0)).unwrap();
        assert_eq!(foreign_keys, 1);

        // Verify synchronous mode
        let synchronous: i32 =
            conn.pragma_query_value(None, "synchronous", |row| row.get(0)).unwrap();
        assert_eq!(synchronous, 1); // 1 = NORMAL
    }
}
