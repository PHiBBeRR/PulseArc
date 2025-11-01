//! SQLCipher-backed implementation of the `BlockRepository` port.
//!
//! Provides async persistence for proposed time blocks as well as helper
//! utilities for approval workflows and historical lookups. All queries are
//! issued through the shared `DbManager` SQLCipher pool.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use pulsearc_common::storage::error::{StorageError, StorageResult};
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_core::classification::ports::BlockRepository as BlockRepositoryPort;
use pulsearc_domain::types::classification::{
    ActivityBreakdown, BlockConfig, ProposedBlock, WorkLocation,
};
use pulsearc_domain::{PulseArcError, Result as DomainResult};
use rusqlite::{params, OptionalExtension, Row, ToSql};
use tokio::task;
use tracing::warn;

use super::manager::DbManager;

type UtcDateTime = DateTime<Utc>;

/// SQLCipher-backed block repository.
pub struct SqlCipherBlockRepository {
    db: Arc<DbManager>,
}

impl SqlCipherBlockRepository {
    /// Create a new repository backed by the shared `DbManager`.
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }

    /// Approve a block by setting its status to `"accepted"` and recording the
    /// review timestamp.
    pub async fn approve_block(
        &self,
        block_id: &str,
        reviewed_at: DateTime<Utc>,
    ) -> DomainResult<()> {
        self.update_block_status(block_id, "accepted", Some(reviewed_at.timestamp())).await
    }

    /// Reject a block by setting its status to `"rejected"` and recording the
    /// review timestamp.
    pub async fn reject_block(
        &self,
        block_id: &str,
        reviewed_at: DateTime<Utc>,
    ) -> DomainResult<()> {
        self.update_block_status(block_id, "rejected", Some(reviewed_at.timestamp())).await
    }

    /// Fetch the historical block versions that reference the given snapshot
    /// identifier. Results are ordered most-recent-first by `created_at`.
    pub async fn get_block_history(&self, snapshot_id: &str) -> DomainResult<Vec<ProposedBlock>> {
        let snapshot_pattern = format!("%\"{}\"%", snapshot_id);
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Vec<ProposedBlock>> {
            let conn = db.get_connection()?;
            let params: [&dyn ToSql; 1] = [&snapshot_pattern];
            query_blocks(&conn, BLOCK_SELECT_BY_SNAPSHOT, &params).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn update_block_status(
        &self,
        block_id: &str,
        status: &str,
        reviewed_at: Option<i64>,
    ) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let status = status.to_owned();
        let block_id = block_id.to_owned();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            conn.execute(
                "UPDATE proposed_time_blocks SET status = ?, reviewed_at = ? WHERE id = ?",
                params![status, reviewed_at, block_id],
            )
            .map_err(StorageError::from)
            .map_err(map_storage_error)?;
            Ok(())
        })
        .await
        .map_err(map_join_error)?
    }
}

#[async_trait]
impl BlockRepositoryPort for SqlCipherBlockRepository {
    async fn save_proposed_block(&self, block: &ProposedBlock) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let block = block.clone();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            insert_block(&conn, &block).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_proposed_blocks(&self, date: NaiveDate) -> DomainResult<Vec<ProposedBlock>> {
        let (start, end) = day_bounds(date);
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Vec<ProposedBlock>> {
            let conn = db.get_connection()?;
            let start_ts = start.timestamp();
            let end_ts = end.timestamp();
            let params: [&dyn ToSql; 2] = [&start_ts, &end_ts];
            query_blocks(&conn, BLOCK_SELECT_FOR_DAY, &params).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_proposed_block(&self, block_id: &str) -> DomainResult<Option<ProposedBlock>> {
        let db = Arc::clone(&self.db);
        let block_id = block_id.to_owned();

        task::spawn_blocking(move || -> DomainResult<Option<ProposedBlock>> {
            let conn = db.get_connection()?;
            let params: [&dyn ToSql; 1] = [&block_id];
            let rows =
                query_blocks(&conn, BLOCK_SELECT_BY_ID, &params).map_err(map_storage_error)?;
            Ok(rows.into_iter().next())
        })
        .await
        .map_err(map_join_error)?
    }

    async fn approve_block(&self, block_id: &str, reviewed_at: DateTime<Utc>) -> DomainResult<()> {
        SqlCipherBlockRepository::approve_block(self, block_id, reviewed_at).await
    }

    async fn reject_block(&self, block_id: &str, reviewed_at: DateTime<Utc>) -> DomainResult<()> {
        SqlCipherBlockRepository::reject_block(self, block_id, reviewed_at).await
    }

    async fn get_block_history(&self, snapshot_id: &str) -> DomainResult<Vec<ProposedBlock>> {
        SqlCipherBlockRepository::get_block_history(self, snapshot_id).await
    }

    async fn get_block_config(&self) -> DomainResult<BlockConfig> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<BlockConfig> {
            let conn = db.get_connection()?;
            let config = conn
                .inner()
                .query_row(BLOCK_CONFIG_SELECT, rusqlite::params![], map_block_config)
                .optional()
                .map_err(StorageError::from)
                .map_err(map_storage_error)?;
            Ok(config.unwrap_or_default())
        })
        .await
        .map_err(map_join_error)?
    }
}

