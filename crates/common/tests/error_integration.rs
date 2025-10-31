//! Integration tests for `pulsearc_common::error`.
//!
//! These suites validate classification, logging payloads, retry helpers, and
//! module error delegation to ensure downstream systems receive consistent
//! failure semantics.

#![allow(clippy::doc_lazy_continuation)]

use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use pulsearc_common::error::{
    CommonError, CommonResult, ErrorClassification, ErrorContext, ErrorSeverity,
};
use thiserror::Error;

/// Validates `CommonError::Detailed` behavior for the classification matrix
/// matches expected contract scenario.
///
/// Assertions:
/// - Confirms `err.is_retryable()` equals `retryable`.
/// - Confirms `err.severity()` equals `severity`.
/// - Confirms `err.is_critical()` equals `critical`.
/// Validates that `CommonError` classification surfaces the expected retryable,
/// severity, and criticality combinations for each variant.
#[test]
fn classification_matrix_matches_expected_contract() {
    let detailed = CommonError::Detailed {
        message: "structured failure".to_string(),
        severity: ErrorSeverity::Critical,
        context: Some("ingest::pipeline".to_string()),
    };

    let cases = vec![
        (CommonError::config("missing api key"), false, ErrorSeverity::Error, false),
        (CommonError::lock("poisoned mutex"), true, ErrorSeverity::Warning, false),
        (
            CommonError::circuit_breaker_with_retry("payments", Duration::from_secs(5)),
            true,
            ErrorSeverity::Warning,
            false,
        ),
        (CommonError::serialization("invalid JSON"), false, ErrorSeverity::Error, false),
        (CommonError::persistence("disk full"), false, ErrorSeverity::Error, false),
        (
            CommonError::rate_limit_detailed(
                120,
                Duration::from_secs(60),
                Some(Duration::from_secs(2)),
            ),
            true,
            ErrorSeverity::Warning,
            false,
        ),
        (
            CommonError::timeout("db query", Duration::from_secs(3)),
            true,
            ErrorSeverity::Warning,
            false,
        ),
        (
            CommonError::backend("inventory-service", "502 gateway", true),
            true,
            ErrorSeverity::Error,
            false,
        ),
        (CommonError::validation("email", "must be valid"), false, ErrorSeverity::Error, false),
        (CommonError::not_found_with_id("User", "42"), false, ErrorSeverity::Info, false),
        (
            CommonError::unauthorized_with_perm("write", "admin"),
            false,
            ErrorSeverity::Warning,
            false,
        ),
        (
            CommonError::internal_with_context("corrupted state", "scheduler"),
            false,
            ErrorSeverity::Critical,
            true,
        ),
        (
            CommonError::Storage { message: "transaction failed".to_string(), operation: None },
            false,
            ErrorSeverity::Error,
            false,
        ),
        (detailed.clone(), false, ErrorSeverity::Critical, false),
        (
            CommonError::task_cancelled_with_reason("sync-task", "user request"),
            false,
            ErrorSeverity::Info,
            false,
        ),
        (
            CommonError::async_timeout("aggregation", Duration::from_millis(3500)),
            true,
            ErrorSeverity::Warning,
            false,
        ),
    ];

    for (err, retryable, severity, critical) in cases {
        assert_eq!(err.is_retryable(), retryable, "retryable mismatch for {}", err);
        assert_eq!(err.severity(), severity, "severity mismatch for {}", err);
        assert_eq!(err.is_critical(), critical, "critical mismatch for {}", err);
    }
}

