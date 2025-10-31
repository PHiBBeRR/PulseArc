//! Scheduler error types

use std::time::Duration;

use pulsearc_domain::PulseArcError;
use thiserror::Error;
use tokio::task::JoinError;
use tokio::time::error::Elapsed;
use tokio_cron_scheduler::JobSchedulerError;

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
    #[error("Failed to create scheduler")]
    CreationFailed {
        #[source]
        source: JobSchedulerError,
    },

    /// Failed to start scheduler
    #[error("Failed to start scheduler")]
    StartFailed {
        #[source]
        source: JobSchedulerError,
    },

    /// Failed to stop scheduler
    #[error("Failed to stop scheduler")]
    StopFailed {
        #[source]
        source: JobSchedulerError,
    },

    /// Failed to register job
    #[error("Failed to register job")]
    JobRegistrationFailed {
        #[source]
        source: JobSchedulerError,
    },

    /// Operation timed out
    #[error("Operation timed out after {duration:?}")]
    Timeout {
        duration: Duration,
        #[source]
        source: Elapsed,
    },

    /// Task join failed
    #[error("Task join failed")]
    JoinFailed {
        #[from]
        source: JoinError,
    },
}

impl From<SchedulerError> for InfraError {
    fn from(err: SchedulerError) -> Self {
        let message = err.to_string();
        let pulse_err = match err {
            SchedulerError::AlreadyRunning | SchedulerError::NotRunning => {
                PulseArcError::InvalidInput(message)
            }
            _ => PulseArcError::Internal(message),
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
