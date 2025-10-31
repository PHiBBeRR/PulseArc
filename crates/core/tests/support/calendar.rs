use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_core::tracking::ports::CalendarEventRepository;
use pulsearc_domain::{CalendarEventRow, Result as DomainResult};

/// In-memory mock for `CalendarEventRepository`.
///
/// Stores a fixed set of rows and returns the first event that overlaps the
/// requested timestamp window. Designed for classification/unit tests where
/// deterministic responses are required.
#[derive(Default, Clone)]
pub struct MockCalendarEventRepository {
    events: Arc<Vec<CalendarEventRow>>,
}

impl MockCalendarEventRepository {
    /// Create a new mock seeded with the provided events.
    pub fn new(events: Vec<CalendarEventRow>) -> Self {
        Self {
            events: Arc::new(events),
        }
    }

    /// Convenience helper for adding a single event to the mock.
    pub fn with_event(mut self, event: CalendarEventRow) -> Self {
        Arc::make_mut(&mut self.events).push(event);
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
            .iter()
            .find(|event| {
                (event.start_ts <= upper && event.end_ts >= lower)
                    || (event.start_ts == timestamp && event.end_ts == timestamp)
            })
            .cloned())
    }
}
