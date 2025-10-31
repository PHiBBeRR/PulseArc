use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use pulsearc_core::tracking::ports::CalendarEventRepository;
use pulsearc_domain::{CalendarEventParams, CalendarEventRow, Result as DomainResult};

/// In-memory mock for `CalendarEventRepository`.
///
/// Stores a fixed set of rows and returns the first event that overlaps the
/// requested timestamp window. Designed for classification/unit tests where
/// deterministic responses are required.
#[derive(Default, Clone)]
pub struct MockCalendarEventRepository {
    events: Arc<Mutex<Vec<CalendarEventRow>>>,
}

impl MockCalendarEventRepository {
    /// Create a new mock seeded with the provided events.
    pub fn new(events: Vec<CalendarEventRow>) -> Self {
        Self {
            events: Arc::new(Mutex::new(events)),
        }
    }

    /// Convenience helper for adding a single event to the mock.
    pub fn with_event(self, event: CalendarEventRow) -> Self {
        self.events.lock().unwrap().push(event);
        self
    }
}

#[async_trait]
impl CalendarEventRepository for MockCalendarEventRepository {
    async fn find_event_by_timestamp(
        &self,
        timestamp: i64,
        window_secs: i64,
    ) -> DomainResult<Option<CalendarEventRow>> {
        let lower = timestamp - window_secs;
        let upper = timestamp + window_secs;

        Ok(self
            .events
            .lock()
            .unwrap()
            .iter()
            .find(|event| {
                (event.start_ts <= upper && event.end_ts >= lower)
                    || (event.start_ts == timestamp && event.end_ts == timestamp)
            })
            .cloned())
    }

    async fn insert_calendar_event(&self, params: CalendarEventParams) -> DomainResult<()> {
        // Mock implementation: convert params to row and store
        let row = CalendarEventRow {
            id: params.id,
            google_event_id: params.google_event_id,
            user_email: params.user_email,
            summary: params.summary,
            description: params.description,
            start_ts: params.when.start_ts,
            end_ts: params.when.end_ts,
            is_all_day: params.when.is_all_day,
            recurring_event_id: params.recurring_event_id,
            parsed_project: params.parsed.project,
            parsed_workstream: params.parsed.workstream,
            parsed_task: params.parsed.task,
            confidence_score: params.parsed.confidence_score,
            meeting_platform: params.meeting_platform,
            is_recurring_series: params.is_recurring_series,
            is_online_meeting: params.is_online_meeting,
            has_external_attendees: params.has_external_attendees,
            organizer_email: params.organizer_email,
            organizer_domain: params.organizer_domain,
            meeting_id: params.meeting_id,
            attendee_count: params.attendee_count,
            external_attendee_count: params.external_attendee_count,
            created_at: Utc::now().timestamp(),
        };
        self.events.lock().unwrap().push(row);
        Ok(())
    }

    async fn get_calendar_events_by_time_range(
        &self,
        user_email: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> DomainResult<Vec<CalendarEventRow>> {
        Ok(self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| {
                e.user_email == user_email && e.start_ts >= start_ts && e.end_ts <= end_ts
            })
            .cloned()
            .collect())
    }

    async fn get_today_calendar_events(&self) -> DomainResult<Vec<CalendarEventRow>> {
        // Mock: return all events (doesn't actually filter by today)
        Ok(self.events.lock().unwrap().clone())
    }

    async fn delete_calendar_events_older_than(&self, days: i64) -> DomainResult<usize> {
        let cutoff = Utc::now().timestamp() - (days * 86400);
        let mut events = self.events.lock().unwrap();
        let initial_len = events.len();
        events.retain(|e| e.created_at >= cutoff);
        Ok(initial_len - events.len())
    }
}
