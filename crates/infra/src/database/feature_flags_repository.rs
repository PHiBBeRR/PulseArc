//! SQLCipher-backed feature flags repository.
//!
//! Provides persistence for runtime feature flags with helpers for detecting
//! fallback usage (when a flag is missing from storage and the caller's
//! default is used instead).

use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_common::storage::StorageError;
use pulsearc_core::feature_flags_ports::{FeatureFlag, FeatureFlagEvaluation, FeatureFlagsPort};
use pulsearc_domain::{PulseArcError, Result as DomainResult};
use rusqlite::params;
use tokio::task;

use super::manager::DbManager;

/// SQLCipher-backed feature flags repository.
pub struct SqlCipherFeatureFlagsRepository {
    db: Arc<DbManager>,
}

impl SqlCipherFeatureFlagsRepository {
    /// Create a new repository with the given database manager.
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }

    /// Evaluate a feature flag, reporting whether the fallback default was
    /// used.
    pub async fn evaluate(
        &self,
        flag_name: &str,
        default: bool,
    ) -> DomainResult<FeatureFlagEvaluation> {
        let db = Arc::clone(&self.db);
        let flag = flag_name.to_owned();

        task::spawn_blocking(move || -> DomainResult<FeatureFlagEvaluation> {
            let conn = db.get_connection().map_err(|e| PulseArcError::Database(e.to_string()))?;
            query_flag_evaluation(&conn, &flag, default)
                .map_err(|e| PulseArcError::Database(e.to_string()))
        })
        .await
        .map_err(map_join_error)?
    }

    /// Check if a feature flag is enabled, returning the fallback value when
    /// the flag is missing.
    pub async fn is_enabled(&self, flag_name: &str, default: bool) -> DomainResult<bool> {
        self.evaluate(flag_name, default).await.map(|state| state.enabled)
    }

    /// Set a feature flag's enabled status (upsert semantics).
    pub async fn set_enabled(&self, flag_name: &str, enabled: bool) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let flag = flag_name.to_owned();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection().map_err(|e| PulseArcError::Database(e.to_string()))?;
            update_flag_enabled(&conn, &flag, enabled)
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
    async fn evaluate(
        &self,
        flag_name: &str,
        default: bool,
    ) -> DomainResult<FeatureFlagEvaluation> {
        <SqlCipherFeatureFlagsRepository>::evaluate(self, flag_name, default).await
    }
    async fn set_enabled(&self, flag_name: &str, enabled: bool) -> DomainResult<()> {
        <SqlCipherFeatureFlagsRepository>::set_enabled(self, flag_name, enabled).await
    }

    async fn list_all(&self) -> DomainResult<Vec<FeatureFlag>> {
        <SqlCipherFeatureFlagsRepository>::list_all(self).await
    }
}

// ============================================================================
// Synchronous SQL helpers (invoked inside spawn_blocking)
// ============================================================================

fn query_flag_evaluation(
    conn: &SqlCipherConnection,
    flag_name: &str,
    default: bool,
) -> Result<FeatureFlagEvaluation, StorageError> {
    match conn.query_row(
        "SELECT enabled FROM feature_flags WHERE flag_name = ?1",
        params![flag_name],
        |row| row.get::<_, i64>(0),
    ) {
        Ok(value) => Ok(FeatureFlagEvaluation { enabled: value != 0, fallback_used: false }),
        Err(StorageError::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => {
            Ok(FeatureFlagEvaluation { enabled: default, fallback_used: true })
        }
        Err(e) => Err(e),
    }
}

fn update_flag_enabled(
    conn: &SqlCipherConnection,
    flag_name: &str,
    enabled: bool,
) -> Result<(), StorageError> {
    let now = chrono::Utc::now().timestamp();
    let enabled_int = if enabled { 1 } else { 0 };

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

fn query_all_flags(conn: &SqlCipherConnection) -> Result<Vec<FeatureFlag>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT flag_name, enabled, description, updated_at
         FROM feature_flags
         ORDER BY flag_name",
    )?;

    stmt.query_map(params![], |row| {
        Ok(FeatureFlag {
            flag_name: row.get(0)?,
            enabled: row.get::<_, i64>(1)? != 0,
            description: row.get(2)?,
            updated_at: row.get(3)?,
        })
    })
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
    use crate::database::DbManager;

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn missing_flag_reports_fallback() {
        let (repo, _mgr, _dir) = setup().await;

        let state = repo.evaluate("nonexistent_flag", true).await.expect("query succeeded");
        assert!(state.enabled);
        assert!(state.fallback_used);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn existing_flag_reports_actual_value() {
        let (repo, _mgr, _dir) = setup().await;

        let state = repo.evaluate("new_blocks_cmd", false).await.expect("query succeeded");
        assert!(state.enabled);
        assert!(!state.fallback_used);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn set_and_get_updates_record() {
        let (repo, _mgr, _dir) = setup().await;

        repo.set_enabled("new_blocks_cmd", false).await.expect("update succeeded");

        let state = repo.evaluate("new_blocks_cmd", true).await.expect("query succeeded");
        assert!(!state.enabled);
        assert!(!state.fallback_used);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn list_all_returns_all_records() {
        let (repo, _mgr, _dir) = setup().await;

        let flags = repo.list_all().await.expect("list_all succeeded");
        assert!(flags.len() >= 2, "bootstrap migration inserts default flags");
    }

    async fn setup() -> (SqlCipherFeatureFlagsRepository, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("flags.db");

        let manager =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        manager.run_migrations().expect("migrations executed");

        let repo = SqlCipherFeatureFlagsRepository::new(manager.clone());
        (repo, manager, temp_dir)
    }
}
