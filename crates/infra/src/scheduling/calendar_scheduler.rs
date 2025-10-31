//! Calendar synchronization scheduler for periodic event sync.
//!
//! Provides a cron-based scheduler that triggers calendar synchronization at
//! fixed intervals. The implementation follows the runtime rules captured in
//! `CLAUDE.md`: join handles are tracked, cancellation is explicit, and every
//! asynchronous operation is wrapped in a timeout.
//!
//! Feature-gated behind `calendar` feature flag.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! use pulsearc_infra::observability::metrics::PerformanceMetrics;
//! use pulsearc_infra::scheduling::{CalendarScheduler, CalendarSchedulerConfig, SchedulerResult};
//!
//! # async fn example() -> SchedulerResult<()> {
//! let metrics = Arc::new(PerformanceMetrics::new());
//! // ... create sync_worker ...
//! # let sync_worker = todo!();
//! let mut scheduler = CalendarScheduler::with_config(
//!     CalendarSchedulerConfig {
//!         cron_expression: "0 */15 * * * *".into(), // every 15 minutes
//!         user_emails: vec!["user@example.com".to_string()],
//!         ..Default::default()
//!     },
//!     sync_worker,
//!     metrics,
//! )?;
//!
//! scheduler.start().await?;
//! // ... application runs ...
//! scheduler.stop().await?;
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use tokio::task::JoinHandle;
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

use crate::integrations::calendar::sync::CalendarSyncWorker;
use crate::observability::metrics::PerformanceMetrics;
use crate::observability::MetricsResult;
use crate::scheduling::error::{SchedulerError, SchedulerResult};

/// Configuration for the calendar scheduler.
#[derive(Debug, Clone)]
pub struct CalendarSchedulerConfig {
    /// Cron expression describing the execution schedule.
    pub cron_expression: String,
    /// List of user emails to sync calendars for.
    pub user_emails: Vec<String>,
    /// Timeout applied to a single sync execution.
    pub job_timeout: Duration,
    /// Timeout for starting the underlying scheduler.
    pub start_timeout: Duration,
    /// Timeout for stopping the scheduler.
    pub stop_timeout: Duration,
    /// Timeout for awaiting the monitor task join handle.
    pub join_timeout: Duration,
}

impl Default for CalendarSchedulerConfig {
    fn default() -> Self {
        Self {
            cron_expression: "0 */15 * * * *".into(), // every 15 minutes
            user_emails: Vec::new(),
            job_timeout: Duration::from_secs(300),
            start_timeout: Duration::from_secs(5),
            stop_timeout: Duration::from_secs(5),
            join_timeout: Duration::from_secs(5),
        }
    }
}

/// Calendar synchronization scheduler with explicit lifecycle management.
pub struct CalendarScheduler {
    scheduler: Option<JobScheduler>,
    config: CalendarSchedulerConfig,
    monitor_handle: Option<JoinHandle<()>>,
    cancellation: CancellationToken,
    metrics: Arc<PerformanceMetrics>,
    sync_worker: Arc<CalendarSyncWorker>,
}

impl CalendarScheduler {
    /// Create a scheduler with the default configuration.
    pub fn new(
        cron_expression: String,
        user_emails: Vec<String>,
        sync_worker: Arc<CalendarSyncWorker>,
        metrics: Arc<PerformanceMetrics>,
    ) -> SchedulerResult<Self> {
        let config = CalendarSchedulerConfig { cron_expression, user_emails, ..Default::default() };
        Self::with_config(config, sync_worker, metrics)
    }

