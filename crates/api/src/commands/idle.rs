//! Idle period management commands
//!
//! FEATURE-028: Tauri commands for querying and managing idle periods detected
//! by the idle detection system. Users can review idle periods and decide
//! whether to keep or discard them from their activity tracking.
//!
//! Migration Status: Phase 4B.3
//! - Feature flag: `new_idle_commands`
//! - Legacy: `legacy/api/src/commands/idle.rs`

use std::sync::Arc;
use std::time::Instant;

use pulsearc_domain::{IdlePeriod, IdleSummary, PulseArcError};
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::task;
use tracing::{debug, error, info, warn};

use crate::context::AppContext;
use crate::utils::logging::{log_command_execution, record_command_metric, MetricRecord};

/// Idle detection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleSettings {
    pub pause_on_idle: bool,
    pub idle_threshold_secs: i64,
}

/// Valid user actions for idle periods
const VALID_ACTIONS: &[&str] = &["kept", "discarded", "auto_excluded", "pending"];

// =============================================================================
// Public Tauri Commands
// =============================================================================

/// Get idle periods within a time range
///
/// # Arguments
/// * `context` - Application context with idle periods repository
/// * `start_ts` - Start of time range (Unix epoch seconds)
/// * `end_ts` - End of time range (Unix epoch seconds)
///
/// # Returns
/// Vector of idle periods sorted by start time (ascending in new impl,
/// descending in legacy)
///
/// # Feature Flag
/// Controlled by `new_idle_commands` feature flag
#[tauri::command]
pub async fn get_idle_periods(
    context: State<'_, Arc<AppContext>>,
    start_ts: i64,
    end_ts: i64,
) -> Result<Vec<IdlePeriod>, String> {
    let start_time = Instant::now();
    debug!(start_ts, end_ts, "get_idle_periods called");

    let app_ctx = Arc::clone(context.inner());
    let use_new_impl =
        app_ctx.feature_flags.is_enabled("new_idle_commands", false).await.unwrap_or(false);

    let implementation = if use_new_impl { "new" } else { "legacy" };

    let result = if use_new_impl {
        debug!("using new idle periods implementation");
        get_idle_periods_new(&app_ctx, start_ts, end_ts).await
    } else {
        debug!("using legacy idle periods implementation");
        get_idle_periods_legacy(&app_ctx, start_ts, end_ts).await
    };

    let success = result.is_ok();
    let error_label = if success { None } else { Some("idle_error") };
    let elapsed = start_time.elapsed();

    log_command_execution("idle::get_idle_periods", implementation, elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: "idle::get_idle_periods",
            implementation,
            elapsed,
            success,
            error_type: error_label,
        },
    )
    .await;

    if let Err(ref err) = result {
        error!(error = %err, "get_idle_periods failed");
    }

    result
}

/// Update user decision on an idle period
///
/// # Arguments
/// * `context` - Application context with idle periods repository
/// * `period_id` - ID of the idle period to update
/// * `action` - User decision (must be one of: kept, discarded, auto_excluded,
///   pending)
/// * `notes` - Optional user notes about the decision
///
/// # Returns
/// Success or error message
///
/// # Feature Flag
/// Controlled by `new_idle_commands` feature flag
#[tauri::command]
pub async fn update_idle_period_action(
    context: State<'_, Arc<AppContext>>,
    period_id: String,
    action: String,
    notes: Option<String>,
) -> Result<(), String> {
    let start_time = Instant::now();
    debug!(period_id, action, "update_idle_period_action called");

    // Validate action before proceeding
    if !VALID_ACTIONS.contains(&action.as_str()) {
        warn!(action, "invalid idle period action");
        return Err(format!("Invalid action: '{}'. Must be one of: {:?}", action, VALID_ACTIONS));
    }

    let app_ctx = Arc::clone(context.inner());
    let use_new_impl =
        app_ctx.feature_flags.is_enabled("new_idle_commands", false).await.unwrap_or(false);

    let implementation = if use_new_impl { "new" } else { "legacy" };

    let result = if use_new_impl {
        debug!("using new update idle period implementation");
        update_idle_period_action_new(&app_ctx, &period_id, &action, notes).await
    } else {
        debug!("using legacy update idle period implementation");
        update_idle_period_action_legacy(&app_ctx, &period_id, &action, notes).await
    };

    let success = result.is_ok();
    let error_label = if success { None } else { Some("idle_error") };
    let elapsed = start_time.elapsed();

    log_command_execution("idle::update_idle_period_action", implementation, elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: "idle::update_idle_period_action",
            implementation,
            elapsed,
            success,
            error_type: error_label,
        },
    )
    .await;

    if let Err(ref err) = result {
        error!(error = %err, period_id, "update_idle_period_action failed");
    }

    result
}

