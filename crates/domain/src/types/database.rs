//! Database model types
//!
//! These types represent the database schema and are used by repository ports.
//! Phase 1 migration includes all types from legacy/api/src/db/models.rs

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
#[cfg(feature = "ts-gen")]
use ts_rs::TS;
use uuid::Uuid;

use crate::types::ActivityContext;
use crate::{PulseArcError, Result as DomainResult};

/// Activity snapshot - raw 30s activity capture
///
/// This is a minimal definition for Phase 0. Full type with all fields
/// will be migrated from legacy/api/src/db/models.rs in Phase 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct ActivitySnapshot {
    pub id: String,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub timestamp: i64,
    pub activity_context_json: String,
    pub detected_activity: String,
    pub work_type: Option<String>,
    pub activity_category: Option<String>,
    pub primary_app: String,
    pub processed: bool,
    pub batch_id: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub processed_at: Option<i64>,
    pub is_idle: bool,
    pub idle_duration_secs: Option<i32>,
}

/// Metadata required to construct a new `ActivitySnapshot`.
#[derive(Debug, Clone)]
pub struct SnapshotMetadata {
    pub id: String,
    pub timestamp: i64,
    pub created_at: i64,
    pub batch_id: Option<String>,
}

impl SnapshotMetadata {
    /// Generate metadata for the current timestamp using a UUIDv7 identifier.
    #[must_use]
    pub fn now() -> Self {
        let now = Utc::now().timestamp();
        Self { id: Uuid::now_v7().to_string(), timestamp: now, created_at: now, batch_id: None }
    }

    /// Override the batch identifier associated with the snapshot.
    #[must_use]
    pub fn with_batch_id(mut self, batch_id: Option<String>) -> Self {
        self.batch_id = batch_id;
        self
    }
}

/// Activity segment - 5-minute aggregated segment
///
/// This is a minimal definition for Phase 0. Full type with all fields
/// will be migrated from legacy/api/src/db/models.rs in Phase 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct ActivitySegment {
    pub id: String,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub start_ts: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub end_ts: i64,
    pub primary_app: String,
    pub normalized_label: String,
    pub sample_count: i32,
    pub dictionary_keys: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,
    pub processed: bool,
    pub snapshot_ids: Vec<String>,
    pub work_type: Option<String>,
    pub activity_category: String,
    pub detected_activity: String,
    pub extracted_signals_json: Option<String>,
    pub project_match_json: Option<String>,
    pub idle_time_secs: i32,
    pub active_time_secs: i32,
    pub user_action: Option<String>,
}

impl ActivitySnapshot {
    /// Get timestamp as DateTime<Utc>
    pub fn timestamp_utc(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.timestamp, 0)
    }

    /// Construct a snapshot from an `ActivityContext` using the supplied
    /// metadata.
    ///
    /// Serializes the context into JSON and derives legacy string fields so
    /// that downstream repositories can persist the record without
    /// additional mapping.
    pub fn from_activity_context(
        context: &ActivityContext,
        metadata: SnapshotMetadata,
    ) -> DomainResult<Self> {
        let serialized = serde_json::to_string(context).map_err(|err| {
            PulseArcError::Internal(format!(
                "failed to serialize activity context for snapshot: {err}"
            ))
        })?;

        let SnapshotMetadata { id, timestamp, created_at, batch_id } = metadata;

        let work_type = context.work_type.as_ref().and_then(Self::enum_to_string);
        let activity_category = Self::enum_to_string(&context.activity_category);

        let primary_app = context
            .active_app
            .bundle_id
            .clone()
            .unwrap_or_else(|| context.active_app.app_name.clone());

        Ok(Self {
            id,
            timestamp,
            activity_context_json: serialized,
            detected_activity: context.detected_activity.clone(),
            work_type,
            activity_category,
            primary_app,
            processed: false,
            batch_id,
            created_at,
            processed_at: None,
            is_idle: context.detected_activity.eq_ignore_ascii_case("idle"),
            idle_duration_secs: None,
        })
    }

    /// Deserialize the embedded activity context JSON into the strongly-typed
    /// structure.
    pub fn activity_context(&self) -> DomainResult<ActivityContext> {
        serde_json::from_str(&self.activity_context_json).map_err(|err| {
            PulseArcError::Database(format!("invalid activity_context_json payload: {err}"))
        })
    }

    fn enum_to_string<T: Serialize>(value: &T) -> Option<String> {
        serde_json::to_value(value).ok().and_then(|val| val.as_str().map(str::to_string))
    }
}