/// Validates `CommonError::rate_limit_detailed` behavior for the structured
/// logging payloads include expected keys scenario.
///
/// Assertions:
/// - Confirms `rate_fields.get("error_type")` equals
///   `Some(&"rate_limit_exceeded".to_string())`.
/// - Confirms `rate_fields.get("limit")` equals `Some(&"100".to_string())`.
/// - Confirms `rate_fields.get("window_ms")` equals
///   `Some(&Duration::from_secs(30).as_millis().to_string())`.
/// - Confirms `rate_fields.get("retry_after_ms")` equals
///   `Some(&Duration::from_secs(3).as_millis().to_string())`.
/// - Confirms `timeout_fields.get("error_type")` equals
///   `Some(&"async_timeout".to_string())`.
/// - Confirms `timeout_fields.get("future_name")` equals
///   `Some(&"heartbeat".to_string())`.
/// - Confirms `timeout_fields.get("duration_ms")` equals
///   `Some(&Duration::from_millis(4500).as_millis().to_string())`.
/// Ensures structured logging payloads expose stable key/value pairs for rate
/// limit and async timeout errors so observability pipelines stay compatible.
#[test]
fn structured_logging_payloads_include_expected_keys() {
    let rate_limit = CommonError::rate_limit_detailed(
        100,
        Duration::from_secs(30),
        Some(Duration::from_secs(3)),
    );
    let rate_fields: HashMap<_, _> = rate_limit.as_tracing_fields().into_iter().collect();
    assert_eq!(rate_fields.get("error_type"), Some(&"rate_limit_exceeded".to_string()));
    assert_eq!(rate_fields.get("limit"), Some(&"100".to_string()));
    assert_eq!(
        rate_fields.get("window_ms"),
        Some(&Duration::from_secs(30).as_millis().to_string())
    );
    assert_eq!(
        rate_fields.get("retry_after_ms"),
        Some(&Duration::from_secs(3).as_millis().to_string())
    );

    let async_timeout = CommonError::async_timeout("heartbeat", Duration::from_millis(4500));
    let timeout_fields: HashMap<_, _> = async_timeout.as_tracing_fields().into_iter().collect();
    assert_eq!(timeout_fields.get("error_type"), Some(&"async_timeout".to_string()));
    assert_eq!(timeout_fields.get("future_name"), Some(&"heartbeat".to_string()));
    assert_eq!(
        timeout_fields.get("duration_ms"),
        Some(&Duration::from_millis(4500).as_millis().to_string())
    );
}

/// Validates `VecDeque::new` behavior for the retry helper respects common
/// error classification scenario.
///
/// Assertions:
/// - Confirms `attempts` equals `3`.
/// - Confirms `backoffs` equals `vec![Duration::from_secs(1)]`.
/// - Confirms `result.expect("final attempt returns success")` equals `"ok"`.
/// Confirms the retry helper honors `CommonError` retryability rules and
/// propagates retry-after hints bubbled up through error metadata.
#[test]
fn retry_helper_respects_common_error_classification() {
    let mut responses: VecDeque<CommonResult<&'static str>> = VecDeque::new();
    responses.push_back(Err(CommonError::rate_limit_detailed(
        50,
        Duration::from_secs(60),
        Some(Duration::from_secs(1)),
    )));
    responses.push_back(Err(CommonError::timeout("fetch", Duration::from_secs(2))));
    responses.push_back(Ok("ok"));

    let (result, attempts, backoffs) = run_with_retry(|_| {
        responses.pop_front().expect("simulated responses should cover all attempts")
    });

    assert_eq!(attempts, 3, "should retry until success");
    assert_eq!(backoffs, vec![Duration::from_secs(1)], "should capture retry-after hints");
    assert_eq!(result.expect("final attempt returns success"), "ok");
}

/// Validates `VecDeque::new` behavior for the retry helper stops on first non
/// retryable error scenario.
///
/// Assertions:
/// - Confirms `attempts` equals `1`.
/// - Ensures `backoffs.is_empty()` evaluates to true.
/// - Confirms `field` equals `"payload"`.
/// - Confirms `message` equals `"missing id"`.
/// Ensures the retry helper aborts immediately when a non-retryable validation
/// error surfaces rather than looping indefinitely.
#[test]
fn retry_helper_stops_on_first_non_retryable_error() {
    let mut responses: VecDeque<CommonResult<()>> = VecDeque::new();
    responses.push_back(Err(CommonError::validation("payload", "missing id")));
    responses.push_back(Ok(()));

    let (result, attempts, backoffs) = run_with_retry(|_| {
        responses.pop_front().expect("at least one simulated response must exist")
    });

    assert_eq!(attempts, 1, "non-retryable errors should halt immediately");
    assert!(backoffs.is_empty(), "non-retryable errors should not expose retry hints");

    let err = result.expect_err("validation failure is returned");
    match err {
        CommonError::Validation { field, message, .. } => {
            assert_eq!(field, "payload");
            assert_eq!(message, "missing id");
        }
        other => panic!("Unexpected error variant: {:?}", other),
    }
}

