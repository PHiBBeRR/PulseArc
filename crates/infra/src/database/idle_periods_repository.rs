//! Idle periods repository implementation using SQLCipher
//!
//! Provides persistence for idle period tracking (FEATURE-028)

use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_common::storage::error::StorageError;
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_core::tracking::ports::IdlePeriodsRepository as IdlePeriodsRepositoryPort;
use pulsearc_domain::{IdlePeriod, PulseArcError, Result as DomainResult};
use rusqlite::{params, Row, ToSql};
use tokio::task;

use super::manager::DbManager;

/// SQLCipher-backed implementation of `IdlePeriodsRepository`
pub struct SqlCipherIdlePeriodsRepository {
    db: Arc<DbManager>,
}

impl SqlCipherIdlePeriodsRepository {
    /// Create a new repository instance
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl IdlePeriodsRepositoryPort for SqlCipherIdlePeriodsRepository {
    async fn save_idle_period(&self, period: IdlePeriod) -> DomainResult<()> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            insert_idle_period(&conn, &period).map_err(map_storage_error)?;
            Ok(())
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_idle_period(&self, id: &str) -> DomainResult<Option<IdlePeriod>> {
        let db = Arc::clone(&self.db);
        let id = id.to_string();

        task::spawn_blocking(move || -> DomainResult<Option<IdlePeriod>> {
            let conn = db.get_connection()?;

            let result = conn.query_row(
                "SELECT id, start_ts, end_ts, duration_secs, system_trigger, user_action,
                        threshold_secs, created_at, reviewed_at, notes
                 FROM idle_periods WHERE id = ?1",
                params![&id],
                map_idle_period_row,
            );

            match result {
                Ok(period) => Ok(Some(period)),
                Err(StorageError::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
                Err(err) => Err(map_storage_error(err)),
            }
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_idle_periods_in_range(
        &self,
        start_ts: i64,
        end_ts: i64,
    ) -> DomainResult<Vec<IdlePeriod>> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Vec<IdlePeriod>> {
            let conn = db.get_connection()?;
            query_idle_periods_in_range(&conn, start_ts, end_ts).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_pending_idle_periods(&self) -> DomainResult<Vec<IdlePeriod>> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Vec<IdlePeriod>> {
            let conn = db.get_connection()?;
            query_pending_idle_periods(&conn).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn update_idle_period_action(
        &self,
        id: &str,
        user_action: &str,
        notes: Option<String>,
    ) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let id = id.to_string();
        let user_action = user_action.to_string();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            update_idle_period_user_action(&conn, &id, &user_action, notes)
                .map_err(map_storage_error)?;
            Ok(())
        })
        .await
        .map_err(map_join_error)?
    }

    async fn delete_idle_periods_before(&self, before_ts: i64) -> DomainResult<usize> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<usize> {
            let conn = db.get_connection()?;
            delete_idle_periods_before(&conn, before_ts).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Map a row to an IdlePeriod
fn map_idle_period_row(row: &Row) -> rusqlite::Result<IdlePeriod> {
    Ok(IdlePeriod {
        id: row.get(0)?,
        start_ts: row.get(1)?,
        end_ts: row.get(2)?,
        duration_secs: row.get(3)?,
        system_trigger: row.get(4)?,
        user_action: row.get(5)?,
        threshold_secs: row.get(6)?,
        created_at: row.get(7)?,
        reviewed_at: row.get(8)?,
        notes: row.get(9)?,
    })
}

/// Insert an idle period
fn insert_idle_period(conn: &SqlCipherConnection, period: &IdlePeriod) -> Result<(), StorageError> {
    let params: [&dyn ToSql; 10] = [
        &period.id,
        &period.start_ts,
        &period.end_ts,
        &period.duration_secs,
        &period.system_trigger,
        &period.user_action,
        &period.threshold_secs,
        &period.created_at,
        &period.reviewed_at,
        &period.notes,
    ];

    conn.execute(
        "INSERT INTO idle_periods (
            id, start_ts, end_ts, duration_secs, system_trigger, user_action,
            threshold_secs, created_at, reviewed_at, notes
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params.as_slice(),
    )?;

    Ok(())
}

/// Query idle periods within a time range
fn query_idle_periods_in_range(
    conn: &SqlCipherConnection,
    start_ts: i64,
    end_ts: i64,
) -> Result<Vec<IdlePeriod>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT id, start_ts, end_ts, duration_secs, system_trigger, user_action,
                threshold_secs, created_at, reviewed_at, notes
         FROM idle_periods
         WHERE start_ts >= ?1 AND end_ts <= ?2
         ORDER BY start_ts ASC",
    )?;

    let params: [&dyn ToSql; 2] = [&start_ts, &end_ts];
    stmt.query_map(params.as_slice(), map_idle_period_row)
}

/// Query pending idle periods
fn query_pending_idle_periods(conn: &SqlCipherConnection) -> Result<Vec<IdlePeriod>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT id, start_ts, end_ts, duration_secs, system_trigger, user_action,
                threshold_secs, created_at, reviewed_at, notes
         FROM idle_periods
         WHERE user_action IS NULL OR user_action = 'pending'
         ORDER BY start_ts ASC",
    )?;

    stmt.query_map(&[], map_idle_period_row)
}

/// Update an idle period's user action and reviewed timestamp
fn update_idle_period_user_action(
    conn: &SqlCipherConnection,
    id: &str,
    user_action: &str,
    notes: Option<String>,
) -> Result<(), StorageError> {
    let now = chrono::Utc::now().timestamp();
    let params: [&dyn ToSql; 4] = [&user_action, &now, &notes, &id];

    conn.execute(
        "UPDATE idle_periods
         SET user_action = ?1, reviewed_at = ?2, notes = ?3
         WHERE id = ?4",
        params.as_slice(),
    )?;

    Ok(())
}

/// Delete idle periods before a specific timestamp
fn delete_idle_periods_before(
    conn: &SqlCipherConnection,
    before_ts: i64,
) -> Result<usize, StorageError> {
    let params: [&dyn ToSql; 1] = [&before_ts];
    Ok(conn.execute("DELETE FROM idle_periods WHERE end_ts < ?1", params.as_slice())?)
}

// =============================================================================
// Error Mapping
// =============================================================================

fn map_storage_error(err: StorageError) -> PulseArcError {
    match err {
        StorageError::WrongKeyOrNotEncrypted => {
            PulseArcError::Database("Database key error or not encrypted".into())
        }
        StorageError::Connection(msg) => PulseArcError::Database(msg),
        StorageError::Query(msg) => PulseArcError::Database(msg),
        StorageError::DatabaseError(msg) => PulseArcError::Database(msg),
        StorageError::Encryption(msg) => {
            PulseArcError::Database(format!("Encryption error: {msg}"))
        }
        StorageError::Migration(msg) => PulseArcError::Database(format!("Migration error: {msg}")),
        StorageError::Keychain(msg) => PulseArcError::Database(format!("Keychain error: {msg}")),
        StorageError::Rusqlite(err) => PulseArcError::Database(format!("SQLite error: {err}")),
        _ => PulseArcError::Database(format!("Storage error: {err}")),
    }
}

fn map_join_error(err: task::JoinError) -> PulseArcError {
    PulseArcError::Internal(format!("Task join error: {err}"))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use tempfile::TempDir;

    use super::*;

    fn setup_test_db() -> (Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let manager = DbManager::new(db_path.to_str().unwrap(), 5, Some("test-key"))
            .expect("create db manager");
        manager.run_migrations().expect("run migrations");
        (Arc::new(manager), temp_dir)
    }

    fn create_test_idle_period() -> IdlePeriod {
        let now = Utc::now().timestamp();
        IdlePeriod {
            id: "test-idle-period-123".into(),
            start_ts: now - 600,
            end_ts: now - 300,
            duration_secs: 300,
            system_trigger: "threshold".into(),
            user_action: None,
            threshold_secs: 300,
            created_at: now,
            reviewed_at: None,
            notes: None,
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_save_and_get_idle_period() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherIdlePeriodsRepository::new(db);
        let period = create_test_idle_period();

        // Save
        repo.save_idle_period(period.clone()).await.expect("save period");

        // Get
        let retrieved = repo.get_idle_period(&period.id).await.expect("get period");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, period.id);
        assert_eq!(retrieved.duration_secs, period.duration_secs);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_nonexistent_returns_none() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherIdlePeriodsRepository::new(db);

        let retrieved = repo.get_idle_period("nonexistent").await.expect("get period");
        assert!(retrieved.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_idle_periods_in_range() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherIdlePeriodsRepository::new(db);
        let period = create_test_idle_period();

        repo.save_idle_period(period.clone()).await.expect("save period");

        let retrieved = repo
            .get_idle_periods_in_range(period.start_ts - 100, period.end_ts + 100)
            .await
            .expect("get periods in range");

        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].id, period.id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_pending_idle_periods() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherIdlePeriodsRepository::new(db);
        let mut period = create_test_idle_period();
        period.user_action = None;

        repo.save_idle_period(period.clone()).await.expect("save period");

        let pending = repo.get_pending_idle_periods().await.expect("get pending periods");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, period.id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_idle_period_action() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherIdlePeriodsRepository::new(db);
        let period = create_test_idle_period();

        repo.save_idle_period(period.clone()).await.expect("save period");

        // Update action
        repo.update_idle_period_action(&period.id, "kept", Some("User chose to keep".into()))
            .await
            .expect("update period action");

        // Verify
        let retrieved = repo.get_idle_period(&period.id).await.expect("get period").unwrap();
        assert_eq!(retrieved.user_action, Some("kept".into()));
        assert_eq!(retrieved.notes, Some("User chose to keep".into()));
        assert!(retrieved.reviewed_at.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_idle_periods_before() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherIdlePeriodsRepository::new(db);
        let mut period = create_test_idle_period();
        period.end_ts = Utc::now().timestamp() - 1000; // Old period

        repo.save_idle_period(period.clone()).await.expect("save period");

        let deleted = repo
            .delete_idle_periods_before(Utc::now().timestamp() - 500)
            .await
            .expect("delete old periods");

        assert_eq!(deleted, 1);

        let retrieved = repo.get_idle_period(&period.id).await.expect("get period");
        assert!(retrieved.is_none());
    }
}
