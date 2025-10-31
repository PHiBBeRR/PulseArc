//! SAP time entry forwarder with batch processing and retry logic.
//!
//! This module provides:
//! - Pure data conversion functions (easily testable)
//! - Batch processing with exponential backoff
//! - Circuit breaker integration for fault tolerance
//! - Structured tracing for observability
//!
//! # Architecture
//!
//! Following the worker pattern for testability:
//! - `SapForwarder`: Pure converter (no async, no side effects)
//! - `BatchForwarder`: Async batch submission with retry logic
//!
//! # Usage
//!
//! ```no_run
//! use std::sync::Arc;
//! use pulsearc_infra::integrations::sap::{SapForwarder, BatchForwarder, SapClient};
//! use pulsearc_domain::TimeEntryOutbox;
//!
//! # async fn example(client: Arc<SapClient>, entries: Vec<TimeEntryOutbox>) -> pulsearc_domain::Result<()> {
//! // Pure conversion (testable without async)
//! let converter = SapForwarder::new();
//! let prepared = converter.prepare_batch(&entries);
//! for entry in &prepared {
//!     match &entry.result {
//!         Ok(sap_entry) => println!("Ready to submit {}", sap_entry.wbs_code),
//!         Err(err) => eprintln!("Conversion failed for {}: {err}", entry.outbox_id),
//!     }
//! }
//!
//! // Batch submission with retry
//! let batch_forwarder = BatchForwarder::new(client);
//! let results = batch_forwarder.submit_batch(&entries).await?;
//! for outcome in results.entry_results {
//!     match outcome.status {
//!         EntrySubmissionStatus::Submitted { sap_entry_id } => println!("Sent {}", sap_entry_id),
//!         EntrySubmissionStatus::Failed { error } => eprintln!("Failed {}: {}", outcome.outbox_id, error),
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use pulsearc_common::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use pulsearc_core::sap_ports::{
    SapClient as SapClientTrait, SapEntryId, TimeEntry as SapTimeEntry,
};
use pulsearc_domain::{PulseArcError, Result, TimeEntryOutbox};
use serde_json::Value;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use super::errors::{SapError, SapErrorCategory};
use super::validation::normalize_wbs_code;

/// Pure converter for outbox entries to SAP time entries.
///
/// This struct contains no async methods or external dependencies,
/// making it trivial to test without mocking or sleeping.
pub struct SapForwarder;

impl SapForwarder {
    /// Create a new forwarder instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Prepare a batch of SAP time entries from outbox records.
    ///
    /// This is a pure function that attempts to map each outbox entry to an SAP time
    /// entry, returning the conversion result per entry. Conversion failures are logged
    /// and surfaced to callers via the returned `PreparedEntry`.
    ///
    /// # Arguments
    ///
    /// * `entries` - Outbox entries to convert
    ///
    /// # Returns
    ///
    /// Vector of per-entry conversion results.
    pub fn prepare_batch(&self, entries: &[TimeEntryOutbox]) -> Vec<PreparedEntry> {
        entries
            .iter()
            .map(|entry| {
                let result = self.prepare_entry(entry);

                if let Err(ref err) = result {
                    warn!(
                        entry_id = %entry.id,
                        error = %err,
                        "Failed to prepare entry for SAP submission"
                    );
                }

                PreparedEntry { outbox_id: entry.id.clone(), result }
            })
            .collect()
    }

    /// Prepare an SAP time entry from an outbox record.
    ///
    /// The forwarder prefers explicit values from `payload_json` but gracefully
    /// falls back to the outbox record (or sensible defaults) to avoid the
    /// legacy anti-patterns documented in Phase 3 pre-migration fixes.
    pub fn prepare_entry(&self, entry: &TimeEntryOutbox) -> Result<SapTimeEntry> {
        let payload = Self::parse_payload(entry);

        let date = self.resolve_date(entry, &payload);
        let duration_hours = payload
            .get("duration")
            .and_then(Value::as_f64)
            .map(|seconds| (seconds / 3600.0) as f32)
            .unwrap_or(0.0);

        let description = payload
            .get("note")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| entry.description.clone())
            .unwrap_or_default();

        let payload_wbs = payload.get("wbs_code").and_then(Value::as_str).map(normalize_wbs_code);

        let entry_wbs = entry.wbs_code.as_deref().map(normalize_wbs_code);

        let wbs_code =
            payload_wbs.or(entry_wbs).filter(|wbs| !wbs.is_empty()).ok_or_else(|| {
                PulseArcError::InvalidInput(format!("Outbox entry {} missing WBS code", entry.id))
            })?;

