//! Storage error types
//!
//! Defines error types for the storage layer, integrating with the agent's
//! common error system.

use thiserror::Error;

use crate::error::{ErrorClassification, ErrorSeverity};

/// Storage error type
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database connection error: {0}")]
    Connection(String),

    #[error("Database query error: {0}")]
    Query(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Database encryption error: {0}")]
    Encryption(String),

    #[error("Database migration error: {0}")]
    Migration(String),

    #[error("Keychain error: {0}")]
    Keychain(String),

    #[error("Wrong encryption key or database not encrypted")]
    WrongKeyOrNotEncrypted,

    #[error("Database pool exhausted")]
    PoolExhausted,

    #[error("Connection timeout after {0}s")]
    Timeout(u64),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Schema version mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: i32, found: i32 },

    #[error(transparent)]
    Common(#[from] crate::CommonError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Rusqlite(#[from] rusqlite::Error),

    #[error(transparent)]
    R2d2(#[from] r2d2::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}

/// Storage result type
pub type StorageResult<T> = Result<T, StorageError>;

impl ErrorClassification for StorageError {
    /// Check if this error is retryable
    ///
    /// Retryable errors include:
    /// - Connection timeouts
    /// - Pool exhaustion
    /// - Transient database locks
    fn is_retryable(&self) -> bool {
        match self {
            Self::PoolExhausted => true,
            Self::Timeout(_) => true,
            Self::Connection(_) => true, // Connection errors may be transient
            Self::Rusqlite(err) => {
                // SQLite BUSY and LOCKED errors are retryable
                matches!(
                    err.sqlite_error_code(),
                    Some(rusqlite::ErrorCode::DatabaseBusy)
                        | Some(rusqlite::ErrorCode::DatabaseLocked)
                )
            }
            Self::Common(common_err) => common_err.is_retryable(),
            _ => false,
        }
    }

    /// Get the error severity level
    ///
    /// Used for monitoring and alerting. Maps to CommonError severity levels.
    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Connection(_) => ErrorSeverity::Error,
            Self::Query(_) => ErrorSeverity::Error,
            Self::DatabaseError(_) => ErrorSeverity::Error,
            Self::Encryption(_) => ErrorSeverity::Critical,
            Self::Migration(_) => ErrorSeverity::Critical,
            Self::Keychain(_) => ErrorSeverity::Critical,
            Self::WrongKeyOrNotEncrypted => ErrorSeverity::Critical,
            Self::PoolExhausted => ErrorSeverity::Warning,
            Self::Timeout(_) => ErrorSeverity::Warning,
            Self::InvalidConfig(_) => ErrorSeverity::Error,
            Self::SchemaVersionMismatch { .. } => ErrorSeverity::Critical,
            Self::Common(common_err) => common_err.severity(),
            Self::Io(_) => ErrorSeverity::Error,
            Self::Rusqlite(_) => ErrorSeverity::Error,
            Self::R2d2(_) => ErrorSeverity::Error,
            Self::SerdeJson(_) => ErrorSeverity::Error,
        }
    }

    /// Check if this is a critical error requiring immediate attention
    fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::Encryption(_)
                | Self::Migration(_)
                | Self::Keychain(_)
                | Self::WrongKeyOrNotEncrypted
                | Self::SchemaVersionMismatch { .. }
        ) || matches!(self, Self::Common(err) if err.is_critical())
    }

    /// Get the suggested retry delay if applicable
    fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            Self::Common(common_err) => common_err.retry_after(),
            _ => None,
        }
    }
}

impl StorageError {
    /// Add operation context to the error
    ///
    /// Creates a CommonError with the operation context for better debugging.
    pub fn with_operation(self, operation: impl Into<String>) -> Self {
        let operation = operation.into();
        Self::Common(crate::CommonError::Storage {
            message: self.to_string(),
            operation: Some(operation),
        })
    }
}

/// Convert StorageError to CommonError for integration
impl From<StorageError> for crate::CommonError {
    fn from(err: StorageError) -> Self {
        // If already a CommonError variant, preserve it
        if let StorageError::Common(common_err) = err {
            return common_err;
        }

        crate::CommonError::Storage { message: err.to_string(), operation: None }
    }
}

