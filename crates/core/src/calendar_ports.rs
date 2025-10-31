//! Calendar integration port interfaces (feature: calendar)
//!
//! This module is only compiled when the `calendar` feature is enabled.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use pulsearc_domain::Result;

/// Calendar event (simplified representation)
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub attendees: Vec<String>,
}

/// Sync status result
pub struct SyncStatus {
    pub last_sync: Option<DateTime<Utc>>,
    pub events_synced: usize,
    pub success: bool,
}

/// Trait for calendar provider operations
#[async_trait]
pub trait CalendarProvider: Send + Sync {
    /// Fetch events within a time range
    async fn fetch_events(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>>;

    /// Sync calendar data
    async fn sync(&self) -> Result<SyncStatus>;
}
