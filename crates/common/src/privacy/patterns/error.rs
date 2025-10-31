use thiserror::Error;

use crate::error::{CommonError, ErrorSeverity};
use crate::{impl_error_classification, impl_error_conversion};

/// Comprehensive error types for PII pattern processing
#[derive(Debug, Error)]
pub enum PiiError {
    // Common errors (Configuration, Validation, IoError)
    #[error(transparent)]
    Common(#[from] CommonError),

    // PII-specific errors
    #[error("Pattern compilation error: {0}")]
    PatternCompilation(String),

    #[error("Pattern matching error: {0}")]
    PatternMatching(String),

    #[error("Processing error: {0}")]
    Processing(String),

    #[error("Audit error: {0}")]
    Audit(String),

    #[error("Metrics error: {0}")]
    Metrics(String),

    #[error("Compliance error: {0}")]
    Compliance(String),

    #[error("Detection engine error: {0}")]
    DetectionEngine(String),

    #[error("Redaction error: {0}")]
    Redaction(String),

    #[error("Classification error: {0}")]
    Classification(String),

    #[error("Performance error: {0}")]
    Performance(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
}

pub type PiiResult<T> = Result<T, PiiError>;

// Auto-convert std types via CommonError
impl_error_conversion!(PiiError, Common);

// Implement ErrorClassification for PiiError
impl_error_classification!(PiiError, Common,
    Self::PatternCompilation(_) => {
        retryable: false,  // Compilation errors are permanent
        severity: ErrorSeverity::Error,
        critical: false,
    },
    Self::PatternMatching(_) => {
        retryable: false,  // Pattern matching failures are not transient
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::Processing(_) => {
        retryable: true,  // Processing may succeed on retry
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::Audit(_) => {
        retryable: true,  // Audit logging may be temporarily unavailable
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::Metrics(_) => {
        retryable: true,  // Metrics collection failures are transient
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::Compliance(_) => {
        retryable: false,  // Compliance violations are not retryable
        severity: ErrorSeverity::Critical,  // Compliance is critical
        critical: true,
    },
    Self::DetectionEngine(_) => {
        retryable: false,  // Detection engine errors need investigation
        severity: ErrorSeverity::Error,
        critical: false,
    },
    Self::Redaction(_) => {
        retryable: false,  // Redaction failures are serious
        severity: ErrorSeverity::Error,
        critical: false,
    },
    Self::Classification(_) => {
        retryable: false,  // Classification errors are permanent
        severity: ErrorSeverity::Error,
        critical: false,
    },
    Self::Performance(_) => {
        retryable: true,  // Performance issues may be transient
        severity: ErrorSeverity::Warning,
        critical: false,
    },
    Self::Security(_) => {
        retryable: false,  // Security errors are not retryable
        severity: ErrorSeverity::Critical,  // Security is critical
        critical: true,
    },
    Self::RegexError(_) => {
        retryable: false,  // Regex compilation errors are permanent
        severity: ErrorSeverity::Error,
        critical: false,
    }
);

// Manual implementation of From<PiiError> for CommonError
impl From<PiiError> for CommonError {
    fn from(err: PiiError) -> Self {
        match err {
            PiiError::Common(e) => e,
            PiiError::PatternCompilation(msg) => {
                CommonError::internal(format!("Pattern compilation: {}", msg))
            }
            PiiError::PatternMatching(msg) => {
                CommonError::internal(format!("Pattern matching: {}", msg))
            }
            PiiError::Processing(msg) => CommonError::internal(format!("Processing: {}", msg)),
            PiiError::Audit(msg) => CommonError::internal(format!("Audit: {}", msg)),
            PiiError::Metrics(msg) => CommonError::internal(format!("Metrics: {}", msg)),
            PiiError::Compliance(msg) => {
                CommonError::internal(format!("Compliance violation: {}", msg))
            }
            PiiError::DetectionEngine(msg) => {
                CommonError::internal(format!("Detection engine: {}", msg))
            }
            PiiError::Redaction(msg) => CommonError::internal(format!("Redaction: {}", msg)),
            PiiError::Classification(msg) => {
                CommonError::internal(format!("Classification: {}", msg))
            }
            PiiError::Performance(msg) => CommonError::internal(format!("Performance: {}", msg)),
            PiiError::Security(msg) => CommonError::internal(format!("Security: {}", msg)),
            PiiError::RegexError(e) => CommonError::internal(format!("Regex error: {}", e)),
        }
    }
}
