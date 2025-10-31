//! Database model types
//!
//! These types represent the database schema and are used by repository ports.
//! During Phase 0, these are minimal definitions. Full migration will happen in Phase 1.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Activity snapshot - raw 30s activity capture
///
/// This is a minimal definition for Phase 0. Full type with all fields
/// will be migrated from legacy/api/src/db/models.rs in Phase 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySnapshot {
    pub id: String,
    pub timestamp: i64,
    pub activity_context_json: String,
    pub detected_activity: String,
    pub work_type: Option<String>,
    pub activity_category: Option<String>,
    pub primary_app: String,
    pub processed: bool,
    pub batch_id: Option<String>,
    pub created_at: i64,
    pub processed_at: Option<i64>,
    pub is_idle: bool,
    pub idle_duration_secs: Option<i32>,
}

/// Activity segment - 5-minute aggregated segment
///
/// This is a minimal definition for Phase 0. Full type with all fields
/// will be migrated from legacy/api/src/db/models.rs in Phase 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySegment {
    pub id: String,
    pub start_ts: i64,
    pub end_ts: i64,
    pub primary_app: String,
    pub normalized_label: String,
    pub sample_count: i32,
    pub dictionary_keys: Option<String>,
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
}