/// Validates `ModuleError::Module` behavior for the module error macros and
/// context delegation scenario.
///
/// Assertions:
/// - Ensures `!bespoke.is_retryable()` evaluates to true.
/// - Confirms `bespoke.severity()` equals `ErrorSeverity::Error`.
/// - Ensures `matches!(module_err, ModuleError::Common(_))` evaluates to true.
/// - Confirms `module_err.severity()` equals `ErrorSeverity::Error`.
/// - Ensures `!module_err.is_retryable()` evaluates to true.
/// - Ensures `matches!(module_err, ModuleError::Common(_))` evaluates to true.
/// - Confirms `module_err.severity()` equals `ErrorSeverity::Error`.
/// - Ensures `contextual.is_retryable()` evaluates to true.
/// - Confirms `contextual.severity()` equals `ErrorSeverity::Warning`.
/// - Confirms `context` equals `"worker::run"`.
/// - Ensures `matches!(source, CommonError::Timeout { .. })` evaluates to true.
/// - Ensures `critical.is_critical()` evaluates to true.
/// - Confirms `throttled.retry_after()` equals
///   `Some(Duration::from_millis(400))`.
/// Verifies module error helpers wrap external failures, preserve severity, and
/// maintain contextual metadata during conversions.
#[test]
fn module_error_macros_and_context_delegation() {
    let bespoke = ModuleError::Module("manual failure".to_string());
    assert!(!bespoke.is_retryable());
    assert_eq!(bespoke.severity(), ErrorSeverity::Error);

    let json_err = serde_json::from_str::<serde_json::Value>("not-json")
        .expect_err("string is intentionally invalid JSON");
    let module_err = ModuleError::from_common(CommonError::from(json_err));
    assert!(matches!(module_err, ModuleError::Common(_)));
    assert_eq!(module_err.severity(), ErrorSeverity::Error);
    assert!(!module_err.is_retryable());

    let io_err = std::io::Error::other("disk offline");
    let module_err = ModuleError::from_common(CommonError::from(io_err));
    assert!(matches!(module_err, ModuleError::Common(_)));
    assert_eq!(module_err.severity(), ErrorSeverity::Error);

    let contextual = ModuleError::from_common(CommonError::timeout("sync", Duration::from_secs(4)))
        .with_context("worker::run");
    assert!(contextual.is_retryable(), "timeout should remain retryable through context");
    assert_eq!(contextual.severity(), ErrorSeverity::Warning);
    if let ModuleError::Contextual { context, source } = &contextual {
        assert_eq!(context, "worker::run");
        assert!(matches!(source, CommonError::Timeout { .. }));
    } else {
        panic!("contextual variant expected");
    }

    let critical =
        ModuleError::from_common(CommonError::internal("corrupted state")).with_context("ingest");
    assert!(critical.is_critical(), "internal errors remain critical after contextualization");

    let throttled = ModuleError::from_common(CommonError::rate_limit_detailed(
        10,
        Duration::from_secs(10),
        Some(Duration::from_millis(400)),
    ))
    .with_context("api::bulk-upload");
    assert_eq!(
        throttled.retry_after(),
        Some(Duration::from_millis(400)),
        "retry hint should be preserved through module error wrapper"
    );
}

type RetryOutcome<T> = (CommonResult<T>, usize, Vec<Duration>);

fn run_with_retry<T, F>(mut operation: F) -> RetryOutcome<T>
where
    F: FnMut(usize) -> CommonResult<T>,
{
    let mut attempts = 0;
    let mut backoffs = Vec::new();

    loop {
        attempts += 1;
        match operation(attempts) {
            Ok(value) => return (Ok(value), attempts, backoffs),
            Err(err) => {
                if let Some(delay) = err.retry_after() {
                    backoffs.push(delay);
                }

                if !err.is_retryable() || attempts >= 8 {
                    return (Err(err), attempts, backoffs);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Error)]
enum ModuleError {
    #[error("module failure: {0}")]
    Module(String),

    #[error("contextual failure ({context}): {source}")]
    Contextual {
        context: String,
        #[source]
        source: CommonError,
    },

    #[error(transparent)]
    Common(#[from] CommonError),
}

impl ErrorContext for ModuleError {
    fn from_common(err: CommonError) -> Self {
        Self::Common(err)
    }

    fn with_context<S: Into<String>>(self, context: S) -> Self {
        match self {
            Self::Common(source) => Self::Contextual { context: context.into(), source },
            Self::Contextual { context: existing, source } => {
                Self::Contextual { context: format!("{} -> {}", existing, context.into()), source }
            }
            other => other,
        }
    }
}

impl From<serde_json::Error> for ModuleError {
    fn from(err: serde_json::Error) -> Self {
        Self::Common(CommonError::from(err))
    }
}

impl From<std::io::Error> for ModuleError {
    fn from(err: std::io::Error) -> Self {
        Self::Common(CommonError::from(err))
    }
}

pulsearc_common::impl_error_classification!(
    ModuleError,
    Common,
    ModuleError::Module(_) => {
        retryable: false,
        severity: ErrorSeverity::Error,
        critical: false,
    },
    ModuleError::Contextual { source, .. } => {
        retryable: source.is_retryable(),
        severity: source.severity(),
        critical: source.is_critical(),
        retry_after: source.retry_after(),
    }
);
