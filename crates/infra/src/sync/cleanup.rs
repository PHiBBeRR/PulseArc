//! Data cleanup service for storage management
//!
//! This module provides periodic cleanup of old/stale data with configurable
//! retention policies. It follows CLAUDE.md runtime rules with explicit
//! lifecycle management.
//!
//! # Features
//!
//! - Periodic cleanup of segments, snapshots, batches, token usage
//! - Configurable retention periods per table
//! - Dry-run mode for testing
//! - Batch deletion (max 1000 records/batch)
//! - Graceful shutdown with cancellation
//!
//! # Compliance
//!
//! - **CLAUDE.md §5**: Join handle tracking, cancellation, timeouts
//! - **CLAUDE.md §3**: Structured tracing only

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use pulsearc_common::error::{CommonError, CommonResult};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, instrument, warn};

use crate::database::DbManager;

/// Type alias for task handle to avoid complexity warnings
type TaskHandle = Arc<Mutex<Option<JoinHandle<()>>>>;

/// Configuration for cleanup service
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    /// Retention period for segments (days)
    pub segment_retention_days: u32,
    /// Retention period for snapshots (days)
    pub snapshot_retention_days: u32,
    /// Retention period for token usage (days)
    pub cost_retention_days: u32,
    /// Cleanup interval
    pub cleanup_interval: Duration,
    /// Max records to delete per batch (prevents long-running transactions)
    pub max_batch_size: usize,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            segment_retention_days: 90,
            snapshot_retention_days: 30,
            cost_retention_days: 90,
            cleanup_interval: Duration::from_secs(3600), // 1 hour
            max_batch_size: 1000,
        }
    }
}

/// Statistics from cleanup operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupStats {
    pub segments_deleted: usize,
    pub snapshots_deleted: usize,
    pub batches_deleted: usize,
    pub token_usage_deleted: usize,
    pub duration_secs: f64,
}

/// Dry-run result (what would be deleted)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunResult {
    pub segments: usize,
    pub snapshots: usize,
    pub batches: usize,
    pub token_usage: usize,
}

/// Background cleanup service with lifecycle management
pub struct CleanupService {
    db: Arc<DbManager>,
    config: CleanupConfig,
    cancellation_token: CancellationToken,
    task_handle: TaskHandle,
}

impl CleanupService {
    /// Create a new cleanup service
    ///
    /// # Arguments
    ///
    /// * `db` - Database manager
    /// * `config` - Cleanup configuration
    ///
    /// # Returns
    ///
    /// Configured cleanup service
    pub fn new(db: Arc<DbManager>, config: CleanupConfig) -> Self {
        Self {
            db,
            config,
            cancellation_token: CancellationToken::new(),
            task_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the cleanup service
    ///
    /// Spawns a background task that runs cleanup periodically.
    ///
    /// # Errors
    ///
    /// Returns error if service is already running
    #[instrument(skip(self))]
    pub async fn start(&mut self) -> CommonResult<()> {
        if self.is_running().await {
            return Err(pulsearc_common::error::CommonError::config(
                "Cleanup service already running",
            ));
        }

        info!("Starting cleanup service");

        // Create a new cancellation token (supports restart after stop)
        self.cancellation_token = CancellationToken::new();

        let db = Arc::clone(&self.db);
        let config = self.config.clone();
        let cancel = self.cancellation_token.clone();

        let handle = tokio::spawn(async move {
            Self::cleanup_loop(db, config, cancel).await;
        });

        *self.task_handle.lock().await = Some(handle);

        info!("Cleanup service started");

        Ok(())
    }

    /// Stop the cleanup service gracefully
    ///
    /// Cancels the background task and awaits completion.
    ///
    /// # Errors
    ///
    /// Returns error if service is not running
    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> CommonResult<()> {
        if !self.is_running().await {
            return Err(pulsearc_common::error::CommonError::config("Cleanup service not running"));
        }

        info!("Stopping cleanup service");

        // Cancel background task
        self.cancellation_token.cancel();

        // Await handle with timeout
        if let Some(handle) = self.task_handle.lock().await.take() {
            match tokio::time::timeout(Duration::from_secs(5), handle).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    warn!("Cleanup task panicked: {}", e);
                    return Err(CommonError::internal(format!("Cleanup task panicked: {}", e)));
                }
                Err(_) => {
                    warn!("Cleanup task did not complete within timeout");
                    return Err(CommonError::timeout("cleanup_task", Duration::from_secs(5)));
                }
            }
        }

        info!("Cleanup service stopped");

        Ok(())
    }

    /// Check if cleanup service is running
    ///
    /// A service is considered running if it has an active task handle.
    pub async fn is_running(&self) -> bool {
        let guard = self.task_handle.lock().await;
        guard.as_ref().is_some_and(|handle| !handle.is_finished())
    }

