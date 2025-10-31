//! Unified error system for Agent observability
//!
//! This module provides comprehensive error handling for observability
//! concerns:
//! - Metrics collection errors
//! - Monitoring failures
//! - Classification tracking errors
//!
//! Ported from macos-production/src-tauri/src/observability/errors/app.rs

use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "serde")]
mod duration_ms {
    use std::time::Duration;

    use serde::ser::Error as SerError;
    use serde::{Deserialize, Deserializer, Serializer};

    type SerializerResult<S> = Result<<S as Serializer>::Ok, <S as Serializer>::Error>;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> SerializerResult<S>
    where
        S: Serializer,
    {
        let millis: u128 = duration.as_millis();
        let millis = u64::try_from(millis).map_err(|_| {
            SerError::custom("duration too large to fit into a 64-bit millisecond representation")
        })?;
        serializer.serialize_u64(millis)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

/* -------------------------------------------------------------------------- */
/* Public result types */
/* -------------------------------------------------------------------------- */

pub type AppResult<T> = Result<T, AppError>;

/* -------------------------------------------------------------------------- */
/* Stable error codes for telemetry */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ErrorCode {
    // Database / Storage
    DbOpenFailed,
    DbQueryFailed,
    DbBusy,
    DbTimeout,
    DbIntegrityFailed,

    // AI / OpenAI
    AiRateLimited,
    AiInvalidApiKey,
    AiQuotaExceeded,
    AiModelNotFound,
    AiBadRequest,
    AiTimeout,
    AiServerError,
    AiContentPolicyViolation,
    AiParseResponseFailed,
    AiOutputInvalidSchema,
    AiTokenLimitExceeded,

    // HTTP / Network
    HttpNetwork,
    HttpTimeout,
    HttpUnauthorized,
    HttpForbidden,
    HttpTooManyRequests,
    HttpServerError,
    HttpStatus,

    // Metrics / Monitoring
    MetricsCollectionFailed,
    MetricsTrackerUnavailable,

    // Generic
    Serialization,
    Io,
    ValidationFailed,
    Unknown,
}

/* -------------------------------------------------------------------------- */
/* Action hints for error recovery */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ActionHint {
    None,

    /// Retry after a relative delay in milliseconds
    RetryAfter {
        #[cfg_attr(feature = "serde", serde(with = "duration_ms"))]
        duration: Duration,
    },

    /// Generic exponential backoff
    Backoff,

    /// Check configuration key
    CheckConfig {
        key: String,
    },

    /// Check network connectivity
    CheckNetwork,

    /// Verify OpenAI API key
    CheckOpenAiKey,

    /// Reduce batch size
    ReduceBatchSize,

    /// Switch to compatible model
    SwitchModel {
        model: String,
    },
}

/* -------------------------------------------------------------------------- */
/* Top-level application error */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Error, Clone)]
#[non_exhaustive]
pub enum AppError {
    #[error(transparent)]
    Ai(#[from] AiError),

    #[error(transparent)]
    Http(#[from] HttpError),

    #[error(transparent)]
    Metrics(#[from] MetricsError),

    #[error("Serialization error: {0}")]
    Serde(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Unexpected error: {0}")]
    Other(String),
}

impl AppError {
    pub fn code(&self) -> ErrorCode {
        use ErrorCode::*;
        match self {
            AppError::Ai(e) => e.code(),
            AppError::Http(e) => e.code(),
            AppError::Metrics(e) => e.code(),
            AppError::Serde(_) => Serialization,
            AppError::Io(_) => Io,
            AppError::Validation(_) => ValidationFailed,
            AppError::Other(_) => Unknown,
        }
    }

    pub fn action(&self) -> ActionHint {
        use ActionHint::*;
        match self {
            // AI errors
            AppError::Ai(AiError::RateLimited { retry_after }) => {
                if let Some(duration) = retry_after {
                    RetryAfter { duration: *duration }
                } else {
                    Backoff
                }
            }
            AppError::Ai(AiError::InvalidApiKey) => CheckOpenAiKey,
            AppError::Ai(AiError::QuotaExceeded) => Backoff,
            AppError::Ai(AiError::ModelNotFound { .. }) => {
                SwitchModel { model: "gpt-4o-mini".into() }
            }
            AppError::Ai(AiError::Timeout) => Backoff,
            AppError::Ai(AiError::TokenLimitExceeded { .. }) => ReduceBatchSize,
            AppError::Ai(AiError::ServerError(_)) => Backoff,

            // HTTP errors
            AppError::Http(HttpError::Timeout) => Backoff,
            AppError::Http(HttpError::Network(_)) => CheckNetwork,
            AppError::Http(HttpError::TooManyRequests { retry_after }) => {
                if let Some(duration) = retry_after {
                    RetryAfter { duration: *duration }
                } else {
                    Backoff
                }
            }
            AppError::Http(HttpError::ServerError { .. }) => Backoff,

            _ => None,
        }
    }

    pub fn to_ui(&self) -> UiError {
        UiError { code: self.code(), message: self.to_string(), action: self.action() }
    }

    /// Indicates whether the error represents a transient failure that callers
    /// can safely retry.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AppError::Ai(
                AiError::RateLimited { .. }
                    | AiError::Timeout
                    | AiError::ServerError(_)
                    | AiError::QuotaExceeded
            ) | AppError::Http(
                HttpError::Timeout
                    | HttpError::TooManyRequests { .. }
                    | HttpError::ServerError { .. }
            ) | AppError::Metrics(MetricsError::TrackerUnavailable)
        )
    }
}

/* -------------------------------------------------------------------------- */
/* Frontend-facing error type */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Serialize)]
pub struct UiError {
    pub code: ErrorCode,
    pub message: String,
    pub action: ActionHint,
}

