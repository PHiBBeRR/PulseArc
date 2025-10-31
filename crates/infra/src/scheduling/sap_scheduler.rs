//! SAP time entry scheduler for periodic outbox processing.
//!
//! Provides a cron-based scheduler that triggers SAP batch forwarding at fixed
//! intervals. The implementation follows the runtime rules captured in
//! `CLAUDE.md`: join handles are tracked, cancellation is explicit, and every
//! asynchronous operation is wrapped in a timeout.
//!
//! Feature-gated behind `sap` feature flag.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! use pulsearc_infra::observability::metrics::PerformanceMetrics;
//! use pulsearc_infra::scheduling::{SapScheduler, SapSchedulerConfig, SchedulerResult};
//!
//! # async fn example() -> SchedulerResult<()> {
//! let metrics = Arc::new(PerformanceMetrics::new());
//! // ... create batch_forwarder and outbox_repo ...
//! # let batch_forwarder = todo!();
//! # let outbox_repo = todo!();
//! let mut scheduler = SapScheduler::with_config(
//!     SapSchedulerConfig {
//!         cron_expression: "0 */30 * * * *".into(), // every 30 minutes
//!         ..Default::default()
//!     },
//!     batch_forwarder,
//!     outbox_repo,
//!     metrics,
//! )?;
//!
//! scheduler.start().await?;
//! // ... application runs ...
//! scheduler.stop().await?;
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};

use pulsearc_core::OutboxQueue as OutboxQueuePort;
use pulsearc_domain::PulseArcError;
use thiserror::Error;
use tokio::task::JoinHandle;
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

use crate::integrations::sap::BatchForwarder;
use crate::observability::metrics::PerformanceMetrics;
use crate::observability::MetricsResult;
use crate::scheduling::error::{SchedulerError, SchedulerResult};

/// Configuration for the SAP scheduler.
#[derive(Debug, Clone)]
pub struct SapSchedulerConfig {
    /// Cron expression describing the execution schedule.
    pub cron_expression: String,
    /// Maximum number of entries to process per batch.
    pub batch_size: usize,
    /// Timeout applied to a single batch processing execution.
    pub job_timeout: Duration,
    /// Timeout for starting the underlying scheduler.
    pub start_timeout: Duration,
    /// Timeout for stopping the scheduler.
    pub stop_timeout: Duration,
    /// Timeout for awaiting the monitor task join handle.
    pub join_timeout: Duration,
}

impl Default for SapSchedulerConfig {
    fn default() -> Self {
        Self {
            cron_expression: "0 */30 * * * *".into(), // every 30 minutes
            batch_size: 50,
            job_timeout: Duration::from_secs(300),
            start_timeout: Duration::from_secs(5),
            stop_timeout: Duration::from_secs(5),
            join_timeout: Duration::from_secs(5),
        }
    }
}

#[derive(Debug, Error)]
enum SapBatchError {
    #[error("failed to dequeue SAP outbox batch")]
    Dequeue {
        #[source]
        source: PulseArcError,
    },
    #[error("SAP batch submission failed")]
    Submit {
        #[source]
        source: PulseArcError,
    },
}

impl SapBatchError {
    fn kind(&self) -> &'static str {
        match self {
            SapBatchError::Dequeue { .. } => "dequeue_failed",
            SapBatchError::Submit { .. } => "submit_failed",
        }
    }
}

/// SAP time entry scheduler with explicit lifecycle management.
pub struct SapScheduler {
    scheduler: Option<JobScheduler>,
    config: SapSchedulerConfig,
    monitor_handle: Option<JoinHandle<()>>,
    cancellation: CancellationToken,
    metrics: Arc<PerformanceMetrics>,
    batch_forwarder: Arc<BatchForwarder>,
    outbox_repo: Arc<dyn OutboxQueuePort>,
}

impl SapScheduler {
    /// Create a scheduler with the default configuration.
    pub fn new<Q>(
        cron_expression: String,
        batch_forwarder: Arc<BatchForwarder>,
        outbox_repo: Arc<Q>,
        metrics: Arc<PerformanceMetrics>,
    ) -> SchedulerResult<Self>
    where
        Q: OutboxQueuePort + 'static,
    {
        let config = SapSchedulerConfig { cron_expression, ..Default::default() };
        Self::with_config(config, batch_forwarder, outbox_repo, metrics)
    }