        Ok(SapTimeEntry { wbs_code, description, duration_hours, date })
    }

    fn parse_payload(entry: &TimeEntryOutbox) -> Value {
        serde_json::from_str(&entry.payload_json).unwrap_or_else(|err| {
            warn!(
                entry_id = %entry.id,
                error = %err,
                "failed to parse payload_json; defaulting to empty object"
            );
            Value::Null
        })
    }

    fn resolve_date(&self, entry: &TimeEntryOutbox, payload: &Value) -> String {
        if let Some(date) = payload.get("date").and_then(Value::as_str) {
            return date.to_string();
        }

        self.derive_date_from_created_at(entry)
    }

    fn derive_date_from_created_at(&self, entry: &TimeEntryOutbox) -> String {
        if let Some(created_at) = DateTime::<Utc>::from_timestamp(entry.created_at, 0) {
            let derived = created_at.format("%Y-%m-%d").to_string();
            warn!(
                entry_id = %entry.id,
                derived_date = %derived,
                "missing date field; deriving from created_at timestamp"
            );
            derived
        } else {
            let now = Utc::now();
            let derived = now.format("%Y-%m-%d").to_string();
            warn!(
                entry_id = %entry.id,
                fallback_date = %derived,
                "missing date field and invalid created_at; falling back to current date"
            );
            derived
        }
    }
}

fn map_conversion_error(err: PulseArcError, outbox_id: &str) -> SapError {
    map_pulsearc_error(err, outbox_id, "conversion")
}

fn map_submission_error(err: PulseArcError, outbox_id: &str) -> SapError {
    map_pulsearc_error(err, outbox_id, "submission")
}

fn map_pulsearc_error(err: PulseArcError, outbox_id: &str, stage: &str) -> SapError {
    let category = match &err {
        PulseArcError::InvalidInput(_) | PulseArcError::Config(_) => SapErrorCategory::Validation,
        PulseArcError::Network(_) => SapErrorCategory::NetworkOffline,
        PulseArcError::Auth(_) => SapErrorCategory::Authentication,
        _ => SapErrorCategory::Unknown,
    };

    let message = err.to_string();
    SapError::new(category, message).with_context(format!("outbox_id={outbox_id}, stage={stage}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::sap::SapErrorCategory;
    use async_trait::async_trait;
    use pulsearc_core::sap_ports::SapClient as SapClientTrait;
    use pulsearc_domain::{OutboxStatus, PulseArcError, TimeEntryOutbox};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    fn base_outbox_entry(id: &str) -> TimeEntryOutbox {
        TimeEntryOutbox {
            id: id.to_string(),
            idempotency_key: format!("{}-key", id),
            user_id: "user-123".to_string(),
            payload_json: r#"{"duration":3600,"note":"Work","date":"2025-10-31"}"#.to_string(),
            backend_cuid: None,
            status: OutboxStatus::Pending,
            attempts: 0,
            last_error: None,
            retry_after: None,
            created_at: 1_700_000_000,
            sent_at: None,
            correlation_id: None,
            local_status: None,
            remote_status: None,
            sap_entry_id: None,
            next_attempt_at: None,
            error_code: None,
            last_forwarded_at: None,
            wbs_code: Some("USC0063201.1.1".to_string()),
            target: "sap".to_string(),
            description: Some("Work item".to_string()),
            auto_applied: false,
            version: 1,
            last_modified_by: "user-123".to_string(),
            last_modified_at: None,
        }
    }

    #[test]
    fn prepare_entry_requires_wbs_code() {
        let forwarder = SapForwarder::new();
        let mut entry = base_outbox_entry("missing-wbs");
        entry.wbs_code = None;
        entry.payload_json = "{}".to_string();

        let result = forwarder.prepare_entry(&entry);
        assert!(matches!(result, Err(PulseArcError::InvalidInput(_))));
    }

    struct MockSapClient {
        responses: Mutex<VecDeque<Result<SapEntryId>>>,
    }

    impl MockSapClient {
        fn new(responses: Vec<Result<SapEntryId>>) -> Self {
            Self { responses: Mutex::new(VecDeque::from(responses)) }
        }
    }

    #[async_trait]
    impl SapClientTrait for MockSapClient {
        async fn forward_entry(&self, _entry: &SapTimeEntry) -> Result<SapEntryId> {
            self.responses
                .lock()
                .expect("responses mutex poisoned")
                .pop_front()
                .unwrap_or_else(|| Err(PulseArcError::Internal("no mock response".to_string())))
        }

        async fn validate_wbs(&self, _wbs_code: &str) -> Result<bool> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn submit_batch_returns_per_entry_results() {
        let mut conversion_fail = base_outbox_entry("conv-fail");
        conversion_fail.wbs_code = None;
        conversion_fail.payload_json = "{}".to_string();

        let success_entry = base_outbox_entry("success-1");

        let mut submission_fail_entry = base_outbox_entry("submit-fail");
        submission_fail_entry.payload_json =
            r#"{"duration":1800,"note":"Fail","date":"2025-10-31"}"#.to_string();

        let responses = vec![
            Ok("sap-entry-1".to_string()),
            Err(PulseArcError::Network("network down".to_string())),
        ];
        let client: Arc<dyn SapClientTrait> = Arc::new(MockSapClient::new(responses));
        let forwarder = BatchForwarder::new(client);

        let results = forwarder
            .submit_batch(&[conversion_fail, success_entry, submission_fail_entry])
            .await
            .expect("batch submission should complete");

        assert_eq!(results.entry_results.len(), 3);
        assert_eq!(results.successful, 1);
        assert_eq!(results.failed, 2);

        match &results.entry_results[0].status {
            EntrySubmissionStatus::Failed { error } => {
                assert_eq!(results.entry_results[0].outbox_id, "conv-fail");
                assert_eq!(*error.category(), SapErrorCategory::Validation);
            }
            _ => panic!("expected conversion failure"),
        }

        match &results.entry_results[1].status {
            EntrySubmissionStatus::Submitted { sap_entry_id } => {
                assert_eq!(sap_entry_id, "sap-entry-1");
            }
            _ => panic!("expected submission success"),
        }

        match &results.entry_results[2].status {
            EntrySubmissionStatus::Failed { error } => {
                assert_eq!(results.entry_results[2].outbox_id, "submit-fail");
                assert!(error.category().is_retryable());
            }
            _ => panic!("expected submission failure"),
        }
    }
}

impl Default for SapForwarder {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of preparing an outbox entry for submission.
pub struct PreparedEntry {
    /// Outbox identifier associated with this entry.
    pub outbox_id: String,
    /// Conversion result; `Ok` contains the prepared SAP entry, `Err` contains validation error.
    pub result: Result<SapTimeEntry>,
}

/// Outcome for a single outbox entry submission.
#[derive(Debug, Clone)]
pub struct EntrySubmissionResult {
    /// Identifier of the originating outbox entry.
    pub outbox_id: String,
    /// Submission status, including success or failure metadata.
    pub status: EntrySubmissionStatus,
}

/// Detailed status for a submitted entry.
#[derive(Debug, Clone)]
pub enum EntrySubmissionStatus {
    /// Entry submitted successfully; contains SAP entry identifier.
    Submitted { sap_entry_id: SapEntryId },
    /// Entry failed (either conversion or submission) with classified error.
    Failed { error: SapError },
}

/// Result of batch submission
#[derive(Debug, Clone)]
pub struct BatchSubmissionResult {
    /// Number of entries successfully submitted
    pub successful: usize,
    /// Number of entries that failed
    pub failed: usize,
    /// Detailed per-entry outcomes (success or failure metadata)
    pub entry_results: Vec<EntrySubmissionResult>,
}

/// Configuration for batch retry logic
#[derive(Debug, Clone)]
pub struct BatchRetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Backoff multiplier
    pub multiplier: f64,
    /// Maximum delay between retries
    pub max_delay: Duration,
}

impl Default for BatchRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            multiplier: 2.0,
            max_delay: Duration::from_secs(30),
        }
    }
}

