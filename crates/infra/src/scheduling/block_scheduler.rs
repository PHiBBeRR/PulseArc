//! Block generation scheduler for inference workloads.
//!
//! Provides a cron-based scheduler that triggers a user-supplied job at fixed
//! intervals. The implementation follows the runtime rules captured in
//! `CLAUDE.md`: join handles are tracked, cancellation is explicit, and every
//! asynchronous operation is wrapped in a timeout.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! use async_trait::async_trait;
//! use pulsearc_infra::observability::metrics::PerformanceMetrics;
//! use pulsearc_infra::scheduling::{
//!     BlockJob, BlockScheduler, BlockSchedulerConfig, SchedulerResult,
//! };
//!
//! struct NoopJob;
//!
//! #[async_trait]
//! impl BlockJob for NoopJob {
//!     async fn run(&self) -> Result<(), pulsearc_infra::errors::InfraError> {
//!         Ok(())
//!     }
//! }
//!
//! # async fn example() -> SchedulerResult<()> {
//! let metrics = Arc::new(PerformanceMetrics::new());
//! let job = Arc::new(NoopJob);
//! let mut scheduler = BlockScheduler::with_config(
//!     BlockSchedulerConfig {
//!         cron_expression: "0 */5 * * * *".into(), // every 5 minutes
//!         ..Default::default()
//!     },
//!     job,
//!     metrics,
//! )
//! .await?;
//!
//! scheduler.start().await?;
//! // ... application runs ...
//! scheduler.stop().await?;
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::errors::InfraError;
use crate::observability::metrics::PerformanceMetrics;
use crate::observability::MetricsResult;
use crate::scheduling::error::{SchedulerError, SchedulerResult};

/// Trait representing a block generation job.
#[async_trait]
pub trait BlockJob: Send + Sync {
    /// Execute the job.
    async fn run(&self) -> Result<(), InfraError>;
}

/// Configuration for the block scheduler.
#[derive(Debug, Clone)]
pub struct BlockSchedulerConfig {
    /// Cron expression describing the execution schedule.
    pub cron_expression: String,
    /// Timeout applied to a single job execution.
    pub job_timeout: Duration,
    /// Timeout for starting the underlying scheduler.
    pub start_timeout: Duration,
    /// Timeout for stopping the scheduler.
    pub stop_timeout: Duration,
    /// Timeout for awaiting the monitor task join handle.
    pub join_timeout: Duration,
}

impl Default for BlockSchedulerConfig {
    fn default() -> Self {
        Self {
            cron_expression: "0 */15 * * * *".into(), // every 15 minutes
            job_timeout: Duration::from_secs(300),
            start_timeout: Duration::from_secs(5),
            stop_timeout: Duration::from_secs(5),
            join_timeout: Duration::from_secs(5),
        }
    }
}

/// Block scheduler with explicit lifecycle management.
pub struct BlockScheduler {
    scheduler: Arc<RwLock<JobScheduler>>,
    config: BlockSchedulerConfig,
    job_id: Uuid,
    monitor_handle: Option<JoinHandle<()>>,
    cancellation: CancellationToken,
    metrics: Arc<PerformanceMetrics>,
    job: Arc<dyn BlockJob>,
}

impl BlockScheduler {
    /// Create a scheduler with the default configuration.
    pub async fn new(
        cron_expression: String,
        job: Arc<dyn BlockJob>,
        metrics: Arc<PerformanceMetrics>,
    ) -> SchedulerResult<Self> {
        let mut config = BlockSchedulerConfig::default();
        config.cron_expression = cron_expression;
        Self::with_config(config, job, metrics).await
    }

    /// Create a scheduler with a custom configuration.
    pub async fn with_config(
        config: BlockSchedulerConfig,
        job: Arc<dyn BlockJob>,
        metrics: Arc<PerformanceMetrics>,
    ) -> SchedulerResult<Self> {
        let raw_scheduler = JobScheduler::new()
            .await
            .map_err(|source| SchedulerError::CreationFailed { source })?;

        let mut scheduler = Self {
            scheduler: Arc::new(RwLock::new(raw_scheduler)),
            config,
            job_id: Uuid::nil(),
            monitor_handle: None,
            cancellation: CancellationToken::new(),
            metrics,
            job,
        };

        scheduler.job_id = scheduler.register_block_job().await?;
        Ok(scheduler)
    }

    /// Start the scheduler, spawning the monitoring task.
    #[instrument(skip(self))]
    pub async fn start(&mut self) -> SchedulerResult<()> {
        if self.is_running() {
            return Err(SchedulerError::AlreadyRunning);
        }

        self.cancellation = CancellationToken::new();

        let scheduler = self.scheduler.clone();
        let start_timeout = self.config.start_timeout;
        let start_result = tokio::time::timeout(start_timeout, async move {
            let guard = scheduler.write().await;
            guard.start().await
        })
        .await
        .map_err(|source| SchedulerError::Timeout { duration: start_timeout, source })?;

        start_result.map_err(|source| SchedulerError::StartFailed { source })?;

        let cancel = self.cancellation.clone();
        let metrics = self.metrics.clone();
        let handle = tokio::spawn(async move {
            Self::monitor_task(cancel, metrics).await;
        });

        self.monitor_handle = Some(handle);
        info!("Block scheduler started");
        log_metric(self.metrics.record_call(), "scheduler.block.start");
        Ok(())
    }

