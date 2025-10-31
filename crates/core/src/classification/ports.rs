//! Port interfaces for activity classification

use async_trait::async_trait;
use pulsearc_domain::{ActivitySnapshot, Result, TimeEntry};

/// Trait for classifying activities into time entries
#[async_trait]
pub trait Classifier: Send + Sync {
    /// Classify a set of snapshots into a time entry
    async fn classify(&self, snapshots: Vec<ActivitySnapshot>) -> Result<TimeEntry>;
}

/// Trait for persisting classified time entries
#[async_trait]
pub trait TimeEntryRepository: Send + Sync {
    /// Save a time entry
    async fn save_entry(&self, entry: TimeEntry) -> Result<()>;

    /// Get time entries within a time range
    async fn get_entries(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<TimeEntry>>;

    /// Update an existing time entry
    async fn update_entry(&self, entry: TimeEntry) -> Result<()>;

    /// Delete a time entry
    async fn delete_entry(&self, id: uuid::Uuid) -> Result<()>;
}