/// Async batch forwarder with retry logic and circuit breaker.
///
/// This worker handles the async submission of time entries to SAP with:
/// - Exponential backoff retry
/// - Circuit breaker for fault isolation
/// - Structured tracing with batch metrics
/// - Per-entry error handling (partial batch success)
pub struct BatchForwarder {
    client: Arc<dyn SapClientTrait>,
    converter: SapForwarder,
    circuit_breaker: Arc<CircuitBreaker>,
    retry_config: BatchRetryConfig,
}

impl BatchForwarder {
    /// Create a new batch forwarder with default retry and circuit breaker config.
    ///
    /// Default configuration:
    /// - Max 3 retry attempts
    /// - Exponential backoff starting at 1 second (2x multiplier, max 30s)
    /// - Circuit breaker opens after 5 failures
    /// - Circuit breaker half-open timeout: 30 seconds
    pub fn new(client: Arc<dyn SapClientTrait>) -> Self {
        let retry_config = BatchRetryConfig::default();

        let circuit_breaker_config = CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(30),
            half_open_max_calls: 1,
            reset_on_success: true,
        };

        // Unwrap the Result before wrapping in Arc
        let circuit_breaker = CircuitBreaker::new(circuit_breaker_config)
            .expect("Failed to create circuit breaker with default config");