    /// Stop the scheduler and wait for the monitor task to finish.
    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> SchedulerResult<()> {
        if !self.is_running() {
            return Err(SchedulerError::NotRunning);
        }

        self.cancellation.cancel();

        let scheduler = self.scheduler.clone();
        let stop_timeout = self.config.stop_timeout;
        let stop_result = tokio::time::timeout(stop_timeout, async move {
            let mut guard = scheduler.write().await;
            guard.shutdown().await
        })
        .await
        .map_err(|source| SchedulerError::Timeout { duration: stop_timeout, source })?;

        stop_result.map_err(|source| SchedulerError::StopFailed { source })?;

        if let Some(handle) = self.monitor_handle.take() {
            let join_timeout = self.config.join_timeout;
            tokio::time::timeout(join_timeout, handle)
                .await
                .map_err(|source| SchedulerError::Timeout { duration: join_timeout, source })??
        }

        info!("Block scheduler stopped");
        self.cancellation = CancellationToken::new();
        Ok(())
    }

    /// Returns true when the monitor task is active.
    pub fn is_running(&self) -> bool {
        self.monitor_handle.as_ref().map_or(false, |handle| !handle.is_finished())
    }

    async fn register_block_job(&mut self) -> SchedulerResult<Uuid> {
        if self.job_id != Uuid::nil() {
            return Ok(self.job_id);
        }

        let cron_expr = self.config.cron_expression.clone();
        let metrics = self.metrics.clone();
        let job = self.job.clone();
        let job_timeout = self.config.job_timeout;

        let job_definition = Job::new_async(cron_expr.as_str(), move |_id, _lock| {
            let metrics = metrics.clone();
            let job = job.clone();

            Box::pin(async move {
                log_metric(metrics.record_call(), "scheduler.block.job.invoked");
                let started = Instant::now();

                match tokio::time::timeout(job_timeout, job.run()).await {
                    Ok(Ok(())) => {
                        log_metric(
                            metrics.record_fetch_time(started.elapsed()),
                            "scheduler.block.job.duration",
                        );
                        debug!("Block generation finished successfully");
                    }
                    Ok(Err(err)) => {
                        log_metric(metrics.record_fetch_error(), "scheduler.block.job.error");
                        log_metric(
                            metrics.record_fetch_time(started.elapsed()),
                            "scheduler.block.job.duration",
                        );
                        error!(error = ?err, "Block generation failed");
                    }
                    Err(elapsed) => {
                        log_metric(metrics.record_fetch_timeout(), "scheduler.block.job.timeout");
                        warn!(timeout_secs = job_timeout.as_secs(), "Block generation timed out");
                        debug!(elapsed = ?elapsed, "Timeout details");
                    }
                }
            })
        })
        .map_err(|source| SchedulerError::JobRegistrationFailed { source })?;

        let job_id = job_definition.guid();
        let scheduler = self.scheduler.write().await;
        scheduler
            .add(job_definition)
            .await
            .map_err(|source| SchedulerError::JobRegistrationFailed { source })?;

        debug!(cron = %self.config.cron_expression, job_id = %job_id, "Registered block generation job");
        Ok(job_id)
    }

    async fn monitor_task(cancel: CancellationToken, metrics: Arc<PerformanceMetrics>) {
        tokio::select! {
            _ = cancel.cancelled() => {
                debug!("Block scheduler monitor cancelled");
            }
        }

        log_metric(metrics.record_call(), "scheduler.block.monitor_exit");
    }
}

fn log_metric(result: MetricsResult<()>, metric: &'static str) {
    if let Err(err) = result {
        warn!(metric = metric, error = ?err, "Failed to record scheduler metric");
    }
}

impl Drop for BlockScheduler {
    fn drop(&mut self) {
        if self.is_running() {
            warn!("BlockScheduler dropped while running; cancelling tasks");
            self.cancellation.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduling::error::SchedulerError;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingJob {
        runs: AtomicUsize,
    }

    impl CountingJob {
        fn new() -> Self {
            Self { runs: AtomicUsize::new(0) }
        }

        fn run_count(&self) -> usize {
            self.runs.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl BlockJob for CountingJob {
        async fn run(&self) -> Result<(), InfraError> {
            self.runs.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn fast_config() -> BlockSchedulerConfig {
        BlockSchedulerConfig {
            cron_expression: "*/1 * * * * *".into(), // every second
            job_timeout: Duration::from_secs(2),
            start_timeout: Duration::from_secs(2),
            stop_timeout: Duration::from_secs(2),
            join_timeout: Duration::from_secs(2),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn lifecycle_runs_successfully() {
        let metrics = Arc::new(PerformanceMetrics::new());
        let job = Arc::new(CountingJob::new());
        let mut scheduler = BlockScheduler::with_config(fast_config(), job.clone(), metrics)
            .await
            .expect("scheduler created");

        scheduler.start().await.expect("start succeeds");
        tokio::time::sleep(Duration::from_secs(2)).await;
        scheduler.stop().await.expect("stop succeeds");

        assert!(job.run_count() >= 1);
        assert!(!scheduler.is_running());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn double_start_is_rejected() {
        let metrics = Arc::new(PerformanceMetrics::new());
        let job = Arc::new(CountingJob::new());
        let mut scheduler = BlockScheduler::with_config(fast_config(), job, metrics)
            .await
            .expect("scheduler created");

        scheduler.start().await.expect("first start");
        let err = scheduler.start().await.expect_err("second start fails");
        matches!(err, SchedulerError::AlreadyRunning);
        scheduler.stop().await.expect("stop succeeds");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn restart_after_stop_succeeds() {
        let metrics = Arc::new(PerformanceMetrics::new());
        let job = Arc::new(CountingJob::new());
        let mut scheduler = BlockScheduler::with_config(fast_config(), job, metrics)
            .await
            .expect("scheduler created");

        scheduler.start().await.expect("start succeeds");
        scheduler.stop().await.expect("stop succeeds");
        assert!(!scheduler.is_running());

        scheduler.start().await.expect("start again");
        scheduler.stop().await.expect("stop again");
    }
}
