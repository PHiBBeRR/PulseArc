use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pulsearc_core::CommandMetric;
use pulsearc_domain::PulseArcError;
use tracing::{error, info, warn};

use crate::AppContext;

/// Log the outcome of a command execution with structured fields.
///
/// # Parameters
/// * `command` - Logical command identifier (e.g. `"database::get_database_stats"`).
/// * `implementation` - Implementation path in use (`"legacy"` or `"new"`).
/// * `elapsed` - Duration the command execution took.
/// * `success` - Whether the command completed successfully.
///
/// The helper keeps our command wrappers concise and ensures consistency across
/// the migration. Callers must avoid forwarding sensitive values in `command`
/// or `implementation`.
#[inline]
pub fn log_command_execution(
    command: &str,
    implementation: &str,
    elapsed: Duration,
    success: bool,
) {
    let duration_ms = elapsed.as_millis() as u64;

    if success {
        info!(command, implementation, duration_ms, "command_execution_success");
    } else {
        warn!(command, implementation, duration_ms, "command_execution_failure");
    }
}

/// Log the outcome of a feature flag evaluation.
///
/// `flag_name` should be a stable identifier without sensitive data.
#[inline]
pub fn log_feature_flag_check(flag_name: &str, is_enabled: bool, fallback_used: bool) {
    info!(flag_name, is_enabled, fallback_used, "feature_flag_evaluated");
}

/// Convert a `PulseArcError` into a stable label suitable for metrics/logging.
#[inline]
pub fn error_label(error: &PulseArcError) -> &'static str {
    match error {
        PulseArcError::Database(_) => "database",
        PulseArcError::Config(_) => "config",
        PulseArcError::Platform(_) => "platform",
        PulseArcError::Network(_) => "network",
        PulseArcError::Auth(_) => "auth",
        PulseArcError::Security(_) => "security",
        PulseArcError::NotFound(_) => "not_found",
        PulseArcError::InvalidInput(_) => "invalid_input",
        PulseArcError::Internal(_) => "internal",
    }
}

/// Persist command execution metrics for Phase 4 validation.
///
/// Any failure to record metrics is logged and ignored so command execution
/// is never blocked by observability plumbing.
pub async fn record_command_metric(
    context: &Arc<AppContext>,
    command: &str,
    implementation: &str,
    elapsed: Duration,
    success: bool,
    error_type: Option<&str>,
) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_secs() as i64)
        .unwrap_or_default();

    let metric = CommandMetric {
        id: uuid::Uuid::new_v4().to_string(),
        command: command.to_string(),
        implementation: implementation.to_string(),
        timestamp,
        duration_ms: elapsed.as_millis() as u64,
        success,
        error_type: error_type.map(|label| label.to_string()),
    };

    if let Err(err) = context.command_metrics.record_execution(metric).await {
        error!(
            command,
            implementation,
            error = %err,
            "failed to record command metric"
        );
    }
}