    /// Create a scheduler with a custom configuration.
    pub fn with_config<Q>(
        config: SapSchedulerConfig,
        batch_forwarder: Arc<BatchForwarder>,
        outbox_repo: Arc<Q>,
        metrics: Arc<PerformanceMetrics>,
    ) -> SchedulerResult<Self>
    where
        Q: OutboxQueuePort + 'static,
    {
        let outbox_repo: Arc<dyn OutboxQueuePort> = outbox_repo;
        let scheduler = Self {
            scheduler: None,
            config,
            monitor_handle: None,
            cancellation: CancellationToken::new(),
            metrics,
            batch_forwarder,
            outbox_repo,
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
        info!(scheduler = "sap", event = "start", "SAP scheduler started");
        log_metric(self.metrics.record_call(), "scheduler.sap.start");
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

        info!(scheduler = "sap", event = "stop", "SAP scheduler stopped");
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
        let batch_forwarder = self.batch_forwarder.clone();
        let outbox_repo = self.outbox_repo.clone();
        let job_timeout = self.config.job_timeout;
        let batch_size = self.config.batch_size;

        let job_definition = Job::new_async(cron_expr.as_str(), move |_id, _lock| {
            let metrics = metrics.clone();
            let batch_forwarder = batch_forwarder.clone();
            let outbox_repo = outbox_repo.clone();

            Box::pin(async move {
                log_metric(metrics.record_call(), "scheduler.sap.job.invoked");
                let started = Instant::now();

                match tokio::time::timeout(
                    job_timeout,
                    Self::process_sap_batch(batch_forwarder, outbox_repo, batch_size),
                )
                .await
                {
                    Ok(Ok(())) => {
                        log_metric(
                            metrics.record_fetch_time(started.elapsed()),
                            "scheduler.sap.job.duration",
                        );
                        debug!(
                            scheduler = "sap",
                            event = "job_complete",
                            "SAP batch processing finished successfully"
                        );
                    }
                    Ok(Err(err)) => {
                        log_metric(metrics.record_fetch_error(), "scheduler.sap.job.error");
                        log_metric(
                            metrics.record_fetch_time(started.elapsed()),
                            "scheduler.sap.job.duration",
                        );
                        error!(
                            scheduler = "sap",
                            error = ?err,
                            error_kind = err.kind(),
                            "SAP batch processing failed"
                        );
                    }
                    Err(elapsed) => {
                        log_metric(metrics.record_fetch_timeout(), "scheduler.sap.job.timeout");
                        warn!(
                            scheduler = "sap",
                            event = "job_timeout",
                            timeout_secs = job_timeout.as_secs(),
                            "SAP batch processing timed out"
                        );
                        debug!(
                            scheduler = "sap",
                            event = "job_timeout_details",
                            elapsed = ?elapsed,
                            "Timeout details"
                        );
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

        debug!(cron = %self.config.cron_expression, job_id = %job_id, "Registered SAP batch processing job");
        Ok(scheduler)
    }

    async fn process_sap_batch(
        batch_forwarder: Arc<BatchForwarder>,
        outbox_repo: Arc<dyn OutboxQueuePort>,
        batch_size: usize,
    ) -> Result<(), SapBatchError> {
        // Dequeue pending SAP entries
        let entries = outbox_repo
            .dequeue_batch(batch_size)
            .await
            .map_err(|source| SapBatchError::Dequeue { source })?;

        if entries.is_empty() {
            debug!(
                scheduler = "sap",
                event = "no_pending_entries",
                "No pending SAP entries to process"
            );
            return Ok(());
        }

        info!(
            scheduler = "sap",
            event = "job_started",
            count = entries.len(),
            "Processing SAP batch"
        );

        // Submit batch via BatchForwarder
        let result = batch_forwarder
            .submit_batch(&entries)
            .await
            .map_err(|source| SapBatchError::Submit { source })?;

        info!(
            scheduler = "sap",
            event = "job_finished",
            successful = result.successful,
            failed = result.failed,
            "SAP batch submission completed"
        );

        // Mark entries as sent or failed based on results
        for entry_result in result.entry_results {
            match entry_result.status {
                crate::integrations::sap::EntrySubmissionStatus::Submitted { .. } => {
                    if let Err(e) = outbox_repo.mark_sent(&entry_result.outbox_id).await {
                        warn!(
                            scheduler = "sap",
                            id = %entry_result.outbox_id,
                            error = ?e,
                            "Failed to mark entry as sent"
                        );
                    }
                }
                crate::integrations::sap::EntrySubmissionStatus::Failed { error } => {
                    let error_message = error.to_string();
                    if let Err(e) =
                        outbox_repo.mark_failed(&entry_result.outbox_id, &error_message).await
                    {
                        warn!(
                            scheduler = "sap",
                            id = %entry_result.outbox_id,
                            sap_error = %error_message,
                            error = ?e,
                            "Failed to mark entry as failed"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    async fn monitor_task(cancel: CancellationToken, metrics: Arc<PerformanceMetrics>) {
        tokio::select! {
            _ = cancel.cancelled() => {
                debug!(
                    scheduler = "sap",
                    event = "monitor_cancelled",
                    "SAP scheduler monitor cancelled"
                );
            }
        }

        log_metric(metrics.record_call(), "scheduler.sap.monitor_exit");
    }
}

fn log_metric(result: MetricsResult<()>, metric: &'static str) {
    if let Err(err) = result {
        warn!(
            scheduler = "sap",
            metric = metric,
            error = ?err,
            "Failed to record scheduler metric"
        );
    }
}

impl Drop for SapScheduler {
    fn drop(&mut self) {
        if self.is_running() {
            warn!(
                scheduler = "sap",
                event = "drop_cancel",
                "SapScheduler dropped while running; cancelling tasks"
            );
            self.cancellation.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use pulsearc_core::sap_ports::{
        SapClient as SapClientTrait, SapEntryId, TimeEntry as SapTimeEntry,
    };
    use pulsearc_core::OutboxQueue as OutboxQueuePort;
    use pulsearc_domain::{OutboxStatus, PulseArcError, Result as DomainResult, TimeEntryOutbox};

    use super::*;

    // Mock SAP client for testing
    #[derive(Clone)]
    struct MockSapClient {
        call_count: Arc<AtomicUsize>,
    }

    impl MockSapClient {
        fn new() -> Self {
            Self { call_count: Arc::new(AtomicUsize::new(0)) }
        }

        fn calls(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    fn sample_outbox_entry(id: &str) -> TimeEntryOutbox {
        TimeEntryOutbox {
            id: id.to_string(),
            idempotency_key: format!("idem-{id}"),
            user_id: "user-123".to_string(),
            payload_json: serde_json::json!({
                "duration": 3600,
                "note": "Work session",
                "wbs_code": "WBS-123",
                "date": "2025-01-01"
            })
            .to_string(),
            backend_cuid: None,
            status: OutboxStatus::Pending,
            attempts: 0,
            last_error: None,
            retry_after: None,
            created_at: 1_735_000_000, // arbitrary timestamp
            sent_at: None,
            correlation_id: None,
            local_status: None,
            remote_status: None,
            sap_entry_id: None,
            next_attempt_at: None,
            error_code: None,
            last_forwarded_at: None,
            wbs_code: Some("WBS-123".to_string()),
            target: "sap".to_string(),
            description: Some("Work session".to_string()),
            auto_applied: false,
            version: 1,
            last_modified_by: "tester".to_string(),
            last_modified_at: None,
        }
    }

    #[async_trait]
    impl SapClientTrait for MockSapClient {
        async fn forward_entry(&self, _entry: &SapTimeEntry) -> DomainResult<SapEntryId> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok("test-id".to_string())
        }

        async fn validate_wbs(&self, _wbs_code: &str) -> DomainResult<bool> {
            Ok(true)
        }
    }

    // Type alias to avoid complexity warning
    type FailedEntries = Arc<tokio::sync::Mutex<Vec<(String, String)>>>;

    // Mock outbox repository for testing
    struct MockOutboxRepo {
        entries: Arc<tokio::sync::Mutex<Vec<TimeEntryOutbox>>>,
        sent: Arc<tokio::sync::Mutex<Vec<String>>>,
        failed: FailedEntries,
    }

    impl MockOutboxRepo {
        fn new() -> Self {
            Self {
                entries: Arc::new(tokio::sync::Mutex::new(Vec::new())),
                sent: Arc::new(tokio::sync::Mutex::new(Vec::new())),
                failed: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            }
        }

        async fn add_entries(&self, new_entries: Vec<TimeEntryOutbox>) {
            let mut entries = self.entries.lock().await;
            entries.extend(new_entries);
        }

        async fn sent_entries(&self) -> Vec<String> {
            self.sent.lock().await.clone()
        }

        async fn failed_entries(&self) -> Vec<(String, String)> {
            self.failed.lock().await.clone()
        }
    }

    #[async_trait]
    impl OutboxQueuePort for MockOutboxRepo {
        async fn enqueue(&self, entry: &TimeEntryOutbox) -> DomainResult<()> {
            self.entries.lock().await.push(entry.clone());
            Ok(())
        }

        async fn dequeue_batch(&self, limit: usize) -> DomainResult<Vec<TimeEntryOutbox>> {
            let mut entries = self.entries.lock().await;
            let drain_count = limit.min(entries.len());
            let batch: Vec<_> = entries.drain(..drain_count).collect();
            Ok(batch)
        }

        async fn mark_sent(&self, _id: &str) -> DomainResult<()> {
            self.sent.lock().await.push(_id.to_string());
            Ok(())
        }

        async fn mark_failed(&self, _id: &str, _error: &str) -> DomainResult<()> {
            self.failed.lock().await.push((_id.to_string(), _error.to_string()));
            Ok(())
        }
    }

    fn fast_config() -> SapSchedulerConfig {
        SapSchedulerConfig {
            cron_expression: "*/1 * * * * *".into(), // every second
            batch_size: 10,
            job_timeout: Duration::from_secs(2),
            start_timeout: Duration::from_secs(2),
            stop_timeout: Duration::from_secs(2),
            join_timeout: Duration::from_secs(2),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn lifecycle_runs_successfully() {
        let metrics = Arc::new(PerformanceMetrics::new());
        let client: Arc<dyn SapClientTrait> = Arc::new(MockSapClient::new());
        let batch_forwarder = Arc::new(BatchForwarder::new(client));
        let outbox_repo = Arc::new(MockOutboxRepo::new());

        let mut scheduler =
            SapScheduler::with_config(fast_config(), batch_forwarder, outbox_repo, metrics)
                .expect("scheduler created");

        scheduler.start().await.expect("start succeeds");
        tokio::time::sleep(Duration::from_secs(2)).await;
        scheduler.stop().await.expect("stop succeeds");

        assert!(!scheduler.is_running());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn double_start_is_rejected() {
        let metrics = Arc::new(PerformanceMetrics::new());
        let client: Arc<dyn SapClientTrait> = Arc::new(MockSapClient::new());
        let batch_forwarder = Arc::new(BatchForwarder::new(client));
        let outbox_repo = Arc::new(MockOutboxRepo::new());

        let mut scheduler =
            SapScheduler::with_config(fast_config(), batch_forwarder, outbox_repo, metrics)
                .expect("scheduler created");

        scheduler.start().await.expect("first start");
        let err = scheduler.start().await.expect_err("second start fails");
        assert!(matches!(err, SchedulerError::AlreadyRunning));
        scheduler.stop().await.expect("stop succeeds");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn restart_after_stop_succeeds() {
        let metrics = Arc::new(PerformanceMetrics::new());
        let client: Arc<dyn SapClientTrait> = Arc::new(MockSapClient::new());
        let batch_forwarder = Arc::new(BatchForwarder::new(client));
        let outbox_repo = Arc::new(MockOutboxRepo::new());

        let mut scheduler =
            SapScheduler::with_config(fast_config(), batch_forwarder, outbox_repo, metrics)
                .expect("scheduler created");

        scheduler.start().await.expect("start succeeds");
        scheduler.stop().await.expect("stop succeeds");
        assert!(!scheduler.is_running());

        scheduler.start().await.expect("start again");
        scheduler.stop().await.expect("stop again");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn process_batch_marks_entries_as_sent() {
        let mock_client = Arc::new(MockSapClient::new());
        let client: Arc<dyn SapClientTrait> = mock_client.clone();
        let batch_forwarder = Arc::new(BatchForwarder::new(client));
        let outbox_repo = Arc::new(MockOutboxRepo::new());

        outbox_repo
            .add_entries(vec![sample_outbox_entry("entry-1"), sample_outbox_entry("entry-2")])
            .await;

        SapScheduler::process_sap_batch(batch_forwarder, outbox_repo.clone(), 10)
            .await
            .expect("batch processing succeeds");

        assert_eq!(mock_client.calls(), 2);
        let sent = outbox_repo.sent_entries().await;
        assert_eq!(sent, vec!["entry-1".to_string(), "entry-2".to_string()]);
        assert!(outbox_repo.failed_entries().await.is_empty());
    }

    #[test]
    fn sap_batch_error_kind_reports_variants() {
        let dequeue = SapBatchError::Dequeue { source: PulseArcError::Internal("boom".into()) };
        let submit = SapBatchError::Submit { source: PulseArcError::Internal("zap".into()) };

        assert_eq!(dequeue.kind(), "dequeue_failed");
        assert_eq!(submit.kind(), "submit_failed");
    }

    struct FailingOutboxRepo;

    #[async_trait]
    impl OutboxQueuePort for FailingOutboxRepo {
        async fn enqueue(&self, _entry: &TimeEntryOutbox) -> DomainResult<()> {
            Ok(())
        }

        async fn dequeue_batch(&self, _limit: usize) -> DomainResult<Vec<TimeEntryOutbox>> {
            Err(PulseArcError::Internal("forced failure".into()))
        }

        async fn mark_sent(&self, _id: &str) -> DomainResult<()> {
            Ok(())
        }

        async fn mark_failed(&self, _id: &str, _error: &str) -> DomainResult<()> {
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn process_batch_propagates_dequeue_error() {
        let client: Arc<dyn SapClientTrait> = Arc::new(MockSapClient::new());
        let batch_forwarder = Arc::new(BatchForwarder::new(client));
        let err = SapScheduler::process_sap_batch(batch_forwarder, Arc::new(FailingOutboxRepo), 5)
            .await
            .expect_err("process should surface dequeue error");
        assert!(matches!(err, SapBatchError::Dequeue { .. }));
        assert_eq!(err.kind(), "dequeue_failed");
    }
}
