use thiserror::Error;

use crate::error::{CommonError, ErrorSeverity};
use crate::{impl_error_classification, impl_error_conversion};

/// Queue operation errors
///
/// This enum represents all possible errors that can occur during queue
/// operations. It uses the `thiserror` crate for automatic `Error` trait
/// implementation and includes automatic conversions from common error types.
///
/// ## Error Conversion Strategy
///
/// The queue module uses automatic error conversion via the `From` trait to
/// enable idiomatic Rust error handling with the `?` operator. Supported
/// conversions:
///
/// - `serde_json::Error` → `CommonError::serialization`
/// - `std::io::Error` → `CommonError::io`
/// - `RetryError` → `QueueError::RetryError`
/// - Common errors (lock, compression, encryption) → `CommonError`
///
/// This allows seamless error propagation from dependencies without manual
/// mapping.
///
/// ## Lock Poisoning
///
/// All public methods that acquire locks handle poisoning gracefully by
/// converting `PoisonError` to `CommonError::lock`. This prevents panics and
/// allows the caller to decide how to handle lock poisoning.
#[derive(Debug, Error)]
pub enum QueueError {
    // Common errors handled by CommonError (Serialization, Io, Compression, Encryption, Lock,
    // Persistence)
    #[error(transparent)]
    Common(#[from] CommonError),

    // Queue-specific errors
    #[error("Queue is at maximum capacity ({0})")]
    CapacityExceeded(usize),

    #[error("Item not found: {0}")]
    ItemNotFound(String),

    #[error("Queue is shutting down")]
    ShuttingDown,

    #[error("Invalid priority: {0}")]
    InvalidPriority(u8),

    #[error("Duplicate item ID: {0}")]
    DuplicateItem(String),

    #[error("Queue locked for maintenance")]
    Locked,

    #[error("Invalid queue state: {0}")]
    InvalidState(String),

    #[error("Maintenance task failed: {0}")]
    MaintenanceError(String),

    #[error("Retry error: {0}")]
    RetryError(#[from] crate::sync::retry::error::RetryError),
}

// Auto-convert std types via CommonError
impl_error_conversion!(QueueError, Common);

// Implement ErrorClassification for QueueError
impl_error_classification!(QueueError, Common,
    Self::CapacityExceeded(_) => {
        retryable: true,  // Might have space later
        severity: ErrorSeverity::Warning,
        critical: false,
        retry_after: Some(std::time::Duration::from_millis(100)),
    },
    Self::ItemNotFound(_) => {
        retryable: false,
        severity: ErrorSeverity::Info,
        critical: false,
    },
    Self::ShuttingDown => {
        retryable: false,
        severity: ErrorSeverity::Info,
        critical: false,
    },
    Self::InvalidPriority(_) => {
        retryable: false,
        severity: ErrorSeverity::Error,
        critical: false,
    },
    Self::DuplicateItem(_) => {
        retryable: false,
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::Locked => {
        retryable: true,  // Lock might be released
        severity: ErrorSeverity::Warning,
        critical: false,
        retry_after: Some(std::time::Duration::from_millis(50)),
    },
    Self::InvalidState(_) => {
        retryable: false,
        severity: ErrorSeverity::Error,
        critical: false,
    },
    Self::MaintenanceError(_) => {
        retryable: true,
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::RetryError(e) => {
        retryable: e.is_retryable(),
        severity: e.severity(),
        critical: e.is_critical(),
        retry_after: e.retry_after(),
    }
);

// Manual implementation of From<QueueError> for CommonError for cross-module
// errors
impl From<QueueError> for CommonError {
    fn from(err: QueueError) -> Self {
        match err {
            QueueError::Common(e) => e,
            QueueError::CapacityExceeded(size) => {
                CommonError::internal(format!("Queue capacity exceeded: {size}"))
            }
            QueueError::ItemNotFound(id) => {
                CommonError::internal(format!("Queue item not found: {id}"))
            }
            QueueError::ShuttingDown => CommonError::internal("Queue is shutting down".to_string()),
            QueueError::InvalidPriority(p) => {
                CommonError::internal(format!("Invalid queue priority: {p}"))
            }
            QueueError::DuplicateItem(id) => {
                CommonError::internal(format!("Duplicate queue item: {id}"))
            }
            QueueError::Locked => CommonError::internal("Queue locked for maintenance".to_string()),
            QueueError::InvalidState(msg) => {
                CommonError::internal(format!("Invalid queue state: {msg}"))
            }
            QueueError::MaintenanceError(msg) => {
                CommonError::internal(format!("Queue maintenance failed: {msg}"))
            }
            QueueError::RetryError(e) => e.into(),
        }
    }
}

/// Queue operation result type
pub type QueueResult<T> = Result<T, QueueError>;