/// Convert KeychainError to StorageError
impl From<crate::security::KeychainError> for StorageError {
    fn from(e: crate::security::KeychainError) -> Self {
        Self::Keychain(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for storage::error.
    use super::*;
    use crate::error::{ErrorClassification, ErrorSeverity};

    /// Validates `StorageError::Connection` behavior for the error display
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Database connection error: Failed
    ///   to connect"`.
    /// - Confirms `err.to_string()` equals `"Wrong encryption key or database
    ///   not encrypted"`.
    /// - Confirms `err.to_string()` equals `"Connection timeout after 5s"`.
    #[test]
    fn test_error_display() {
        let err = StorageError::Connection("Failed to connect".to_string());
        assert_eq!(err.to_string(), "Database connection error: Failed to connect");

        let err = StorageError::WrongKeyOrNotEncrypted;
        assert_eq!(err.to_string(), "Wrong encryption key or database not encrypted");

        let err = StorageError::Timeout(5);
        assert_eq!(err.to_string(), "Connection timeout after 5s");
    }

    /// Validates `StorageError::SchemaVersionMismatch` behavior for the schema
    /// version mismatch scenario.
    ///
    /// Assertions:
    /// - Confirms `err.to_string()` equals `"Schema version mismatch: expected
    ///   11`.
    #[test]
    fn test_schema_version_mismatch() {
        let err = StorageError::SchemaVersionMismatch { expected: 11, found: 10 };
        assert_eq!(err.to_string(), "Schema version mismatch: expected 11, found 10");
    }

    /// Validates `StorageError::PoolExhausted` behavior for the error
    /// retryability scenario.
    ///
    /// Assertions:
    /// - Ensures `StorageError::PoolExhausted.is_retryable()` evaluates to
    ///   true.
    /// - Ensures `StorageError::Timeout(5).is_retryable()` evaluates to true.
    /// - Ensures `StorageError::Connection("test".to_string()).is_retryable()`
    ///   evaluates to true.
    /// - Ensures `!StorageError::InvalidConfig("test".to_string()).
    ///   is_retryable()` evaluates to true.
    /// - Ensures `!StorageError::WrongKeyOrNotEncrypted.is_retryable()`
    ///   evaluates to true.
    #[test]
    fn test_error_retryability() {
        assert!(StorageError::PoolExhausted.is_retryable());
        assert!(StorageError::Timeout(5).is_retryable());
        assert!(StorageError::Connection("test".to_string()).is_retryable());
        assert!(!StorageError::InvalidConfig("test".to_string()).is_retryable());
        assert!(!StorageError::WrongKeyOrNotEncrypted.is_retryable());
    }

    /// Validates `StorageError::Timeout` behavior for the error severity
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `StorageError::Timeout(5).severity()` equals
    ///   `ErrorSeverity::Warning`.
    /// - Confirms `StorageError::Encryption("test".to_string()).severity()`
    ///   equals `ErrorSeverity::Critical`.
    /// - Confirms `StorageError::Connection("test".to_string()).severity()`
    ///   equals `ErrorSeverity::Error`.
    #[test]
    fn test_error_severity() {
        assert_eq!(StorageError::Timeout(5).severity(), ErrorSeverity::Warning);
        assert_eq!(
            StorageError::Encryption("test".to_string()).severity(),
            ErrorSeverity::Critical
        );
        assert_eq!(StorageError::Connection("test".to_string()).severity(), ErrorSeverity::Error);
    }

    /// Validates `StorageError::Encryption` behavior for the error criticality
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `StorageError::Encryption("test".to_string()).is_critical()`
    ///   evaluates to true.
    /// - Ensures `StorageError::WrongKeyOrNotEncrypted.is_critical()` evaluates
    ///   to true.
    /// - Ensures `StorageError::SchemaVersionMismatch { expected: 2, found: 1
    ///   }.is_critical()` evaluates to true.
    /// - Ensures `!StorageError::Timeout(5).is_critical()` evaluates to true.
    /// - Ensures `!StorageError::PoolExhausted.is_critical()` evaluates to
    ///   true.
    #[test]
    fn test_error_criticality() {
        assert!(StorageError::Encryption("test".to_string()).is_critical());
        assert!(StorageError::WrongKeyOrNotEncrypted.is_critical());
        assert!(StorageError::SchemaVersionMismatch { expected: 2, found: 1 }.is_critical());
        assert!(!StorageError::Timeout(5).is_critical());
        assert!(!StorageError::PoolExhausted.is_critical());
    }

    /// Validates `StorageError::Query` behavior for the with operation
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `common_err.to_string().contains("fetch_user")` evaluates to
    ///   true.
    #[test]
    fn test_with_operation() {
        let err = StorageError::Query("SELECT failed".to_string()).with_operation("fetch_user");

        if let StorageError::Common(common_err) = err {
            assert!(common_err.to_string().contains("fetch_user"));
        } else {
            panic!("Expected Common error variant");
        }
    }
}
