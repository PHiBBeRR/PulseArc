//! Activity tracking service - core business logic

use std::sync::Arc;

use pulsearc_domain::{ActivityContext, Result};

use super::ports::{ActivityEnricher, ActivityProvider, ActivityRepository};

/// Activity tracking service
pub struct TrackingService {
    provider: Arc<dyn ActivityProvider>,
    repository: Arc<dyn ActivityRepository>,
    enrichers: Vec<Arc<dyn ActivityEnricher>>,
}

impl TrackingService {
    /// Create a new tracking service
    pub fn new(
        provider: Arc<dyn ActivityProvider>,
        repository: Arc<dyn ActivityRepository>,
    ) -> Self {
        Self { provider, repository, enrichers: Vec::new() }
    }

    /// Add an enricher to the service
    pub fn with_enricher(mut self, enricher: Arc<dyn ActivityEnricher>) -> Self {
        self.enrichers.push(enricher);
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
}
