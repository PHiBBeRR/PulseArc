//! Port interfaces for activity classification

use async_trait::async_trait;
use chrono::NaiveDate;
use pulsearc_domain::types::classification::{ContextSignals, ProjectMatch, ProposedBlock};
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

/// Trait for persisting proposed time blocks
#[async_trait]
pub trait BlockRepository: Send + Sync {
    /// Save a proposed block
    async fn save_proposed_block(&self, block: &ProposedBlock) -> Result<()>;

    /// Get proposed blocks for a specific date
    async fn get_proposed_blocks(&self, date: NaiveDate) -> Result<Vec<ProposedBlock>>;
}

/// Trait for matching activity signals to projects
///
/// Analyzes context signals extracted from activity snapshots and matches them
/// to known projects based on keywords, URLs, file paths, and other signals.
#[async_trait]
pub trait ProjectMatcher: Send + Sync {
    /// Match activity signals to a project
    ///
    /// # Arguments
    /// * `signals` - Context signals extracted from activity snapshots
    ///
    /// # Returns
    /// A ProjectMatch if signals match a known project, or None if no match found
    async fn match_project(&self, signals: &ContextSignals) -> Result<Option<ProjectMatch>>;
}
