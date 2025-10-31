//! Classification job scheduler for activity classification workloads.
//!
//! Provides a cron-based scheduler that triggers classification jobs at fixed
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
//!     ClassificationJob, ClassificationScheduler, ClassificationSchedulerConfig, SchedulerResult,
//! };
//!
//! struct NoopClassificationJob;
//!
//! #[async_trait]
//! impl ClassificationJob for NoopClassificationJob {
//!     async fn run(&self) -> Result<(), pulsearc_infra::errors::InfraError> {
//!         Ok(())
//!     }
//! }
//!
//! # async fn example() -> SchedulerResult<()> {
//! let metrics = Arc::new(PerformanceMetrics::new());
//! let job = Arc::new(NoopClassificationJob);
//! let mut scheduler = ClassificationScheduler::with_config(
//!     ClassificationSchedulerConfig {
//!         cron_expression: "0 */10 * * * *".into(), // every 10 minutes
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

use crate::errors::InfraError;
use crate::observability::metrics::PerformanceMetrics;
use crate::observability::MetricsResult;
use crate::scheduling::error::{SchedulerError, SchedulerResult};

/// Trait representing a classification job.
#[async_trait]
pub trait ClassificationJob: Send + Sync {
    /// Execute the classification job.
    async fn run(&self) -> Result<(), InfraError>;
}

/// Configuration for the classification scheduler.
#[derive(Debug, Clone)]
pub struct ClassificationSchedulerConfig {
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

impl Default for ClassificationSchedulerConfig {
    fn default() -> Self {
        Self {
            cron_expression: "0 */10 * * * *".into(), // every 10 minutes
            job_timeout: Duration::from_secs(600),    // 10 minutes
            start_timeout: Duration::from_secs(5),
            stop_timeout: Duration::from_secs(5),
            join_timeout: Duration::from_secs(5),
        }
    }
}

/// Classification scheduler with explicit lifecycle management.
pub struct ClassificationScheduler {
    scheduler: Arc<RwLock<Option<JobScheduler>>>,
    config: ClassificationSchedulerConfig,
    monitor_handle: Option<JoinHandle<()>>,
    cancellation: CancellationToken,
    metrics: Arc<PerformanceMetrics>,
    job: Arc<dyn ClassificationJob>,
}

impl ClassificationScheduler {
    /// Create a scheduler with the default configuration.
    pub async fn new(
        cron_expression: String,
        job: Arc<dyn ClassificationJob>,
        metrics: Arc<PerformanceMetrics>,
    ) -> SchedulerResult<Self> {
        let config = ClassificationSchedulerConfig { cron_expression, ..Default::default() };
        Self::with_config(config, job, metrics).await
    }

    /// Create a scheduler with a custom configuration.
    pub async fn with_config(
        config: ClassificationSchedulerConfig,
        job: Arc<dyn ClassificationJob>,
        metrics: Arc<PerformanceMetrics>,
    ) -> SchedulerResult<Self> {
        let scheduler = Self {
            scheduler: Arc::new(RwLock::new(None)),
            config,
            monitor_handle: None,
            cancellation: CancellationToken::new(),
            metrics,
            job,
        };
        Ok(scheduler)
    }

    /// Start the scheduler, spawning the monitoring task.
    #[instrument(skip(self))]
    pub async fn start(&mut self) -> SchedulerResult<()> {
        if self.is_running() {
            return Err(SchedulerError::AlreadyRunning);
        }

        self.cancellation = CancellationToken::new();

        let scheduler_instance = self.build_scheduler().await?;
        let start_timeout = self.config.start_timeout;

        let start_result = tokio::time::timeout(start_timeout, scheduler_instance.start())
            .await
            .map_err(|source| SchedulerError::Timeout { duration: start_timeout, source })?;

        start_result.map_err(|source| SchedulerError::StartFailed { source })?;

        {
            let mut guard = self.scheduler.write().await;
            *guard = Some(scheduler_instance);
        }

        let cancel = self.cancellation.clone();
        let metrics = self.metrics.clone();
        let handle = tokio::spawn(async move {
            Self::monitor_task(cancel, metrics).await;
        });

        self.monitor_handle = Some(handle);
        info!("Classification scheduler started");
        log_metric(self.metrics.record_call(), "scheduler.classification.start");
        Ok(())
    }

    /// Stop the scheduler and wait for the monitor task to finish.
    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> SchedulerResult<()> {
        if !self.is_running() {
            return Err(SchedulerError::NotRunning);
        }

        self.cancellation.cancel();

        let scheduler = {
            let mut guard = self.scheduler.write().await;
            guard.take()
        };

        let mut scheduler = match scheduler {
            Some(scheduler) => scheduler,
            None => return Err(SchedulerError::NotRunning),
        };

        let stop_timeout = self.config.stop_timeout;
        let stop_result =
            tokio::time::timeout(stop_timeout, async move { scheduler.shutdown().await })
                .await
                .map_err(|source| SchedulerError::Timeout { duration: stop_timeout, source })?;

        stop_result.map_err(|source| SchedulerError::StopFailed { source })?;

        if let Some(handle) = self.monitor_handle.take() {
            let join_timeout = self.config.join_timeout;
            tokio::time::timeout(join_timeout, handle)
                .await
                .map_err(|source| SchedulerError::Timeout { duration: join_timeout, source })??;
        }

        info!("Classification scheduler stopped");
        Ok(())
    }

