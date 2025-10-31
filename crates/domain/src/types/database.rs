//! Database model types
//!
//! These types represent the database schema and are used by repository ports.
//! Phase 1 migration includes all types from legacy/api/src/db/models.rs

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-gen")]
use ts_rs::TS;

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
