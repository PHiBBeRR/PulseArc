//! Adapter to convert new granular DatabaseStatsPort → legacy DatabaseStats
//! aggregate.
//!
//! This adapter bridges the gap between the new hexagonal architecture (with
//! granular DatabaseStatsPort methods) and the legacy monolithic DatabaseStats
//! type expected by the frontend. It fans out to multiple port methods and
//! aggregates the results.

use std::sync::Arc;

use pulsearc_core::DatabaseStatsPort;
use pulsearc_domain::types::stats::{BatchStats, DatabaseStats};
use pulsearc_domain::Result;

/// Build legacy DatabaseStats aggregate from new granular DatabaseStatsPort.
///
/// This adapter calls multiple port methods and combines the results into the
/// legacy DatabaseStats format. The `batch_stats` field is left empty with a
/// TODO note, as the frontend does not currently use this field.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
///
/// use pulsearc_app::adapters::database_stats::build_database_stats;
/// use pulsearc_core::DatabaseStatsPort;
///
/// async fn get_stats(port: &Arc<dyn DatabaseStatsPort + Send + Sync>) {
///     let stats = build_database_stats(port).await.unwrap();
///     println!("Total snapshots: {}", stats.snapshot_count);
///     println!("Unprocessed: {}", stats.unprocessed_count);
/// }
/// ```
pub async fn build_database_stats(
    port: &Arc<dyn DatabaseStatsPort + Send + Sync>,
) -> Result<DatabaseStats> {
    // Get table stats from port
    let table_stats = port.get_table_stats().await?;

    // Extract snapshot count from table stats
    let snapshot_count = table_stats
        .iter()
        .find(|t| t.name == "activity_snapshots")
        .map(|t| t.row_count as i64)
        .unwrap_or(0);

    // Extract segment count from table stats
    let segment_count = table_stats
        .iter()
        .find(|t| t.name == "activity_segments")
        .map(|t| t.row_count as i64)
        .unwrap_or(0);

    // Get unprocessed count via new port method
    let unprocessed_count = port.get_unprocessed_count().await?;

    // batch_stats: Frontend does NOT use this field (verified in Phase 4A.1
    // investigation) Return empty struct for now. Future work: populate from
    // SqlCipherBatchRepository.get_batch_stats() when batch monitoring UI is
    // added.
    let batch_stats = BatchStats { pending: 0, processing: 0, completed: 0, failed: 0 };

    Ok(DatabaseStats { snapshot_count, unprocessed_count, segment_count, batch_stats })
}
