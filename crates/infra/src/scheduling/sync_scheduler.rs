//! Sync scheduler for periodic outbox processing.
//!
//! **STATUS: PENDING** - Awaiting repository implementations.
//!
//! Provides interval-based synchronization scheduler for API operations with
//! lifecycle management. Processes pending segments and snapshots from the
//! outbox and forwards them to the API.
//!
//! Always compiled (not feature-gated).
//!
//! # Pending Dependencies
//!
//! This scheduler requires segment and snapshot repository implementations
//! that provide `get_pending_for_sync()` and `mark_synced()` operations.
//! These repositories are planned but not yet implemented in Phase 3.
//!
//! Placeholder traits are defined below. When proper repositories land in
//! `crates/infra/src/repositories/`, these traits should be replaced with
//! the actual repository ports from `pulsearc-core`.
//!
//! **TODO**: Replace placeholder traits with actual repositories once
//! available. **Tracked in**: Phase 3D follow-up work
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! use pulsearc_infra::observability::metrics::PerformanceMetrics;
//! use pulsearc_infra::scheduling::{SyncScheduler, SyncSchedulerConfig};
//!
//! # async fn example() -> Result<(), String> {
//! let metrics = Arc::new(PerformanceMetrics::new());
//! // ... create forwarder and segment/snapshot repos ...
//! # let forwarder = todo!();
//! # let segment_repo = todo!();
//! # let snapshot_repo = todo!();
//! let mut scheduler = SyncScheduler::new(
//!     forwarder,
//!     segment_repo,
//!     snapshot_repo,
//!     SyncSchedulerConfig {
//!         interval: Duration::from_secs(900), // 15 minutes
//!         batch_size: 50,
//!     },
//!     metrics,
//! );
//!
//! scheduler.start().await?;
//! // ... application runs ...
//! scheduler.stop().await?;
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};

// TODO: Remove these placeholder traits when repositories module is implemented
use async_trait::async_trait;
use pulsearc_domain::types::{ActivitySegment, ActivitySnapshot};
use pulsearc_domain::PulseArcError;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

use crate::api::forwarder::ApiForwarder;
use crate::observability::metrics::PerformanceMetrics;
use crate::observability::MetricsResult;
use crate::scheduling::error::{SchedulerError, SchedulerResult};

// =============================================================================
// PLACEHOLDER TRAITS - TODO: REPLACE WITH ACTUAL REPOSITORIES
// =============================================================================
//
// These traits are temporary placeholders until proper repository
// implementations are available. They define the minimum interface needed for
// sync operations.
//
// When repositories are implemented:
// 1. Remove these placeholder traits
// 2. Import actual repository traits from pulsearc-core/tracking/ports
// 3. Update SyncScheduler to use Arc<dyn ActualRepositoryTrait>
// 4. Ensure repositories provide get_pending_for_sync() and mark_synced()
//    methods
//
// Tracked in: Phase 3D follow-up (repository implementations)
// =============================================================================

/// Placeholder trait for segment repository operations.
///
/// **TODO**: Replace with actual `SegmentRepository` port from pulsearc-core
/// once repository implementations land in Phase 3.
#[allow(dead_code)]
#[async_trait]
pub trait ActivitySegmentRepository: Send + Sync {
    /// Get pending segments for sync
    async fn get_pending_for_sync(
        &self,
        batch_size: usize,
    ) -> Result<Vec<ActivitySegment>, PulseArcError>;
    /// Mark segment as synced
    async fn mark_synced(&self, id: &str) -> Result<(), PulseArcError>;
}

/// Placeholder trait for snapshot repository operations.
///
/// **TODO**: Replace with actual `SnapshotRepository` port from pulsearc-core
/// once repository implementations land in Phase 3.
#[allow(dead_code)]
#[async_trait]
pub trait ActivitySnapshotRepository: Send + Sync {
    /// Get pending snapshots for sync
    async fn get_pending_for_sync(
        &self,
        batch_size: usize,
    ) -> Result<Vec<ActivitySnapshot>, PulseArcError>;
    /// Mark snapshot as synced
    async fn mark_synced(&self, id: &str) -> Result<(), PulseArcError>;
}

/// Type alias for task handle to avoid complexity warnings
type TaskHandle = Arc<Mutex<Option<JoinHandle<()>>>>;

/// Configuration for sync scheduler
#[derive(Debug, Clone)]
pub struct SyncSchedulerConfig {
    /// Sync interval
    pub interval: Duration,
    /// Maximum number of items to process per batch
    pub batch_size: usize,
    /// Timeout for repository operations
    pub repo_timeout: Duration,
    /// Timeout for forwarding operations
    pub forward_timeout: Duration,
    /// Timeout for mark_synced operations
    pub mark_synced_timeout: Duration,
}