impl UiError {
    pub fn from_app_error(error: AppError) -> Self {
        error.to_ui()
    }

    pub fn from_message(message: &str) -> Self {
        Self { code: ErrorCode::Unknown, message: message.to_string(), action: ActionHint::None }
    }
}

/* -------------------------------------------------------------------------- */
/* AI / OpenAI errors */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Error, Clone)]
#[non_exhaustive]
pub enum AiError {
    #[error("OpenAI rate limited")]
    RateLimited { retry_after: Option<Duration> },

    #[error("Invalid OpenAI API key")]
    InvalidApiKey,

    #[error("OpenAI quota exceeded")]
    QuotaExceeded,

    #[error("Requested model not found: {model}")]
    ModelNotFound { model: String },

    #[error("Bad request to OpenAI API: {0}")]
    BadRequest(String),

    #[error("OpenAI request timed out")]
    Timeout,

    #[error("OpenAI server error: {0}")]
    ServerError(String),

    #[error("OpenAI content policy violation: {0}")]
    ContentPolicyViolation(String),

    #[error("Failed to parse OpenAI response JSON: {0}")]
    ParseResponse(String),

    #[error("Output failed schema validation: {0}")]
    OutputInvalidSchema(String),

    #[error("Estimated token count {estimated} exceeds model limit {limit}")]
    TokenLimitExceeded { estimated: usize, limit: usize },
}

impl AiError {
    fn code(&self) -> ErrorCode {
        use ErrorCode::*;
        match self {
            AiError::RateLimited { .. } => AiRateLimited,
            AiError::InvalidApiKey => AiInvalidApiKey,
            AiError::QuotaExceeded => AiQuotaExceeded,
            AiError::ModelNotFound { .. } => AiModelNotFound,
            AiError::BadRequest(_) => AiBadRequest,
            AiError::Timeout => AiTimeout,
            AiError::ServerError(_) => AiServerError,
            AiError::ContentPolicyViolation(_) => AiContentPolicyViolation,
            AiError::ParseResponse(_) => AiParseResponseFailed,
            AiError::OutputInvalidSchema(_) => AiOutputInvalidSchema,
            AiError::TokenLimitExceeded { .. } => AiTokenLimitExceeded,
        }
    }
}

/* -------------------------------------------------------------------------- */
/* HTTP / Network errors */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Error, Clone)]
#[non_exhaustive]
pub enum HttpError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("HTTP timeout")]
    Timeout,

    #[error("HTTP 401 Unauthorized")]
    Unauthorized,

    #[error("HTTP 403 Forbidden")]
    Forbidden,

    #[error("HTTP 429 Too Many Requests")]
    TooManyRequests { retry_after: Option<Duration> },

    #[error("HTTP 5xx Server Error: {status}")]
    ServerError { status: u16 },

    #[error("HTTP status: {status}")]
    Status { status: u16 },
}