/// Get idle time summary for a specific date
///
/// Calculates aggregated statistics including total active time, total idle
/// time, idle period count, and idle time breakdowns by user action
/// (kept/discarded/pending).
///
/// # Arguments
/// * `context` - Application context with idle periods repository
/// * `date` - Date string in format "YYYY-MM-DD"
///
/// # Returns
/// `IdleSummary` with aggregated idle time statistics
///
/// # Feature Flag
/// Controlled by `new_idle_commands` feature flag
#[tauri::command]
pub async fn get_idle_summary(
    context: State<'_, Arc<AppContext>>,
    date: String,
) -> Result<IdleSummary, String> {
    let start_time = Instant::now();
    debug!(date, "get_idle_summary called");

    let app_ctx = Arc::clone(context.inner());
    let use_new_impl =
        app_ctx.feature_flags.is_enabled("new_idle_commands", false).await.unwrap_or(false);

    let implementation = if use_new_impl { "new" } else { "legacy" };

    let result = if use_new_impl {
        debug!("using new idle summary implementation");
        get_idle_summary_new(&app_ctx, &date).await
    } else {
        debug!("using legacy idle summary implementation");
        get_idle_summary_legacy(&app_ctx, &date).await
    };

    let success = result.is_ok();
    let error_label = if success { None } else { Some("idle_error") };
    let elapsed = start_time.elapsed();

    log_command_execution("idle::get_idle_summary", implementation, elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: "idle::get_idle_summary",
            implementation,
            elapsed,
            success,
            error_type: error_label,
        },
    )
    .await;

    if let Err(ref err) = result {
        error!(error = %err, date, "get_idle_summary failed");
    }

    result
}

// =============================================================================
// Idle Settings Commands (Configuration)
// =============================================================================

/// Get idle detection settings
///
/// Returns the current idle detection configuration including whether tracking
/// should pause on idle and the idle threshold in seconds.
#[tauri::command]
pub async fn get_idle_settings(
    context: State<'_, Arc<AppContext>>,
) -> std::result::Result<IdleSettings, String> {
    let start_time = Instant::now();
    let command_name = "idle::get_idle_settings";
    info!(command = command_name, "Getting idle settings");

    let app_ctx = Arc::clone(context.inner());
    let db = Arc::clone(&app_ctx.db);

    let result = task::spawn_blocking(move || -> Result<IdleSettings, PulseArcError> {
        let conn = db.get_connection()?;

        // Query settings from database, or use defaults if not found
        let settings = match conn.query_row(
            "SELECT pause_on_idle, idle_threshold_secs FROM idle_settings WHERE id = 1",
            rusqlite::params![],
            |row| {
                Ok(IdleSettings {
                    pause_on_idle: row.get::<_, i64>(0)? != 0,
                    idle_threshold_secs: row.get(1)?,
                })
            },
        ) {
            Ok(settings) => settings,
            Err(_) => {
                // If no settings exist, return defaults
                IdleSettings {
                    pause_on_idle: true,
                    idle_threshold_secs: 600, // 10 minutes default
                }
            }
        };

        Ok(settings)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    let elapsed = start_time.elapsed();
    let success = result.is_ok();

    log_command_execution(command_name, "new", elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation: "new",
            elapsed,
            success,
            error_type: if !success { Some("settings_error") } else { None },
        },
    )
    .await;

    result.map_err(|e| e.to_string())
}

/// Set whether idle detection is enabled
///
/// # Arguments
/// * `enabled` - Whether to pause tracking when idle is detected
#[tauri::command]
pub async fn set_idle_enabled(
    context: State<'_, Arc<AppContext>>,
    enabled: bool,
) -> std::result::Result<(), String> {
    let start_time = Instant::now();
    let command_name = "idle::set_idle_enabled";
    info!(command = command_name, enabled, "Setting idle enabled status");

    let app_ctx = Arc::clone(context.inner());
    let db = Arc::clone(&app_ctx.db);

    let result = task::spawn_blocking(move || -> Result<(), PulseArcError> {
        let conn = db.get_connection()?;

        // Ensure table exists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS idle_settings (
                id INTEGER PRIMARY KEY,
                pause_on_idle BOOLEAN NOT NULL DEFAULT 1,
                idle_threshold_secs INTEGER NOT NULL DEFAULT 600
            )",
            rusqlite::params![],
        )
        .map_err(|e| PulseArcError::Database(format!("Failed to create settings table: {}", e)))?;

        // Insert or update
        conn.execute(
            "INSERT INTO idle_settings (id, pause_on_idle) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET pause_on_idle = ?1",
            rusqlite::params![enabled as i64],
        )
        .map_err(|e| PulseArcError::Database(format!("Failed to update idle enabled: {}", e)))?;

        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    let elapsed = start_time.elapsed();
    let success = result.is_ok();

    log_command_execution(command_name, "new", elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation: "new",
            elapsed,
            success,
            error_type: if !success { Some("settings_error") } else { None },
        },
    )
    .await;

    result.map_err(|e| e.to_string())
}

