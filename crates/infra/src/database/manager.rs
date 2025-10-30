//! Database connection manager with pooling

use std::path::Path;

use pulsearc_shared::{PulseArcError, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

/// Database manager with connection pooling
pub struct DbManager {
    pool: Pool<SqliteConnectionManager>,
}

impl DbManager {
    /// Create a new database manager with optional encryption
    pub fn new<P: AsRef<Path>>(
        db_path: P,
        pool_size: u32,
        encryption_key: Option<&str>,
    ) -> Result<Self> {
        let path_str = db_path.as_ref().display().to_string();
        let key = encryption_key.map(|k| k.to_string());

        if key.is_some() {
            log::info!("Creating encrypted database at: {}", path_str);
        } else {
            log::info!("Creating unencrypted database at: {}", path_str);
        }

        let manager = SqliteConnectionManager::file(db_path).with_init(move |conn| {
            // Set up SQLCipher encryption if key is provided
            // MUST be the first thing before any other operation
            if let Some(ref key) = key {
                // Set the encryption key - this must be done before any other operations
                log::debug!("Setting SQLCipher encryption key");
                conn.pragma_update(None, "key", key)?;
            }

            // Configure database settings after key is set
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                     PRAGMA synchronous = NORMAL;
                     PRAGMA cache_size = -64000;
                     PRAGMA busy_timeout = 5000;",
            )?;
            Ok(())
        });

        let pool = Pool::builder()
            .max_size(pool_size)
            .build(manager)
            .map_err(|e| PulseArcError::Database(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Get a database connection from the pool
    pub fn get_connection(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool
            .get()
            .map_err(|e| PulseArcError::Database(format!("Failed to get connection: {}", e)))
    }

    /// Run database migrations
    pub fn run_migrations(&self) -> Result<()> {
        let conn = self.get_connection()?;

        // Create tables if they don't exist
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS activity_snapshots (
                id TEXT PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                context TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS time_entries (
                id TEXT PRIMARY KEY,
                start_time INTEGER NOT NULL,
                end_time INTEGER NOT NULL,
                duration_seconds INTEGER NOT NULL,
                description TEXT NOT NULL,
                project TEXT,
                wbs_code TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp
                ON activity_snapshots(timestamp);

            CREATE INDEX IF NOT EXISTS idx_entries_start_time
                ON time_entries(start_time);",
        )
        .map_err(|e| PulseArcError::Database(format!("Migration failed: {}", e)))?;

        Ok(())
    }
}
