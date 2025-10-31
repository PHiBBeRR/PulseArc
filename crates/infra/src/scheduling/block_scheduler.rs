//! Block generation scheduler for inference system
//!
//! This module provides a cron-based scheduler that triggers block generation
//! at configurable intervals. It follows CLAUDE.md runtime rules with explicit
//! lifecycle management, join handle tracking, and cancellation support.
//!
//! # Architecture
//!
//! The scheduler wraps `tokio-cron-scheduler` with:
//! - Explicit start/stop lifecycle with 5-second timeouts
//! - Join handle tracking for spawned tasks
//! - CancellationToken for graceful shutdown
//! - Job ID tracking for proper cleanup
//! - Structured tracing with span context
//!
//! # Usage
//!
//! ```no_run
//! use pulsearc_infra::scheduling::BlockScheduler;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut scheduler = BlockScheduler::new("0 */5 * * * *".to_string()).await?;
//!
//! // Start scheduler
//! scheduler.start().await?;
//!
//! // ... application runs ...
//!
//! // Stop scheduler gracefully
//! scheduler.stop().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Compliance
//!
//! - **CLAUDE.md §5**: All spawns tracked with join handles
//! - **CLAUDE.md §3**: Structured tracing (no println!/log::)
//! - **Runtime rules**: Explicit shutdown with timeout, cancellation tests

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use super::error::{SchedulerError, SchedulerResult};

/// Configuration for block generation scheduler
#[derive(Debug, Clone)]
pub struct BlockSchedulerConfig {
    /// Cron expression for scheduling (e.g., "0 */5 * * * *" for every 5 minutes)
    pub cron_expression: String,
    /// Timeout for individual job execution
    pub job_timeout_secs: u64,
    /// Timeout for scheduler start operation
    pub start_timeout_secs: u64,
    /// Timeout for scheduler stop operation
    pub stop_timeout_secs: u64,
}

impl Default for BlockSchedulerConfig {
    fn default() -> Self {
        Self {
            cron_expression: "0 */15 * * * *".to_string(), // Every 15 minutes
            job_timeout_secs: 300,                         // 5 minutes
            start_timeout_secs: 5,
            stop_timeout_secs: 5,
        }
    }
}

/// Block generation scheduler with lifecycle management
///
/// Wraps `tokio-cron-scheduler` with CLAUDE.md-compliant patterns:
/// - Join handle tracking
/// - CancellationToken for graceful shutdown
/// - Timeout wrapping on all operations
/// - Job ID tracking for cleanup
pub struct BlockScheduler {
    scheduler: Arc<RwLock<JobScheduler>>,
    config: BlockSchedulerConfig,
    task_handle: Option<JoinHandle<()>>,
    job_id: Option<Uuid>,
    cancellation: Option<CancellationToken>,
}

impl BlockScheduler {
    /// Create a new block scheduler
    ///
    /// # Arguments
    ///
    /// * `cron_expression` - Cron expression for scheduling (e.g., "0 */5 * * * *")
    ///
    /// # Returns
    ///
    /// A configured block scheduler ready to start
    ///
    /// # Errors
    ///
    /// Returns error if scheduler creation fails
    pub async fn new(cron_expression: String) -> SchedulerResult<Self> {
        let config = BlockSchedulerConfig {
            cron_expression,
            ..Default::default()
        };

        Self::with_config(config).await
    }

