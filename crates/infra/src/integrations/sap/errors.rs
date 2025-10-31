//! SAP-specific error types and classification
//!
//! This module provides user-friendly error categorization for SAP integration errors,
//! with retry recommendations and conversion to domain error types.

use pulsearc_domain::PulseArcError;
use reqwest::StatusCode;
use std::fmt;

/// SAP error category for external consumption
///
/// Classifies errors by type to enable appropriate retry strategies
/// and user-facing messaging.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(Copy))]
pub enum SapErrorCategory {
    /// Network is offline or unreachable
    NetworkOffline,

    /// Network request timed out
    NetworkTimeout,

    /// SAP server is unavailable (5xx errors)
    ServerUnavailable,

    /// Authentication failed (401, 403)
    Authentication,

    /// Rate limit exceeded (429)
    RateLimited,

    /// Invalid request or data (4xx except 401, 403, 429)
    Validation,

    /// Unknown or unclassified error
    Unknown,
}

impl SapErrorCategory {
    /// Returns true if this error type should be retried
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::NetworkOffline
                | Self::NetworkTimeout
                | Self::ServerUnavailable
                | Self::RateLimited
        )
    }

    /// Returns recommended retry delay in seconds
    pub fn retry_delay_secs(&self) -> Option<u64> {
        match self {
            Self::NetworkOffline => Some(30),
            Self::NetworkTimeout => Some(10),
            Self::ServerUnavailable => Some(60),
            Self::RateLimited => Some(120),
            _ => None,
        }
    }

    /// Returns user-friendly message for this category
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::NetworkOffline => {
                "No network connection. Please check your internet connection and try again."
            }
            Self::NetworkTimeout => {
                "The SAP server took too long to respond. Please try again in a few moments."
            }
            Self::ServerUnavailable => {
                "The SAP server is temporarily unavailable. This is usually temporary - please \
                 try again in a minute."
            }
            Self::Authentication => {
                "Authentication failed. Please sign out and sign in again to refresh your \
                 credentials."
            }
            Self::RateLimited => {
                "Too many requests. Please wait a couple minutes before trying again."
            }
            Self::Validation => {
                "Invalid request data. Please check your WBS code and time entry details."
            }
            Self::Unknown => {
                "An unexpected error occurred. Please try again or contact support if the problem \
                 persists."
            }
        }
    }
}

impl fmt::Display for SapErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkOffline => write!(f, "Network Offline"),
            Self::NetworkTimeout => write!(f, "Network Timeout"),
            Self::ServerUnavailable => write!(f, "Server Unavailable"),
            Self::Authentication => write!(f, "Authentication Failed"),
            Self::RateLimited => write!(f, "Rate Limited"),
            Self::Validation => write!(f, "Validation Error"),
            Self::Unknown => write!(f, "Unknown Error"),
        }
    }
}

/// Internal SAP-specific error with retry metadata
///
/// This type is used internally within the SAP module for detailed
/// error handling. External callers receive `PulseArcError` via conversion.
#[derive(Debug, Clone)]
pub struct SapError {
    category: SapErrorCategory,
    message: String,
    context: Option<String>,
}

impl SapError {
    /// Create a new SAP error
    pub fn new(category: SapErrorCategory, message: impl Into<String>) -> Self {
        Self { category, message: message.into(), context: None }
    }

    /// Create an unknown error (used for unexpected failures)
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::new(SapErrorCategory::Unknown, message)
    }

    /// Add context to the error
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Get the error category
    pub fn category(&self) -> &SapErrorCategory {
        &self.category
    }

    /// Get the error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the error context
    pub fn context(&self) -> Option<&str> {
        self.context.as_deref()
    }

    /// Returns true if this error should be retried
    pub fn is_retryable(&self) -> bool {
        self.category.is_retryable()
    }

    /// Returns recommended retry delay
    pub fn retry_delay_secs(&self) -> Option<u64> {
        self.category.retry_delay_secs()
    }

    /// Get user-friendly message
    pub fn user_message(&self) -> String {
        let base = self.category.user_message();
        if let Some(ctx) = &self.context {
            format!("{} Details: {}", base, ctx)
        } else {
            base.to_string()
        }
    }

    /// Classify HTTP status code into error category
    pub fn from_status_code(status: StatusCode) -> Self {
        let category = match status.as_u16() {
            401 | 403 => SapErrorCategory::Authentication,
            429 => SapErrorCategory::RateLimited,
            400 | 404 | 422 => SapErrorCategory::Validation,
            500..=599 => SapErrorCategory::ServerUnavailable,
            _ => SapErrorCategory::Unknown,
        };

        Self::new(
            category,
            format!("HTTP {}: {}", status.as_u16(), status.canonical_reason().unwrap_or("Unknown")),
        )
    }

    /// Convert to domain error type
    pub fn into_domain_error(self) -> PulseArcError {
        match self.category {
            SapErrorCategory::Authentication => PulseArcError::Auth(self.user_message()),
            SapErrorCategory::Validation => PulseArcError::InvalidInput(self.user_message()),
            SapErrorCategory::NetworkOffline
            | SapErrorCategory::NetworkTimeout
            | SapErrorCategory::ServerUnavailable
            | SapErrorCategory::RateLimited => PulseArcError::Network(self.user_message()),
            SapErrorCategory::Unknown => PulseArcError::Internal(self.user_message()),
        }
    }
}

