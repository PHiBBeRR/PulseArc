//! SQLCipher-backed activity snapshot repository.
//!
//! Implements both the async `ActivityRepository` port used by tracking
//! services and the synchronous `SnapshotRepository` port required by the
//! segmenter. All queries operate on the shared SQLCipher connection pool
//! provided by `DbManager`.

use std::convert::TryFrom;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use pulsearc_common::error::{CommonError, CommonResult};
use pulsearc_common::storage::error::{StorageError, StorageResult};
use pulsearc_common::storage::sqlcipher::connection::SqlCipherStatement;
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_core::tracking::ports::SnapshotRepository as SnapshotRepositoryPort;
use pulsearc_core::ActivityRepository as ActivityRepositoryPort;
use pulsearc_domain::types::database::ActivitySnapshot;
use pulsearc_domain::{PulseArcError, Result as DomainResult};
use rusqlite::{Row, ToSql};
use tokio::task;

use super::manager::DbManager;
use crate::errors::InfraError;

/// Async activity repository + synchronous snapshot repository backed by
/// SQLCipher.
pub struct SqlCipherActivityRepository {
    db: Arc<DbManager>,
}

impl SqlCipherActivityRepository {
    /// Construct a repository backed by the shared database manager.
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }

    /// Fetch snapshots in the provided range with pagination support.
    ///
    /// Returns results ordered by timestamp, using `[start, end)` bounds.
    pub fn find_snapshots_page(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: usize,
        offset: usize,
    ) -> CommonResult<Vec<ActivitySnapshot>> {
        let conn = self
            .db
            .get_connection()
            .map_err(|err| map_to_common_error("activity_snapshots.page_connection", err))?;

        let snapshots =
            query_snapshots(&conn, start.timestamp(), end.timestamp(), Some(limit), Some(offset))
                .map_err(|err| map_storage_to_common("activity_snapshots.page_query", err))?;

        Ok(snapshots)
    }
}

