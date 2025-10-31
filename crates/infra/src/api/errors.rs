//! API-specific error types
//!
//! Provides error classification for API operations with retry metadata.

use std::time::Duration;

use thiserror::Error;

/// Categories of API errors for retry logic
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiErrorCategory {
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
    /// Configuration errors - non-retryable
    Config,
}

/// API operation errors
#[derive(Debug, Error)]
pub enum ApiError {
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

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Timeout after {0:?}")]
    Timeout(Duration),

    #[error("Operation cancelled")]
    Cancelled,

    #[error("Circuit breaker open")]
    CircuitBreakerOpen,
}

impl ApiError {
    /// Get the error category for this error
    pub fn category(&self) -> ApiErrorCategory {
        match self {
            Self::Auth(_) => ApiErrorCategory::Authentication,
            Self::RateLimit(_) => ApiErrorCategory::RateLimit,
            Self::Server(_) => ApiErrorCategory::Server,
            Self::Client(_) => ApiErrorCategory::Client,
            Self::Network(_) | Self::Timeout(_) => ApiErrorCategory::Network,
            Self::Config(_) | Self::Cancelled | Self::CircuitBreakerOpen => {
                ApiErrorCategory::Config
            }
        }
    }

    /// Check if this error should be retried
    pub fn should_retry(&self) -> bool {
        matches!(
            self.category(),
            ApiErrorCategory::Authentication
                | ApiErrorCategory::RateLimit
                | ApiErrorCategory::Server
                | ApiErrorCategory::Network
        )
    }

    /// Get suggested retry delay in seconds
    pub fn retry_delay_secs(&self) -> u64 {
        match self.category() {
            ApiErrorCategory::Authentication => 5,  // Quick retry after token refresh
            ApiErrorCategory::RateLimit => 60,      // Wait for rate limit window
            ApiErrorCategory::Server => 10,         // Moderate delay for server issues
            ApiErrorCategory::Network => 5,         // Quick retry for network
            ApiErrorCategory::Client | ApiErrorCategory::Config => 0, // No retry
        }
    }
}

/// Convert from CommonError to ApiError
impl From<pulsearc_common::error::CommonError> for ApiError {
    fn from(err: pulsearc_common::error::CommonError) -> Self {
        // Simple conversion based on error message patterns
        let msg = err.to_string();
        if msg.contains("network") || msg.contains("Network") || msg.contains("timeout") {
            Self::Network(msg)
        } else if msg.contains("auth") || msg.contains("security") || msg.contains("Security") {
            Self::Auth(msg)
        } else if msg.contains("config") || msg.contains("Config") {
            Self::Config(msg)
        } else {
            Self::Client(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        assert_eq!(
            ApiError::Auth("test".to_string()).category(),
            ApiErrorCategory::Authentication
        );
        assert_eq!(
            ApiError::RateLimit("test".to_string()).category(),
            ApiErrorCategory::RateLimit
        );
        assert_eq!(
            ApiError::Server("test".to_string()).category(),
            ApiErrorCategory::Server
        );
        assert_eq!(
            ApiError::Network("test".to_string()).category(),
            ApiErrorCategory::Network
        );
    }

    #[test]
    fn test_should_retry() {
        assert!(ApiError::Auth("test".to_string()).should_retry());
        assert!(ApiError::RateLimit("test".to_string()).should_retry());
        assert!(ApiError::Server("test".to_string()).should_retry());
        assert!(ApiError::Network("test".to_string()).should_retry());
        assert!(!ApiError::Client("test".to_string()).should_retry());
        assert!(!ApiError::Config("test".to_string()).should_retry());
    }

    #[test]
    fn test_retry_delays() {
        assert_eq!(ApiError::Auth("test".to_string()).retry_delay_secs(), 5);
        assert_eq!(ApiError::RateLimit("test".to_string()).retry_delay_secs(), 60);
        assert_eq!(ApiError::Server("test".to_string()).retry_delay_secs(), 10);
        assert_eq!(ApiError::Network("test".to_string()).retry_delay_secs(), 5);
        assert_eq!(ApiError::Client("test".to_string()).retry_delay_secs(), 0);
    }
}
