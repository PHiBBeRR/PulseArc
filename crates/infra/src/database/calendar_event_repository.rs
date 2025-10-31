//! SqlCipher-backed implementation of the CalendarEventRepository port.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use pulsearc_common::storage::sqlcipher::SqlCipherPool;
use pulsearc_core::tracking::ports::CalendarEventRepository;
use pulsearc_domain::{CalendarEventParams, CalendarEventRow, Result};
use rusqlite::ToSql;
use tracing::{debug, instrument};

use crate::errors::InfraError;

/// SqlCipher implementation of CalendarEventRepository
pub struct SqlCipherCalendarEventRepository {
    pool: Arc<SqlCipherPool>,
}

impl SqlCipherCalendarEventRepository {
    /// Create a new calendar event repository
    pub fn new(pool: Arc<SqlCipherPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CalendarEventRepository for SqlCipherCalendarEventRepository {
    #[instrument(skip(self), fields(timestamp, window_secs))]
    async fn find_event_by_timestamp(
        &self,
        timestamp: i64,
        window_secs: i64,
    ) -> Result<Option<CalendarEventRow>> {
        let conn = self.pool.get_sqlcipher_connection().map_err(|e| {
            InfraError(pulsearc_domain::PulseArcError::Database(format!("pool error: {}", e)))
        })?;

        let lower = timestamp - window_secs;
        let upper = timestamp + window_secs;

        debug!(timestamp, window_secs, lower, upper, "searching for calendar event");

        let result = conn.query_row(
            "SELECT id, google_event_id, user_email, summary, description,
                    start_ts, end_ts, is_all_day, recurring_event_id,
                    parsed_project, parsed_workstream, parsed_task,
                    confidence_score, meeting_platform, is_recurring_series,
                    is_online_meeting, created_at
             FROM calendar_events
             WHERE (start_ts <= ?1 AND end_ts >= ?2)
                OR (start_ts = ?3 AND end_ts = ?3)
             LIMIT 1",
            [&upper as &dyn ToSql, &lower, &timestamp].as_ref(),
            |row| {
                Ok(CalendarEventRow {
                    id: row.get(0)?,
                    google_event_id: row.get(1)?,
                    user_email: row.get(2)?,
                    summary: row.get(3)?,
                    description: row.get(4)?,
                    start_ts: row.get(5)?,
                    end_ts: row.get(6)?,
                    is_all_day: row.get(7)?,
                    recurring_event_id: row.get(8)?,
                    parsed_project: row.get(9)?,
                    parsed_workstream: row.get(10)?,
                    parsed_task: row.get(11)?,
                    confidence_score: row.get(12)?,
                    meeting_platform: row.get(13)?,
                    is_recurring_series: row.get(14)?,
                    is_online_meeting: row.get(15)?,
                    created_at: row.get(16)?,
                    has_external_attendees: None,
                    organizer_email: None,
                    organizer_domain: None,
                    meeting_id: None,
                    attendee_count: None,
                    external_attendee_count: None,
                })
            },
        );

        match result {
            Ok(event) => Ok(Some(event)),
            Err(e) => match e {
                pulsearc_common::storage::error::StorageError::Query(msg)
                    if msg.contains("no rows") =>
                {
                    Ok(None)
                }
                _ => Err(InfraError::from(e).into()),
            },
        }
    }

    #[instrument(skip(self, params))]
    async fn insert_calendar_event(&self, params: CalendarEventParams) -> Result<()> {
        let conn = self.pool.get_sqlcipher_connection().map_err(|e| {
            InfraError(pulsearc_domain::PulseArcError::Database(format!("pool error: {}", e)))
        })?;

        let now = Utc::now().timestamp();

        // UPSERT logic: INSERT with ON CONFLICT UPDATE
        conn.execute(
            "INSERT INTO calendar_events (
                id, google_event_id, user_email, summary, description,
                start_ts, end_ts, is_all_day, recurring_event_id,
                parsed_project, parsed_workstream, parsed_task,
                confidence_score, meeting_platform, is_recurring_series,
                is_online_meeting, has_external_attendees, organizer_email,
                organizer_domain, meeting_id, attendee_count, external_attendee_count,
                created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)
            ON CONFLICT(google_event_id, user_email) DO UPDATE SET
                summary = excluded.summary,
                description = excluded.description,
                start_ts = excluded.start_ts,
                end_ts = excluded.end_ts,
                is_all_day = excluded.is_all_day,
                parsed_project = excluded.parsed_project,
                parsed_workstream = excluded.parsed_workstream,
                parsed_task = excluded.parsed_task,
                confidence_score = excluded.confidence_score,
                meeting_platform = excluded.meeting_platform,
                is_recurring_series = excluded.is_recurring_series,
                is_online_meeting = excluded.is_online_meeting,
                has_external_attendees = excluded.has_external_attendees,
                organizer_email = excluded.organizer_email,
                organizer_domain = excluded.organizer_domain,
                meeting_id = excluded.meeting_id,
                attendee_count = excluded.attendee_count,
                external_attendee_count = excluded.external_attendee_count",
            [
                &params.id as &dyn ToSql,
                &params.google_event_id,
                &params.user_email,
                &params.summary,
                &params.description,
                &params.when.start_ts,
                &params.when.end_ts,
                &params.when.is_all_day,
                &params.recurring_event_id,
                &params.parsed.project,
                &params.parsed.workstream,
                &params.parsed.task,
                &params.parsed.confidence_score,
                &params.meeting_platform,
                &params.is_recurring_series,
                &params.is_online_meeting,
                &params.has_external_attendees,
                &params.organizer_email,
                &params.organizer_domain,
                &params.meeting_id,
                &params.attendee_count,
                &params.external_attendee_count,
                &now,
            ].as_ref(),
        )
        .map_err(InfraError::from)?;

        debug!(
            google_event_id = %params.google_event_id,
            user_email = %params.user_email,
            "inserted/updated calendar event"
        );

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_calendar_events_by_time_range(
        &self,
        user_email: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<CalendarEventRow>> {
        let conn = self.pool.get_sqlcipher_connection().map_err(|e| {
            InfraError(pulsearc_domain::PulseArcError::Database(format!("pool error: {}", e)))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, google_event_id, user_email, summary, description,
                        start_ts, end_ts, is_all_day, recurring_event_id,
                        parsed_project, parsed_workstream, parsed_task,
                        confidence_score, meeting_platform, is_recurring_series,
                        is_online_meeting, has_external_attendees, organizer_email,
                        organizer_domain, meeting_id, attendee_count, external_attendee_count,
                        created_at
                 FROM calendar_events
                 WHERE user_email = ?1 AND start_ts >= ?2 AND end_ts <= ?3
                 ORDER BY start_ts ASC",
            )
            .map_err(InfraError::from)?;

        let rows = stmt
            .query_map(&[&user_email as &dyn ToSql, &start_ts, &end_ts], |row| {
                Ok(CalendarEventRow {
                    id: row.get(0)?,
                    google_event_id: row.get(1)?,
                    user_email: row.get(2)?,
                    summary: row.get(3)?,
                    description: row.get(4)?,
                    start_ts: row.get(5)?,
                    end_ts: row.get(6)?,
                    is_all_day: row.get(7)?,
                    recurring_event_id: row.get(8)?,
                    parsed_project: row.get(9)?,
                    parsed_workstream: row.get(10)?,
                    parsed_task: row.get(11)?,
                    confidence_score: row.get(12)?,
                    meeting_platform: row.get(13)?,
                    is_recurring_series: row.get(14)?,
                    is_online_meeting: row.get(15)?,
                    has_external_attendees: row.get(16)?,
                    organizer_email: row.get(17)?,
                    organizer_domain: row.get(18)?,
                    meeting_id: row.get(19)?,
                    attendee_count: row.get(20)?,
                    external_attendee_count: row.get(21)?,
                    created_at: row.get(22)?,
                })
            })
            .map_err(InfraError::from)?;

        debug!(user_email, start_ts, end_ts, count = rows.len(), "retrieved calendar events");

        Ok(rows)
    }

    #[instrument(skip(self))]
    async fn get_today_calendar_events(&self) -> Result<Vec<CalendarEventRow>> {
        let conn = self.pool.get_sqlcipher_connection().map_err(|e| {
            InfraError(pulsearc_domain::PulseArcError::Database(format!("pool error: {}", e)))
        })?;

        let today_start =
            Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        let today_end =
            Utc::now().date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp();

        let mut stmt = conn
            .prepare(
                "SELECT id, google_event_id, user_email, summary, description,
                        start_ts, end_ts, is_all_day, recurring_event_id,
                        parsed_project, parsed_workstream, parsed_task,
                        confidence_score, meeting_platform, is_recurring_series,
                        is_online_meeting, has_external_attendees, organizer_email,
                        organizer_domain, meeting_id, attendee_count, external_attendee_count,
                        created_at
                 FROM calendar_events
                 WHERE start_ts >= ?1 AND end_ts <= ?2
                 ORDER BY start_ts ASC",
            )
            .map_err(InfraError::from)?;

        let rows = stmt
            .query_map([&today_start as &dyn ToSql, &today_end].as_ref(), |row| {
                Ok(CalendarEventRow {
                    id: row.get(0)?,
                    google_event_id: row.get(1)?,
                    user_email: row.get(2)?,
                    summary: row.get(3)?,
                    description: row.get(4)?,
                    start_ts: row.get(5)?,
                    end_ts: row.get(6)?,
                    is_all_day: row.get(7)?,
                    recurring_event_id: row.get(8)?,
                    parsed_project: row.get(9)?,
                    parsed_workstream: row.get(10)?,
                    parsed_task: row.get(11)?,
                    confidence_score: row.get(12)?,
                    meeting_platform: row.get(13)?,
                    is_recurring_series: row.get(14)?,
                    is_online_meeting: row.get(15)?,
                    has_external_attendees: row.get(16)?,
                    organizer_email: row.get(17)?,
                    organizer_domain: row.get(18)?,
                    meeting_id: row.get(19)?,
                    attendee_count: row.get(20)?,
                    external_attendee_count: row.get(21)?,
                    created_at: row.get(22)?,
                })
            })
            .map_err(InfraError::from)?;

        debug!(count = rows.len(), "retrieved today's calendar events");

        Ok(rows)
    }

    #[instrument(skip(self))]
    async fn delete_calendar_events_older_than(&self, days: i64) -> Result<usize> {
        let conn = self.pool.get_sqlcipher_connection().map_err(|e| {
            InfraError(pulsearc_domain::PulseArcError::Database(format!("pool error: {}", e)))
        })?;

        let cutoff = Utc::now().timestamp() - (days * 24 * 60 * 60);

        let deleted = conn
            .execute("DELETE FROM calendar_events WHERE end_ts < ?1", rusqlite::params![cutoff])
            .map_err(InfraError::from)?;

        debug!(days, deleted, "deleted old calendar events");

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use pulsearc_common::storage::sqlcipher::SqlCipherPoolConfig;
    use pulsearc_domain::types::database::{ParsedFields, TimeRange};
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;

    fn setup_test_db() -> (Arc<SqlCipherPool>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let encryption_key =
            "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();

        let config = SqlCipherPoolConfig::default();

        let pool = Arc::new(SqlCipherPool::new(&db_path, encryption_key, config).unwrap());

        // Create calendar_events table
        let conn = pool.get_sqlcipher_connection().unwrap();
        conn.execute_batch(
            "CREATE TABLE calendar_events (
                id TEXT PRIMARY KEY,
                google_event_id TEXT NOT NULL,
                user_email TEXT NOT NULL,
                summary TEXT NOT NULL,
                description TEXT,
                start_ts INTEGER NOT NULL,
                end_ts INTEGER NOT NULL,
                is_all_day INTEGER NOT NULL DEFAULT 0,
                recurring_event_id TEXT,
                parsed_project TEXT,
                parsed_workstream TEXT,
                parsed_task TEXT,
                confidence_score REAL,
                meeting_platform TEXT,
                is_recurring_series INTEGER NOT NULL DEFAULT 0,
                is_online_meeting INTEGER NOT NULL DEFAULT 0,
                has_external_attendees INTEGER,
                organizer_email TEXT,
                organizer_domain TEXT,
                meeting_id TEXT,
                attendee_count INTEGER,
                external_attendee_count INTEGER,
                created_at INTEGER NOT NULL,
                UNIQUE(google_event_id, user_email)
            );",
        )
        .unwrap();

        (pool, temp_dir)
    }

    #[tokio::test]
    async fn test_insert_and_find_event() {
        let (pool, _temp) = setup_test_db();
        let repo = SqlCipherCalendarEventRepository::new(pool);

        let now = Utc::now().timestamp();

        let params = CalendarEventParams {
            id: Uuid::now_v7().to_string(),
            google_event_id: "evt-123".to_string(),
            user_email: "test@example.com".to_string(),
            summary: "Test Meeting".to_string(),
            description: Some("Test description".to_string()),
            when: TimeRange { start_ts: now, end_ts: now + 3600, is_all_day: false },
            recurring_event_id: None,
            parsed: ParsedFields {
                project: Some("PulseArc".to_string()),
                workstream: Some("Backend".to_string()),
                task: Some("sync".to_string()),
                confidence_score: Some(0.9),
            },
            meeting_platform: None,
            is_recurring_series: false,
            is_online_meeting: false,
            has_external_attendees: None,
            organizer_email: None,
            organizer_domain: None,
            meeting_id: None,
            attendee_count: None,
            external_attendee_count: None,
        };

        repo.insert_calendar_event(params).await.unwrap();

        // Find event by timestamp
        let found = repo.find_event_by_timestamp(now + 1800, 1800).await.unwrap();
        assert!(found.is_some());

        let event = found.unwrap();
        assert_eq!(event.google_event_id, "evt-123");
        assert_eq!(event.summary, "Test Meeting");
    }

    #[tokio::test]
    async fn test_upsert_updates_existing_event() {
        let (pool, _temp) = setup_test_db();
        let repo = SqlCipherCalendarEventRepository::new(pool);

        let id = Uuid::now_v7().to_string();
        let now = Utc::now().timestamp();

        // Insert first version
        let params1 = CalendarEventParams {
            id: id.clone(),
            google_event_id: "evt-456".to_string(),
            user_email: "test@example.com".to_string(),
            summary: "Original Title".to_string(),
            description: None,
            when: TimeRange { start_ts: now, end_ts: now + 3600, is_all_day: false },
            recurring_event_id: None,
            parsed: ParsedFields {
                project: None,
                workstream: None,
                task: None,
                confidence_score: None,
            },
            meeting_platform: None,
            is_recurring_series: false,
            is_online_meeting: false,
            has_external_attendees: None,
            organizer_email: None,
            organizer_domain: None,
            meeting_id: None,
            attendee_count: None,
            external_attendee_count: None,
        };

        repo.insert_calendar_event(params1).await.unwrap();

        // Insert again with updated summary
        let params2 = CalendarEventParams {
            id: Uuid::now_v7().to_string(),         // Different ID
            google_event_id: "evt-456".to_string(), // Same google_event_id + user_email
            user_email: "test@example.com".to_string(),
            summary: "Updated Title".to_string(),
            description: Some("New description".to_string()),
            when: TimeRange { start_ts: now, end_ts: now + 3600, is_all_day: false },
            recurring_event_id: None,
            parsed: ParsedFields {
                project: Some("PulseArc".to_string()),
                workstream: None,
                task: None,
                confidence_score: None,
            },
            meeting_platform: None,
            is_recurring_series: false,
            is_online_meeting: false,
            has_external_attendees: None,
            organizer_email: None,
            organizer_domain: None,
            meeting_id: None,
            attendee_count: None,
            external_attendee_count: None,
        };

        repo.insert_calendar_event(params2).await.unwrap();

        // Should have updated, not created duplicate
        let found = repo.find_event_by_timestamp(now, 60).await.unwrap();
        assert!(found.is_some());

        let event = found.unwrap();
        assert_eq!(event.summary, "Updated Title");
        assert_eq!(event.description, Some("New description".to_string()));
        assert_eq!(event.parsed_project, Some("PulseArc".to_string()));
    }
}