#[async_trait]
impl ActivityRepositoryPort for SqlCipherActivityRepository {
    async fn save_snapshot(&self, snapshot: ActivitySnapshot) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            insert_snapshot(&conn, &snapshot).map_err(map_storage_error)?;
            Ok(())
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_snapshots(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> DomainResult<Vec<ActivitySnapshot>> {
        validate_range(start, end)?;

        let db = Arc::clone(&self.db);
        task::spawn_blocking(move || -> DomainResult<Vec<ActivitySnapshot>> {
            let conn = db.get_connection()?;
            query_snapshots(&conn, start.timestamp(), end.timestamp(), None, None)
                .map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn delete_old_snapshots(&self, before: DateTime<Utc>) -> DomainResult<usize> {
        let db = Arc::clone(&self.db);
        task::spawn_blocking(move || -> DomainResult<usize> {
            let conn = db.get_connection()?;
            delete_snapshots_before(&conn, before.timestamp()).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }
}

impl SnapshotRepositoryPort for SqlCipherActivityRepository {
    fn find_snapshots_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> CommonResult<Vec<ActivitySnapshot>> {
        validate_range(start, end)
            .map_err(|err| map_to_common_error("activity_snapshots.range_validation", err))?;

        let conn = self
            .db
            .get_connection()
            .map_err(|err| map_to_common_error("activity_snapshots.range_connection", err))?;

        query_snapshots(&conn, start.timestamp(), end.timestamp(), None, None)
            .map_err(|err| map_storage_to_common("activity_snapshots.range_query", err))
    }

    fn count_snapshots_by_date(&self, date: NaiveDate) -> CommonResult<usize> {
        let conn = self
            .db
            .get_connection()
            .map_err(|err| map_to_common_error("activity_snapshots.count_connection", err))?;

        let (start_ts, end_ts) = day_bounds(date);

        let params: [&dyn ToSql; 2] = [&start_ts, &end_ts];
        let count: i64 = conn
            .query_row(SNAPSHOT_COUNT_BY_DATE_QUERY, &params, |row| row.get(0))
            .map_err(|err| map_storage_to_common("activity_snapshots.count_query", err))?;

        Ok(count as usize)
    }

    fn store_snapshot(&self, snapshot: &ActivitySnapshot) -> CommonResult<()> {
        let conn = self
            .db
            .get_connection()
            .map_err(|err| map_to_common_error("activity_snapshots.store_connection", err))?;

        insert_or_replace_snapshot(&conn, snapshot)
            .map_err(|err| map_storage_to_common("activity_snapshots.store", err))
    }

    fn store_snapshots_batch(&self, snapshots: &[ActivitySnapshot]) -> CommonResult<()> {
        let conn = self
            .db
            .get_connection()
            .map_err(|err| map_to_common_error("activity_snapshots.batch_connection", err))?;

        // Use a transaction for batch insert
        conn.execute("BEGIN TRANSACTION", []).map_err(|err| {
            map_storage_to_common("activity_snapshots.batch_begin", StorageError::from(err))
        })?;

        for snapshot in snapshots {
            if let Err(err) = insert_or_replace_snapshot(&conn, snapshot) {
                // Rollback on error
                let _ = conn.execute("ROLLBACK", []);
                return Err(map_storage_to_common("activity_snapshots.batch_insert", err));
            }
        }

        conn.execute("COMMIT", []).map_err(|err| {
            let _ = conn.execute("ROLLBACK", []);
            map_storage_to_common("activity_snapshots.batch_commit", StorageError::from(err))
        })?;

        Ok(())
    }

    fn count_active_snapshots(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> CommonResult<i64> {
        let conn = self
            .db
            .get_connection()
            .map_err(|err| map_to_common_error("activity_snapshots.count_active", err))?;

        let params: [&dyn ToSql; 2] = [&start.timestamp(), &end.timestamp()];
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM activity_snapshots WHERE timestamp >= ?1 AND timestamp < ?2 AND is_idle = 0",
                &params,
                |row| row.get(0),
            )
            .map_err(|err| map_storage_to_common("activity_snapshots.count_active", err))?;

        Ok(count)
    }
}

const INSERT_SNAPSHOT_SQL: &str = "INSERT INTO activity_snapshots (
        id, timestamp, activity_context_json, detected_activity,
        work_type, activity_category, primary_app, processed,
        batch_id, created_at, processed_at, is_idle, idle_duration_secs
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)";

const SNAPSHOT_RANGE_BASE: &str = "SELECT id, timestamp, activity_context_json, detected_activity,
        work_type, activity_category, primary_app, processed, batch_id,
        created_at, processed_at, is_idle, idle_duration_secs
    FROM activity_snapshots
    WHERE timestamp >= ?1 AND timestamp < ?2
    ORDER BY timestamp";

const SNAPSHOT_RANGE_WITH_LIMIT: &str =
    "SELECT id, timestamp, activity_context_json, detected_activity,
        work_type, activity_category, primary_app, processed, batch_id,
        created_at, processed_at, is_idle, idle_duration_secs
    FROM activity_snapshots
    WHERE timestamp >= ?1 AND timestamp < ?2
    ORDER BY timestamp
    LIMIT ?3";

const SNAPSHOT_RANGE_WITH_LIMIT_OFFSET: &str =
    "SELECT id, timestamp, activity_context_json, detected_activity,
        work_type, activity_category, primary_app, processed, batch_id,
        created_at, processed_at, is_idle, idle_duration_secs
    FROM activity_snapshots
    WHERE timestamp >= ?1 AND timestamp < ?2
    ORDER BY timestamp
    LIMIT ?3 OFFSET ?4";

const DELETE_OLD_SNAPSHOTS_SQL: &str = "DELETE FROM activity_snapshots WHERE timestamp < ?1";

const SNAPSHOT_COUNT_BY_DATE_QUERY: &str =
    "SELECT COUNT(*) FROM activity_snapshots WHERE timestamp >= ?1 AND timestamp < ?2";

const INSERT_OR_REPLACE_SNAPSHOT_SQL: &str = "INSERT OR REPLACE INTO activity_snapshots (
        id, timestamp, activity_context_json, detected_activity,
        work_type, activity_category, primary_app, processed,
        batch_id, created_at, processed_at, is_idle, idle_duration_secs
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)";

fn insert_or_replace_snapshot(
    conn: &SqlCipherConnection,
    snapshot: &ActivitySnapshot,
) -> StorageResult<()> {
    let processed = bool_to_int(snapshot.processed);
    let is_idle = bool_to_int(snapshot.is_idle);

    let params: [&dyn ToSql; 13] = [
        &snapshot.id,
        &snapshot.timestamp,
        &snapshot.activity_context_json,
        &snapshot.detected_activity,
        &snapshot.work_type,
        &snapshot.activity_category,
        &snapshot.primary_app,
        &processed,
        &snapshot.batch_id,
        &snapshot.created_at,
        &snapshot.processed_at,
        &is_idle,
        &snapshot.idle_duration_secs,
    ];

    conn.execute(INSERT_OR_REPLACE_SNAPSHOT_SQL, params.as_slice())?;
    Ok(())
}

fn insert_snapshot(conn: &SqlCipherConnection, snapshot: &ActivitySnapshot) -> StorageResult<()> {
    let processed = bool_to_int(snapshot.processed);
    let is_idle = bool_to_int(snapshot.is_idle);

    let params: [&dyn ToSql; 13] = [
        &snapshot.id,
        &snapshot.timestamp,
        &snapshot.activity_context_json,
        &snapshot.detected_activity,
        &snapshot.work_type,
        &snapshot.activity_category,
        &snapshot.primary_app,
        &processed,
        &snapshot.batch_id,
        &snapshot.created_at,
        &snapshot.processed_at,
        &is_idle,
        &snapshot.idle_duration_secs,
    ];

    conn.execute(INSERT_SNAPSHOT_SQL, params.as_slice())?;
    Ok(())
}

fn query_snapshots(
    conn: &SqlCipherConnection,
    start_ts: i64,
    end_ts: i64,
    limit: Option<usize>,
    offset: Option<usize>,
) -> StorageResult<Vec<ActivitySnapshot>> {
    let mut stmt = build_snapshot_statement(conn, limit, offset)?;
    let start_param = start_ts;
    let end_param = end_ts;

    match (limit, offset) {
        (Some(limit), Some(offset)) => {
            let limit_param = usize_to_i64(limit);
            let offset_param = usize_to_i64(offset);
            let params: [&dyn ToSql; 4] = [&start_param, &end_param, &limit_param, &offset_param];
            stmt.query_map(params.as_slice(), map_snapshot_row)
        }
        (Some(limit), None) => {
            let limit_param = usize_to_i64(limit);
            let params: [&dyn ToSql; 3] = [&start_param, &end_param, &limit_param];
            stmt.query_map(params.as_slice(), map_snapshot_row)
        }
        _ => {
            let params: [&dyn ToSql; 2] = [&start_param, &end_param];
            stmt.query_map(params.as_slice(), map_snapshot_row)
        }
    }
}

fn build_snapshot_statement<'conn>(
    conn: &'conn SqlCipherConnection,
    limit: Option<usize>,
    offset: Option<usize>,
) -> StorageResult<SqlCipherStatement<'conn>> {
    match (limit, offset) {
        (Some(_), Some(_)) => conn.prepare(SNAPSHOT_RANGE_WITH_LIMIT_OFFSET),
        (Some(_), None) => conn.prepare(SNAPSHOT_RANGE_WITH_LIMIT),
        _ => conn.prepare(SNAPSHOT_RANGE_BASE),
    }
}

fn delete_snapshots_before(conn: &SqlCipherConnection, before_ts: i64) -> StorageResult<usize> {
    let params: [&dyn ToSql; 1] = [&before_ts];
    conn.execute(DELETE_OLD_SNAPSHOTS_SQL, params.as_slice()).map_err(StorageError::from)
}

fn map_snapshot_row(row: &Row<'_>) -> rusqlite::Result<ActivitySnapshot> {
    Ok(ActivitySnapshot {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        activity_context_json: row.get(2)?,
        detected_activity: row.get(3)?,
        work_type: row.get(4)?,
        activity_category: row.get(5)?,
        primary_app: row.get(6)?,
        processed: int_to_bool(row.get(7)?),
        batch_id: row.get(8)?,
        created_at: row.get(9)?,
        processed_at: row.get(10)?,
        is_idle: int_to_bool(row.get(11)?),
        idle_duration_secs: row.get(12)?,
    })
}

fn validate_range(start: DateTime<Utc>, end: DateTime<Utc>) -> DomainResult<()> {
    if start >= end {
        return Err(PulseArcError::InvalidInput(
            "start timestamp must be before end timestamp".into(),
        ));
    }
    Ok(())
}

fn day_bounds(date: NaiveDate) -> (i64, i64) {
    let start_dt = date.and_time(NaiveTime::MIN).and_utc();
    let end_dt = start_dt.checked_add_signed(Duration::days(1)).unwrap_or(start_dt);
    (start_dt.timestamp(), end_dt.timestamp())
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn int_to_bool(value: i64) -> bool {
    value != 0
}

fn usize_to_i64(value: usize) -> i64 {
    match i64::try_from(value) {
        Ok(val) => val,
        Err(_) => i64::MAX,
    }
}

fn map_storage_error(err: StorageError) -> PulseArcError {
    match err {
        StorageError::WrongKeyOrNotEncrypted => {
            PulseArcError::Security("sqlcipher key rejected or database not encrypted".into())
        }
        StorageError::Timeout(seconds) => {
            PulseArcError::Database(format!("database timeout after {seconds}s"))
        }
        StorageError::Connection(message)
        | StorageError::Query(message)
        | StorageError::DatabaseError(message)
        | StorageError::Encryption(message)
        | StorageError::Migration(message)
        | StorageError::Keychain(message)
        | StorageError::InvalidConfig(message) => PulseArcError::Database(message),
        StorageError::SchemaVersionMismatch { expected, found } => PulseArcError::Database(
            format!("schema version mismatch (expected {expected}, found {found})"),
        ),
        StorageError::PoolExhausted => PulseArcError::Database("connection pool exhausted".into()),
        StorageError::Common(common_err) => PulseArcError::Database(common_err.to_string()),
        StorageError::Io(io_err) => PulseArcError::Database(io_err.to_string()),
        StorageError::Rusqlite(sql_err) => PulseArcError::from(InfraError::from(sql_err)),
        StorageError::R2d2(r2d2_err) => PulseArcError::Database(r2d2_err.to_string()),
        StorageError::SerdeJson(json_err) => PulseArcError::Database(json_err.to_string()),
    }
}

fn map_storage_to_common(operation: &str, err: StorageError) -> CommonError {
    CommonError::from(err.with_operation(operation.to_string()))
}

fn map_to_common_error(operation: &str, err: PulseArcError) -> CommonError {
    CommonError::Storage { message: err.to_string(), operation: Some(operation.to_string()) }
}

fn map_join_error(err: task::JoinError) -> PulseArcError {
    if err.is_cancelled() {
        PulseArcError::Internal("blocking activity repository task cancelled".into())
    } else {
        PulseArcError::Internal(format!("blocking activity repository task failed: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn saves_and_fetches_snapshot() {
        let (repo, _manager, _temp_dir) = setup_repository().await;
        let snapshot = sample_snapshot("snap-1", 1_700_000_000);

        repo.save_snapshot(snapshot.clone()).await.expect("save snapshot succeeds");

        let start = DateTime::<Utc>::from_timestamp(1_699_999_999, 0).expect("start ts valid");
        let end = DateTime::<Utc>::from_timestamp(1_700_000_100, 0).expect("end ts valid");
        let snapshots = repo.get_snapshots(start, end).await.expect("snapshots fetched");

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, snapshot.id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn delete_old_snapshots_prunes_expected_rows() {
        let (repo, _manager, _temp_dir) = setup_repository().await;
        let old_snapshot = sample_snapshot("old", 1_600_000_000);
        let recent_snapshot = sample_snapshot("recent", 1_700_000_000);

        repo.save_snapshot(old_snapshot).await.expect("old snapshot saved");
        repo.save_snapshot(recent_snapshot.clone()).await.expect("recent snapshot saved");

        let cutoff = DateTime::<Utc>::from_timestamp(1_650_000_000, 0).expect("cutoff valid");
        let deleted = repo.delete_old_snapshots(cutoff).await.expect("delete succeeds");
        assert_eq!(deleted, 1);

        let start = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).expect("start valid");
        let end = DateTime::<Utc>::from_timestamp(1_700_000_100, 0).expect("end valid");
        let snapshots = repo.get_snapshots(start, end).await.expect("fetch snapshots");
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, recent_snapshot.id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_snapshots_returns_error_for_invalid_range() {
        let (repo, _manager, _temp_dir) = setup_repository().await;
        let start = DateTime::<Utc>::from_timestamp(1_700_000_100, 0).expect("start valid");
        let end = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).expect("end valid");

        let err = repo.get_snapshots(start, end).await.expect_err("range invalid");
        assert!(matches!(err, PulseArcError::InvalidInput(_)));
    }

    #[test]
    fn find_snapshots_by_time_range_uses_half_open_bounds() {
        let (_repo_async, manager, _temp_dir) = setup_repository_sync();
        let repo = SqlCipherActivityRepository::new(manager.clone());

        let conn = manager.get_connection().expect("connection");
        insert_snapshot(&conn, &sample_snapshot("snap-a", 10)).expect("insert snap-a");
        insert_snapshot(&conn, &sample_snapshot("snap-b", 20)).expect("insert snap-b");
        insert_snapshot(&conn, &sample_snapshot("snap-c", 30)).expect("insert snap-c");

        let start = DateTime::<Utc>::from_timestamp(10, 0).expect("start valid");
        let end = DateTime::<Utc>::from_timestamp(30, 0).expect("end valid");

        let snapshots = repo.find_snapshots_by_time_range(start, end).expect("range query");
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].id, "snap-a");
        assert_eq!(snapshots[1].id, "snap-b");
    }

    #[test]
    fn count_snapshots_by_date_returns_expected_value() {
        let (_repo_async, manager, _temp_dir) = setup_repository_sync();
        let repo = SqlCipherActivityRepository::new(manager.clone());

        let conn = manager.get_connection().expect("connection");
        insert_snapshot(&conn, &sample_snapshot("snap-1", 86_400)).expect("insert snap-1");
        insert_snapshot(&conn, &sample_snapshot("snap-2", 172_900)).expect("insert snap-2");

        let date = NaiveDate::from_ymd_opt(1970, 1, 2).expect("date valid");
        let count = repo.count_snapshots_by_date(date).expect("count query");
        assert_eq!(count, 1);
    }

    #[test]
    fn count_snapshots_by_date_returns_error_when_table_missing() {
        let (_repo_async, manager, _temp_dir) = setup_repository_sync();
        let repo = SqlCipherActivityRepository::new(manager.clone());

        {
            let conn = manager.get_connection().expect("connection");
            conn.execute("DROP TABLE activity_snapshots", []).expect("drop table");
        }

        let date = NaiveDate::from_ymd_opt(1970, 1, 1).expect("date valid");
        let err = repo.count_snapshots_by_date(date).expect_err("should error");
        assert!(matches!(err, CommonError::Storage { .. }));
    }

    async fn setup_repository() -> (SqlCipherActivityRepository, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("tempdir created");
        let db_path = temp_dir.path().join("activity.db");

        let manager =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        manager.run_migrations().expect("migrations run");

        let repo = SqlCipherActivityRepository::new(manager.clone());
        (repo, manager, temp_dir)
    }

    fn setup_repository_sync() -> (SqlCipherActivityRepository, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("tempdir created");
        let db_path = temp_dir.path().join("activity.db");

        let manager =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        manager.run_migrations().expect("migrations run");

        let repo = SqlCipherActivityRepository::new(manager.clone());
        (repo, manager, temp_dir)
    }

    fn sample_snapshot(id: &str, timestamp: i64) -> ActivitySnapshot {
        ActivitySnapshot {
            id: id.to_string(),
            timestamp,
            activity_context_json: "{}".to_string(),
            detected_activity: "working".to_string(),
            work_type: Some("focus".to_string()),
            activity_category: Some("development".to_string()),
            primary_app: "PulseArc".to_string(),
            processed: false,
            batch_id: None,
            created_at: timestamp,
            processed_at: None,
            is_idle: false,
            idle_duration_secs: None,
        }
    }
}
