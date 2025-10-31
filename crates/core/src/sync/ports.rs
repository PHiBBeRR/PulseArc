//! Port interfaces for sync operations

use async_trait::async_trait;
use pulsearc_domain::{Result, TimeEntryOutbox};

/// Trait for managing outbox queue operations
#[async_trait]
pub trait OutboxQueue: Send + Sync {
    /// Enqueue a time entry for sync
    async fn enqueue(&self, entry: &TimeEntryOutbox) -> Result<()>;

    /// Dequeue a batch of entries for processing
    async fn dequeue_batch(&self, limit: usize) -> Result<Vec<TimeEntryOutbox>>;

    /// Mark an entry as successfully sent
    async fn mark_sent(&self, id: &str) -> Result<()>;

    /// Mark an entry as failed with error message
    async fn mark_failed(&self, id: &str, error: &str) -> Result<()>;
}
