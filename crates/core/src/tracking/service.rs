//! Activity tracking service - core business logic

use std::sync::Arc;

use pulsearc_domain::types::database::{ActivitySnapshot, SnapshotMetadata};
use pulsearc_domain::{ActivityContext, Result};
use tokio::sync::Mutex;
use tracing::error;

use super::ports::{ActivityEnricher, ActivityProvider, ActivityRepository};

/// Shared, thread-safe activity provider
type SharedProvider = Arc<Mutex<Box<dyn ActivityProvider + Send + Sync>>>;

/// Activity tracking service
pub struct TrackingService {
    provider: SharedProvider,
    repository: Arc<dyn ActivityRepository>,
    enrichers: Vec<Arc<dyn ActivityEnricher>>,
    persist_captures: bool,
}

impl TrackingService {
    /// Create a new tracking service
    pub fn new<P>(provider: P, repository: Arc<dyn ActivityRepository>) -> Self
    where
        P: ActivityProvider + 'static,
    {
        Self {
            provider: Arc::new(Mutex::new(Box::new(provider))),
            repository,
            enrichers: Vec::new(),
            persist_captures: true,
        }
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
        let mut context = {
            let provider = self.provider.lock().await;
            provider.get_activity().await?
        };

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
    pub async fn is_paused(&self) -> bool {
        let provider = self.provider.lock().await;
        provider.is_paused()
    }

    /// Pause activity tracking via the provider.
    pub async fn pause(&self) -> Result<()> {
        let mut provider = self.provider.lock().await;
        provider.pause()
    }

    /// Resume activity tracking via the provider.
    pub async fn resume(&self) -> Result<()> {
        let mut provider = self.provider.lock().await;
        provider.resume()
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

    /// Save a manual time entry with a description
    ///
    /// Creates a manual activity snapshot with the provided description and
    /// persists it to the repository.
    ///
    /// # Arguments
    /// * `description` - Text description of the manual activity
    ///
    /// # Returns
    /// ID of the created activity snapshot
    pub async fn save_manual_entry(&self, description: &str) -> Result<String> {
        use pulsearc_domain::types::WindowContext;

        let metadata = SnapshotMetadata::now();

        // Create a manual window context with the description
        let window_context = WindowContext {
            app_name: "Manual Entry".to_string(),
            window_title: description.to_string(),
            bundle_id: Some("com.pulsearc.manual".to_string()),
            url: None,
            url_host: None,
            document_name: None,
            file_path: None,
        };

        // Create a minimal manual activity context
        let manual_context = ActivityContext {
            active_app: window_context,
            recent_apps: vec![],
            detected_activity: "Manual time entry".to_string(),
            work_type: None,
            activity_category: Default::default(),
            billable_confidence: 0.0,
            suggested_client: None,
            suggested_matter: None,
            suggested_task_code: None,
            extracted_metadata: Default::default(),
            evidence: Default::default(),
            calendar_event: None,
            location: None,
            temporal_context: None,
            classification: None,
        };

        // Create and save the snapshot
        let snapshot = ActivitySnapshot::from_activity_context(&manual_context, metadata)?;
        let snapshot_id = snapshot.id.clone();

        self.repository.save_snapshot(snapshot).await?;

        Ok(snapshot_id)
    }

    async fn persist_activity(&self, context: &ActivityContext) -> Result<()> {
        let metadata = SnapshotMetadata::now();
        let snapshot = ActivitySnapshot::from_activity_context(context, metadata)?;
        self.repository.save_snapshot(snapshot).await
    }
}