const INSERT_BLOCK_SQL: &str = "INSERT OR REPLACE INTO proposed_time_blocks (
        id, start_ts, end_ts, duration_secs,
        inferred_project_id, inferred_wbs_code, inferred_deal_name, inferred_workstream,
        billable, confidence,
        activities_json, snapshot_ids_json, segment_ids, reasons_json,
        status, created_at, reviewed_at,
        total_idle_secs, idle_handling,
        has_calendar_overlap, overlapping_event_ids, is_double_booked,
        timezone, work_location, is_travel, is_weekend, is_after_hours
    ) VALUES (
        ?1, ?2, ?3, ?4,
        ?5, ?6, ?7, ?8,
        ?9, ?10,
        ?11, ?12, ?13, ?14,
        ?15, ?16, ?17,
        ?18, ?19,
        ?20, ?21, ?22,
        ?23, ?24, ?25, ?26, ?27
    )";

const BLOCK_SELECT_FOR_DAY: &str = "SELECT
        id, start_ts, end_ts, duration_secs,
        inferred_project_id, inferred_wbs_code, inferred_deal_name, inferred_workstream,
        billable, confidence,
        activities_json, snapshot_ids_json, segment_ids, reasons_json,
        status, created_at, reviewed_at,
        total_idle_secs, idle_handling,
        has_calendar_overlap, overlapping_event_ids, is_double_booked,
        timezone, work_location, is_travel, is_weekend, is_after_hours
    FROM proposed_time_blocks
    WHERE start_ts >= ?1 AND start_ts < ?2
    ORDER BY start_ts ASC";

const BLOCK_SELECT_BY_SNAPSHOT: &str = "SELECT
        id, start_ts, end_ts, duration_secs,
        inferred_project_id, inferred_wbs_code, inferred_deal_name, inferred_workstream,
        billable, confidence,
        activities_json, snapshot_ids_json, segment_ids, reasons_json,
        status, created_at, reviewed_at,
        total_idle_secs, idle_handling,
        has_calendar_overlap, overlapping_event_ids, is_double_booked,
        timezone, work_location, is_travel, is_weekend, is_after_hours
    FROM proposed_time_blocks
    WHERE snapshot_ids_json LIKE ?1
    ORDER BY created_at DESC";

const BLOCK_SELECT_BY_ID: &str = "SELECT
        id, start_ts, end_ts, duration_secs,
        inferred_project_id, inferred_wbs_code, inferred_deal_name, inferred_workstream,
        billable, confidence,
        activities_json, snapshot_ids_json, segment_ids, reasons_json,
        status, created_at, reviewed_at,
        total_idle_secs, idle_handling,
        has_calendar_overlap, overlapping_event_ids, is_double_booked,
        timezone, work_location, is_travel, is_weekend, is_after_hours
    FROM proposed_time_blocks
    WHERE id = ?1";

const BLOCK_CONFIG_SELECT: &str = "SELECT
        min_block_duration_secs,
        max_gap_for_merge_secs,
        consolidation_window_secs,
        min_billing_increment_secs
    FROM block_config
    WHERE id = 1";

fn insert_block(conn: &SqlCipherConnection, block: &ProposedBlock) -> StorageResult<()> {
    let activities_json = serialize_json(&block.activities)?;
    let snapshot_ids_json = serialize_json(&block.snapshot_ids)?;
    let segment_ids_json = serialize_json(&block.segment_ids)?;
    let reasons_json = serialize_json(&block.reasons)?;
    let overlapping_event_ids_json = serialize_json(&block.overlapping_event_ids)?;
    let work_location = block.work_location.as_ref().map(work_location_to_str);

    conn.execute(
        INSERT_BLOCK_SQL,
        params![
            block.id,
            block.start_ts,
            block.end_ts,
            block.duration_secs,
            block.inferred_project_id,
            block.inferred_wbs_code,
            block.inferred_deal_name,
            block.inferred_workstream,
            block.billable,
            block.confidence,
            activities_json,
            snapshot_ids_json,
            segment_ids_json,
            reasons_json,
            block.status,
            block.created_at,
            block.reviewed_at,
            block.total_idle_secs,
            block.idle_handling,
            block.has_calendar_overlap,
            overlapping_event_ids_json,
            block.is_double_booked,
            block.timezone,
            work_location,
            block.is_travel,
            block.is_weekend,
            block.is_after_hours,
        ],
    )
    .map_err(StorageError::from)?;

    Ok(())
}

