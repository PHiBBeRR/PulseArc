//! Idle time tracking types
//!
//! FEATURE-028: Types for tracking and managing idle periods

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-gen")]
use ts_rs::TS;

/// IdlePeriod - Represents a detected idle time period
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct IdlePeriod {
    pub id: String, // UUIDv7
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub start_ts: i64, // Unix timestamp when idle started
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub end_ts: i64, // Unix timestamp when idle ended
    pub duration_secs: i32, // Duration in seconds
    pub system_trigger: String, // 'threshold' | 'lock_screen' | 'sleep' | 'manual'
    pub user_action: Option<String>, // 'kept' | 'discarded' | 'pending' | 'auto_excluded'
    pub threshold_secs: i32, // Idle threshold at detection time (e.g., 300s)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64, // When the period was created
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub reviewed_at: Option<i64>, // When user made decision
    pub notes: Option<String>, // User notes if manually overridden
}

/// IdleSummary - Aggregated idle time statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct IdleSummary {
    pub total_active_secs: i32,   // Total active time
    pub total_idle_secs: i32,     // Total idle time
    pub idle_periods_count: i32,  // Number of idle periods
    pub idle_kept_secs: i32,      // Idle time user chose to keep
    pub idle_discarded_secs: i32, // Idle time user chose to discard
    pub idle_pending_secs: i32,   // Idle time awaiting user decision
}