    /// Create a new block scheduler with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Scheduler configuration
    ///
    /// # Returns
    ///
    /// A configured block scheduler ready to start
    pub async fn with_config(config: BlockSchedulerConfig) -> SchedulerResult<Self> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| SchedulerError::CreationFailed(e.to_string()))?;

        Ok(Self {
            scheduler: Arc::new(RwLock::new(scheduler)),
            config,
            task_handle: None,
            job_id: None,
            cancellation: None,
        })
    }

    /// Start the scheduler
    ///
    /// Registers the block generation job, creates a fresh cancellation token,
    /// and starts the scheduler. Spawns a monitoring task to track scheduler state.
    ///
    /// # Returns
    ///
    /// Ok on successful start, error if already running or start fails
    ///
    /// # Timeouts
    ///
    /// Start operation has a 5-second timeout (configurable)
    #[instrument(skip(self), fields(cron = %self.config.cron_expression))]
    pub async fn start(&mut self) -> SchedulerResult<()> {
        if self.is_running() {
            return Err(SchedulerError::AlreadyRunning);
        }

        info!("Starting block scheduler");

        // Create fresh cancellation token
        let cancel = CancellationToken::new();
        self.cancellation = Some(cancel.clone());

        // Register block generation job
        let job_id = self.register_block_job().await?;
        self.job_id = Some(job_id);

        // Start scheduler with timeout
        let scheduler = self.scheduler.clone();
        let start_timeout = Duration::from_secs(self.config.start_timeout_secs);

        tokio::time::timeout(start_timeout, async move {
            let mut sched = scheduler.write().await;
            sched.start().await
        })
        .await
        .map_err(|_| SchedulerError::Timeout {
            seconds: self.config.start_timeout_secs,
        })?
        .map_err(|e| SchedulerError::StartFailed(e.to_string()))?;

        // Spawn monitoring task with handle tracking
        let scheduler = self.scheduler.clone();

        let handle = tokio::spawn(async move {
            Self::monitor_task(scheduler, cancel).await;
        });

        self.task_handle = Some(handle);

        info!("Block scheduler started successfully");

        Ok(())
    }

    /// Stop the scheduler gracefully
    ///
    /// Cancels the monitoring task, removes the registered job, stops the scheduler,
    /// and awaits join handle completion with timeout.
    ///
    /// # Returns
    ///
    /// Ok on successful stop, error if not running or stop fails
    ///
    /// # Timeouts
    ///
    /// - Scheduler stop: 5 seconds (configurable)
    /// - Join handle await: 5 seconds
    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> SchedulerResult<()> {
        if !self.is_running() {
            return Err(SchedulerError::NotRunning);
        }

        info!("Stopping block scheduler");

        // Cancel monitoring task
        if let Some(ref cancel) = self.cancellation {
            cancel.cancel();
        }

        // Remove registered job
        if let Some(job_id) = self.job_id.take() {
            let mut sched = self.scheduler.write().await;
            if let Err(e) = sched.remove(&job_id).await {
                warn!(job_id = %job_id, error = %e, "Failed to remove job");
            }
        }

        // Stop scheduler with timeout
        let scheduler = self.scheduler.clone();
        let stop_timeout = Duration::from_secs(self.config.stop_timeout_secs);

        tokio::time::timeout(stop_timeout, async move {
            let mut sched = scheduler.write().await;
            sched.shutdown().await
        })
        .await
        .map_err(|_| SchedulerError::Timeout {
            seconds: self.config.stop_timeout_secs,
        })?
        .map_err(|e| SchedulerError::StopFailed(e.to_string()))?;

        // Await join handle with timeout
        if let Some(handle) = self.task_handle.take() {
            let handle_timeout = Duration::from_secs(5);
            tokio::time::timeout(handle_timeout, handle)
                .await
                .map_err(|_| {
                    warn!("Monitor task did not complete within timeout");
                    SchedulerError::Timeout { seconds: 5 }
                })?
                .map_err(|e| SchedulerError::TaskJoinFailed(e.to_string()))?;
        }

        // Clear cancellation token
        self.cancellation = None;

        info!("Block scheduler stopped successfully");

        Ok(())
    }

    /// Check if scheduler is currently running
    pub fn is_running(&self) -> bool {
        self.task_handle.is_some()
            && self
                .cancellation
                .as_ref()
                .map_or(false, |c| !c.is_cancelled())
    }

    /// Register the block generation job with the scheduler
    ///
    /// Creates a cron job that executes block generation at configured intervals.
    /// Returns the job UUID for later removal.
    async fn register_block_job(&self) -> SchedulerResult<Uuid> {
        let cron_expr = self.config.cron_expression.clone();
        let job_timeout = Duration::from_secs(self.config.job_timeout_secs);

        let job = Job::new_async(cron_expr.as_str(), move |uuid, _lock| {
            Box::pin(async move {
                let start = std::time::Instant::now();

                debug!(job_id = %uuid, "Block generation job triggered");

                // Execute block generation with timeout
                match tokio::time::timeout(job_timeout, Self::execute_block_generation()).await {
                    Ok(Ok(())) => {
                        let duration = start.elapsed().as_secs_f64();
                        info!(duration_secs = duration, "Block generation completed");
                    }
                    Ok(Err(e)) => {
                        let duration = start.elapsed().as_secs_f64();
                        error!(error = %e, duration_secs = duration, "Block generation failed");
                    }
                    Err(_) => {
                        warn!(timeout_secs = job_timeout.as_secs(), "Block generation timeout");
                    }
                }
            })
        })
        .map_err(|e| SchedulerError::JobRegistrationFailed(e.to_string()))?;

        let mut scheduler = self.scheduler.write().await;
        let job_id = scheduler
            .add(job)
            .await
            .map_err(|e| SchedulerError::JobRegistrationFailed(e.to_string()))?;

        debug!(job_id = %job_id, cron = %cron_expr, "Block generation job registered");

        Ok(job_id)
    }

    /// Execute block generation (placeholder for actual implementation)
    ///
    /// This will be integrated with the inference system in future PRs.
    /// For now, it's a stub that demonstrates the execution pattern.
    async fn execute_block_generation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Integrate with inference system (Phase 3D follow-up)
        // For now, simulate work
        tokio::time::sleep(Duration::from_millis(100)).await;
        debug!("Block generation executed (stub)");
        Ok(())
    }

    /// Monitoring task that runs while scheduler is active
    ///
    /// This task uses tokio::select! to wait for cancellation signal.
    /// It's a pure async function separated for testability.
    async fn monitor_task(_scheduler: Arc<RwLock<JobScheduler>>, cancel: CancellationToken) {
        tokio::select! {
            _ = cancel.cancelled() => {
                debug!("Block scheduler monitor task cancelled");
            }
        }
    }
}