fn query_blocks(
    conn: &SqlCipherConnection,
    sql: &str,
    params: &[&dyn ToSql],
) -> StorageResult<Vec<ProposedBlock>> {
    let mut stmt = conn.prepare(sql)?;
    stmt.query_map(params, map_block_row)
}

fn map_block_row(row: &Row<'_>) -> rusqlite::Result<ProposedBlock> {
    let activities: Vec<ActivityBreakdown> = deserialize_json(row.get(10)?, 10)?;
    let snapshot_ids: Vec<String> = deserialize_json(row.get(11)?, 11)?;
    let segment_ids: Vec<String> = match deserialize_json(row.get(12)?, 12) {
        Ok(ids) => ids,
        Err(err) => {
            warn!(error = %err, "failed to parse segment_ids JSON for proposed block");
            Vec::new()
        }
    };
    let reasons: Vec<String> = deserialize_json(row.get(13)?, 13)?;
    let overlapping_event_ids: Vec<String> = deserialize_json(row.get(20)?, 20)?;

    let confidence: f64 = row.get(9)?;
    let billable: bool = row.get::<_, i64>(8)? != 0;
    let has_calendar_overlap: bool = row.get::<_, i64>(19)? != 0;
    let is_double_booked: bool = row.get::<_, i64>(21)? != 0;
    let is_travel: bool = row.get::<_, i64>(24)? != 0;
    let is_weekend: bool = row.get::<_, i64>(25)? != 0;
    let is_after_hours: bool = row.get::<_, i64>(26)? != 0;

    let timezone: Option<String> = row.get(22)?;
    let work_location = row.get::<_, Option<String>>(23)?.and_then(|value| match value.as_str() {
        "Home" => Some(WorkLocation::Home),
        "Office" => Some(WorkLocation::Office),
        "Travel" => Some(WorkLocation::Travel),
        other => {
            warn!(%other, "unrecognised work_location value in proposed_time_blocks");
            None
        }
    });

    Ok(ProposedBlock {
        id: row.get(0)?,
        start_ts: row.get(1)?,
        end_ts: row.get(2)?,
        duration_secs: row.get(3)?,
        inferred_project_id: row.get(4)?,
        inferred_wbs_code: row.get(5)?,
        inferred_deal_name: row.get(6)?,
        inferred_workstream: row.get(7)?,
        billable,
        confidence: confidence as f32,
        classifier_used: None,
        activities,
        snapshot_ids,
        segment_ids,
        reasons,
        status: row.get(14)?,
        created_at: row.get(15)?,
        reviewed_at: row.get(16)?,
        total_idle_secs: row.get(17)?,
        idle_handling: row.get(18)?,
        timezone,
        work_location,
        is_travel,
        is_weekend,
        is_after_hours,
        has_calendar_overlap,
        overlapping_event_ids,
        is_double_booked,
    })
}

fn map_block_config(row: &Row<'_>) -> rusqlite::Result<BlockConfig> {
    Ok(BlockConfig {
        min_block_duration_secs: row.get(0)?,
        max_gap_for_merge_secs: row.get(1)?,
        consolidation_window_secs: row.get(2)?,
        min_billing_increment_secs: row.get(3)?,
    })
}

fn day_bounds(date: NaiveDate) -> (UtcDateTime, UtcDateTime) {
    let start = date.and_time(NaiveTime::MIN).and_utc();
    let end = start + Duration::days(1);
    (start, end)
}

fn serialize_json<T: serde::Serialize>(value: &T) -> StorageResult<String> {
    serde_json::to_string(value).map_err(StorageError::SerdeJson)
}

fn deserialize_json<T: serde::de::DeserializeOwned>(
    value: String,
    column_index: usize,
) -> rusqlite::Result<T> {
    serde_json::from_str(&value).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            column_index,
            rusqlite::types::Type::Text,
            Box::new(err),
        )
    })
}

fn work_location_to_str(location: &WorkLocation) -> &str {
    match location {
        WorkLocation::Home => "Home",
        WorkLocation::Office => "Office",
        WorkLocation::Travel => "Travel",
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
        StorageError::Rusqlite(sql_err) => PulseArcError::Database(sql_err.to_string()),
        StorageError::R2d2(r2d2_err) => PulseArcError::Database(r2d2_err.to_string()),
        StorageError::SerdeJson(json_err) => PulseArcError::Database(json_err.to_string()),
    }
}

