//! SQLCipher-backed feature flags repository.
//!
//! Implements the `FeatureFlagsPort` trait for database-persisted feature
//! flags. All database operations run in `spawn_blocking` to avoid blocking the
//! async runtime.

use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_common::storage::StorageError;
use pulsearc_core::feature_flags_ports::{FeatureFlag, FeatureFlagsPort};
use pulsearc_domain::{PulseArcError, Result as DomainResult};
use rusqlite::params;
use tokio::task;

use super::manager::DbManager;

/// SQLCipher-backed feature flags repository.
///
/// Provides database-backed storage for feature flags with upsert semantics.
/// All operations use `spawn_blocking` to avoid blocking the async runtime.
pub struct SqlCipherFeatureFlagsRepository {
    db: Arc<DbManager>,
}

impl SqlCipherFeatureFlagsRepository {
    /// Create a new repository with the given database manager.
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }

    /// Check if a feature flag is enabled (returns default if not found).
    pub async fn is_enabled(&self, flag_name: &str, default: bool) -> DomainResult<bool> {
        let db = Arc::clone(&self.db);
        let flag_name = flag_name.to_string();

        task::spawn_blocking(move || -> DomainResult<bool> {
            let conn = db.get_connection().map_err(|e| PulseArcError::Database(e.to_string()))?;
            query_flag_enabled(&conn, &flag_name, default)
                .map_err(|e| PulseArcError::Database(e.to_string()))
        })
        .await
        .map_err(map_join_error)?
    }

    /// Set a feature flag's enabled status (upsert).
    pub async fn set_enabled(&self, flag_name: &str, enabled: bool) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let flag_name = flag_name.to_string();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection().map_err(|e| PulseArcError::Database(e.to_string()))?;
            update_flag_enabled(&conn, &flag_name, enabled)
                .map_err(|e| PulseArcError::Database(e.to_string()))
        })
        .await
        .map_err(map_join_error)?
    }

    /// List all feature flags ordered by name.
    pub async fn list_all(&self) -> DomainResult<Vec<FeatureFlag>> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Vec<FeatureFlag>> {
            let conn = db.get_connection().map_err(|e| PulseArcError::Database(e.to_string()))?;
            query_all_flags(&conn).map_err(|e| PulseArcError::Database(e.to_string()))
        })
        .await
        .map_err(map_join_error)?
    }
}

#[async_trait]
impl FeatureFlagsPort for SqlCipherFeatureFlagsRepository {
    async fn is_enabled(&self, flag_name: &str, default: bool) -> DomainResult<bool> {
        Self::is_enabled(self, flag_name, default).await
    }

    async fn set_enabled(&self, flag_name: &str, enabled: bool) -> DomainResult<()> {
        Self::set_enabled(self, flag_name, enabled).await
    }

    async fn list_all(&self) -> DomainResult<Vec<FeatureFlag>> {
        Self::list_all(self).await
    }
}

// ============================================================================
// Synchronous SQL Operations (called inside spawn_blocking)
// ============================================================================

/// Query whether a feature flag is enabled.
/// Returns `default` if the flag doesn't exist.
fn query_flag_enabled(
    conn: &SqlCipherConnection,
    flag_name: &str,
    default: bool,
) -> Result<bool, StorageError> {
    match conn.query_row(
        "SELECT enabled FROM feature_flags WHERE flag_name = ?1",
        params![flag_name],
        |row| row.get::<_, i64>(0),
    ) {
        Ok(value) => Ok(value != 0),
        Err(StorageError::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => Ok(default),
        Err(e) => Err(e),
    }
}

/// Update a feature flag's enabled status (upsert).
/// Creates the flag if it doesn't exist, updates if it does.
fn update_flag_enabled(
    conn: &SqlCipherConnection,
    flag_name: &str,
    enabled: bool,
) -> Result<(), StorageError> {
    let now = chrono::Utc::now().timestamp();
    let enabled_int = if enabled { 1 } else { 0 };

    // Upsert pattern (SQLite 3.24.0+)
    conn.execute(
        "INSERT INTO feature_flags (flag_name, enabled, description, updated_at)
         VALUES (?1, ?2, NULL, ?3)
         ON CONFLICT(flag_name) DO UPDATE SET
            enabled = excluded.enabled,
            updated_at = excluded.updated_at",
        params![flag_name, enabled_int, now],
    )?;
    Ok(())
}

/// Query all feature flags ordered by name.
/// Returns a Vec directly (query_map in SqlCipherConnection returns Vec, not
/// iterator).
fn query_all_flags(conn: &SqlCipherConnection) -> Result<Vec<FeatureFlag>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT flag_name, enabled, description, updated_at
         FROM feature_flags
         ORDER BY flag_name",
    )?;

    // CRITICAL: query_map returns Vec<T> directly, NOT an iterator
    stmt.query_map(params![], |row| {
        Ok(FeatureFlag {
            flag_name: row.get(0)?,
            enabled: row.get::<_, i64>(1)? != 0,
            description: row.get(2)?,
            updated_at: row.get(3)?,
        })
    })
}