/// Set the idle detection threshold
///
/// # Arguments
/// * `threshold_secs` - Number of seconds of inactivity before considering idle
#[tauri::command]
pub async fn set_idle_threshold(
    context: State<'_, Arc<AppContext>>,
    threshold_secs: i64,
) -> std::result::Result<(), String> {
    let start_time = Instant::now();
    let command_name = "idle::set_idle_threshold";
    info!(command = command_name, threshold_secs, "Setting idle threshold");

    // Validate threshold
    if threshold_secs < 0 {
        return Err("Idle threshold must be non-negative".to_string());
    }

    let app_ctx = Arc::clone(context.inner());
    let db = Arc::clone(&app_ctx.db);

    let result = task::spawn_blocking(move || -> Result<(), PulseArcError> {
        let conn = db.get_connection()?;

        // Ensure table exists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS idle_settings (
                id INTEGER PRIMARY KEY,
                pause_on_idle BOOLEAN NOT NULL DEFAULT 1,
                idle_threshold_secs INTEGER NOT NULL DEFAULT 600
            )",
            rusqlite::params![],
        )
        .map_err(|e| PulseArcError::Database(format!("Failed to create settings table: {}", e)))?;

        // Insert or update
        conn.execute(
            "INSERT INTO idle_settings (id, idle_threshold_secs) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET idle_threshold_secs = ?1",
            rusqlite::params![threshold_secs],
        )
        .map_err(|e| PulseArcError::Database(format!("Failed to update idle threshold: {}", e)))?;

        Ok(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    let elapsed = start_time.elapsed();
    let success = result.is_ok();

    log_command_execution(command_name, "new", elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation: "new",
            elapsed,
            success,
            error_type: if !success { Some("settings_error") } else { None },
        },
    )
    .await;

    result.map_err(|e| e.to_string())
}

// =============================================================================
// New Implementation (Phase 4B.3)
// =============================================================================

async fn get_idle_periods_new(
    context: &Arc<AppContext>,
    start_ts: i64,
    end_ts: i64,
) -> Result<Vec<IdlePeriod>, String> {
    context
        .idle_periods
        .get_idle_periods_in_range(start_ts, end_ts)
        .await
        .map_err(|e| format!("Failed to fetch idle periods: {e}"))
}

async fn update_idle_period_action_new(
    context: &Arc<AppContext>,
    period_id: &str,
    action: &str,
    notes: Option<String>,
) -> Result<(), String> {
    context
        .idle_periods
        .update_idle_period_action(period_id, action, notes)
        .await
        .map_err(|e| format!("Failed to update idle period: {e}"))
}

async fn get_idle_summary_new(
    context: &Arc<AppContext>,
    date: &str,
) -> Result<IdleSummary, String> {
    // Parse date to get start and end timestamps
    let date_parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format '{}': {}. Expected YYYY-MM-DD", date, e))?;

    // Convert to timestamps (start of day and end of day in UTC)
    let start_ts = date_parsed
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| format!("Failed to create start timestamp for date '{}'", date))?
        .and_utc()
        .timestamp();

    let end_ts = start_ts + 86_400;

    debug!(date, start_ts, end_ts, "parsed date to timestamp range");

    context
        .idle_periods
        .get_idle_summary(start_ts, end_ts)
        .await
        .map_err(|e| format!("Failed to compute idle summary: {e}"))
}

// =============================================================================
// Legacy Implementation (Temporary)
// =============================================================================

async fn get_idle_periods_legacy(
    _context: &AppContext,
    _start_ts: i64,
    _end_ts: i64,
) -> Result<Vec<IdlePeriod>, String> {
    // Legacy implementation would query DbManager directly
    // For now, return empty vec (commands will fail until wired)
    warn!("legacy get_idle_periods not implemented - feature flag disabled by default");
    Ok(Vec::new())
}

async fn update_idle_period_action_legacy(
    _context: &AppContext,
    _period_id: &str,
    _action: &str,
    _notes: Option<String>,
) -> Result<(), String> {
    // Legacy implementation would query DbManager directly
    // For now, return error (commands will fail until wired)
    warn!("legacy update_idle_period_action not implemented - feature flag disabled by default");
    Err("Legacy implementation not available".to_string())
}

async fn get_idle_summary_legacy(
    _context: &AppContext,
    _date: &str,
) -> Result<IdleSummary, String> {
    // Legacy implementation would query DbManager directly
    // For now, return error (commands will fail until wired)
    warn!("legacy get_idle_summary not implemented - feature flag disabled by default");
    Err("Legacy implementation not available".to_string())
}
