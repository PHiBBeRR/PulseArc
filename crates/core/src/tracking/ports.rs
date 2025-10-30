//! Port interfaces for activity tracking
//!
//! These traits define the boundaries between core business logic
//! and infrastructure implementations.

use async_trait::async_trait;
use pulsearc_shared::{ActivityContext, ActivitySnapshot, Result};

/// Trait for capturing activity from the operating system
#[async_trait]
pub trait ActivityProvider: Send + Sync {
    /// Get the current activity context
    async fn get_activity(&self) -> Result<ActivityContext>;

    /// Check if tracking is paused
    fn is_paused(&self) -> bool;

    /// Pause activity tracking
    fn pause(&mut self) -> Result<()>;

    /// Resume activity tracking
    fn resume(&mut self) -> Result<()>;
}

/// Trait for persisting activity snapshots
#[async_trait]
pub trait ActivityRepository: Send + Sync {
    /// Save an activity snapshot
    async fn save_snapshot(&self, snapshot: ActivitySnapshot) -> Result<()>;

    /// Get snapshots within a time range
    async fn get_snapshots(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ActivitySnapshot>>;

    /// Delete snapshots older than the specified date
    async fn delete_old_snapshots(&self, before: chrono::DateTime<chrono::Utc>) -> Result<usize>;
}

/// Trait for enriching activity context with additional metadata
#[async_trait]
pub trait ActivityEnricher: Send + Sync {
    /// Enrich an activity context with additional information
    async fn enrich(&self, context: &mut ActivityContext) -> Result<()>;
}
