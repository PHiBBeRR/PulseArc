use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rand::Rng;
use rusqlite::{params, Connection};

/// Minimal copy of the legacy DbManager used for baseline benchmarks.
///
/// The real implementation in `legacy/api` handles metrics integration,
/// keychain storage, and full migration support. For benchmarking purposes
/// we mirror the connection pooling and schema setup logic so that the
/// hot paths being measured (single inserts, range queries, bulk inserts)
/// behave the same as the legacy adapter.
pub struct DbManager {
    pool: Pool<SqliteConnectionManager>,
    db_path: PathBuf,
}

impl DbManager {
    /// Create a new SQLCipher-backed manager with the same pool settings as the
    /// legacy code.
    pub fn new(db_path: &Path) -> Result<Self> {
        let key = get_or_create_encryption_key()?;

        let manager = SqliteConnectionManager::file(db_path).with_init(move |conn| {
            configure_sqlcipher(conn, &key)?;
            configure_pragmas(conn)?;
            Ok(())
        });

        let pool = Pool::builder()
            .max_size(10)
            .connection_timeout(Duration::from_secs(5))
            .build(manager)
            .context("failed to create connection pool")?;

        {
            let conn = pool.get().context("failed to get connection during init")?;
            initialize_schema(&conn)?;
        }

        Ok(Self { pool, db_path: db_path.to_path_buf() })
    }

    /// Borrow a pooled connection.
    pub fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        self.pool.get().context("failed to get connection from pool")
    }

    /// Convenience accessor used by the benchmarks when generating temp
    /// databases.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

fn configure_sqlcipher(conn: &Connection, key: &str) -> rusqlite::Result<()> {
    conn.pragma_update(None, "key", key)?;
    conn.pragma_update(None, "cipher_compatibility", 4)?;
    conn.pragma_update(None, "kdf_iter", 256000)?;
    conn.pragma_update(None, "cipher_memory_security", "ON")?;
    Ok(())
}

fn configure_pragmas(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;\n         PRAGMA synchronous=NORMAL;\n         PRAGMA foreign_keys=ON;\n         PRAGMA busy_timeout=5000;",
    )?;
    Ok(())
}

fn initialize_schema(conn: &Connection) -> Result<()> {
    // The baseline benchmarks only touch activity_snapshots, so we recreate the
    // exact table definition from the legacy schema along with the key indexes.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS activity_snapshots (
            id TEXT PRIMARY KEY,
            timestamp INTEGER NOT NULL,
            activity_context_json TEXT NOT NULL,
            detected_activity TEXT NOT NULL,
            work_type TEXT,
            activity_category TEXT,
            primary_app TEXT NOT NULL,
            processed INTEGER NOT NULL,
            batch_id TEXT,
            created_at INTEGER NOT NULL,
            processed_at INTEGER,
            is_idle INTEGER NOT NULL,
            idle_duration_secs INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_activity_snapshots_timestamp
            ON activity_snapshots(timestamp);
        CREATE INDEX IF NOT EXISTS idx_activity_snapshots_processed
            ON activity_snapshots(processed);
    ",
    )
    .context("failed to initialize activity_snapshots schema")?;
    Ok(())
}

fn get_or_create_encryption_key() -> Result<String> {
    if let Ok(key) = std::env::var("PULSARC_TEST_DB_KEY") {
        return Ok(key);
    }

    Ok(generate_encryption_key())
}

fn generate_encryption_key() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Helper used in tests/benches that want to clear the table quickly.
pub fn truncate_snapshots(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM activity_snapshots", params![])
        .context("failed to truncate activity_snapshots")?;
    Ok(())
}
