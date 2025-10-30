//! Activity tracking service - core business logic

use chrono::Utc;
use pulsearc_shared::{ActivitySnapshot, Result};
use std::sync::Arc;
use uuid::Uuid;

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
    pub async fn capture_activity(&self) -> Result<ActivitySnapshot> {
        // Get activity from provider
        let mut context = self.provider.get_activity().await?;

        // Enrich the context
        for enricher in &self.enrichers {
            enricher.enrich(&mut context).await?;
        }

        // Create snapshot
        let snapshot = ActivitySnapshot { id: Uuid::new_v4(), timestamp: Utc::now(), context };

        // Save to repository
        self.repository.save_snapshot(snapshot.clone()).await?;

        Ok(snapshot)
    }

    /// Check if tracking is paused
    pub fn is_paused(&self) -> bool {
        self.provider.is_paused()
    }

    /// Get snapshots within a time range
    pub async fn get_snapshots(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ActivitySnapshot>> {
        self.repository.get_snapshots(start, end).await
    }
}