    /// Returns true when the monitor task is active.
    pub fn is_running(&self) -> bool {
        self.monitor_handle.as_ref().is_some_and(|handle| !handle.is_finished())
    }

    async fn build_scheduler(&self) -> SchedulerResult<JobScheduler> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|source| SchedulerError::CreationFailed { source })?;
        let cron_expr = self.config.cron_expression.clone();
        let metrics = self.metrics.clone();
        let job = self.job.clone();
        let job_timeout = self.config.job_timeout;

        let job_definition = Job::new_async(cron_expr.as_str(), move |_id, _lock| {
            let metrics = metrics.clone();
            let job = job.clone();

            Box::pin(async move {
                log_metric(metrics.record_call(), "scheduler.classification.job.invoked");
                let started = Instant::now();

                match tokio::time::timeout(job_timeout, job.run()).await {
                    Ok(Ok(())) => {
                        log_metric(
                            metrics.record_fetch_time(started.elapsed()),
                            "scheduler.classification.job.duration",
                        );
                        debug!("Classification job finished successfully");
                    }
                    Ok(Err(err)) => {
                        log_metric(
                            metrics.record_fetch_error(),
                            "scheduler.classification.job.error",
                        );
                        log_metric(
                            metrics.record_fetch_time(started.elapsed()),
                            "scheduler.classification.job.duration",
                        );
                        error!(error = ?err, "Classification job failed");
                    }
                    Err(elapsed) => {
                        log_metric(
                            metrics.record_fetch_timeout(),
                            "scheduler.classification.job.timeout",
                        );
                        warn!(timeout_secs = job_timeout.as_secs(), "Classification job timed out");
                        debug!(elapsed = ?elapsed, "Timeout details");
                    }
                }
            })
        })
        .map_err(|source| SchedulerError::JobRegistrationFailed { source })?;

        let job_id = job_definition.guid();
        scheduler
            .add(job_definition)
            .await
            .map_err(|source| SchedulerError::JobRegistrationFailed { source })?;

        debug!(cron = %self.config.cron_expression, job_id = %job_id, "Registered classification job");
        Ok(scheduler)
    }

    async fn monitor_task(cancel: CancellationToken, metrics: Arc<PerformanceMetrics>) {
        tokio::select! {
            _ = cancel.cancelled() => {
                debug!("Classification scheduler monitor cancelled");
            }
        }

        log_metric(metrics.record_call(), "scheduler.classification.monitor_exit");
    }
}

fn log_metric(result: MetricsResult<()>, metric: &'static str) {
    if let Err(err) = result {
        warn!(metric = metric, error = ?err, "Failed to record scheduler metric");
    }
}

impl Drop for ClassificationScheduler {
    fn drop(&mut self) {
        if self.is_running() {
            warn!("ClassificationScheduler dropped while running; cancelling tasks");
            self.cancellation.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use pulsearc_domain::PulseArcError;

    use super::*;

    struct CountingClassificationJob {
        runs: AtomicUsize,
    }

    impl CountingClassificationJob {
        fn new() -> Self {
            Self { runs: AtomicUsize::new(0) }
        }

        fn run_count(&self) -> usize {
            self.runs.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl ClassificationJob for CountingClassificationJob {
        async fn run(&self) -> Result<(), InfraError> {
            self.runs.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn fast_config() -> ClassificationSchedulerConfig {
        ClassificationSchedulerConfig {
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
        let job = Arc::new(CountingClassificationJob::new());
        let mut scheduler =
            ClassificationScheduler::with_config(fast_config(), job.clone(), metrics)
                .await
                .expect("scheduler created");

        scheduler.start().await.expect("start succeeds");
        tokio::time::sleep(Duration::from_secs(2)).await;
        scheduler.stop().await.expect("stop succeeds");

        assert!(job.run_count() >= 1);
        assert!(!scheduler.is_running());
    }

    struct FailingClassificationJob;

    #[async_trait]
    impl ClassificationJob for FailingClassificationJob {
        async fn run(&self) -> Result<(), InfraError> {
            Err(PulseArcError::Internal("classification failure".into()).into())
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn job_error_increments_metrics_and_keeps_scheduler_running() {
        let metrics = Arc::new(PerformanceMetrics::new());
        let job = Arc::new(FailingClassificationJob);
        let mut scheduler =
            ClassificationScheduler::with_config(fast_config(), job, metrics.clone())
                .await
                .expect("scheduler created");

        scheduler.start().await.expect("start succeeds");
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(scheduler.is_running());
        scheduler.stop().await.expect("stop succeeds");

        assert!(metrics.fetch.get_error_count() >= 1, "error metric recorded");
        assert!(metrics.fetch.get_fetch_count() >= metrics.fetch.get_error_count());
        assert!(!scheduler.is_running());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn double_start_is_rejected() {
        let metrics = Arc::new(PerformanceMetrics::new());
        let job = Arc::new(CountingClassificationJob::new());
        let mut scheduler = ClassificationScheduler::with_config(fast_config(), job, metrics)
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
        let job = Arc::new(CountingClassificationJob::new());
        let mut scheduler = ClassificationScheduler::with_config(fast_config(), job, metrics)
            .await
            .expect("scheduler created");

        scheduler.start().await.expect("start succeeds");
        scheduler.stop().await.expect("stop succeeds");
        assert!(!scheduler.is_running());

        scheduler.start().await.expect("start again");
        scheduler.stop().await.expect("stop again");
    }
}
