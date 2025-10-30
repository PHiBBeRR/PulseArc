//! Classification service - core business logic

use std::sync::Arc;

use pulsearc_shared::{ActivitySnapshot, Result, TimeEntry};

use super::ports::{Classifier, TimeEntryRepository};

/// Classification service for converting snapshots to time entries
pub struct ClassificationService {
    classifier: Arc<dyn Classifier>,
    repository: Arc<dyn TimeEntryRepository>,
}

impl ClassificationService {
    /// Create a new classification service
    pub fn new(classifier: Arc<dyn Classifier>, repository: Arc<dyn TimeEntryRepository>) -> Self {
        Self { classifier, repository }
    }

    /// Classify snapshots into a time entry and save it
    pub async fn classify_and_save(&self, snapshots: Vec<ActivitySnapshot>) -> Result<TimeEntry> {
        // Classify the snapshots
        let entry = self.classifier.classify(snapshots).await?;

        // Save the entry
        self.repository.save_entry(entry.clone()).await?;

        Ok(entry)
    }

    /// Get time entries within a time range
    pub async fn get_entries(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<TimeEntry>> {
        self.repository.get_entries(start, end).await
    }

    /// Update an existing time entry
    pub async fn update_entry(&self, entry: TimeEntry) -> Result<()> {
        self.repository.update_entry(entry).await
    }

    /// Delete a time entry
    pub async fn delete_entry(&self, id: uuid::Uuid) -> Result<()> {
        self.repository.delete_entry(id).await
    }
}