impl ActivitySegment {
    /// Get start time as DateTime<Utc>
    pub fn start_time_utc(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.start_ts, 0)
    }

    /// Get end time as DateTime<Utc>
    pub fn end_time_utc(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.end_ts, 0)
    }

    /// Get date from start timestamp
    pub fn date(&self) -> Option<NaiveDate> {
        self.start_time_utc().map(|dt| dt.date_naive())
    }

    /// Estimate token count for this segment (for OpenAI API batching)
    /// Uses rough approximation: 1 token ≈ 4 characters
    pub fn estimated_token_count(&self) -> usize {
        let text_len = self.primary_app.len() + self.normalized_label.len();
        // Add overhead for JSON structure
        (text_len + 50) / 4
    }
}

// ============================================================================
// Batch Processing Types
// ============================================================================

/// Batch queue entry for AI processing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct BatchQueue {
    pub batch_id: String, // UUIDv7
    pub activity_count: i32,
    pub status: BatchStatus,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub processed_at: Option<i64>,
    pub error_message: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub processing_started_at: Option<i64>,
    pub worker_id: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub lease_expires_at: Option<i64>,
    pub time_entries_created: i32,
    pub openai_cost: f64,
}

/// Batch processing status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
#[serde(rename_all = "lowercase")]
pub enum BatchStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

// Use the domain status macro
crate::impl_domain_status_conversions!(BatchStatus {
    Pending => "pending",
    Processing => "processing",
    Completed => "completed",
    Failed => "failed"
});

// ============================================================================
// Outbox Pattern Types
// ============================================================================

/// TimeEntryOutbox - Outbox pattern for idempotent writes to shared DB
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct TimeEntryOutbox {
    pub id: String,
    pub idempotency_key: String,
    pub user_id: String,
    pub payload_json: String,
    pub backend_cuid: Option<String>,
    pub status: OutboxStatus,
    pub attempts: i32,
    pub last_error: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub retry_after: Option<i64>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub sent_at: Option<i64>,
    pub correlation_id: Option<String>,
    pub local_status: Option<String>,
    pub remote_status: Option<String>,
    pub sap_entry_id: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub next_attempt_at: Option<i64>,
    pub error_code: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub last_forwarded_at: Option<i64>,
    pub wbs_code: Option<String>,
    pub target: String,
    pub description: Option<String>,
    pub auto_applied: bool,
    pub version: i32,
    pub last_modified_by: String,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub last_modified_at: Option<i64>,
}

/// Outbox entry status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
#[serde(rename_all = "lowercase")]
pub enum OutboxStatus {
    Pending,
    Sent,
    Failed,
    Dismissed,
}

crate::impl_domain_status_conversions!(OutboxStatus {
    Pending => "pending",
    Sent => "sent",
    Failed => "failed",
    Dismissed => "dismissed"
});

/// IdMapping - Maps local UUIDv7 to backend CUID
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct IdMapping {
    pub local_uuid: String,
    pub backend_cuid: String,
    pub entity_type: String,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub updated_at: i64,
}

/// PrismaTimeEntryDto - DTO that maps to Prisma TimeEntry model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct PrismaTimeEntryDto {
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "orgId")]
    pub org_id: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "entryDate")]
    pub entry_date: String,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    #[serde(rename = "durationMinutes")]
    pub duration_minutes: i32,
    #[serde(rename = "notes", skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(rename = "billable", skip_serializing_if = "Option::is_none")]
    pub billable: Option<bool>,
    #[serde(rename = "source")]
    pub source: String,
    #[serde(rename = "status", skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(rename = "startTime", skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(rename = "endTime", skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(rename = "durationSec", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub duration_sec: Option<i32>,
    #[serde(rename = "_displayProject", skip_serializing_if = "Option::is_none")]
    pub display_project: Option<String>,
    #[serde(rename = "_displayWorkstream", skip_serializing_if = "Option::is_none")]
    pub display_workstream: Option<String>,
    #[serde(rename = "_displayTask", skip_serializing_if = "Option::is_none")]
    pub display_task: Option<String>,
    #[serde(rename = "_confidence", skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(rename = "_contextBreakdown", skip_serializing_if = "Option::is_none")]
    pub context_breakdown: Option<Vec<ContextPart>>,
    #[serde(rename = "_wbsCode", skip_serializing_if = "Option::is_none")]
    pub wbs_code: Option<String>,
}

