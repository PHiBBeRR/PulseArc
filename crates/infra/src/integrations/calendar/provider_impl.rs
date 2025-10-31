//! Implementation of the CalendarProvider trait
//!
//! Bridges the calendar integration infrastructure to the core CalendarProvider
//! port.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use pulsearc_common::storage::sqlcipher::SqlCipherPool;
use pulsearc_core::calendar_ports::{
    CalendarEvent as CoreCalendarEvent, CalendarProvider, SyncStatus,
};
use pulsearc_core::tracking::ports::CalendarEventRepository;
use pulsearc_core::OutboxQueue;
use pulsearc_domain::Result;
use tracing::{instrument, warn};

use super::client::CalendarClient;
use super::providers::RawCalendarEvent;
use super::sync::CalendarSyncWorker;

/// Calendar provider implementation
///
/// Thread-safe implementation that can be shared across async tasks.
pub struct CalendarProviderImpl {
    client: CalendarClient,
    calendar_repo: Arc<dyn CalendarEventRepository>,
    outbox_queue: Arc<dyn OutboxQueue>,
    pool: Arc<SqlCipherPool>,
    user_email: String,
}

impl CalendarProviderImpl {
    /// Create a new calendar provider instance
    pub fn new(
        client: CalendarClient,
        calendar_repo: Arc<dyn CalendarEventRepository>,
        outbox_queue: Arc<dyn OutboxQueue>,
        pool: Arc<SqlCipherPool>,
        user_email: String,
    ) -> Self {
        Self { client, calendar_repo, outbox_queue, pool, user_email }
    }
}

#[async_trait]
impl CalendarProvider for CalendarProviderImpl {
    /// Fetch events within a time range
    ///
    /// Converts time range to query parameters and fetches from provider API,
    /// then converts infrastructure CalendarEvent to core CalendarEvent.
    #[instrument(skip(self))]
    async fn fetch_events(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<CoreCalendarEvent>> {
        // Build query params with time range
        let query_params = if self.client.provider().eq_ignore_ascii_case("microsoft") {
            vec![
                ("startDateTime", start.to_rfc3339()),
                ("endDateTime", end.to_rfc3339()),
                // Keep payload lean; provider controls defaults for attendees
                ("$select", "id,subject,bodyPreview,start,end,isAllDay,attendees".to_string()),
                ("$orderby", "start/dateTime asc".to_string()),
            ]
        } else {
            vec![
                ("singleEvents", "true".to_string()),
                ("orderBy", "startTime".to_string()),
                ("timeMin", start.to_rfc3339()),
                ("timeMax", end.to_rfc3339()),
                ("timeZone", "UTC".to_string()),
                (
                    "fields",
                    "items(id,summary,description,start,end,attendees),nextPageToken".to_string(),
                ),
            ]
        };

        // Fetch raw events
        let response = self.client.fetch_events("primary", &query_params).await?;

        // Convert to core CalendarEvent type, skipping events that fail parsing
        let mut skipped = 0usize;
        let events: Vec<CoreCalendarEvent> = response
            .events
            .into_iter()
            .filter_map(|raw| match Self::convert_provider_event(raw) {
                Ok(event) => Some(event),
                Err(err) => {
                    skipped += 1;
                    warn!(
                        event_id = %err.event_id,
                        field = err.field,
                        error = %err.reason,
                        "skipping provider calendar event due to parse failure"
                    );
                    None
                }
            })
            .collect();

        if skipped > 0 {
            warn!(skipped, kept = events.len(), "dropped malformed calendar events");
        }

        Ok(events)
    }

    /// Sync calendar data
    ///
    /// Creates a sync worker on-demand with fresh pool reference.
    #[instrument(skip(self))]
    async fn sync(&self) -> Result<SyncStatus> {
        // Create sync worker with pool (not connection)
        let sync_worker = CalendarSyncWorker::new(
            self.client.clone(),
            self.calendar_repo.clone(),
            self.outbox_queue.clone(),
            self.pool.clone(),
        );

        sync_worker.perform_sync(&self.user_email).await
    }
}

struct EventParseFailure {
    event_id: String,
    field: &'static str,
    reason: String,
}

impl EventParseFailure {
    fn new(event_id: String, field: &'static str, reason: String) -> Self {
        Self { event_id, field, reason }
    }
}

impl CalendarProviderImpl {
    fn convert_provider_event(
        raw: RawCalendarEvent,
    ) -> std::result::Result<CoreCalendarEvent, EventParseFailure> {
        let event_id = raw.id.clone();
        let start_time = if raw.is_all_day {
            Self::parse_all_day_timestamp(&raw.start)
                .map_err(|reason| EventParseFailure::new(event_id.clone(), "start", reason))?
        } else {
            Self::parse_event_timestamp(&raw.start)
                .map_err(|reason| EventParseFailure::new(event_id.clone(), "start", reason))?
        };

        let end_time = if raw.is_all_day {
            Self::parse_all_day_timestamp(&raw.end)
                .map_err(|reason| EventParseFailure::new(event_id.clone(), "end", reason))?
        } else {
            Self::parse_event_timestamp(&raw.end)
                .map_err(|reason| EventParseFailure::new(event_id.clone(), "end", reason))?
        };

        Ok(CoreCalendarEvent {
            id: raw.id,
            title: raw.subject.unwrap_or_else(|| "Untitled Event".to_string()),
            start_time,
            end_time,
            attendees: Vec::new(), // TODO: Parse attendees from provider response
        })
    }

    fn parse_all_day_timestamp(value: &str) -> std::result::Result<DateTime<Utc>, String> {
        let date = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|e| format!("invalid all-day date '{value}': {e}"))?;

        let midnight = date.and_hms_opt(0, 0, 0).ok_or_else(|| {
            format!("invalid all-day date '{value}': could not derive midnight timestamp")
        })?;

        Ok(midnight.and_utc())
    }

    fn parse_event_timestamp(value: &str) -> std::result::Result<DateTime<Utc>, String> {
        let trimmed = value.trim();
        let has_explicit_timezone = trimmed.ends_with('Z')
            || trimmed
                .rfind('T')
                .is_some_and(|idx| trimmed[idx + 1..].chars().any(|c| matches!(c, '+' | '-')));

        let candidate =
            if has_explicit_timezone { trimmed.to_string() } else { format!("{trimmed}Z") };

        chrono::DateTime::parse_from_rfc3339(&candidate)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| format!("invalid timestamp '{value}': {e}"))
    }
}