impl HttpError {
    fn code(&self) -> ErrorCode {
        use ErrorCode::*;
        match self {
            HttpError::Network(_) => HttpNetwork,
            HttpError::Timeout => HttpTimeout,
            HttpError::Unauthorized => HttpUnauthorized,
            HttpError::Forbidden => HttpForbidden,
            HttpError::TooManyRequests { .. } => HttpTooManyRequests,
            HttpError::ServerError { .. } => HttpServerError,
            HttpError::Status { .. } => HttpStatus,
        }
    }
}

/* -------------------------------------------------------------------------- */
/* Metrics / Monitoring errors */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Error, Clone)]
#[non_exhaustive]
pub enum MetricsError {
    #[error("Metrics collection failed: {0}")]
    CollectionFailed(String),

    #[error("Metrics tracker unavailable")]
    TrackerUnavailable,

    #[error("Metrics error: {0}")]
    Other(String),
}

impl MetricsError {
    fn code(&self) -> ErrorCode {
        use ErrorCode::*;
        match self {
            MetricsError::CollectionFailed(_) => MetricsCollectionFailed,
            MetricsError::TrackerUnavailable => MetricsTrackerUnavailable,
            MetricsError::Other(_) => Unknown,
        }
    }
}

/* -------------------------------------------------------------------------- */
/* Generic error conversions */
/* -------------------------------------------------------------------------- */

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Serde(e.to_string())
    }
}

// Storage error conversion
#[cfg(feature = "platform")]
impl From<crate::storage::error::StorageError> for AppError {
    fn from(e: crate::storage::error::StorageError) -> Self {
        AppError::Other(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for observability::errors::app.
    use super::*;

    /// Validates `AppError::Ai` behavior for the retryable detection handles
    /// transient errors scenario.
    ///
    /// Assertions:
    /// - Ensures `AppError::Ai(AiError::Timeout).is_retryable()` evaluates to
    ///   true.
    /// - Ensures `AppError::Ai(AiError::RateLimited { retry_after:
    ///   Some(Duration::from_secs(5)) }) .is_retryable()` evaluates to true.
    /// - Ensures `AppError::Http(HttpError::TooManyRequests { retry_after: None
    ///   }).is_retryable()` evaluates to true.
    /// - Ensures `!AppError::Http(HttpError::Unauthorized).is_retryable()`
    ///   evaluates to true.
    /// - Ensures `!AppError::Validation("bad input".into()).is_retryable()`
    ///   evaluates to true.
    #[test]
    fn retryable_detection_handles_transient_errors() {
        assert!(AppError::Ai(AiError::Timeout).is_retryable());
        assert!(AppError::Ai(AiError::RateLimited { retry_after: Some(Duration::from_secs(5)) })
            .is_retryable());
        assert!(AppError::Http(HttpError::TooManyRequests { retry_after: None }).is_retryable());
        assert!(!AppError::Http(HttpError::Unauthorized).is_retryable());
        assert!(!AppError::Validation("bad input".into()).is_retryable());
    }

    /// Validates `ActionHint::RetryAfter` behavior for the retry after serde
    /// roundtrip preserves duration scenario.
    ///
    /// Assertions:
    /// - Confirms `serialized["kind"]` equals `"RETRY_AFTER"`.
    /// - Confirms `serialized["duration"]` equals `1500`.
    /// - Confirms `duration` equals `Duration::from_millis(1500)`.
    #[cfg(feature = "serde")]
    #[test]
    fn retry_after_serde_roundtrip_preserves_duration() {
        let hint = ActionHint::RetryAfter { duration: Duration::from_millis(1500) };
        let serialized = serde_json::to_value(&hint).expect("serialize duration in millis");
        assert_eq!(serialized["kind"], "RETRY_AFTER");
        assert_eq!(serialized["duration"], 1500);

        let roundtrip: ActionHint =
            serde_json::from_value(serialized).expect("deserialize duration in millis");
        match roundtrip {
            ActionHint::RetryAfter { duration } => {
                assert_eq!(duration, Duration::from_millis(1500))
            }
            other => panic!("expected retry-after action hint, got {other:?}"),
        }
    }
}