impl Default for SyncSchedulerConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(900), // 15 minutes
            batch_size: 50,
            repo_timeout: Duration::from_secs(30), // 30 seconds
            forward_timeout: Duration::from_secs(60), // 60 seconds
            mark_synced_timeout: Duration::from_secs(10), // 10 seconds
        }
    }
}

/// Context for sync loop to avoid too many arguments (clippy)
struct SyncLoopContext {
    forwarder: Arc<ApiForwarder>,
    segment_repo: Arc<dyn ActivitySegmentRepository>,
    snapshot_repo: Arc<dyn ActivitySnapshotRepository>,
    metrics: Arc<PerformanceMetrics>,
}

/// Sync scheduler for periodic outbox processing
pub struct SyncScheduler {
    forwarder: Arc<ApiForwarder>,
    segment_repo: Arc<dyn ActivitySegmentRepository>,
    snapshot_repo: Arc<dyn ActivitySnapshotRepository>,
    config: SyncSchedulerConfig,
    cancellation_token: CancellationToken,
    task_handle: TaskHandle,
    metrics: Arc<PerformanceMetrics>,
}

impl SyncScheduler {
    /// Create a new sync scheduler
    ///
    /// # Arguments
    ///
    /// * `forwarder` - API forwarder
    /// * `segment_repo` - Activity segment repository
    /// * `snapshot_repo` - Activity snapshot repository
    /// * `config` - Scheduler configuration
    /// * `metrics` - Performance metrics
    pub fn new(
        forwarder: Arc<ApiForwarder>,
        segment_repo: Arc<dyn ActivitySegmentRepository>,
        snapshot_repo: Arc<dyn ActivitySnapshotRepository>,
        config: SyncSchedulerConfig,
        metrics: Arc<PerformanceMetrics>,
    ) -> Self {
        Self {
            forwarder,
            segment_repo,
            snapshot_repo,
            config,
            cancellation_token: CancellationToken::new(),
            task_handle: Arc::new(Mutex::new(None)),
            metrics,
        }
    }

    /// Start the scheduler
    ///
    /// Spawns a background task that runs sync periodically.
    ///
    /// # Errors
    ///
    /// Returns error if scheduler is already running
    #[instrument(skip(self))]
    pub async fn start(&mut self) -> SchedulerResult<()> {
        if self.is_running() {
            return Err(SchedulerError::AlreadyRunning);
        }

        info!("Starting sync scheduler");

        // Create a new cancellation token (supports restart after stop)
        self.cancellation_token = CancellationToken::new();

        let context = SyncLoopContext {
            forwarder: Arc::clone(&self.forwarder),
            segment_repo: Arc::clone(&self.segment_repo),
            snapshot_repo: Arc::clone(&self.snapshot_repo),
            metrics: Arc::clone(&self.metrics),
        };
        let config = self.config.clone();
        let cancel = self.cancellation_token.clone();

        let handle = tokio::spawn(async move {
            Self::sync_loop(context, config, cancel).await;
        });

        *self.task_handle.lock().await = Some(handle);

        info!("Sync scheduler started");
        log_metric(self.metrics.record_call(), "scheduler.sync.start");

        Ok(())
    }

    /// Stop the scheduler gracefully
    ///
    /// Cancels the background task and awaits completion.
    ///
    /// # Errors
    ///
    /// Returns error if scheduler is not running
    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> SchedulerResult<()> {
        if !self.is_running() {
            return Err(SchedulerError::NotRunning);
        }

        info!("Stopping sync scheduler");

        // Cancel background task
        self.cancellation_token.cancel();

        // Await handle with timeout
        if let Some(handle) = self.task_handle.lock().await.take() {
            let join_timeout = Duration::from_secs(5);
            tokio::time::timeout(join_timeout, handle)
                .await
                .map_err(|source| SchedulerError::Timeout { duration: join_timeout, source })??;
        }

        info!("Sync scheduler stopped");
        log_metric(self.metrics.record_call(), "scheduler.sync.stop");

        Ok(())
    }