fn map_join_error(err: task::JoinError) -> PulseArcError {
    if err.is_cancelled() {
        PulseArcError::Internal("blocking block repository task cancelled".into())
    } else {
        PulseArcError::Internal(format!("blocking block repository task failed: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use pulsearc_domain::types::classification::{ActivityBreakdown, BlockConfig};
    use tempfile::TempDir;

    use super::*;

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn saves_and_fetches_block() {
        let (repo, _manager, _dir) = setup_repository().await;
        let block = sample_block("block-1", 1_700_000_000);

        repo.save_proposed_block(&block).await.expect("block saved");

        let date = NaiveDate::from_ymd_opt(2023, 11, 14).expect("date valid");
        let blocks = repo.get_proposed_blocks(date).await.expect("blocks fetched");

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].id, "block-1");
        assert_eq!(blocks[0].activities[0].name, "Coding");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn approve_and_reject_update_status() {
        let (repo, manager, _dir) = setup_repository().await;
        let block = sample_block("block-2", 1_700_010_000);
        repo.save_proposed_block(&block).await.expect("block saved");

        let review_ts = DateTime::from_timestamp(1_700_020_000, 0).unwrap();
        repo.approve_block("block-2", review_ts).await.expect("approved");

        assert_eq!(status_for(&manager, "block-2"), "accepted");

        let reject_ts = DateTime::from_timestamp(1_700_030_000, 0).unwrap();
        repo.reject_block("block-2", reject_ts).await.expect("rejected");

        assert_eq!(status_for(&manager, "block-2"), "rejected");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn history_returns_blocks_for_snapshot() {
        let (repo, _manager, _dir) = setup_repository().await;
        let mut block_a = sample_block("history-a", 1_700_100_000);
        block_a.snapshot_ids = vec!["snap-1".into(), "snap-2".into()];
        repo.save_proposed_block(&block_a).await.expect("block a saved");

        let mut block_b = sample_block("history-b", 1_700_200_000);
        block_b.snapshot_ids = vec!["snap-3".into()];
        repo.save_proposed_block(&block_b).await.expect("block b saved");

        let mut block_c = sample_block("history-c", 1_700_300_000);
        block_c.snapshot_ids = vec!["snap-1".into()];
        repo.save_proposed_block(&block_c).await.expect("block c saved");

        let history = repo.get_block_history("snap-1").await.expect("history fetched");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].id, "history-c");
        assert_eq!(history[1].id, "history-a");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_block_config_returns_defaults() {
        let (repo, _manager, _dir) = setup_repository().await;
        let config = repo.get_block_config().await.expect("config fetched");

        let defaults = BlockConfig::default();
        assert_eq!(config.min_block_duration_secs, defaults.min_block_duration_secs);
        assert_eq!(config.max_gap_for_merge_secs, defaults.max_gap_for_merge_secs);
        assert_eq!(config.consolidation_window_secs, defaults.consolidation_window_secs);
        assert_eq!(config.min_billing_increment_secs, defaults.min_billing_increment_secs);
    }

    async fn setup_repository() -> (SqlCipherBlockRepository, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("blocks.db");

        let manager =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        manager.run_migrations().expect("migrations run");

        let repo = SqlCipherBlockRepository::new(manager.clone());
        (repo, manager, temp_dir)
    }

    fn sample_block(id: &str, start_ts: i64) -> ProposedBlock {
        ProposedBlock {
            id: id.to_string(),
            start_ts,
            end_ts: start_ts + 1_800,
            duration_secs: 1_800,
            inferred_project_id: Some("PRJ-123".into()),
            inferred_wbs_code: Some("PRJ-123.001".into()),
            inferred_deal_name: Some("Project Atlas".into()),
            inferred_workstream: Some("analysis".into()),
            billable: true,
            confidence: 0.92,
            classifier_used: Some("hybrid".into()),
            activities: vec![ActivityBreakdown {
                name: "Coding".into(),
                duration_secs: 1_200,
                percentage: 66.6,
            }],
            snapshot_ids: vec!["snap-1".into()],
            segment_ids: vec!["seg-1".into()],
            reasons: vec!["High confidence classification".into()],
            status: "suggested".into(),
            created_at: start_ts,
            reviewed_at: None,
            total_idle_secs: 120,
            idle_handling: "include".into(),
            timezone: Some("America/New_York".into()),
            work_location: Some(WorkLocation::Office),
            is_travel: false,
            is_weekend: false,
            is_after_hours: false,
            has_calendar_overlap: true,
            overlapping_event_ids: vec!["event-xyz".into()],
            is_double_booked: false,
        }
    }

    fn status_for(manager: &Arc<DbManager>, id: &str) -> String {
        let conn = manager.get_connection().expect("connection");
        conn.query_row(
            "SELECT status FROM proposed_time_blocks WHERE id = ?1",
            params![id],
            |row| row.get::<_, String>(0),
        )
        .expect("status fetch")
    }
}