/// ContextPart - Per-app activity contribution for a time entry suggestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct ContextPart {
    pub app: String,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub duration_sec: i32,
    pub contribution: f32,
}

/// AcceptPatch - Partial update for editing time entry suggestions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct AcceptPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wbs_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub duration_sec: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_date: Option<String>,
}

/// Project - Minimal project info for frontend display
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct Project {
    pub id: String,
    pub name: String,
}

/// ProjectWithWbs - Project with optional WBS code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct ProjectWithWbs {
    pub id: String,
    pub name: String,
    pub wbs_code: Option<String>,
}

/// Row type for calendar_tokens table
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct CalendarTokenRow {
    pub id: String,
    pub token_ref: String,
    pub user_email: String,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub expires_at: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub updated_at: i64,
    pub provider: String,
}

/// Row type for calendar_sync_settings table
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct CalendarSyncSettingsRow {
    pub id: String,
    pub user_email: String,
    pub enabled: bool,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub sync_interval_minutes: u32,
    pub include_all_day_events: bool,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub min_event_duration_minutes: u32,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub lookback_hours: u32,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub lookahead_hours: u32,
    pub excluded_calendar_ids: String,
    pub sync_token: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub last_sync_epoch: Option<i64>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub updated_at: i64,
}

/// CalendarEventRow - Parsed calendar events stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct CalendarEventRow {
    pub id: String,
    pub google_event_id: String,
    pub user_email: String,
    pub summary: String,
    pub description: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub start_ts: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub end_ts: i64,
    pub is_all_day: bool,
    pub recurring_event_id: Option<String>,
    pub parsed_project: Option<String>,
    pub parsed_workstream: Option<String>,
    pub parsed_task: Option<String>,
    pub confidence_score: Option<f64>,
    pub meeting_platform: Option<String>,
    pub is_recurring_series: bool,
    pub is_online_meeting: bool,
    pub has_external_attendees: Option<bool>,
    pub organizer_email: Option<String>,
    pub organizer_domain: Option<String>,
    pub meeting_id: Option<String>,
    pub attendee_count: Option<i32>,
    pub external_attendee_count: Option<i32>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,
}

// ============================================================================
// Calendar Parameter Types (for insertion/updates)
// ============================================================================

/// Time range for calendar events.
///
/// # Invariants
/// - `end_ts` >= `start_ts`
/// - Timestamps are Unix seconds (not milliseconds)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct TimeRange {
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub start_ts: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub end_ts: i64,
    pub is_all_day: bool,
}

/// Parsed fields from calendar event title.
///
/// # Invariants
/// - `confidence_score`: Range [0.0, 1.0] if present
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct ParsedFields {
    pub project: Option<String>,
    pub workstream: Option<String>,
    pub task: Option<String>,
    pub confidence_score: Option<f64>,
}

/// Parameters for inserting calendar events.
///
/// # Field Invariants
/// - `when.start_ts`, `when.end_ts`: Unix timestamps in seconds (not
///   milliseconds)
/// - `when.end_ts` must be >= `when.start_ts`
/// - `parsed.confidence_score`: Parsing confidence in range [0.0, 1.0]
/// - `summary`: Event title from Google Calendar
/// - `description`: Optional event description
/// - `parsed.*`: Fields extracted by event title parser
/// - `meeting_platform`: Optional meeting platform ("zoom", "google_meet",
///   "teams", "phone")
/// - `is_recurring_series`: True if event is part of recurring series
/// - `is_online_meeting`: True if event has online meeting link
/// - `has_external_attendees`: True if non-company domains present in attendees
/// - `organizer_email`: Meeting organizer email address
/// - `organizer_domain`: Domain extracted from organizer email
/// - `meeting_id`: Google Meet / Teams meeting ID
/// - `attendee_count`: Total number of attendees
/// - `external_attendee_count`: Number of external attendees
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct CalendarEventParams {
    pub id: String,
    pub google_event_id: String,
    pub user_email: String,
    pub summary: String,
    pub description: Option<String>,
    pub when: TimeRange,
    pub recurring_event_id: Option<String>,
    pub parsed: ParsedFields,
    // FEATURE-029 Phase 4: Meeting platform detection
    pub meeting_platform: Option<String>,
    pub is_recurring_series: bool,
    pub is_online_meeting: bool,
    // FEATURE-033 Phase 4: Attendee metadata
    pub has_external_attendees: Option<bool>,
    pub organizer_email: Option<String>,
    pub organizer_domain: Option<String>,
    pub meeting_id: Option<String>,
    pub attendee_count: Option<i32>,
    pub external_attendee_count: Option<i32>,
}

