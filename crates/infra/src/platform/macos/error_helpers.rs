//! Error mapping helpers for macOS platform operations
//!
//! This module provides helper functions to convert platform-specific errors
//! (IO errors, task join errors, AX API errors) into the domain's
//! `PulseArcError` type.
//!
//! # Error Mapping Strategy
//!
//! - `tokio::task::JoinError` → `PulseArcError::Internal` (runtime/task errors)
//! - AppleScript/IO errors → `PulseArcError::Platform` (macOS-specific
//!   failures)
//! - AX API errors → `PulseArcError::Platform` (accessibility-related failures)
//! - Enrichment failures → `PulseArcError::Internal` (non-critical failures)

use pulsearc_domain::PulseArcError;

/// Map a `tokio::task::JoinError` to `PulseArcError::Internal`.
///
/// This handles errors from `spawn_blocking` calls, differentiating between
/// task cancellation and task panics.
///
/// # Examples
///
/// ```rust,ignore
/// tokio::task::spawn_blocking(move || {
///     // ... blocking AX API call ...
/// })
/// .await
/// .map_err(map_join_error)?;
/// ```
#[inline]
pub(crate) fn map_join_error(err: tokio::task::JoinError) -> PulseArcError {
    if err.is_cancelled() {
        PulseArcError::Internal("platform task cancelled".into())
    } else {
        PulseArcError::Internal(format!("platform task panicked: {err}"))
    }
}

/// Map an IO error with context to `PulseArcError::Platform`.
///
/// This is used for platform-specific IO operations like AppleScript execution,
/// where the operation name provides useful debugging context.
///
/// # Arguments
///
/// * `operation` - Human-readable description of the operation (e.g.,
///   "AppleScript execution")
/// * `err` - The IO error that occurred
///
/// # Examples
///
/// ```rust,ignore
/// Command::new("osascript")
///     .arg("-e")
///     .arg(script)
///     .spawn()
///     .map_err(|e| map_platform_io_error("AppleScript spawn", e))?;
/// ```
#[inline]
#[allow(dead_code)] // Used in Day 2 (enrichers)
pub(crate) fn map_platform_io_error(operation: &str, err: std::io::Error) -> PulseArcError {
    PulseArcError::Platform(format!("{operation} failed: {err}"))
}

/// Create an accessibility permission denied error.
///
/// Returns a user-friendly error message directing the user to enable
/// accessibility permissions in System Settings.
///
/// # Examples
///
/// ```rust,ignore
/// if !is_ax_permission_granted() {
///     return Err(ax_permission_error());
/// }
/// ```
#[inline]
#[allow(dead_code)] // Used in Day 2 (enrichers)
pub(crate) fn ax_permission_error() -> PulseArcError {
    PulseArcError::Platform(
        "Accessibility permission denied. Enable in: \
         System Settings > Privacy & Security > Accessibility"
            .to_string(),
    )
}

/// Create an AppleScript timeout error with the duration.
///
/// # Arguments
///
/// * `duration_ms` - Timeout duration in milliseconds
///
/// # Examples
///
/// ```rust,ignore
/// match child.wait_timeout(timeout) {
///     Ok(None) => return Err(applescript_timeout_error(200)),
///     Ok(Some(status)) => { /* ... */ }
///     Err(e) => return Err(map_platform_io_error("wait", e)),
/// }
/// ```
#[inline]
#[allow(dead_code)] // Used in Day 2 (enrichers)
pub(crate) fn applescript_timeout_error(duration_ms: u64) -> PulseArcError {
    PulseArcError::Platform(format!("AppleScript timeout after {duration_ms}ms"))
}

/// Create an enrichment failure error (non-critical).
///
/// Used when enrichment operations fail but should not prevent the overall
/// activity capture from succeeding.
///
/// # Arguments
///
/// * `context` - Context about what enrichment failed (e.g., "browser URL
///   capture")
/// * `err` - The underlying error
///
/// # Examples
///
/// ```rust,ignore
/// if let Err(e) = enrich_browser_url(context).await {
///     tracing::warn!(error = %e, "enrichment failed");
///     return Err(enrichment_error("browser URL", &e));
/// }
/// ```
#[inline]
#[allow(dead_code)] // Used in Day 2 (enrichers)
pub(crate) fn enrichment_error(context: &str, err: &dyn std::error::Error) -> PulseArcError {
    PulseArcError::Internal(format!("{context} enrichment failed: {err}"))
}

#[cfg(test)]
mod tests {
    use tokio::task;

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_map_join_error_cancelled() {
        // Simulate a cancelled task
        let handle = task::spawn(async {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        });
        handle.abort();

        let result = handle.await;
        assert!(result.is_err());

        if let Err(e) = result {
            let pulse_err = map_join_error(e);
            match pulse_err {
                PulseArcError::Internal(msg) => {
                    assert!(msg.contains("cancelled") || msg.contains("panicked"));
                }
                _ => panic!("Expected Internal error"),
            }
        }
    }

    #[test]
    fn test_map_platform_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let pulse_err = map_platform_io_error("test operation", io_err);

        match pulse_err {
            PulseArcError::Platform(msg) => {
                assert!(msg.contains("test operation"));
                assert!(msg.contains("failed"));
            }
            _ => panic!("Expected Platform error"),
        }
    }

    #[test]
    fn test_ax_permission_error() {
        let err = ax_permission_error();

        match err {
            PulseArcError::Platform(msg) => {
                assert!(msg.contains("Accessibility"));
                assert!(msg.contains("System Settings"));
            }
            _ => panic!("Expected Platform error"),
        }
    }

    #[test]
    fn test_applescript_timeout_error() {
        let err = applescript_timeout_error(200);

        match err {
            PulseArcError::Platform(msg) => {
                assert!(msg.contains("timeout"));
                assert!(msg.contains("200ms"));
            }
            _ => panic!("Expected Platform error"),
        }
    }

    #[test]
    fn test_enrichment_error() {
        let underlying = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
        let err = enrichment_error("browser", &underlying);

        match err {
            PulseArcError::Internal(msg) => {
                assert!(msg.contains("browser"));
                assert!(msg.contains("enrichment failed"));
            }
            _ => panic!("Expected Internal error"),
        }
    }
}