    /// Check if scheduler is running
    ///
    /// A scheduler is considered running if it has an active task handle that
    /// hasn't finished.
    pub fn is_running(&self) -> bool {
        self.task_handle
            .try_lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|h| !h.is_finished()))
            .unwrap_or(false)
    }

    /// Background sync loop
    async fn sync_loop(
        context: SyncLoopContext,
        config: SyncSchedulerConfig,
        cancel: CancellationToken,
    ) {
        let SyncLoopContext { forwarder, segment_repo, snapshot_repo, metrics } = context;
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!("Sync loop cancelled");
                    break;
                }
                _ = tokio::time::sleep(config.interval) => {
                    log_metric(metrics.record_call(), "scheduler.sync.tick");
                    let started = Instant::now();

                    // Process segments
                    if let Err(e) = Self::process_segments(
                        &forwarder,
                        &segment_repo,
                        &config,
                        &metrics,
                    ).await {
                        error!(error = %e, "Failed to process segment batch");
                        log_metric(metrics.record_fetch_error(), "scheduler.sync.segments.error");
                    }

                    // Process snapshots
                    if let Err(e) = Self::process_snapshots(
                        &forwarder,
                        &snapshot_repo,
                        &config,
                        &metrics,
                    ).await {
                        error!(error = %e, "Failed to process snapshot batch");
                        log_metric(metrics.record_fetch_error(), "scheduler.sync.snapshots.error");
                    }

                    log_metric(
                        metrics.record_fetch_time(started.elapsed()),
                        "scheduler.sync.duration",
                    );
                }
            }
        }
    }

    async fn process_segments(
        forwarder: &Arc<ApiForwarder>,
        segment_repo: &Arc<dyn ActivitySegmentRepository>,
        config: &SyncSchedulerConfig,
        metrics: &Arc<PerformanceMetrics>,
    ) -> SchedulerResult<()> {
        // Fetch pending segments from repository with timeout
        let segments = tokio::time::timeout(
            config.repo_timeout,
            segment_repo.get_pending_for_sync(config.batch_size),
        )
        .await
        .map_err(|source| SchedulerError::Timeout { duration: config.repo_timeout, source })?
        .map_err(|source| SchedulerError::RepositoryError {
            operation: "fetch_pending_segments".to_string(),
            source: Box::new(source),
        })?;

        if segments.is_empty() {
            debug!("No pending segments to sync");
            return Ok(());
        }

        info!(count = segments.len(), "Processing segment batch");

        // Forward to API with timeout
        let result = tokio::time::timeout(
            config.forward_timeout,
            forwarder.forward_segments(segments.clone()),
        )
        .await
        .map_err(|source| SchedulerError::Timeout { duration: config.forward_timeout, source })?
        .map_err(|source| SchedulerError::ForwardingError {
            item_type: "segment".to_string(),
            source: Box::new(source),
        })?;

        info!(
            submitted = result.submitted,
            failed = result.failed,
            "Segment batch forwarding completed"
        );

        // Mark forwarded segments with timeout
        for (idx, segment) in segments.iter().enumerate() {
            // Check if this segment failed
            let failed = result.errors.iter().any(|(err_idx, _)| *err_idx == idx);

            if !failed {
                if let Err(e) = tokio::time::timeout(
                    config.mark_synced_timeout,
                    segment_repo.mark_synced(&segment.id),
                )
                .await
                {
                    warn!(id = %segment.id, error = ?e, "Failed to mark segment as synced");
                }
            }
        }

        log_metric(metrics.record_call(), "scheduler.sync.segments.processed");
        Ok(())
    }

    async fn process_snapshots(
        forwarder: &Arc<ApiForwarder>,
        snapshot_repo: &Arc<dyn ActivitySnapshotRepository>,
        config: &SyncSchedulerConfig,
        metrics: &Arc<PerformanceMetrics>,
    ) -> SchedulerResult<()> {
        // Fetch pending snapshots from repository with timeout
        let snapshots = tokio::time::timeout(
            config.repo_timeout,
            snapshot_repo.get_pending_for_sync(config.batch_size),
        )
        .await
        .map_err(|source| SchedulerError::Timeout { duration: config.repo_timeout, source })?
        .map_err(|source| SchedulerError::RepositoryError {
            operation: "fetch_pending_snapshots".to_string(),
            source: Box::new(source),
        })?;

        if snapshots.is_empty() {
            debug!("No pending snapshots to sync");
            return Ok(());
        }

        info!(count = snapshots.len(), "Processing snapshot batch");

        // Forward to API with timeout
        let result = tokio::time::timeout(
            config.forward_timeout,
            forwarder.forward_snapshots(snapshots.clone()),
        )
        .await
        .map_err(|source| SchedulerError::Timeout { duration: config.forward_timeout, source })?
        .map_err(|source| SchedulerError::ForwardingError {
            item_type: "snapshot".to_string(),
            source: Box::new(source),
        })?;

        info!(
            submitted = result.submitted,
            failed = result.failed,
            "Snapshot batch forwarding completed"
        );

        // Mark forwarded snapshots with timeout
        for (idx, snapshot) in snapshots.iter().enumerate() {
            // Check if this snapshot failed
            let failed = result.errors.iter().any(|(err_idx, _)| *err_idx == idx);

            if !failed {
                if let Err(e) = tokio::time::timeout(
                    config.mark_synced_timeout,
                    snapshot_repo.mark_synced(&snapshot.id),
                )
                .await
                {
                    warn!(id = %snapshot.id, error = ?e, "Failed to mark snapshot as synced");
                }
            }
        }

        log_metric(metrics.record_call(), "scheduler.sync.snapshots.processed");
        Ok(())
    }
}