    /// Run cleanup once immediately
    ///
    /// Useful for manual cleanup or testing.
    ///
    /// # Errors
    ///
    /// Returns error if cleanup operations fail
    #[instrument(skip(self))]
    pub async fn cleanup_once(&self) -> CommonResult<CleanupStats> {
        let start = std::time::Instant::now();

        info!("Running cleanup");

        let segments_deleted = self.cleanup_old_segments().await?;
        let snapshots_deleted = self.cleanup_old_snapshots().await?;
        let batches_deleted = self.cleanup_old_batches().await?;
        let token_usage_deleted = self.cleanup_old_token_usage().await?;

        let duration_secs = start.elapsed().as_secs_f64();

        let stats = CleanupStats {
            segments_deleted,
            snapshots_deleted,
            batches_deleted,
            token_usage_deleted,
            duration_secs,
        };

        info!(
            segments = segments_deleted,
            snapshots = snapshots_deleted,
            batches = batches_deleted,
            token_usage = token_usage_deleted,
            duration_secs = duration_secs,
            "Cleanup completed"
        );

        Ok(stats)
    }

    /// Dry run cleanup (shows what would be deleted)
    ///
    /// # Errors
    ///
    /// Returns error if database queries fail
    #[instrument(skip(self))]
    pub async fn dry_run(&self) -> CommonResult<DryRunResult> {
        let db = Arc::clone(&self.db);
        let config = self.config.clone();
        let now = Utc::now().timestamp();

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection().map_err(|e| CommonError::persistence(e.to_string()))?;

            let segment_cutoff = now - (i64::from(config.segment_retention_days) * 86400);
            let snapshot_cutoff = now - (i64::from(config.snapshot_retention_days) * 86400);
            let cost_cutoff = now - (i64::from(config.cost_retention_days) * 86400);

            let segments: usize = conn.query_row(
                "SELECT COUNT(*) FROM activity_segments WHERE created_at < ?1",
                rusqlite::params![segment_cutoff],
                |r| r.get(0),
            )?;

            let snapshots: usize = conn.query_row(
                "SELECT COUNT(*) FROM activity_snapshots WHERE created_at < ?1",
                rusqlite::params![snapshot_cutoff],
                |r| r.get(0),
            )?;

            let batches: usize = conn.query_row(
                "SELECT COUNT(*) FROM batch_queue WHERE status = 'completed' AND processed_at < ?1",
                rusqlite::params![snapshot_cutoff],
                |r| r.get(0),
            )?;

            let token_usage: usize = conn.query_row(
                "SELECT COUNT(*) FROM token_usage WHERE timestamp < ?1",
                rusqlite::params![cost_cutoff],
                |r| r.get(0),
            )?;

            Ok(DryRunResult { segments, snapshots, batches, token_usage })
        })
        .await
        .map_err(|e| pulsearc_common::error::CommonError::Internal {
            message: format!("Task join failed: {}", e),
            context: None,
        })?
    }

    /// Background cleanup loop
    async fn cleanup_loop(db: Arc<DbManager>, config: CleanupConfig, cancel: CancellationToken) {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!("Cleanup loop cancelled");
                    break;
                }
                _ = tokio::time::sleep(config.cleanup_interval) => {
                    // Run cleanup
                    let cleanup_service = CleanupService::new(db.clone(), config.clone());
                    match cleanup_service.cleanup_once().await {
                        Ok(stats) => {
                            debug!(
                                segments = stats.segments_deleted,
                                snapshots = stats.snapshots_deleted,
                                batches = stats.batches_deleted,
                                "Periodic cleanup completed"
                            );
                        }
                        Err(e) => {
                            warn!(error = %e, "Periodic cleanup failed");
                        }
                    }
                }
            }
        }
    }

    /// Delete old segments
    async fn cleanup_old_segments(&self) -> CommonResult<usize> {
        let db = Arc::clone(&self.db);
        let cutoff =
            Utc::now().timestamp() - (i64::from(self.config.segment_retention_days) * 86400);
        let max_batch = self.config.max_batch_size;

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection().map_err(|e| CommonError::persistence(e.to_string()))?;

            let count = conn
                .execute(
                    "DELETE FROM activity_segments WHERE created_at < ?1 AND id IN (SELECT id FROM activity_segments WHERE created_at < ?1 LIMIT ?2)",
                    rusqlite::params![cutoff, max_batch],
                )
                .map_err(|e| CommonError::persistence(e.to_string()))?;

            Ok(count)
        })
        .await
        .map_err(|e| pulsearc_common::error::CommonError::Internal {
            message: format!("Task join failed: {}", e),
            context: None,
        })?
    }

    /// Delete old snapshots
    async fn cleanup_old_snapshots(&self) -> CommonResult<usize> {
        let db = Arc::clone(&self.db);
        let cutoff =
            Utc::now().timestamp() - (i64::from(self.config.snapshot_retention_days) * 86400);
        let max_batch = self.config.max_batch_size;

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection().map_err(|e| CommonError::persistence(e.to_string()))?;

            let count = conn
                .execute(
                    "DELETE FROM activity_snapshots WHERE created_at < ?1 AND id IN (SELECT id FROM activity_snapshots WHERE created_at < ?1 LIMIT ?2)",
                    rusqlite::params![cutoff, max_batch],
                )
                .map_err(|e| CommonError::persistence(e.to_string()))?;

            Ok(count)
        })
        .await
        .map_err(|e| pulsearc_common::error::CommonError::Internal {
            message: format!("Task join failed: {}", e),
            context: None,
        })?
    }

    /// Delete old completed batches
    async fn cleanup_old_batches(&self) -> CommonResult<usize> {
        let db = Arc::clone(&self.db);
        let cutoff =
            Utc::now().timestamp() - (i64::from(self.config.snapshot_retention_days) * 86400);
        let max_batch = self.config.max_batch_size;

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection().map_err(|e| CommonError::persistence(e.to_string()))?;

            let count = conn
                .execute(
                    "DELETE FROM batch_queue WHERE status = 'completed' AND processed_at < ?1 AND batch_id IN (SELECT batch_id FROM batch_queue WHERE status = 'completed' AND processed_at < ?1 LIMIT ?2)",
                    rusqlite::params![cutoff, max_batch],
                )
                .map_err(|e| CommonError::persistence(e.to_string()))?;

            Ok(count)
        })
        .await
        .map_err(|e| pulsearc_common::error::CommonError::Internal {
            message: format!("Task join failed: {}", e),
            context: None,
        })?
    }

    /// Delete old token usage records
    async fn cleanup_old_token_usage(&self) -> CommonResult<usize> {
        let db = Arc::clone(&self.db);
        let cutoff = Utc::now().timestamp() - (i64::from(self.config.cost_retention_days) * 86400);
        let max_batch = self.config.max_batch_size;

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection().map_err(|e| CommonError::persistence(e.to_string()))?;

            let count = conn
                .execute(
                    "DELETE FROM token_usage WHERE timestamp < ?1 AND batch_id IN (SELECT batch_id FROM token_usage WHERE timestamp < ?1 LIMIT ?2)",
                    rusqlite::params![cutoff, max_batch],
                )
                .map_err(|e| CommonError::persistence(e.to_string()))?;

            Ok(count)
        })
        .await
        .map_err(|e| pulsearc_common::error::CommonError::Internal {
            message: format!("Task join failed: {}", e),
            context: None,
        })?
    }
}