/// Parameters for upserting calendar sync settings.
///
/// # Field Invariants
/// - `sync_interval_minutes`: Recommended range 5-1440 (5 min to 24 hours)
/// - `min_event_duration_minutes`: Minimum event duration to sync (typically
///   1-60)
/// - `lookback_hours`: How far back to sync events (e.g., 168 = 1 week)
/// - `lookahead_hours`: How far forward to sync events (e.g., 720 = 30 days)
/// - `excluded_calendar_ids`: Comma-separated calendar IDs to exclude
/// - `last_sync_epoch`: Unix timestamp in seconds (not milliseconds)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct CalendarSyncSettingsParams {
    pub id: String,
    pub user_email: String,
    pub enabled: bool,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub sync_interval_minutes: u32,
    pub include_all_day_events: bool,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub min_event_duration_minutes: u32,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub lookback_hours: u32,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub lookahead_hours: u32,
    pub excluded_calendar_ids: String,
    pub sync_token: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub last_sync_epoch: Option<i64>,
    pub idempotency_key: String,
}

/// Parameters for inserting suggestion feedback.
///
/// # Field Invariants
/// - `action`: One of 'accepted', 'dismissed', 'edited', 'restored'
/// - `edit_type`: Comma-separated field names (e.g., "project,duration")
/// - `source`: One of 'calendar', 'ai', 'unallocated'
/// - `confidence_before`: Range [0.0, 1.0]
/// - `context_json`: Valid JSON string (not validated at compile time)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct SuggestionFeedbackParams {
    pub id: String,
    pub outbox_id: String,
    pub action: String,
    pub reason: Option<String>,
    pub edit_type: Option<String>,
    pub confidence_before: Option<f32>,
    pub source: Option<String>,
    pub context_json: Option<String>,
}

// ============================================================================
// Database Statistics and Health DTOs
// ============================================================================

/// Database size information from PRAGMA introspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct DatabaseSize {
    /// Total database file size in bytes (from filesystem)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub size_bytes: u64,
    /// Number of pages in the database (PRAGMA page_count)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub page_count: u64,
    /// Size of each page in bytes (PRAGMA page_size, typically 4096)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub page_size: u64,
    /// Number of unused pages (PRAGMA freelist_count)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub freelist_count: u64,
}

/// Statistics for a single database table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct TableStats {
    /// Table name
    pub name: String,
    /// Number of rows in the table
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub row_count: u64,
}

/// Database health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct HealthStatus {
    /// Whether the database is healthy
    pub is_healthy: bool,
    /// Human-readable status message
    pub message: String,
    /// Response time for health check query (milliseconds)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub response_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        ActivityCategory, ActivityContext, ActivityMetadata, ConfidenceEvidence, WindowContext,
    };

    #[test]
    fn snapshot_round_trip_from_context() {
        let context = ActivityContext {
            active_app: WindowContext {
                app_name: "Safari".into(),
                window_title: "PulseArc Docs".into(),
                bundle_id: Some("com.apple.Safari".into()),
                url: Some("https://pulsearc.dev".into()),
                url_host: Some("pulsearc.dev".into()),
                document_name: None,
                file_path: None,
            },
            recent_apps: vec![],
            detected_activity: "Browsing".into(),
            work_type: None,
            activity_category: ActivityCategory::Communication,
            billable_confidence: 0.2,
            suggested_client: None,
            suggested_matter: None,
            suggested_task_code: None,
            extracted_metadata: ActivityMetadata::default(),
            evidence: ConfidenceEvidence::default(),
            calendar_event: None,
            location: None,
            temporal_context: None,
            classification: None,
        };

        let metadata = SnapshotMetadata {
            id: "snapshot-1".into(),
            timestamp: 1_700_000_000,
            created_at: 1_700_000_000,
            batch_id: None,
        };

        let snapshot = ActivitySnapshot::from_activity_context(&context, metadata).unwrap();
        assert_eq!(snapshot.detected_activity, "Browsing");
        assert_eq!(snapshot.primary_app, "com.apple.Safari");

        let round_trip = snapshot.activity_context().unwrap();
        assert_eq!(round_trip.active_app.app_name, "Safari");
        assert_eq!(round_trip.activity_category, ActivityCategory::Communication);
    }
}