fn log_metric(result: MetricsResult<()>, metric: &'static str) {
    if let Err(err) = result {
        warn!(metric = metric, error = ?err, "Failed to record scheduler metric");
    }
}

/// Ensure scheduler is stopped when dropped
impl Drop for SyncScheduler {
    fn drop(&mut self) {
        // Note: Can't check task_handle (async), so check if token is not cancelled
        // This is best-effort cleanup in Drop
        if !self.cancellation_token.is_cancelled() {
            warn!("SyncScheduler dropped while running; cancelling");
            self.cancellation_token.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use pulsearc_domain::{ActivitySegment, ActivitySnapshot};

    use super::*;
    use crate::api::auth::AccessTokenProvider;
    use crate::api::client::{ApiClient, ApiClientConfig};
    use crate::api::commands::ApiCommands;
    use crate::api::errors::ApiError;
    use crate::api::forwarder::ForwarderConfig;

    #[derive(Clone)]
    struct MockAuthProvider;

    #[async_trait]
    impl AccessTokenProvider for MockAuthProvider {
        async fn access_token(&self) -> Result<String, ApiError> {
            Ok("test-token".to_string())
        }
    }

    // Mock segment repository
    struct MockSegmentRepo {
        call_count: Arc<AtomicUsize>,
    }

    impl MockSegmentRepo {
        fn new() -> Self {
            Self { call_count: Arc::new(AtomicUsize::new(0)) }
        }
    }

    #[async_trait]
    impl ActivitySegmentRepository for MockSegmentRepo {
        async fn get_pending_for_sync(
            &self,
            _limit: usize,
        ) -> Result<Vec<ActivitySegment>, PulseArcError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new()) // Return empty for now
        }

        async fn mark_synced(&self, _id: &str) -> Result<(), PulseArcError> {
            Ok(())
        }
    }

    // Mock snapshot repository
    struct MockSnapshotRepo {
        call_count: Arc<AtomicUsize>,
    }

    impl MockSnapshotRepo {
        fn new() -> Self {
            Self { call_count: Arc::new(AtomicUsize::new(0)) }
        }
    }

    #[async_trait]
    impl ActivitySnapshotRepository for MockSnapshotRepo {
        async fn get_pending_for_sync(
            &self,
            _limit: usize,
        ) -> Result<Vec<ActivitySnapshot>, PulseArcError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new()) // Return empty for now
        }

        async fn mark_synced(&self, _id: &str) -> Result<(), PulseArcError> {
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_scheduler_lifecycle() {
        let config = ApiClientConfig::default();
        let client = Arc::new(ApiClient::new(config, Arc::new(MockAuthProvider)).unwrap());
        let commands = Arc::new(ApiCommands::new(client));
        let forwarder = Arc::new(ApiForwarder::new(commands, ForwarderConfig::default()));
        let metrics = Arc::new(PerformanceMetrics::new());

        let segment_repo: Arc<dyn ActivitySegmentRepository> = Arc::new(MockSegmentRepo::new());
        let snapshot_repo: Arc<dyn ActivitySnapshotRepository> = Arc::new(MockSnapshotRepo::new());

        let mut scheduler = SyncScheduler::new(
            forwarder,
            segment_repo,
            snapshot_repo,
            SyncSchedulerConfig::default(),
            metrics,
        );

        // Initially not running
        assert!(!scheduler.is_running());

        // Start succeeds
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running());

        // Stop succeeds
        scheduler.stop().await.unwrap();
        assert!(!scheduler.is_running());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_double_start_fails() {
        let config = ApiClientConfig::default();
        let client = Arc::new(ApiClient::new(config, Arc::new(MockAuthProvider)).unwrap());
        let commands = Arc::new(ApiCommands::new(client));
        let forwarder = Arc::new(ApiForwarder::new(commands, ForwarderConfig::default()));
        let metrics = Arc::new(PerformanceMetrics::new());

        let segment_repo: Arc<dyn ActivitySegmentRepository> = Arc::new(MockSegmentRepo::new());
        let snapshot_repo: Arc<dyn ActivitySnapshotRepository> = Arc::new(MockSnapshotRepo::new());

        let mut scheduler = SyncScheduler::new(
            forwarder,
            segment_repo,
            snapshot_repo,
            SyncSchedulerConfig::default(),
            metrics,
        );

        scheduler.start().await.unwrap();

        // Second start should fail
        let result = scheduler.start().await;
        assert!(result.is_err());

        scheduler.stop().await.unwrap();
    }
}
