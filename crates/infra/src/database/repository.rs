//! Repository implementations for core domain

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use pulsearc_core::{ActivityRepository, TimeEntryRepository};
use pulsearc_domain::{ActivitySnapshot, PulseArcError, Result, TimeEntry};
use uuid::Uuid;

use super::manager::DbManager;

/// SQLite implementation of ActivityRepository
pub struct SqliteActivityRepository {
    db: Arc<DbManager>,
}

impl SqliteActivityRepository {
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ActivityRepository for SqliteActivityRepository {
    async fn save_snapshot(&self, snapshot: ActivitySnapshot) -> Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            conn.execute(
                "INSERT INTO activity_snapshots (id, timestamp, activity_context_json, detected_activity, primary_app, processed, created_at, is_idle) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    &snapshot.id,
                    snapshot.timestamp,
                    &snapshot.activity_context_json,
                    &snapshot.detected_activity,
                    &snapshot.primary_app,
                    snapshot.processed,
                    snapshot.created_at,
                    snapshot.is_idle,
                ),
            )
            .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    async fn get_snapshots(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<ActivitySnapshot>> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;
            let mut stmt = conn
                .prepare("SELECT id, timestamp, activity_context_json, detected_activity, work_type, activity_category, primary_app, processed, batch_id, created_at, processed_at, is_idle, idle_duration_secs FROM activity_snapshots WHERE timestamp BETWEEN ?1 AND ?2")
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            let snapshots = stmt
                .query_map((start.timestamp(), end.timestamp()), |row| {
                    Ok(ActivitySnapshot {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        activity_context_json: row.get(2)?,
                        detected_activity: row.get(3)?,
                        work_type: row.get(4)?,
                        activity_category: row.get(5)?,
                        primary_app: row.get(6)?,
                        processed: row.get(7)?,
                        batch_id: row.get(8)?,
                        created_at: row.get(9)?,
                        processed_at: row.get(10)?,
                        is_idle: row.get(11)?,
                        idle_duration_secs: row.get(12)?,
                    })
                })
                .map_err(|e| PulseArcError::Database(e.to_string()))?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(snapshots)
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    async fn delete_old_snapshots(&self, before: DateTime<Utc>) -> Result<usize> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;
            let deleted = conn
                .execute(
                    "DELETE FROM activity_snapshots WHERE timestamp < ?1",
                    [before.timestamp()],
                )
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(deleted)
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }
}

/// SQLite implementation of TimeEntryRepository
pub struct SqliteTimeEntryRepository {
    db: Arc<DbManager>,
}

impl SqliteTimeEntryRepository {
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TimeEntryRepository for SqliteTimeEntryRepository {
    async fn save_entry(&self, entry: TimeEntry) -> Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            conn.execute(
                "INSERT INTO time_entries (id, start_time, end_time, duration_seconds, description, project_id, wbs_code)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    entry.id.to_string(),
                    entry.start_time.timestamp(),
                    entry.end_time.map(|dt| dt.timestamp()),
                    entry.duration_seconds,
                    entry.description,
                    entry.project_id,
                    entry.wbs_code,
                ),
            )
            .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    async fn get_entries(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<TimeEntry>> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;
            let mut stmt = conn
                .prepare("SELECT id, start_time, end_time, duration_seconds, description, project_id, wbs_code FROM time_entries WHERE start_time BETWEEN ?1 AND ?2")
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            let entries = stmt
                .query_map((start.timestamp(), end.timestamp()), |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                })
                .map_err(|e| PulseArcError::Database(e.to_string()))?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| PulseArcError::Database(e.to_string()))?
                .into_iter()
                .filter_map(|(id, start_time, end_time, duration_seconds, description, project_id, wbs_code)| {
                    let id = Uuid::parse_str(&id).ok()?;
                    let start_time = DateTime::from_timestamp(start_time, 0)?;
                    let end_time = end_time.and_then(|ts| DateTime::from_timestamp(ts, 0));

                    Some(TimeEntry {
                        id,
                        start_time,
                        end_time,
                        duration_seconds,
                        description,
                        project_id,
                        wbs_code,
                    })
                })
                .collect();

            Ok(entries)
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    async fn update_entry(&self, entry: TimeEntry) -> Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            conn.execute(
                "UPDATE time_entries SET start_time = ?2, end_time = ?3, duration_seconds = ?4, description = ?5, project_id = ?6, wbs_code = ?7 WHERE id = ?1",
                (
                    entry.id.to_string(),
                    entry.start_time.timestamp(),
                    entry.end_time.map(|dt| dt.timestamp()),
                    entry.duration_seconds,
                    entry.description,
                    entry.project_id,
                    entry.wbs_code,
                ),
            )
            .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }

    async fn delete_entry(&self, id: Uuid) -> Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            conn.execute("DELETE FROM time_entries WHERE id = ?1", [id.to_string()])
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            Ok(())
        })
        .await
        .map_err(|e| PulseArcError::Internal(e.to_string()))?
    }
}
