//! Calendar integration type definitions
//!
//! Shared types for calendar events, sync settings, and connection status.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-gen")]
use ts_rs::TS;

/// Calendar event from provider API (Google or Microsoft)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export, rename_all = "camelCase"))]
pub struct CalendarEvent {
    pub id: String,
    #[cfg_attr(feature = "ts-gen", ts(type = "string", optional))]
    pub summary: Option<String>,
    pub description: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub start: i64, // Unix timestamp (seconds)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub end: i64,
    pub calendar_id: String,
    pub is_all_day: bool,
    pub recurring_event_id: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub original_start_time: Option<i64>,
    pub parsed_project: Option<String>,
    pub parsed_workstream: Option<String>,
    pub parsed_task: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub parsed_confidence: f32,
    pub meeting_platform: Option<String>,
    pub is_recurring_series: bool,
    pub is_online_meeting: bool,
    pub has_external_attendees: Option<bool>,
    pub organizer_email: Option<String>,
    pub organizer_domain: Option<String>,
    pub meeting_id: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub attendee_count: Option<i32>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub external_attendee_count: Option<i32>,
    pub attendees: Vec<String>,
}

/// Calendar sync settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export, rename_all = "camelCase"))]
pub struct CalendarSyncSettings {
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
    pub excluded_calendar_ids: Vec<String>,
    pub sync_token: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub last_sync_epoch: Option<i64>,
}

/// OAuth connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export, rename_all = "camelCase"))]
pub struct CalendarConnectionStatus {
    pub provider: String,
    pub connected: bool,
    pub email: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub last_sync: Option<i64>,
    pub sync_enabled: bool,
}

/// Timeline calendar event used for timeline visualisations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export, rename_all = "camelCase"))]
pub struct TimelineCalendarEvent {
    pub id: String,
    pub project: String,
    pub task: String,
    pub start_time: String,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub start_epoch: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub duration: i64,
    pub status: String,
    pub is_calendar_event: bool,
    pub is_all_day: bool,
    pub original_summary: String,
}
