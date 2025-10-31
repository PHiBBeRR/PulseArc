// Error types for retry module
use thiserror::Error;

use crate::error::{CommonError, ErrorSeverity};
use crate::{impl_error_classification, impl_error_conversion};

/// Errors that can occur during retry operations
#[derive(Debug, Error)]
pub enum RetryError {
    // Common errors handled by CommonError (Timeout, Config, CircuitBreaker, Lock)
    #[error(transparent)]
    Common(#[from] CommonError),

    // Retry-specific errors
    #[error("All retry attempts exhausted after {attempts} attempts")]
    AttemptsExhausted { attempts: u32 },

    #[error("Retry budget exhausted, no tokens available")]
    BudgetExhausted,

    #[error("Operation failed: {source}")]
    OperationFailed {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl RetryError {
    /// Create a new operation failed error from any error type
    pub fn operation_failed<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::OperationFailed { source: Box::new(error) }
    }
}

// Auto-convert std types via CommonError
impl_error_conversion!(RetryError, Common);

// Implement ErrorClassification for RetryError
impl_error_classification!(RetryError, Common,
    Self::AttemptsExhausted { .. } => {
        retryable: false,  // Already exhausted all attempts
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::BudgetExhausted => {
        retryable: true,  // Budget might replenish
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::OperationFailed { .. } => {
        retryable: false,  // Caller decides
        severity: ErrorSeverity::Error,
        critical: false,
    }
);

// Manual implementation of From<RetryError> for CommonError for cross-module
// errors
impl From<RetryError> for CommonError {
    fn from(err: RetryError) -> Self {
        match err {
            RetryError::Common(e) => e,
            RetryError::AttemptsExhausted { attempts } => {
                CommonError::internal(format!("Retry attempts exhausted after {attempts} attempts"))
            }
            RetryError::BudgetExhausted => {
                CommonError::internal("Retry budget exhausted".to_string())
            }
            RetryError::OperationFailed { source } => {
                CommonError::internal(format!("Operation failed: {source}"))
            }
        }
    }
}

/// Result type for retry operations
pub type RetryResult<T> = Result<T, RetryError>;
