//! Outbox worker for periodic batch processing and forwarding.
//!
//! Polls the SQLCipher-backed outbox queue for pending time entries, forwards
//! each entry to the Neon domain API, and updates local outbox status based on
//! the outcome. The implementation follows the runtime rules captured in
//! `CLAUDE.md`: join handles are tracked, cancellation is explicit, and every
//! asynchronous operation is wrapped in a timeout. SAP-bound entries are
//! handled by the dedicated `SapScheduler`.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! use pulsearc_infra::observability::metrics::PerformanceMetrics;
//! use pulsearc_infra::sync::{OutboxWorker, OutboxWorkerConfig};
//!
//! # async fn example() -> Result<(), String> {
//! let metrics = Arc::new(PerformanceMetrics::new());
//! // ... create outbox_repo and Neon client ...
//! # let outbox_repo = todo!(); // Arc<dyn OutboxQueue>
//! # let neon_client = todo!(); // Arc<NeonClient>
//! let mut worker = OutboxWorker::new(
//!     outbox_repo,
//!     neon_client,
//!     OutboxWorkerConfig {
//!         batch_size: 50,
//!         poll_interval: Duration::from_secs(60),
//!         ..Default::default()
//!     },
//!     metrics.clone(),
//! );
//!
//! worker.start().await?;
//! // ... application runs ...
//! worker.stop().await?;
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use pulsearc_core::OutboxQueue;
use pulsearc_domain::types::{PrismaTimeEntryDto, TimeEntryOutbox};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

use crate::observability::metrics::PerformanceMetrics;
use crate::observability::MetricsResult;
use crate::sync::errors::SyncError;
use crate::sync::neon_client::NeonClient;

/// Configuration for the outbox worker.
#[derive(Debug, Clone)]
pub struct OutboxWorkerConfig {
    /// Maximum number of entries to process per batch
    pub batch_size: usize,
    /// Interval between polling attempts
    pub poll_interval: Duration,
    /// Timeout for processing a single batch
    pub processing_timeout: Duration,
    /// Maximum retry attempts before marking as permanently failed
    pub max_retries: usize,
    /// Join timeout when stopping
    pub join_timeout: Duration,
}

impl Default for OutboxWorkerConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            poll_interval: Duration::from_secs(60),
            processing_timeout: Duration::from_secs(300),
            max_retries: 3,
            join_timeout: Duration::from_secs(5),
        }
    }
}

/// Interface for submitting time entries to a remote destination.
#[async_trait]
pub trait TimeEntryForwarder: Send + Sync {
    /// Forward a time entry payload using the provided idempotency key.
    async fn forward_time_entry(
        &self,
        dto: &PrismaTimeEntryDto,
        idempotency_key: &str,
    ) -> Result<String, SyncError>;
}

#[async_trait]
impl TimeEntryForwarder for NeonClient {
    async fn forward_time_entry(
        &self,
        dto: &PrismaTimeEntryDto,
        idempotency_key: &str,
    ) -> Result<String, SyncError> {
        self.submit_time_entry(dto, idempotency_key).await
    }
}

/// Outbox worker with explicit lifecycle management.
pub struct OutboxWorker {
    outbox_repo: Arc<dyn OutboxQueue>,
    forwarder: Arc<dyn TimeEntryForwarder>,
    config: OutboxWorkerConfig,
    cancellation: CancellationToken,
    task_handle: Option<JoinHandle<()>>,
    metrics: Arc<PerformanceMetrics>,
}

impl OutboxWorker {
    /// Create a new outbox worker with the given configuration.
    pub fn new(
        outbox_repo: Arc<dyn OutboxQueue>,
        forwarder: Arc<dyn TimeEntryForwarder>,
        config: OutboxWorkerConfig,
        metrics: Arc<PerformanceMetrics>,
    ) -> Self {
        Self {
            outbox_repo,
            forwarder,
            config,
            cancellation: CancellationToken::new(),
            task_handle: None,
            metrics,
        }
    }