/// Ensure service is stopped when dropped
impl Drop for CleanupService {
    fn drop(&mut self) {
        // Note: Can't check task_handle (async), so check if token is not cancelled
        // This is best-effort cleanup in Drop
        if !self.cancellation_token.is_cancelled() {
            warn!("CleanupService dropped while running; cancelling");
            self.cancellation_token.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn create_test_db() -> Arc<DbManager> {
        let db = Arc::new(
            DbManager::new(":memory:", 1, Some("test-key")).expect("test db should initialize"),
        );
        db.run_migrations().expect("test migrations should succeed");
        db
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cleanup_lifecycle() {
        let db = create_test_db();
        let config = CleanupConfig::default();

        let mut service = CleanupService::new(db, config);

        // Initially not running
        assert!(!service.is_running().await);

        // Start succeeds
        service.start().await.expect("service starts");
        assert!(service.is_running().await);

        // Stop succeeds
        service.stop().await.expect("service stops");
        assert!(!service.is_running().await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_double_start_fails() {
        let db = create_test_db();
        let config = CleanupConfig::default();

        let mut service = CleanupService::new(db, config);

        service.start().await.expect("first start succeeds");

        // Second start should fail
        let result = service.start().await;
        assert!(result.is_err());

        service.stop().await.expect("stop succeeds");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cleanup_once_succeeds() {
        let db = create_test_db();
        let config = CleanupConfig::default();

        let service = CleanupService::new(db, config);

        let stats = service.cleanup_once().await.expect("cleanup once succeeds");

        // No data, so nothing deleted
        assert_eq!(stats.segments_deleted, 0);
        assert_eq!(stats.snapshots_deleted, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dry_run_succeeds() {
        let db = create_test_db();
        let config = CleanupConfig::default();

        let service = CleanupService::new(db, config);

        let result = service.dry_run().await.expect("dry run succeeds");

        // No data, so nothing to delete
        assert_eq!(result.segments, 0);
        assert_eq!(result.snapshots, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cancellation_stops_service() {
        let db = create_test_db();
        let config =
            CleanupConfig { cleanup_interval: Duration::from_millis(100), ..Default::default() };

        let mut service = CleanupService::new(db, config);

        service.start().await.expect("start succeeds");

        // Cancel via token
        service.cancellation_token.cancel();

        // Give time for cancellation
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should no longer be running
        assert!(!service.is_running().await);
    }
}
