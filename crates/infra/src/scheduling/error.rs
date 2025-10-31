//! Scheduler error types

use pulsearc_domain::PulseArcError;
use thiserror::Error;

use crate::errors::InfraError;

/// Scheduler-specific errors
#[derive(Debug, Error)]
pub enum SchedulerError {
    /// Scheduler is already running
    #[error("Scheduler already running")]
    AlreadyRunning,

    /// Scheduler is not running
    #[error("Scheduler not running")]
    NotRunning,

    /// Failed to create scheduler
    #[error("Failed to create scheduler: {0}")]
    CreationFailed(String),

    /// Failed to start scheduler
    #[error("Failed to start scheduler: {0}")]
    StartFailed(String),

    /// Failed to stop scheduler
    #[error("Failed to stop scheduler: {0}")]
    StopFailed(String),

    /// Failed to register job
    #[error("Failed to register job: {0}")]
    JobRegistrationFailed(String),

    /// Operation timed out
    #[error("Operation timed out after {seconds}s")]
    Timeout { seconds: u64 },

    /// Task join failed
    #[error("Task join failed: {0}")]
    TaskJoinFailed(String),
}

impl From<SchedulerError> for InfraError {
    fn from(err: SchedulerError) -> Self {
        let pulse_err = match err {
            SchedulerError::AlreadyRunning | SchedulerError::NotRunning => {
                PulseArcError::InvalidInput(err.to_string())
            }
            SchedulerError::Timeout { .. } => PulseArcError::Internal(err.to_string()),
            _ => PulseArcError::Internal(err.to_string()),
        };
        InfraError(pulse_err)
    }
}

impl From<SchedulerError> for PulseArcError {
    fn from(err: SchedulerError) -> Self {
        InfraError::from(err).into()
    }
}

/// Convenience type alias for scheduler operations
pub type SchedulerResult<T> = Result<T, SchedulerError>;