    /// Start the worker, spawning the background processing task.
    #[instrument(skip(self))]
    pub async fn start(&mut self) -> Result<(), String> {
        if self.is_running() {
            return Err("Worker already running".to_string());
        }

        info!("Starting outbox worker");

        // Create fresh cancellation token
        self.cancellation = CancellationToken::new();

        let outbox_repo = Arc::clone(&self.outbox_repo);
        let forwarder = Arc::clone(&self.forwarder);
        let poll_interval = self.config.poll_interval;
        let batch_size = self.config.batch_size;
        let processing_timeout = self.config.processing_timeout;
        let cancel = self.cancellation.clone();
        let metrics = Arc::clone(&self.metrics);

        let handle = tokio::spawn(async move {
            Self::process_loop(
                outbox_repo,
                forwarder,
                poll_interval,
                batch_size,
                processing_timeout,
                cancel,
                metrics,
            )
            .await;
        });

        self.task_handle = Some(handle);
        info!("Outbox worker started");
        log_metric(self.metrics.record_call(), "outbox_worker.start");

        Ok(())
    }

    /// Stop the worker and wait for the processing task to finish.
    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> Result<(), String> {
        if !self.is_running() {
            return Err("Worker not running".to_string());
        }

        info!("Stopping outbox worker");

        // Cancel background task
        self.cancellation.cancel();

        // Await join handle with timeout
        if let Some(handle) = self.task_handle.take() {
            let join_timeout = self.config.join_timeout;
            match tokio::time::timeout(join_timeout, handle).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    warn!("Worker task panicked: {}", e);
                    return Err("Worker task panicked".to_string());
                }
                Err(_) => {
                    warn!("Worker task did not complete within timeout");
                    return Err("Worker task timeout".to_string());
                }
            }
        }

        info!("Outbox worker stopped");
        self.cancellation = CancellationToken::new();
        log_metric(self.metrics.record_call(), "outbox_worker.stop");

        Ok(())
    }

    /// Returns true when a worker instance is active.
    pub fn is_running(&self) -> bool {
        self.task_handle.is_some()
    }

    /// Background processing loop.
    #[allow(clippy::too_many_arguments)]
    async fn process_loop(
        outbox_repo: Arc<dyn OutboxQueue>,
        forwarder: Arc<dyn TimeEntryForwarder>,
        poll_interval: Duration,
        batch_size: usize,
        processing_timeout: Duration,
        cancel: CancellationToken,
        metrics: Arc<PerformanceMetrics>,
    ) {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!("Outbox worker process loop cancelled");
                    break;
                }
                _ = tokio::time::sleep(poll_interval) => {
                    log_metric(metrics.record_call(), "outbox_worker.tick");
                    let started = Instant::now();

                    match tokio::time::timeout(
                        processing_timeout,
                        Self::process_batch(&outbox_repo, &forwarder, batch_size, &metrics),
                    )
                    .await
                    {
                        Ok(Ok(())) => {
                            log_metric(
                                metrics.record_fetch_time(started.elapsed()),
                                "outbox_worker.batch.duration",
                            );
                        }
                        Ok(Err(e)) => {
                            error!(error = %e, "Batch processing failed");
                            log_metric(metrics.record_fetch_error(), "outbox_worker.batch.error");
                            log_metric(
                                metrics.record_fetch_time(started.elapsed()),
                                "outbox_worker.batch.duration",
                            );
                        }
                        Err(_) => {
                            warn!(timeout_secs = processing_timeout.as_secs(), "Batch processing timed out");
                            log_metric(metrics.record_fetch_timeout(), "outbox_worker.batch.timeout");
                        }
                    }
                }
            }
        }
    }

    /// Process a single batch of outbox entries.
    async fn process_batch(
        outbox_repo: &Arc<dyn OutboxQueue>,
        forwarder: &Arc<dyn TimeEntryForwarder>,
        batch_size: usize,
        metrics: &Arc<PerformanceMetrics>,
    ) -> Result<(), String> {
        // Dequeue pending entries (status = 'pending' and past retry window)
        let entries = outbox_repo
            .dequeue_batch(batch_size)
            .await
            .map_err(|e| format!("Failed to dequeue batch: {e}"))?;

        if entries.is_empty() {
            debug!("No pending entries to process");
            return Ok(());
        }

        info!(count = entries.len(), "Processing outbox batch");

        let mut fatal_errors: Vec<String> = Vec::new();
        let mut forwarded = 0_u32;
        let mut failures = 0_u32;
        let mut skipped = 0_u32;

        for entry in entries {
            if entry.target.eq_ignore_ascii_case("sap") {
                debug!(entry_id = %entry.id, "Skipping SAP-target outbox entry");
                skipped = skipped.saturating_add(1);
                continue;
            }

            let dto = match parse_time_entry(&entry) {
                Ok(dto) => dto,
                Err(err) => {
                    warn!(
                        entry_id = %entry.id,
                        error = %err,
                        "Failed to parse outbox payload"
                    );
                    if let Err(mark_err) =
                        outbox_repo.mark_failed(&entry.id, &truncate_reason(&err.to_string())).await
                    {
                        let msg = mark_err.to_string();
                        warn!(entry_id = %entry.id, error = %msg, "mark_failed failed");
                        fatal_errors.push(format!("mark_failed error for {}: {}", entry.id, msg));
                    }
                    failures = failures.saturating_add(1);
                    continue;
                }
            };

            match forwarder.forward_time_entry(&dto, &entry.idempotency_key).await {
                Ok(remote_id) => {
                    debug!(
                        entry_id = %entry.id,
                        remote_id = %remote_id,
                        "Forwarded outbox entry"
                    );
                    if let Err(err) = outbox_repo.mark_sent(&entry.id).await {
                        let msg = err.to_string();
                        warn!(entry_id = %entry.id, error = %msg, "mark_sent failed");
                        fatal_errors.push(format!("mark_sent error for {}: {}", entry.id, msg));
                    } else {
                        forwarded = forwarded.saturating_add(1);
                    }
                }
                Err(err) => {
                    warn!(
                        entry_id = %entry.id,
                        error = ?err,
                        "Forwarding outbox entry failed"
                    );
                    if let Err(mark_err) =
                        outbox_repo.mark_failed(&entry.id, &truncate_reason(&err.to_string())).await
                    {
                        let msg = mark_err.to_string();
                        warn!(entry_id = %entry.id, error = %msg, "mark_failed failed");
                        fatal_errors.push(format!("mark_failed error for {}: {}", entry.id, msg));
                    }
                    failures = failures.saturating_add(1);
                }
            }
        }

        log_metric(metrics.record_call(), "outbox_worker.batch.processed");
        debug!(
            forwarded = forwarded,
            failures = failures,
            skipped = skipped,
            "Outbox batch completed"
        );

        if failures > 0 {
            log_metric(metrics.record_fetch_error(), "outbox_worker.batch.failure_count");
        }

        if !fatal_errors.is_empty() {
            return Err(fatal_errors.join("; "));
        }

        Ok(())
    }
}

