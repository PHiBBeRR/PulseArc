//! Port interfaces for sync operations

use async_trait::async_trait;
use pulsearc_domain::{IdMapping, Result, TimeEntryOutbox, TokenUsage};

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

/// Trait for managing ID mappings between local and backend systems
#[async_trait]
pub trait IdMappingRepository: Send + Sync {
    /// Create a new ID mapping
    async fn create_id_mapping(&self, mapping: &IdMapping) -> Result<()>;

    /// Get ID mapping by local UUID
    async fn get_id_mapping_by_local_uuid(&self, uuid: &str) -> Result<Option<IdMapping>>;

    /// Get backend CUID by local UUID
    async fn get_backend_cuid_by_local_uuid(&self, uuid: &str) -> Result<Option<String>>;

    /// Get local UUID by backend CUID
    async fn get_local_uuid_by_backend_cuid(&self, cuid: &str) -> Result<Option<String>>;

    /// Get all ID mappings for a specific entity type
    async fn get_id_mappings_by_entity_type(&self, entity_type: &str) -> Result<Vec<IdMapping>>;
}

/// Trait for tracking AI token usage and costs
#[async_trait]
pub trait TokenUsageRepository: Send + Sync {
    /// Record token usage for a batch
    async fn record_token_usage(&self, usage: &TokenUsage) -> Result<()>;

    /// Get token usage for a specific batch
    async fn get_token_usage_by_batch(&self, batch_id: &str) -> Result<TokenUsage>;

    /// Record estimated token usage (before API call)
    async fn record_estimated_usage(&self, usage: &TokenUsage) -> Result<()>;

    /// Record actual token usage (after API call)
    async fn record_actual_usage(&self, usage: &TokenUsage) -> Result<()>;

    /// Delete token usage records for a batch
    async fn delete_token_usage_by_batch(&self, batch_id: &str) -> Result<usize>;
}