impl fmt::Display for SapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.category, self.message)?;
        if let Some(ctx) = &self.context {
            write!(f, " ({})", ctx)?;
        }
        Ok(())
    }
}

impl std::error::Error for SapError {}

/// Convert reqwest errors to SAP errors
impl From<reqwest::Error> for SapError {
    fn from(err: reqwest::Error) -> Self {
        // Classify reqwest error by type
        let (category, message) = if err.is_timeout() {
            (SapErrorCategory::NetworkTimeout, "Request timed out".to_string())
        } else if err.is_connect() {
            (SapErrorCategory::NetworkOffline, "Failed to connect to SAP server".to_string())
        } else if let Some(status) = err.status() {
            // Has HTTP status - classify by status code
            return Self::from_status_code(status).with_context(err.to_string());
        } else if err.is_request() {
            (SapErrorCategory::Validation, "Invalid request".to_string())
        } else {
            (SapErrorCategory::Unknown, "Network error".to_string())
        };

        Self::new(category, message).with_context(err.to_string())
    }
}

/// Convert from StatusCode directly
impl From<StatusCode> for SapError {
    fn from(status: StatusCode) -> Self {
        Self::from_status_code(status)
    }
}

/// Convenience conversion to Result
impl From<SapError> for PulseArcError {
    fn from(err: SapError) -> Self {
        err.into_domain_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_network_offline_is_retryable() {
        let category = SapErrorCategory::NetworkOffline;
        assert!(category.is_retryable());
        assert_eq!(category.retry_delay_secs(), Some(30));
        assert!(category.user_message().contains("network connection"));
    }

    #[test]
    fn category_network_timeout_is_retryable() {
        let category = SapErrorCategory::NetworkTimeout;
        assert!(category.is_retryable());
        assert_eq!(category.retry_delay_secs(), Some(10));
        assert!(category.user_message().contains("took too long"));
    }

    #[test]
    fn category_server_unavailable_is_retryable() {
        let category = SapErrorCategory::ServerUnavailable;
        assert!(category.is_retryable());
        assert_eq!(category.retry_delay_secs(), Some(60));
        assert!(category.user_message().contains("temporarily unavailable"));
    }

    #[test]
    fn category_rate_limited_is_retryable() {
        let category = SapErrorCategory::RateLimited;
        assert!(category.is_retryable());
        assert_eq!(category.retry_delay_secs(), Some(120));
        assert!(category.user_message().contains("Too many requests"));
    }

    #[test]
    fn category_authentication_not_retryable() {
        let category = SapErrorCategory::Authentication;
        assert!(!category.is_retryable());
        assert_eq!(category.retry_delay_secs(), None);
        assert!(category.user_message().contains("Authentication failed"));
    }

    #[test]
    fn category_validation_not_retryable() {
        let category = SapErrorCategory::Validation;
        assert!(!category.is_retryable());
        assert_eq!(category.retry_delay_secs(), None);
        assert!(category.user_message().contains("Invalid request"));
    }

    #[test]
    fn category_unknown_not_retryable() {
        let category = SapErrorCategory::Unknown;
        assert!(!category.is_retryable());
        assert_eq!(category.retry_delay_secs(), None);
        assert!(category.user_message().contains("unexpected error"));
    }

    #[test]
    fn status_401_maps_to_authentication() {
        let err = SapError::from_status_code(StatusCode::UNAUTHORIZED);
        assert_eq!(err.category(), &SapErrorCategory::Authentication);
        assert!(!err.is_retryable());
    }

    #[test]
    fn status_403_maps_to_authentication() {
        let err = SapError::from_status_code(StatusCode::FORBIDDEN);
        assert_eq!(err.category(), &SapErrorCategory::Authentication);
    }

    #[test]
    fn status_429_maps_to_rate_limited() {
        let err = SapError::from_status_code(StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(err.category(), &SapErrorCategory::RateLimited);
        assert!(err.is_retryable());
        assert_eq!(err.retry_delay_secs(), Some(120));
    }

    #[test]
    fn status_400_maps_to_validation() {
        let err = SapError::from_status_code(StatusCode::BAD_REQUEST);
        assert_eq!(err.category(), &SapErrorCategory::Validation);
        assert!(!err.is_retryable());
    }

    #[test]
    fn status_500_maps_to_server_unavailable() {
        let err = SapError::from_status_code(StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.category(), &SapErrorCategory::ServerUnavailable);
        assert!(err.is_retryable());
    }

    #[test]
    fn unknown_status_maps_to_unknown_category() {
        // Test failure path: unusual status code
        let err = SapError::from_status_code(StatusCode::from_u16(999).unwrap());
        assert_eq!(err.category(), &SapErrorCategory::Unknown);
        assert!(!err.is_retryable());
    }

    #[test]
    fn error_with_context_includes_details() {
        let err = SapError::new(SapErrorCategory::Validation, "Invalid WBS code")
            .with_context("Expected format: USC1234567.1.1");

        assert!(err.user_message().contains("Details:"));
        assert!(err.user_message().contains("USC1234567.1.1"));
        assert_eq!(err.context(), Some("Expected format: USC1234567.1.1"));
    }

    #[test]
    fn converts_to_domain_error() {
        let sap_err = SapError::new(SapErrorCategory::Authentication, "Token expired");
        let domain_err: PulseArcError = sap_err.into();

        match domain_err {
            PulseArcError::Auth(msg) => {
                assert!(msg.contains("Authentication failed"));
            }
            _ => panic!("Expected Auth error variant"),
        }
    }
}
