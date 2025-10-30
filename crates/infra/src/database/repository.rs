//! Repository implementations for core domain

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use pulsearc_core::{ActivityRepository, TimeEntryRepository};
use pulsearc_shared::{ActivitySnapshot, PulseArcError, Result, TimeEntry};
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
            let context_json = serde_json::to_string(&snapshot.context)
                .map_err(|e| PulseArcError::Internal(e.to_string()))?;

            conn.execute(
                "INSERT INTO activity_snapshots (id, timestamp, context) VALUES (?1, ?2, ?3)",
                (snapshot.id.to_string(), snapshot.timestamp.timestamp(), context_json),
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
                .prepare("SELECT id, timestamp, context FROM activity_snapshots WHERE timestamp BETWEEN ?1 AND ?2")
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            let snapshots = stmt
                .query_map((start.timestamp(), end.timestamp()), |row| {
                    let id: String = row.get(0)?;
                    let timestamp: i64 = row.get(1)?;
                    let context_json: String = row.get(2)?;

                    Ok((id, timestamp, context_json))
                })
                .map_err(|e| PulseArcError::Database(e.to_string()))?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| PulseArcError::Database(e.to_string()))?
                .into_iter()
                .filter_map(|(id, timestamp, context_json)| {
                    let id = Uuid::parse_str(&id).ok()?;
                    let timestamp = DateTime::from_timestamp(timestamp, 0)?;
                    let context = serde_json::from_str(&context_json).ok()?;

                    Some(ActivitySnapshot {
                        id,
                        timestamp,
                        context,
                    })
                })
                .collect();

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
                "INSERT INTO time_entries (id, start_time, end_time, duration_seconds, description, project, wbs_code)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    entry.id.to_string(),
                    entry.start_time.timestamp(),
                    entry.end_time.timestamp(),
                    entry.duration_seconds,
                    entry.description,
                    entry.project,
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
                .prepare("SELECT id, start_time, end_time, duration_seconds, description, project, wbs_code FROM time_entries WHERE start_time BETWEEN ?1 AND ?2")
                .map_err(|e| PulseArcError::Database(e.to_string()))?;

            let entries = stmt
                .query_map((start.timestamp(), end.timestamp()), |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                })
                .map_err(|e| PulseArcError::Database(e.to_string()))?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| PulseArcError::Database(e.to_string()))?
                .into_iter()
                .filter_map(|(id, start_time, end_time, duration_seconds, description, project, wbs_code)| {
                    let id = Uuid::parse_str(&id).ok()?;
                    let start_time = DateTime::from_timestamp(start_time, 0)?;
                    let end_time = DateTime::from_timestamp(end_time, 0)?;

                    Some(TimeEntry {
                        id,
                        start_time,
                        end_time,
                        duration_seconds,
                        description,
                        project,
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
                "UPDATE time_entries SET start_time = ?2, end_time = ?3, duration_seconds = ?4, description = ?5, project = ?6, wbs_code = ?7 WHERE id = ?1",
                (
                    entry.id.to_string(),
                    entry.start_time.timestamp(),
                    entry.end_time.timestamp(),
                    entry.duration_seconds,
                    entry.description,
                    entry.project,
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
