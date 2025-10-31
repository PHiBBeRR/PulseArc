//! Statistics types for all system components
//!
//! This module centralizes all statistics structs used across the application:
//! - Database statistics (snapshots, segments)
//! - Batch processing statistics
//! - Sync operation statistics
//! - Outbox queue statistics

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-gen")]
use ts_rs::TS;

/* -------------------------------------------------------------------------- */
/* Database Statistics */
/* -------------------------------------------------------------------------- */

/// Database and processing statistics
///
/// Provides counts for snapshots, segments, and batch queue status.
/// Primarily used by the UI for monitoring and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct DatabaseStats {
    /// Total number of activity snapshots in database
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub snapshot_count: i64,

    /// Number of unprocessed snapshots (not yet segmented)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub unprocessed_count: i64,

    /// Total number of activity segments in database
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub segment_count: i64,

    /// Batch queue processing statistics
    pub batch_stats: BatchStats,
}

/* -------------------------------------------------------------------------- */
/* Batch Processing Statistics */
/* -------------------------------------------------------------------------- */

/// Batch queue statistics
///
/// Tracks the status of AI batch processing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct BatchStats {
    /// Number of batches waiting to be processed
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub pending: i64,

    /// Number of batches currently being processed
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub processing: i64,

    /// Number of successfully completed batches
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub completed: i64,

    /// Number of failed batches (moved to DLQ)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub failed: i64,
}

/// Batch queue status summary (alias for BatchStats)
///
/// DEPRECATED: Use `BatchStats` instead. This type alias exists for backward
/// compatibility.
#[deprecated(since = "0.2.0", note = "Use `BatchStats` instead")]
pub type BatchQueueStatus = BatchStats;

/* -------------------------------------------------------------------------- */
/* Sync Statistics */
/* -------------------------------------------------------------------------- */

/// Sync statistics summary
///
/// Tracks backend synchronization operations and queue status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct SyncStats {
    /// Number of activity records in local database
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub local_activity_count: i64,

    /// Number of batches pending sync to backend
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub pending_batches: i64,

    /// Number of batches that failed to sync
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub failed_batches: i64,

    /// Unix timestamp of last successful sync (None if never synced)
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub last_sync_time: Option<i64>,
}

/* -------------------------------------------------------------------------- */
/* Outbox Statistics */
/* -------------------------------------------------------------------------- */

/// Outbox queue statistics
///
/// Tracks status of outbox entries awaiting backend sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct OutboxStats {
    /// Number of entries pending sync
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub pending: i64,

    /// Number of successfully sent entries
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub sent: i64,

    /// Number of failed entries (permanent failures)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub failed: i64,
}

/* -------------------------------------------------------------------------- */
/* DLQ (Dead Letter Queue) Types */
/* -------------------------------------------------------------------------- */

/// DLQ batch with full error details
///
/// Represents a batch that permanently failed processing and was moved to DLQ.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct DlqBatch {
    /// Unique batch identifier
    pub batch_id: String,

    /// Number of activities in the batch
    pub activity_count: i32,

    /// Error message from last failure
    pub error_message: Option<String>,

    /// Error code from last failure
    pub error_code: Option<String>,

    /// Unix timestamp when batch was created
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,

    /// Unix timestamp when batch permanently failed
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub failed_at: i64,

    /// Number of retry attempts before failure
    pub attempts: i32,
}

/* -------------------------------------------------------------------------- */
/* Token Usage & Cost Tracking */
/* -------------------------------------------------------------------------- */

/// Token usage tracking for AI classification costs.
///
/// # Field Invariants
/// - Token counts are u32 (max ~4.2 billion tokens per batch)
/// - Use u64 for aggregated totals (see UserCostSummary)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct TokenUsage {
    pub batch_id: String,
    pub user_id: String,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub input_tokens: u32,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub output_tokens: u32,
    pub estimated_cost_usd: f64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub timestamp: i64,
}

/// Aggregated user cost summary.
///
/// # Field Invariants
/// - Token totals use u64 to aggregate many u32 TokenUsage records
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct UserCostSummary {
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub batch_count: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub total_input_tokens: u64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
}

/// Classification mode for AI processing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub enum ClassificationMode {
    OpenAI,
    RulesOnly,
}

/// Token variance between estimated and actual usage
#[derive(Debug, Clone)]
pub struct TokenVariance {
    pub input_variance_pct: f64,
    pub output_variance_pct: f64,
}

/* -------------------------------------------------------------------------- */
/* Backwards Compatibility Aliases */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_stats_serialization() {
        let stats = DatabaseStats {
            snapshot_count: 100,
            unprocessed_count: 10,
            segment_count: 50,
            batch_stats: BatchStats { pending: 5, processing: 2, completed: 40, failed: 3 },
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("snapshot_count"));
        assert!(json.contains("batch_stats"));

        let deserialized: DatabaseStats = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.snapshot_count, 100);
        assert_eq!(deserialized.batch_stats.pending, 5);
    }

    #[test]
    fn test_batch_stats_conversion() {
        let status = BatchStats { pending: 10, processing: 2, completed: 50, failed: 3 };

        let stats: BatchStats = status.clone();
        assert_eq!(stats.pending, 10);
        assert_eq!(stats.completed, 50);

        let back: BatchStats = stats;
        assert_eq!(back.pending, 10);
        assert_eq!(back.failed, 3);
    }

    #[test]
    fn test_sync_stats() {
        let stats = SyncStats {
            local_activity_count: 1000,
            pending_batches: 5,
            failed_batches: 2,
            last_sync_time: Some(1634567890),
        };

        assert_eq!(stats.local_activity_count, 1000);
        assert_eq!(stats.last_sync_time, Some(1634567890));
    }

    #[test]
    fn test_outbox_stats() {
        let stats = OutboxStats { pending: 10, sent: 90, failed: 5 };

        assert_eq!(stats.pending, 10);
        assert_eq!(stats.sent, 90);
    }

    #[test]
    fn test_dlq_batch() {
        let batch = DlqBatch {
            batch_id: "batch-123".to_string(),
            activity_count: 50,
            error_message: Some("API timeout".to_string()),
            error_code: Some("HTTP_TIMEOUT".to_string()),
            created_at: 1634567890,
            failed_at: 1634571490,
            attempts: 3,
        };

        assert_eq!(batch.batch_id, "batch-123");
        assert_eq!(batch.attempts, 3);
        assert!(batch.error_message.is_some());
    }
}
