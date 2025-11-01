//! Idle periods repository implementation using SQLCipher
//!
//! Provides persistence for idle period tracking (FEATURE-028)

use std::convert::TryFrom;
use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_common::storage::error::StorageError;
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_core::tracking::ports::IdlePeriodsRepository as IdlePeriodsRepositoryPort;
use pulsearc_domain::{IdlePeriod, IdleSummary, PulseArcError, Result as DomainResult};
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

    async fn get_idle_summary(&self, start_ts: i64, end_ts: i64) -> DomainResult<IdleSummary> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<IdleSummary> {
            let conn = db.get_connection()?;
            calculate_idle_summary(&conn, start_ts, end_ts).map_err(map_storage_error)
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

fn i64_to_i32(value: i64, field: &'static str) -> Result<i32, StorageError> {
    i32::try_from(value)
        .map_err(|_| StorageError::DatabaseError(format!("{field} exceeds i32 range ({value})")))
}

/// Calculate idle summary for a time range
///
/// Executes 6 aggregation queries:
/// 1. Count active snapshots (is_idle = 0) × 30 seconds
/// 2. Sum total idle duration
/// 3. Count idle periods
/// 4. Sum kept idle duration (user_action = 'kept')
/// 5. Sum discarded idle duration (user_action = 'discarded')
/// 6. Sum pending idle duration (user_action IS NULL OR 'pending')
fn calculate_idle_summary(
    conn: &SqlCipherConnection,
    start_ts: i64,
    end_ts: i64,
) -> Result<IdleSummary, StorageError> {
    const ACTIVE_SNAPSHOT_INTERVAL_SECS: i64 = 30;
    let range_params: [&dyn ToSql; 2] = [&start_ts, &end_ts];

    // Query 1: Count active snapshots (is_idle = 0)
    let active_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM activity_snapshots
         WHERE is_idle = 0 AND timestamp >= ?1 AND timestamp <= ?2",
        &range_params,
        |row| row.get(0),
    )?;
    let total_active_secs = i64_to_i32(
        active_count.saturating_mul(ACTIVE_SNAPSHOT_INTERVAL_SECS),
        "total_active_secs",
    )?;

    // Query 2: Sum total idle duration
    let total_idle_secs = conn.query_row(
        "SELECT COALESCE(SUM(duration_secs), 0) FROM idle_periods
         WHERE start_ts >= ?1 AND end_ts <= ?2",
        &range_params,
        |row| row.get(0),
    )?;

    // Query 3: Count idle periods
    let idle_periods_count = conn.query_row(
        "SELECT COUNT(*) FROM idle_periods
         WHERE start_ts >= ?1 AND end_ts <= ?2",
        &range_params,
        |row| row.get(0),
    )?;

    // Query 4: Sum kept idle duration
    let idle_kept_secs = conn.query_row(
        "SELECT COALESCE(SUM(duration_secs), 0) FROM idle_periods
         WHERE user_action = 'kept' AND start_ts >= ?1 AND end_ts <= ?2",
        &range_params,
        |row| row.get(0),
    )?;

    // Query 5: Sum discarded idle duration
    let idle_discarded_secs = conn.query_row(
        "SELECT COALESCE(SUM(duration_secs), 0) FROM idle_periods
         WHERE user_action = 'discarded' AND start_ts >= ?1 AND end_ts <= ?2",
        &range_params,
        |row| row.get(0),
    )?;

    // Query 6: Sum pending idle duration (user_action IS NULL OR 'pending')
    let idle_pending_secs = conn.query_row(
        "SELECT COALESCE(SUM(duration_secs), 0) FROM idle_periods
         WHERE (user_action IS NULL OR user_action = 'pending')
           AND start_ts >= ?1 AND end_ts <= ?2",
        &range_params,
        |row| row.get(0),
    )?;

    Ok(IdleSummary {
        total_active_secs,
        total_idle_secs: i64_to_i32(total_idle_secs, "total_idle_secs")?,
        idle_periods_count: i64_to_i32(idle_periods_count, "idle_periods_count")?,
        idle_kept_secs: i64_to_i32(idle_kept_secs, "idle_kept_secs")?,
        idle_discarded_secs: i64_to_i32(idle_discarded_secs, "idle_discarded_secs")?,
        idle_pending_secs: i64_to_i32(idle_pending_secs, "idle_pending_secs")?,
    })
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_idle_summary_returns_zero_when_no_data() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherIdlePeriodsRepository::new(db);

        let start_ts = 1_700_000_000;
        let end_ts = start_ts + 3600;

        let summary = repo.get_idle_summary(start_ts, end_ts).await.expect("idle summary");

        assert_eq!(summary.total_active_secs, 0);
        assert_eq!(summary.total_idle_secs, 0);
        assert_eq!(summary.idle_periods_count, 0);
        assert_eq!(summary.idle_kept_secs, 0);
        assert_eq!(summary.idle_discarded_secs, 0);
        assert_eq!(summary.idle_pending_secs, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_idle_summary_aggregates_statistics() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherIdlePeriodsRepository::new(Arc::clone(&db));

        let range_start = 1_700_000_000;
        let range_end = range_start + 600;

        // Seed activity snapshots: two active, one idle, plus one outside range
        let conn = db.get_connection().expect("connection");
        insert_activity_snapshot(&conn, "snap-active-1", range_start + 10, false);
        insert_activity_snapshot(&conn, "snap-active-2", range_start + 40, false);
        insert_activity_snapshot(&conn, "snap-idle-1", range_start + 70, true);
        insert_activity_snapshot(&conn, "snap-outside", range_start - 120, false);
        drop(conn);

        // Insert idle periods covering different user actions
        repo.save_idle_period(build_idle_period("idle-kept", range_start + 100, 120, Some("kept")))
            .await
            .expect("save kept idle");

        repo.save_idle_period(build_idle_period(
            "idle-discarded",
            range_start + 200,
            60,
            Some("discarded"),
        ))
        .await
        .expect("save discarded idle");

        repo.save_idle_period(build_idle_period("idle-pending-null", range_start + 300, 90, None))
            .await
            .expect("save pending (null) idle");

        repo.save_idle_period(build_idle_period(
            "idle-pending-explicit",
            range_start + 360,
            30,
            Some("pending"),
        ))
        .await
        .expect("save pending (explicit) idle");

        // Out-of-range idle period should be ignored
        repo.save_idle_period(build_idle_period(
            "idle-outside",
            range_start - 500,
            45,
            Some("kept"),
        ))
        .await
        .expect("save out-of-range idle");

        let summary = repo.get_idle_summary(range_start, range_end).await.expect("idle summary");

        assert_eq!(summary.total_active_secs, 60, "two active snapshots → 60 seconds");
        assert_eq!(summary.total_idle_secs, 300, "sum of in-range idle durations");
        assert_eq!(summary.idle_periods_count, 4, "four in-range idle periods counted");
        assert_eq!(summary.idle_kept_secs, 120);
        assert_eq!(summary.idle_discarded_secs, 60);
        assert_eq!(summary.idle_pending_secs, 120, "pending includes NULL + 'pending'");
    }

    fn insert_activity_snapshot(
        conn: &SqlCipherConnection,
        id: &str,
        timestamp: i64,
        is_idle: bool,
    ) {
        let activity_context_json = r#"{"app":"Test"}"#;
        let detected_activity = "working";
        let primary_app = "TestApp";
        let processed: i64 = 0;
        let is_idle_value: i64 = if is_idle { 1 } else { 0 };
        let null_string: Option<String> = None;
        let null_i64: Option<i64> = None;
        let null_i32: Option<i32> = None;
        let params: [&dyn ToSql; 13] = [
            &id,
            &timestamp,
            &activity_context_json,
            &detected_activity,
            &null_string,
            &null_string,
            &primary_app,
            &processed,
            &null_string,
            &timestamp,
            &null_i64,
            &is_idle_value,
            &null_i32,
        ];

        conn.execute(
            "INSERT INTO activity_snapshots (
                id, timestamp, activity_context_json, detected_activity,
                work_type, activity_category, primary_app, processed,
                batch_id, created_at, processed_at, is_idle, idle_duration_secs
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params.as_slice(),
        )
        .expect("insert activity snapshot");
    }

    fn build_idle_period(
        id: &str,
        start_ts: i64,
        duration_secs: i32,
        action: Option<&str>,
    ) -> IdlePeriod {
        let end_ts = start_ts + i64::from(duration_secs);
        let user_action = action.map(std::string::ToString::to_string);
        let reviewed_at = match action {
            Some("kept") | Some("discarded") => Some(end_ts),
            _ => None,
        };

        IdlePeriod {
            id: id.to_string(),
            start_ts,
            end_ts,
            duration_secs,
            system_trigger: "threshold".to_string(),
            user_action,
            threshold_secs: 300,
            created_at: start_ts,
            reviewed_at,
            notes: None,
        }
    }
}