// ============================================================================
// Error Mapping
// ============================================================================

/// Map JoinError from spawn_blocking to PulseArcError.
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

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn test_is_enabled_returns_default_for_missing_flag() {
        let (repo, _mgr, _dir) = setup().await;

        // Non-existent flag returns default
        let enabled = repo.is_enabled("nonexistent_flag", true).await.expect("query succeeded");
        assert!(enabled, "should return default true");

        let disabled = repo.is_enabled("nonexistent_flag", false).await.expect("query succeeded");
        assert!(!disabled, "should return default false");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_set_and_get_flag() {
        let (repo, _mgr, _dir) = setup().await;

        // Default flags exist from schema (enabled=1)
        let enabled = repo.is_enabled("new_blocks_cmd", false).await.expect("query succeeded");
        assert!(enabled, "new_blocks_cmd should be enabled by default");

        // Disable it
        repo.set_enabled("new_blocks_cmd", false).await.expect("update succeeded");

        // Verify disabled
        let enabled = repo.is_enabled("new_blocks_cmd", true).await.expect("query succeeded");
        assert!(!enabled, "new_blocks_cmd should now be disabled");

        // Re-enable it
        repo.set_enabled("new_blocks_cmd", true).await.expect("update succeeded");

        let enabled = repo.is_enabled("new_blocks_cmd", false).await.expect("query succeeded");
        assert!(enabled, "new_blocks_cmd should be re-enabled");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_set_enabled_creates_new_flag() {
        let (repo, _mgr, _dir) = setup().await;

        // Create a new flag that doesn't exist in schema
        repo.set_enabled("custom_flag", true).await.expect("create succeeded");

        let enabled = repo.is_enabled("custom_flag", false).await.expect("query succeeded");
        assert!(enabled, "custom_flag should be enabled");

        // Verify it appears in list_all
        let all_flags = repo.list_all().await.expect("list_all succeeded");
        let custom = all_flags
            .iter()
            .find(|f| f.flag_name == "custom_flag")
            .expect("custom_flag should be in list");
        assert!(custom.enabled);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_list_all_flags() {
        let (repo, _mgr, _dir) = setup().await;

        let flags = repo.list_all().await.expect("list_all succeeded");

        // Schema inserts 2 default flags: new_blocks_cmd, use_new_infra
        assert_eq!(flags.len(), 2, "should have 2 default flags");

        // Verify they're ordered by name
        assert_eq!(flags[0].flag_name, "new_blocks_cmd");
        assert_eq!(flags[1].flag_name, "use_new_infra");

        // Both should be enabled (default in schema)
        assert!(flags[0].enabled);
        assert!(flags[1].enabled);

        // Both should have descriptions
        assert!(flags[0].description.is_some());
        assert!(flags[1].description.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_schema_idempotency_on_existing_v1_db() {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("test.db");

        // Create v1 database
        let mgr =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        mgr.run_migrations().expect("migrations run");

        // Verify schema version is still 1
        let conn = mgr.get_connection().expect("connection acquired");
        let version: i32 = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", params![], |row| row.get(0))
            .expect("query succeeded");
        assert_eq!(version, 1, "schema version should remain 1");
        drop(conn);

        // Run migrations again (simulates app restart after code update)
        mgr.run_migrations().expect("migrations run again");

        // Verify feature_flags table was added
        let conn = mgr.get_connection().expect("connection acquired");
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM feature_flags", params![], |row| row.get(0))
            .expect("query succeeded");
        assert_eq!(count, 2, "should have 2 default flags");
        drop(conn);

        // Verify can query flags via repository
        let repo = SqlCipherFeatureFlagsRepository::new(mgr);
        let enabled = repo.is_enabled("new_blocks_cmd", false).await.expect("query succeeded");
        assert!(enabled, "new_blocks_cmd should be enabled");
    }

    /// Set up a test repository with fresh database.
    async fn setup() -> (SqlCipherFeatureFlagsRepository, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("flags.db");

        let mgr =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        mgr.run_migrations().expect("migrations run");

        let repo = SqlCipherFeatureFlagsRepository::new(mgr.clone());
        (repo, mgr, temp_dir)
    }
}
