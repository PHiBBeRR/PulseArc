//! Command metrics port - tracks command execution metrics for Phase 4 validation
//!
//! This port enables tracking of command performance metrics (latency, error rates)
//! during the Phase 4 migration validation period. Metrics are used to compare
//! legacy vs new implementation performance and detect regressions.

use async_trait::async_trait;
use pulsearc_domain::Result;

/// Command execution record for metrics tracking
#[derive(Debug, Clone)]
pub struct CommandMetric {
    /// Unique ID for this metric record
    pub id: String,
    /// Command name (e.g., "database::get_database_stats")
    pub command: String,
    /// Implementation used ("legacy" or "new")
    pub implementation: String,
    /// Unix timestamp (seconds) when command was executed
    pub timestamp: i64,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Whether the command succeeded
    pub success: bool,
    /// Optional error type (e.g., "DatabaseError", "ValidationError")
    pub error_type: Option<String>,
}

/// Statistics for a command over a time range
#[derive(Debug, Clone)]
pub struct CommandStats {
    /// Command name
    pub command: String,
    /// Implementation ("legacy" or "new")
    pub implementation: String,
    /// Total invocations
    pub total_count: u64,
    /// Successful invocations
    pub success_count: u64,
    /// Failed invocations
    pub error_count: u64,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// P50 latency in milliseconds
    pub p50_latency_ms: u64,
    /// P95 latency in milliseconds
    pub p95_latency_ms: u64,
    /// P99 latency in milliseconds
    pub p99_latency_ms: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
}

/// Port trait for command metrics tracking
///
/// Implementations should be thread-safe and use async database operations
/// to avoid blocking command execution.
#[async_trait]
pub trait CommandMetricsPort: Send + Sync {
    /// Record a command execution
    ///
    /// This should be non-blocking and fast. Errors in metrics recording
    /// should not cause command execution to fail.
    async fn record_execution(&self, metric: CommandMetric) -> Result<()>;

    /// Get statistics for a specific command over a time range
    ///
    /// # Parameters
    /// - `command`: Command name to query
    /// - `implementation`: Optional filter by implementation ("legacy" or "new")
    /// - `start_ts`: Start of time range (Unix timestamp in seconds)
    /// - `end_ts`: End of time range (Unix timestamp in seconds)
    async fn get_stats(
        &self,
        command: &str,
        implementation: Option<&str>,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<CommandStats>;

    /// Get recent executions for debugging
    ///
    /// Returns the most recent N executions for a command, ordered by timestamp DESC.
    async fn get_recent_executions(
        &self,
        command: &str,
        limit: usize,
    ) -> Result<Vec<CommandMetric>>;

    /// Compare legacy vs new implementation performance
    ///
    /// Returns stats for both implementations side-by-side for easy comparison.
    async fn compare_implementations(
        &self,
        command: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<(CommandStats, CommandStats)>;

    /// Clean up old metrics (retention policy)
    ///
    /// Removes metrics older than the specified timestamp.
    /// Useful for keeping database size manageable.
    async fn cleanup_old_metrics(&self, older_than_ts: i64) -> Result<u64>;
}