        Self {
            client,
            converter: SapForwarder::new(),
            circuit_breaker: Arc::new(circuit_breaker),
            retry_config,
        }
    }

    /// Create a batch forwarder with custom retry and circuit breaker config.
    ///
    /// Use this for testing with custom configurations (e.g., faster retries).
    pub fn with_config(
        client: Arc<dyn SapClientTrait>,
        retry_config: BatchRetryConfig,
        circuit_breaker_config: CircuitBreakerConfig,
    ) -> Result<Self> {
        // Propagate configuration errors
        let circuit_breaker = CircuitBreaker::new(circuit_breaker_config)
            .map_err(|e| PulseArcError::Config(format!("Invalid circuit breaker config: {}", e)))?;

        Ok(Self {
            client,
            converter: SapForwarder::new(),
            circuit_breaker: Arc::new(circuit_breaker),
            retry_config,
        })
    }

    /// Submit a batch of outbox entries to SAP with retry logic.
    ///
    /// Conversion and submission happen entry-by-entry so that failures can be reported
    /// precisely. Conversion failures are surfaced alongside submission failures, allowing
    /// callers to update outbox status deterministically.
    pub async fn submit_batch(&self, entries: &[TimeEntryOutbox]) -> Result<BatchSubmissionResult> {
        let batch_size = entries.len();
        debug!(batch_size, "Starting batch submission to SAP");

        let prepared = self.converter.prepare_batch(entries);
        let converted_count = prepared.iter().filter(|entry| entry.result.is_ok()).count();

        if converted_count == 0 && batch_size > 0 {
            warn!(batch_size, "All entries failed conversion; skipping SAP submission");
        }

        let mut entry_results = Vec::with_capacity(batch_size);

        for (index, prepared_entry) in prepared.into_iter().enumerate() {
            let outbox_id = prepared_entry.outbox_id;

            match prepared_entry.result {
                Ok(sap_entry) => match self.submit_with_retry(&sap_entry).await {
                    Ok(sap_entry_id) => {
                        entry_results.push(EntrySubmissionResult {
                            outbox_id,
                            status: EntrySubmissionStatus::Submitted { sap_entry_id },
                        });
                    }
                    Err(err) => {
                        let sap_error = map_submission_error(err, &outbox_id);
                        warn!(
                            outbox_id = %outbox_id,
                            entry_index = index,
                            error = %sap_error,
                            "SAP submission failed after retries"
                        );
                        entry_results.push(EntrySubmissionResult {
                            outbox_id,
                            status: EntrySubmissionStatus::Failed { error: sap_error },
                        });
                    }
                },
                Err(err) => {
                    let sap_error = map_conversion_error(err, &outbox_id);
                    entry_results.push(EntrySubmissionResult {
                        outbox_id,
                        status: EntrySubmissionStatus::Failed { error: sap_error },
                    });
                }
            }
        }

        let successful = entry_results
            .iter()
            .filter(|result| matches!(result.status, EntrySubmissionStatus::Submitted { .. }))
            .count();
        let failed = entry_results.len().saturating_sub(successful);

        info!(
            batch_size,
            converted_count,
            filtered_out = batch_size.saturating_sub(converted_count),
            successful,
            failed,
            "Batch submission complete"
        );

        Ok(BatchSubmissionResult { successful, failed, entry_results })
    }

    /// Submit a single entry with retry and circuit breaker.
    ///
    /// This is an internal method that wraps the SAP client call with:
    /// - Circuit breaker check (fail fast if open)
    /// - Retry logic with exponential backoff
    /// - Error conversion to domain errors
    async fn submit_with_retry(&self, entry: &SapTimeEntry) -> Result<SapEntryId> {
        let mut attempt = 0;
        let mut delay = self.retry_config.initial_delay;

        loop {
            attempt += 1;

            // Try submission through circuit breaker (async execute)
            let client = self.client.clone();
            let result = self
                .circuit_breaker
                .execute(|| async move { client.forward_entry(entry).await })
                .await;

            match result {
                Ok(entry_id) => {
                    if attempt > 1 {
                        info!(
                            wbs_code = %entry.wbs_code,
                            attempt,
                            "Entry submission succeeded after retry"
                        );
                    }
                    return Ok(entry_id);
                }
                Err(e) => {
                    if attempt >= self.retry_config.max_attempts {
                        warn!(
                            wbs_code = %entry.wbs_code,
                            attempt,
                            error = %e,
                            "Entry submission failed after all retries"
                        );
                        return Err(PulseArcError::Network(format!(
                            "SAP submission failed: {}",
                            e
                        )));
                    }

                    debug!(
                        wbs_code = %entry.wbs_code,
                        attempt,
                        delay_secs = delay.as_secs(),
                        error = %e,
                        "Entry submission failed, will retry"
                    );

                    // Sleep before retry
                    sleep(delay).await;

                    // Calculate next delay with exponential backoff
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * self.retry_config.multiplier)
                            .min(self.retry_config.max_delay.as_secs_f64()),
                    );
                }
            }
        }
    }
}