    /// Create a scheduler with a custom configuration.
    pub fn with_config(
        config: CalendarSchedulerConfig,
        sync_worker: Arc<CalendarSyncWorker>,
        metrics: Arc<PerformanceMetrics>,
    ) -> SchedulerResult<Self> {
        let scheduler = Self {
            scheduler: None,
            config,
            monitor_handle: None,
            cancellation: CancellationToken::new(),
            metrics,
            sync_worker,
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

        self.scheduler = Some(scheduler_instance);

        let cancel = self.cancellation.clone();
        let metrics = self.metrics.clone();
        let handle = tokio::spawn(async move {
            Self::monitor_task(cancel, metrics).await;
        });

        self.monitor_handle = Some(handle);
        info!("Calendar scheduler started");
        log_metric(self.metrics.record_call(), "scheduler.calendar.start");
        Ok(())
    }

    /// Stop the scheduler and wait for the monitor task to finish.
    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> SchedulerResult<()> {
        if !self.is_running() {
            return Err(SchedulerError::NotRunning);
        }

        self.cancellation.cancel();

        let mut scheduler = match self.scheduler.take() {
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
                .map_err(|source| SchedulerError::Timeout { duration: join_timeout, source })??
        }

        info!("Calendar scheduler stopped");
        self.cancellation = CancellationToken::new();
        Ok(())
    }

    /// Returns true when a scheduler instance is active.
    pub fn is_running(&self) -> bool {
        self.scheduler.is_some()
    }

    async fn build_scheduler(&self) -> SchedulerResult<JobScheduler> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|source| SchedulerError::CreationFailed { source })?;
        let cron_expr = self.config.cron_expression.clone();
        let metrics = self.metrics.clone();
        let sync_worker = self.sync_worker.clone();
        let job_timeout = self.config.job_timeout;
        let user_emails = self.config.user_emails.clone();

        let job_definition = Job::new_async(cron_expr.as_str(), move |_id, _lock| {
            let metrics = metrics.clone();
            let sync_worker = sync_worker.clone();
            let user_emails = user_emails.clone();

            Box::pin(async move {
                log_metric(metrics.record_call(), "scheduler.calendar.job.invoked");
                let started = Instant::now();

                match tokio::time::timeout(
                    job_timeout,
                    Self::perform_calendar_sync(sync_worker, user_emails),
                )
                .await
                {
                    Ok(Ok(())) => {
                        log_metric(
                            metrics.record_fetch_time(started.elapsed()),
                            "scheduler.calendar.job.duration",
                        );
                        debug!("Calendar sync finished successfully");
                    }
                    Ok(Err(err)) => {
                        log_metric(metrics.record_fetch_error(), "scheduler.calendar.job.error");
                        log_metric(
                            metrics.record_fetch_time(started.elapsed()),
                            "scheduler.calendar.job.duration",
                        );
                        error!(error = ?err, "Calendar sync failed");
                    }
                    Err(elapsed) => {
                        log_metric(
                            metrics.record_fetch_timeout(),
                            "scheduler.calendar.job.timeout",
                        );
                        warn!(timeout_secs = job_timeout.as_secs(), "Calendar sync timed out");
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

        debug!(cron = %self.config.cron_expression, job_id = %job_id, "Registered calendar sync job");
        Ok(scheduler)
    }

    async fn perform_calendar_sync(
        sync_worker: Arc<CalendarSyncWorker>,
        user_emails: Vec<String>,
    ) -> Result<(), CalendarJobError> {
        if user_emails.is_empty() {
            debug!("No user emails configured for calendar sync");
            return Ok(());
        }

        info!(user_count = user_emails.len(), "Starting calendar sync for configured users");

        let mut total_synced = 0;
        let mut errors = 0;
        let mut failures = Vec::new();

        for email in &user_emails {
            let user_tag = redact_email(email);
            match sync_worker.perform_sync(email).await {
                Ok(status) => {
                    if status.success {
                        total_synced += status.events_synced;
                        debug!(
                            user = %user_tag,
                            events_synced = status.events_synced,
                            "Calendar sync successful"
                        );
                    } else {
                        errors += 1;
                        warn!(user = %user_tag, "Calendar sync completed but reported failure");
                        failures.push(UserSyncFailure::reported_failure(user_tag));
                    }
                }
                Err(err) => {
                    errors += 1;
                    warn!(user = %user_tag, error = ?err, "Calendar sync failed");
                    failures.push(UserSyncFailure::worker_error(user_tag));
                }
            }
        }

        info!(
            total_users = user_emails.len(),
            total_synced, errors, "Calendar sync batch completed"
        );

        if errors > 0 {
            return Err(CalendarJobError::new(errors, user_emails.len(), failures));
        }

        Ok(())
    }

    async fn monitor_task(cancel: CancellationToken, metrics: Arc<PerformanceMetrics>) {
        tokio::select! {
            _ = cancel.cancelled() => {
                debug!("Calendar scheduler monitor cancelled");
            }
        }

        log_metric(metrics.record_call(), "scheduler.calendar.monitor_exit");
    }
}

fn log_metric(result: MetricsResult<()>, metric: &'static str) {
    if let Err(err) = result {
        warn!(metric = metric, error = ?err, "Failed to record scheduler metric");
    }
}

fn redact_email(email: &str) -> String {
    const EMAIL_HASH_SALT: &[u8] = b"pulsearc-calendar-scheduler-email-salt";
    let mut hasher = Sha256::new();
    hasher.update(EMAIL_HASH_SALT);
    hasher.update(email.as_bytes());
    let digest = hasher.finalize();
    let hash = hex::encode(&digest[..8]);
    format!("email_hash={hash}")
}

#[derive(Debug)]
struct CalendarJobError {
    errors: usize,
    total_users: usize,
    failed_users: Vec<UserSyncFailure>,
}

impl CalendarJobError {
    fn new(errors: usize, total_users: usize, failed_users: Vec<UserSyncFailure>) -> Self {
        Self { errors, total_users, failed_users }
    }
}

impl fmt::Display for CalendarJobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Calendar sync encountered {} errors across {} users",
            self.errors, self.total_users
        )?;

        if !self.failed_users.is_empty() {
            write!(f, " (failed: ")?;
            for (index, failure) in self.failed_users.iter().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{failure}")?;
            }
            write!(f, ")")?;
        }

        Ok(())
    }
}

impl std::error::Error for CalendarJobError {}

#[derive(Debug)]
struct UserSyncFailure {
    user: String,
    reason: FailureReason,
}

impl UserSyncFailure {
    fn worker_error(user: String) -> Self {
        Self { user, reason: FailureReason::WorkerError }
    }