fn parse_time_entry(entry: &TimeEntryOutbox) -> Result<PrismaTimeEntryDto, serde_json::Error> {
    serde_json::from_str(&entry.payload_json)
}

fn truncate_reason(reason: &str) -> String {
    const MAX_LEN: usize = 256;
    if reason.len() <= MAX_LEN {
        return reason.to_string();
    }

    let mut truncated = reason.chars().take(MAX_LEN.saturating_sub(3)).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn log_metric(result: MetricsResult<()>, metric: &'static str) {
    if let Err(err) = result {
        warn!(metric = metric, error = ?err, "Failed to record worker metric");
    }
}

impl Drop for OutboxWorker {
    fn drop(&mut self) {
        if self.is_running() {
            warn!("OutboxWorker dropped while running; cancelling tasks");
            self.cancellation.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use pulsearc_core::OutboxQueue;
    use pulsearc_domain::{OutboxStatus, PulseArcError, Result as DomainResult};
    use tokio::sync::Mutex as TokioMutex;

    use super::*;

    type EntryStore = Arc<TokioMutex<Vec<TimeEntryOutbox>>>;
    type SentStore = Arc<TokioMutex<Vec<String>>>;
    type FailedStore = Arc<TokioMutex<Vec<(String, String)>>>;
    type ResponseQueue = TokioMutex<Vec<Result<String, SyncError>>>;
    type CallStore = Arc<TokioMutex<Vec<PrismaTimeEntryDto>>>;

    fn sample_time_entry_dto() -> PrismaTimeEntryDto {
        PrismaTimeEntryDto {
            id: None,
            org_id: "org-123".to_string(),
            project_id: "proj-123".to_string(),
            task_id: Some("task-456".to_string()),
            user_id: "user-123".to_string(),
            entry_date: "2025-01-01".to_string(),
            duration_minutes: 60,
            notes: Some("Work session".to_string()),
            billable: Some(true),
            source: "pulsearc".to_string(),
            status: Some("pending".to_string()),
            start_time: Some("2025-01-01T09:00:00Z".to_string()),
            end_time: Some("2025-01-01T10:00:00Z".to_string()),
            duration_sec: Some(3_600),
            display_project: Some("Project Alpha".to_string()),
            display_workstream: Some("analysis".to_string()),
            display_task: None,
            confidence: Some(0.9),
            context_breakdown: None,
            wbs_code: Some("WBS-123".to_string()),
        }
    }

    fn sample_outbox_entry(id: &str) -> TimeEntryOutbox {
        TimeEntryOutbox {
            id: id.to_string(),
            idempotency_key: format!("idem-{id}"),
            user_id: "user-123".to_string(),
            payload_json: serde_json::to_string(&sample_time_entry_dto()).unwrap(),
            backend_cuid: None,
            status: OutboxStatus::Pending,
            attempts: 0,
            last_error: None,
            retry_after: None,
            created_at: 1_735_000_000,
            sent_at: None,
            correlation_id: None,
            local_status: None,
            remote_status: None,
            sap_entry_id: None,
            next_attempt_at: None,
            error_code: None,
            last_forwarded_at: None,
            wbs_code: Some("WBS-123".to_string()),
            target: "neon".to_string(),
            description: Some("Work session".to_string()),
            auto_applied: true,
            version: 1,
            last_modified_by: "tester".to_string(),
            last_modified_at: None,
        }
    }

    struct MockOutboxRepo {
        entries: EntryStore,
        sent: SentStore,
        failed: FailedStore,
        fail_mark_sent: bool,
        fail_mark_failed: bool,
    }

    impl MockOutboxRepo {
        fn new(entries: Vec<TimeEntryOutbox>) -> Self {
            Self {
                entries: Arc::new(TokioMutex::new(entries)),
                sent: Arc::new(TokioMutex::new(Vec::new())),
                failed: Arc::new(TokioMutex::new(Vec::new())),
                fail_mark_sent: false,
                fail_mark_failed: false,
            }
        }

        fn with_fail_mark_sent(mut self) -> Self {
            self.fail_mark_sent = true;
            self
        }

        fn with_fail_mark_failed(mut self) -> Self {
            self.fail_mark_failed = true;
            self
        }

        async fn sent_entries(&self) -> Vec<String> {
            self.sent.lock().await.clone()
        }

        async fn failed_entries(&self) -> Vec<(String, String)> {
            self.failed.lock().await.clone()
        }
    }

    #[async_trait]
    impl OutboxQueue for MockOutboxRepo {
        async fn enqueue(&self, entry: &TimeEntryOutbox) -> DomainResult<()> {
            self.entries.lock().await.push(entry.clone());
            Ok(())
        }

        async fn dequeue_batch(&self, limit: usize) -> DomainResult<Vec<TimeEntryOutbox>> {
            let mut entries = self.entries.lock().await;
            let batch_len = limit.min(entries.len());
            let batch: Vec<_> = entries.drain(..batch_len).collect();
            Ok(batch)
        }

        async fn mark_sent(&self, id: &str) -> DomainResult<()> {
            if self.fail_mark_sent {
                return Err(PulseArcError::Internal("mark_sent failure".into()));
            }
            self.sent.lock().await.push(id.to_string());
            Ok(())
        }

        async fn mark_failed(&self, id: &str, error: &str) -> DomainResult<()> {
            if self.fail_mark_failed {
                return Err(PulseArcError::Internal("mark_failed failure".into()));
            }
            self.failed.lock().await.push((id.to_string(), error.to_string()));
            Ok(())
        }
    }

    struct MockForwarder {
        responses: ResponseQueue,
        calls: CallStore,
    }

    impl MockForwarder {
        fn new(responses: Vec<Result<String, SyncError>>) -> Self {
            Self {
                responses: TokioMutex::new(responses),
                calls: Arc::new(TokioMutex::new(Vec::new())),
            }
        }

        async fn call_count(&self) -> usize {
            self.calls.lock().await.len()
        }
    }

    #[async_trait]
    impl TimeEntryForwarder for MockForwarder {
        async fn forward_time_entry(
            &self,
            dto: &PrismaTimeEntryDto,
            _idempotency_key: &str,
        ) -> Result<String, SyncError> {
            self.calls.lock().await.push(dto.clone());
            let mut responses = self.responses.lock().await;
            if responses.is_empty() {
                Ok("remote-id".to_string())
            } else {
                responses.remove(0)
            }
        }
    }

    #[tokio::test]
    async fn process_batch_marks_sent_on_success() {
        let repo = Arc::new(MockOutboxRepo::new(vec![sample_outbox_entry("entry-1")]));
        let repo_trait: Arc<dyn OutboxQueue> = repo.clone();
        let forwarder = Arc::new(MockForwarder::new(vec![Ok("remote-1".to_string())]));
        let forwarder_trait: Arc<dyn TimeEntryForwarder> = forwarder.clone();
        let metrics = Arc::new(PerformanceMetrics::new());

        let result = OutboxWorker::process_batch(&repo_trait, &forwarder_trait, 10, &metrics).await;
        assert!(result.is_ok());

        let sent = repo.sent_entries().await;
        assert_eq!(sent, vec!["entry-1".to_string()]);
        assert_eq!(forwarder.call_count().await, 1);
    }

    #[tokio::test]
    async fn process_batch_marks_failed_on_parse_error() {
        let mut entry = sample_outbox_entry("entry-parse");
        entry.payload_json = "{invalid json}".to_string();

        let repo = Arc::new(MockOutboxRepo::new(vec![entry]));
        let repo_trait: Arc<dyn OutboxQueue> = repo.clone();
        let forwarder = Arc::new(MockForwarder::new(vec![]));
        let forwarder_trait: Arc<dyn TimeEntryForwarder> = forwarder.clone();
        let metrics = Arc::new(PerformanceMetrics::new());

        let result = OutboxWorker::process_batch(&repo_trait, &forwarder_trait, 5, &metrics).await;
        assert!(result.is_ok());

        let failed = repo.failed_entries().await;
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].0, "entry-parse");
        assert!(failed[0].1.contains("Invalid"));
    }

    #[tokio::test]
    async fn process_batch_skips_sap_entries() {
        let mut entry = sample_outbox_entry("entry-sap");
        entry.target = "sap".to_string();

        let repo = Arc::new(MockOutboxRepo::new(vec![entry]));
        let repo_trait: Arc<dyn OutboxQueue> = repo.clone();
        let forwarder = Arc::new(MockForwarder::new(vec![Ok("remote".to_string())]));
        let forwarder_trait: Arc<dyn TimeEntryForwarder> = forwarder.clone();
        let metrics = Arc::new(PerformanceMetrics::new());

        let result = OutboxWorker::process_batch(&repo_trait, &forwarder_trait, 5, &metrics).await;
        assert!(result.is_ok());

        assert!(repo.sent_entries().await.is_empty());
        assert!(repo.failed_entries().await.is_empty());
        assert_eq!(forwarder.call_count().await, 0);
    }

    #[tokio::test]
    async fn process_batch_propagates_mark_sent_failures() {
        let repo = Arc::new(
            MockOutboxRepo::new(vec![sample_outbox_entry("entry-fail")]).with_fail_mark_sent(),
        );
        let repo_trait: Arc<dyn OutboxQueue> = repo.clone();
        let forwarder = Arc::new(MockForwarder::new(vec![Ok("remote".to_string())]));
        let forwarder_trait: Arc<dyn TimeEntryForwarder> = forwarder.clone();
        let metrics = Arc::new(PerformanceMetrics::new());

        let result = OutboxWorker::process_batch(&repo_trait, &forwarder_trait, 5, &metrics).await;
        assert!(result.is_err());
        assert!(repo.sent_entries().await.is_empty());
    }

    #[tokio::test]
    async fn process_batch_propagates_mark_failed_errors() {
        let repo = Arc::new(
            MockOutboxRepo::new(vec![sample_outbox_entry("entry-mark-failed")])
                .with_fail_mark_failed(),
        );
        let repo_trait: Arc<dyn OutboxQueue> = repo.clone();
        let forwarder =
            Arc::new(MockForwarder::new(vec![Err(SyncError::Server("server boom".into()))]));
        let forwarder_trait: Arc<dyn TimeEntryForwarder> = forwarder.clone();
        let metrics = Arc::new(PerformanceMetrics::new());

        let result = OutboxWorker::process_batch(&repo_trait, &forwarder_trait, 5, &metrics).await;
        assert!(result.is_err());
        assert!(repo.failed_entries().await.is_empty());
    }
}
