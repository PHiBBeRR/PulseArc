//! Port interfaces for batch processing operations

use std::time::Duration;

use async_trait::async_trait;
use pulsearc_domain::{BatchQueue, BatchStats, BatchStatus, DlqBatch, Result};

/// Trait for managing batch queue operations
#[async_trait]
pub trait BatchRepository: Send + Sync {
    // Core CRUD
    /// Save a batch to the queue
    async fn save_batch(&self, batch: &BatchQueue) -> Result<()>;

    /// Get a batch by its ID
    async fn get_batch(&self, batch_id: &str) -> Result<BatchQueue>;

    /// Update the status of a batch
    async fn update_batch_status(&self, batch_id: &str, status: BatchStatus) -> Result<()>;

    // Lease management
    /// Acquire a lease on a batch for processing
    async fn acquire_batch_lease(
        &self,
        batch_id: &str,
        worker_id: &str,
        duration: Duration,
    ) -> Result<()>;

    /// Renew an existing batch lease
    async fn renew_batch_lease(
        &self,
        batch_id: &str,
        worker_id: &str,
        duration: Duration,
    ) -> Result<()>;

    /// Get batches with stale leases (expired)
    async fn get_stale_leases(&self, ttl_secs: i64) -> Result<Vec<BatchQueue>>;

    /// Recover batches with stale leases (reset to pending)
    async fn recover_stale_leases(&self) -> Result<Vec<String>>;

    // Lifecycle
    /// Create a new batch from unprocessed snapshots
    async fn create_batch_from_unprocessed(
        &self,
        max_snapshots: usize,
        worker_id: &str,
        lease_duration_secs: i64,
    ) -> Result<Option<(String, Vec<String>)>>;

    /// Mark a batch as completed
    async fn complete_batch(&self, batch_id: &str) -> Result<()>;

    /// Mark a batch as failed with error message
    async fn mark_batch_failed(&self, batch_id: &str, error: &str) -> Result<()>;

    // Queries
    /// Get batches by status
    async fn get_batches_by_status(&self, status: BatchStatus) -> Result<Vec<BatchQueue>>;

    /// Get batch processing statistics
    async fn get_batch_stats(&self) -> Result<BatchStats>;

    /// Get all pending batches
    async fn get_pending_batches(&self) -> Result<Vec<BatchQueue>>;

    // Cleanup
    /// Clean up old batches (older than specified seconds)
    async fn cleanup_old_batches(&self, older_than_seconds: i64) -> Result<usize>;

    /// Delete a batch
    async fn delete_batch(&self, batch_id: &str) -> Result<()>;
}

/// Trait for managing Dead Letter Queue (DLQ) operations
#[async_trait]
pub trait DlqRepository: Send + Sync {
    /// Move a failed batch to the DLQ
    async fn move_batch_to_dlq(&self, batch_id: &str, error: &str) -> Result<()>;

    /// Get all batches in the DLQ
    async fn get_dlq_batches(&self) -> Result<Vec<BatchQueue>>;

    /// Get DLQ batches with full error details
    async fn get_dlq_batches_with_details(&self) -> Result<Vec<DlqBatch>>;

    /// Reset a batch for retry (move from DLQ back to pending)
    async fn reset_batch_for_retry(&self, batch_id: &str) -> Result<()>;

    /// Retry a failed batch from the DLQ
    async fn retry_failed_batch(&self, batch_id: &str) -> Result<()>;
}