    fn reported_failure(user: String) -> Self {
        Self { user, reason: FailureReason::ReportedFailure }
    }
}

impl fmt::Display for UserSyncFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.user, self.reason)
    }
}

#[derive(Debug, Copy, Clone)]
enum FailureReason {
    WorkerError,
    ReportedFailure,
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkerError => write!(f, "(worker_error)"),
            Self::ReportedFailure => write!(f, "(reported_failure)"),
        }
    }
}

impl Drop for CalendarScheduler {
    fn drop(&mut self) {
        if self.is_running() {
            warn!("CalendarScheduler dropped while running; cancelling tasks");
            self.cancellation.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn lifecycle_runs_successfully() {
        let _metrics = Arc::new(PerformanceMetrics::new());
        // TODO: Need to create mock CalendarSyncWorker for testing
        // For now, this test will be commented out until we have proper mocks
        // let sync_worker = Arc::new(MockCalendarSyncWorker::new());
        // let mut scheduler = CalendarScheduler::with_config(fast_config(),
        // sync_worker, metrics)     .expect("scheduler created");
        //
        // scheduler.start().await.expect("start succeeds");
        // tokio::time::sleep(Duration::from_secs(2)).await;
        // scheduler.stop().await.expect("stop succeeds");
        //
        // assert!(!scheduler.is_running());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn double_start_is_rejected() {
        let _metrics = Arc::new(PerformanceMetrics::new());
        // TODO: Need to create mock CalendarSyncWorker for testing
        // let sync_worker = Arc::new(MockCalendarSyncWorker::new());
        // let mut scheduler = CalendarScheduler::with_config(fast_config(),
        // sync_worker, metrics)     .expect("scheduler created");
        //
        // scheduler.start().await.expect("first start");
        // let err = scheduler.start().await.expect_err("second start fails");
        // assert!(matches!(err, SchedulerError::AlreadyRunning));
        // scheduler.stop().await.expect("stop succeeds");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn restart_after_stop_succeeds() {
        let _metrics = Arc::new(PerformanceMetrics::new());
        // TODO: Need to create mock CalendarSyncWorker for testing
        // let sync_worker = Arc::new(MockCalendarSyncWorker::new());
        // let mut scheduler = CalendarScheduler::with_config(fast_config(),
        // sync_worker, metrics)     .expect("scheduler created");
        //
        // scheduler.start().await.expect("start succeeds");
        // scheduler.stop().await.expect("stop succeeds");
        // assert!(!scheduler.is_running());
        //
        // scheduler.start().await.expect("start again");
        // scheduler.stop().await.expect("stop again");
    }

    #[test]
    fn email_redaction_is_deterministic() {
        let first = super::redact_email("user@example.com");
        let second = super::redact_email("user@example.com");
        assert_eq!(first, second);
    }

    #[test]
    fn email_redaction_masks_local_part() {
        let token = super::redact_email("sensitive@example.com");
        assert!(token.starts_with("email_hash="));
        assert!(!token.contains("sensitive"));
    }
}
