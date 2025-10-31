//! Port interfaces for activity tracking
//!
//! These traits define the boundaries between core business logic
//! and infrastructure implementations.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use pulsearc_common::error::CommonResult;
use pulsearc_domain::types::database::{ActivitySegment, ActivitySnapshot, CalendarEventParams};
use pulsearc_domain::{ActivityContext, CalendarEventRow, Result};

/// Trait for capturing activity from the operating system
#[async_trait]
pub trait ActivityProvider: Send + Sync {
    /// Get the current activity context
    async fn get_activity(&self) -> Result<ActivityContext>;

    /// Check if tracking is paused
    fn is_paused(&self) -> bool;

    /// Pause activity tracking
    fn pause(&mut self) -> Result<()>;

    /// Resume activity tracking
    fn resume(&mut self) -> Result<()>;
}

/// Trait for persisting activity snapshots
///
/// PHASE-0: Uses database::ActivitySnapshot (full legacy schema)
#[async_trait]
pub trait ActivityRepository: Send + Sync {
    /// Save an activity snapshot
    async fn save_snapshot(&self, snapshot: ActivitySnapshot) -> Result<()>;

    /// Get snapshots within a time range
    async fn get_snapshots(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ActivitySnapshot>>;

    /// Delete snapshots older than the specified date
    async fn delete_old_snapshots(&self, before: chrono::DateTime<chrono::Utc>) -> Result<usize>;
}

/// Trait for enriching activity context with additional metadata
#[async_trait]
pub trait ActivityEnricher: Send + Sync {
    /// Enrich an activity context with additional information
    async fn enrich(&self, context: &mut ActivityContext) -> Result<()>;
}

// ============================================================================
// Phase 0: Segmenter Refactor Ports
// ============================================================================
// These ports use synchronous APIs to match SqlCipherPool's synchronous design

/// Port for segment persistence and retrieval
///
/// This trait uses synchronous methods because SqlCipherPool is synchronous.
/// No async/await needed or supported.
pub trait SegmentRepository: Send + Sync {
    /// Save a segment to storage
    fn save_segment(&self, segment: &ActivitySegment) -> CommonResult<()>;

    /// Find segments by date (date derived from start_ts)
    fn find_segments_by_date(&self, date: NaiveDate) -> CommonResult<Vec<ActivitySegment>>;

    /// Find unprocessed segments (processed = false)
    fn find_unprocessed_segments(&self, limit: usize) -> CommonResult<Vec<ActivitySegment>>;

    /// Mark a segment as processed
    fn mark_processed(&self, segment_id: &str) -> CommonResult<()>;
}

/// Port for snapshot retrieval (read-only for segmenter)
///
/// This trait uses synchronous methods because SqlCipherPool is synchronous.
/// No async/await needed or supported.
pub trait SnapshotRepository: Send + Sync {
    /// Find snapshots within a time range
    fn find_snapshots_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> CommonResult<Vec<ActivitySnapshot>>;

    /// Count snapshots for a given date
    fn count_snapshots_by_date(&self, date: NaiveDate) -> CommonResult<usize>;
}

/// Repository for querying calendar events
///
/// Reuses existing CalendarEventRow from domain (no new types needed).
/// Used by signal extractors to correlate activity snapshots with calendar
/// events.
#[async_trait]
pub trait CalendarEventRepository: Send + Sync {
    /// Find calendar event overlapping with timestamp (within ±window_secs)
    ///
    /// # Arguments
    /// * `timestamp` - Unix epoch timestamp to search around
    /// * `window_secs` - Time window in seconds (±window from timestamp)
    ///
    /// # Returns
    /// The calendar event if found, or None if no event overlaps with the time
    /// window
    async fn find_event_by_timestamp(
        &self,
        timestamp: i64,
        window_secs: i64,
    ) -> Result<Option<CalendarEventRow>>;

    /// Insert a calendar event
    ///
    /// # Arguments
    /// * `params` - Calendar event parameters containing all event details
    ///
    /// # Returns
    /// Success or error if insertion fails
    async fn insert_calendar_event(&self, params: CalendarEventParams) -> Result<()>;

    /// Get calendar events within a time range for a specific user
    ///
    /// # Arguments
    /// * `user_email` - User's email address
    /// * `start_ts` - Start of time range (Unix epoch seconds)
    /// * `end_ts` - End of time range (Unix epoch seconds)
    ///
    /// # Returns
    /// Vector of calendar events within the specified time range
    async fn get_calendar_events_by_time_range(
        &self,
        user_email: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<CalendarEventRow>>;

    /// Get all calendar events for today
    ///
    /// # Returns
    /// Vector of today's calendar events across all users
    async fn get_today_calendar_events(&self) -> Result<Vec<CalendarEventRow>>;

    /// Delete calendar events older than the specified number of days
    ///
    /// # Arguments
    /// * `days` - Number of days (events older than this will be deleted)
    ///
    /// # Returns
    /// Number of events deleted
    async fn delete_calendar_events_older_than(&self, days: i64) -> Result<usize>;
}
