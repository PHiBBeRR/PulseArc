//! Activity tracking service - core business logic

use std::sync::Arc;

use pulsearc_domain::types::database::{ActivitySnapshot, SnapshotMetadata};
use pulsearc_domain::{ActivityContext, Result};
use tracing::error;

use super::ports::{ActivityEnricher, ActivityProvider, ActivityRepository};

/// Activity tracking service
pub struct TrackingService {
    provider: Arc<dyn ActivityProvider>,
    repository: Arc<dyn ActivityRepository>,
    enrichers: Vec<Arc<dyn ActivityEnricher>>,
    persist_captures: bool,
}

impl TrackingService {
    /// Create a new tracking service
    pub fn new(
        provider: Arc<dyn ActivityProvider>,
        repository: Arc<dyn ActivityRepository>,
    ) -> Self {
        Self { provider, repository, enrichers: Vec::new(), persist_captures: true }
    }

    /// Add an enricher to the service
    pub fn with_enricher(mut self, enricher: Arc<dyn ActivityEnricher>) -> Self {
        self.enrichers.push(enricher);
        self
    }

    /// Configure whether captured activities should be persisted immediately.
    ///
    /// Persistence is enabled by default to mirror legacy behaviour. This
    /// builder makes it easy for tests or specialised workflows to opt out.
    pub fn with_persistence(mut self, enabled: bool) -> Self {
        self.persist_captures = enabled;
        self
    }

    /// Capture and save the current activity
    ///
    /// PHASE-0: Returns ActivityContext instead of ActivitySnapshot
    /// Snapshot creation happens in infra layer for proper type compatibility
    pub async fn capture_activity(&self) -> Result<ActivityContext> {
        // Get activity from provider
        let mut context = self.provider.get_activity().await?;

        // Enrich the context
        for enricher in &self.enrichers {
            enricher.enrich(&mut context).await?;
        }

        if self.persist_captures {
            if let Err(err) = self.persist_activity(&context).await {
                error!(error = %err, "Failed to persist captured activity snapshot");
            }
        }

        // Return enriched context - snapshot creation happens in infra layer
        Ok(context)
    }

    /// Check if tracking is paused
    pub fn is_paused(&self) -> bool {
        self.provider.is_paused()
    }

    /// Get snapshots within a time range
    ///
    /// PHASE-0: Uses database::ActivitySnapshot
    pub async fn get_snapshots(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<pulsearc_domain::ActivitySnapshot>> {
        self.repository.get_snapshots(start, end).await
    }

    async fn persist_activity(&self, context: &ActivityContext) -> Result<()> {
        let metadata = SnapshotMetadata::now();
        let snapshot = ActivitySnapshot::from_activity_context(context, metadata)?;
        self.repository.save_snapshot(snapshot).await
    }
}