/// Ensure scheduler is stopped when dropped
impl Drop for BlockScheduler {
    fn drop(&mut self) {
        if self.is_running() {
            warn!("BlockScheduler dropped while running; cancelling");
            if let Some(ref cancel) = self.cancellation {
                cancel.cancel();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_scheduler_lifecycle() {
        let mut scheduler = BlockScheduler::new("0 * * * * *".to_string()).await.unwrap();

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
        let mut scheduler = BlockScheduler::new("0 * * * * *".to_string()).await.unwrap();

        scheduler.start().await.unwrap();

        // Second start should fail
        let result = scheduler.start().await;
        assert!(matches!(result, Err(SchedulerError::AlreadyRunning)));

        scheduler.stop().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_stop_without_start_fails() {
        let mut scheduler = BlockScheduler::new("0 * * * * *".to_string()).await.unwrap();

        // Stop without start should fail
        let result = scheduler.stop().await;
        assert!(matches!(result, Err(SchedulerError::NotRunning)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_start_stop_start_cycle() {
        let mut scheduler = BlockScheduler::new("0 * * * * *".to_string()).await.unwrap();

        // First cycle
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running());
        scheduler.stop().await.unwrap();
        assert!(!scheduler.is_running());

        // Second cycle (tests fresh cancellation token and job cleanup)
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running());
        scheduler.stop().await.unwrap();
        assert!(!scheduler.is_running());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cancellation_stops_scheduler() {
        let mut scheduler = BlockScheduler::new("0 * * * * *".to_string()).await.unwrap();

        scheduler.start().await.unwrap();

        // Cancel via cancellation token
        if let Some(ref cancel) = scheduler.cancellation {
            cancel.cancel();
        }

        // Give time for cancellation to propagate
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should no longer be running after cancellation
        assert!(!scheduler.is_running());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_custom_config() {
        let config = BlockSchedulerConfig {
            cron_expression: "0 */10 * * * *".to_string(),
            job_timeout_secs: 600,
            start_timeout_secs: 10,
            stop_timeout_secs: 10,
        };

        let mut scheduler = BlockScheduler::with_config(config).await.unwrap();

        scheduler.start().await.unwrap();
        assert!(scheduler.is_running());

        scheduler.stop().await.unwrap();
        assert!(!scheduler.is_running());
    }
}
