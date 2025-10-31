//! Sync-specific error types
//!
//! Provides error classification for sync operations with retry metadata.

use pulsearc_domain::PulseArcError;
use thiserror::Error;

/// Categories of sync errors for retry logic
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncErrorCategory {
    /// Authentication errors (401, 403) - retry after token refresh
    Authentication,
    /// Rate limiting errors (429) - retry with backoff
    RateLimit,
    /// Server errors (5xx) - retryable
    Server,
    /// Client errors (4xx except auth) - non-retryable
    Client,
    /// Network/connection errors - retryable
    Network,
    /// Database errors - may be retryable
    Database,
    /// Configuration errors - non-retryable
    Config,
}

/// Sync operation errors
#[derive(Debug, Error)]
pub enum SyncError {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Client error: {0}")]
    Client(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Operation cancelled")]
    Cancelled,
}

impl SyncError {
    /// Get the error category for this error
    pub fn category(&self) -> SyncErrorCategory {
        match self {
            Self::Auth(_) => SyncErrorCategory::Authentication,
            Self::RateLimit(_) => SyncErrorCategory::RateLimit,
            Self::Server(_) => SyncErrorCategory::Server,
            Self::Client(_) => SyncErrorCategory::Client,
            Self::Network(_) | Self::Timeout(_) => SyncErrorCategory::Network,
            Self::Database(_) => SyncErrorCategory::Database,
            Self::Config(_) | Self::Cancelled => SyncErrorCategory::Config,
        }
    }

    /// Check if this error should be retried
    pub fn should_retry(&self) -> bool {
        matches!(
            self.category(),
            SyncErrorCategory::Authentication
                | SyncErrorCategory::RateLimit
                | SyncErrorCategory::Server
                | SyncErrorCategory::Network
                | SyncErrorCategory::Database
        )
    }

    /// Get suggested retry delay in seconds
    pub fn retry_delay_secs(&self) -> u64 {
        match self.category() {
            SyncErrorCategory::Authentication => 5,  // Quick retry after token refresh
            SyncErrorCategory::RateLimit => 60,      // Wait for rate limit window
            SyncErrorCategory::Server => 10,         // Moderate delay for server issues
            SyncErrorCategory::Network => 5,         // Quick retry for network
            SyncErrorCategory::Database => 2,        // Quick retry for DB
            SyncErrorCategory::Client | SyncErrorCategory::Config => 0, // No retry
        }
    }
}

/// Convert from PulseArcError to SyncError
impl From<PulseArcError> for SyncError {
    fn from(err: PulseArcError) -> Self {
        match err {
            PulseArcError::Database(message) => Self::Database(message),
            PulseArcError::Config(message) => Self::Config(message),
            PulseArcError::Platform(message) => Self::Server(message),
            PulseArcError::Network(message) => Self::Network(message),
            PulseArcError::Auth(message) | PulseArcError::Security(message) => {
                Self::Auth(message)
            }
            PulseArcError::NotFound(message) | PulseArcError::InvalidInput(message) => {
                Self::Client(message)
            }
            PulseArcError::Internal(message) => Self::Server(message),
        }
    }
}

/// Convert from CommonError to SyncError
impl From<pulsearc_common::error::CommonError> for SyncError {
    fn from(err: pulsearc_common::error::CommonError) -> Self {
        use pulsearc_common::error::{CommonError, ErrorCategory};

        match err.category() {
            ErrorCategory::Network => Self::Network(err.to_string()),
            ErrorCategory::Storage => Self::Database(err.to_string()),
            ErrorCategory::Security => Self::Auth(err.to_string()),
            ErrorCategory::Configuration => Self::Config(err.to_string()),
            _ => Self::Client(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        assert_eq!(
            SyncError::Auth("test".to_string()).category(),
            SyncErrorCategory::Authentication
        );
        assert_eq!(
            SyncError::RateLimit("test".to_string()).category(),
            SyncErrorCategory::RateLimit
        );
        assert_eq!(
            SyncError::Server("test".to_string()).category(),
            SyncErrorCategory::Server
        );
        assert_eq!(
            SyncError::Network("test".to_string()).category(),
            SyncErrorCategory::Network
        );
    }

    #[test]
    fn test_should_retry() {
        assert!(SyncError::Auth("test".to_string()).should_retry());
        assert!(SyncError::RateLimit("test".to_string()).should_retry());
        assert!(SyncError::Server("test".to_string()).should_retry());
        assert!(SyncError::Network("test".to_string()).should_retry());
        assert!(!SyncError::Client("test".to_string()).should_retry());
        assert!(!SyncError::Config("test".to_string()).should_retry());
    }

    #[test]
    fn test_retry_delays() {
        assert_eq!(SyncError::Auth("test".to_string()).retry_delay_secs(), 5);
        assert_eq!(
            SyncError::RateLimit("test".to_string()).retry_delay_secs(),
            60
        );
        assert_eq!(SyncError::Server("test".to_string()).retry_delay_secs(), 10);
        assert_eq!(
            SyncError::Network("test".to_string()).retry_delay_secs(),
            5
        );
        assert_eq!(SyncError::Client("test".to_string()).retry_delay_secs(), 0);
    }
}
