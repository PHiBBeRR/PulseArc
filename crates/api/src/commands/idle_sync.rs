//! Idle sync telemetry commands
//!
//! FEATURE-012: Tauri commands for recording idle detection, timer state
//! events, and autoStartTracker validation metrics. These commands provide
//! frontend visibility into idle synchronization behavior for debugging and
//! performance monitoring.
//!
//! Migration Status: Phase 4C.2
//! - No feature flag (telemetry is low-risk, migrated directly)
//! - Legacy: `legacy/api/src/commands/idle_sync.rs`

use std::sync::Arc;

use tauri::State;
use tracing::debug;

use crate::context::AppContext;

/// Record an idle detection event with latency measurement
///
/// # Arguments
/// * `context` - Application context with idle sync metrics
/// * `latency_ms` - Time in milliseconds to detect idle state
///
/// # Returns
/// Success (always succeeds, non-critical telemetry)
#[tauri::command]
pub fn record_idle_detection(
    context: State<'_, Arc<AppContext>>,
    latency_ms: u64,
) -> Result<(), String> {
    debug!(latency_ms, "recording idle detection");
    context.idle_sync_metrics.record_idle_detection(latency_ms);
    Ok(())
}

/// Record an activity wake event
///
/// # Arguments
/// * `context` - Application context with idle sync metrics
/// * `event_type` - Type of wake event (e.g., "mouse", "keyboard", "system")
///
/// # Returns
/// Success (always succeeds, non-critical telemetry)
#[tauri::command]
pub fn record_activity_wake(
    context: State<'_, Arc<AppContext>>,
    event_type: String,
) -> Result<(), String> {
    debug!(event_type, "recording activity wake");
    context.idle_sync_metrics.record_activity_wake(event_type);
    Ok(())
}

/// Record a timer-state event emission with latency and success status
///
/// # Arguments
/// * `context` - Application context with idle sync metrics
/// * `latency_us` - Emission latency in microseconds
/// * `success` - Whether the emission succeeded
///
/// # Returns
/// Success (always succeeds, non-critical telemetry)
#[tauri::command]
pub fn record_timer_event_emission(
    context: State<'_, Arc<AppContext>>,
    latency_us: u64,
    success: bool,
) -> Result<(), String> {
    debug!(latency_us, success, "recording timer event emission");
    context.idle_sync_metrics.record_timer_event_emission(latency_us, success);
    Ok(())
}

/// Record a timer-state event reception with sync latency
///
/// # Arguments
/// * `context` - Application context with idle sync metrics
/// * `sync_latency_ms` - Synchronization latency in milliseconds
///
/// # Returns
/// Success (always succeeds, non-critical telemetry)
#[tauri::command]
pub fn record_timer_event_reception(
    context: State<'_, Arc<AppContext>>,
    sync_latency_ms: u64,
) -> Result<(), String> {
    debug!(sync_latency_ms, "recording timer event reception");
    context.idle_sync_metrics.record_timer_event_reception(sync_latency_ms);
    Ok(())
}

/// Record an invalid payload rejection
///
/// # Arguments
/// * `context` - Application context with idle sync metrics
///
/// # Returns
/// Success (always succeeds, non-critical telemetry)
#[tauri::command]
pub fn record_invalid_payload(context: State<'_, Arc<AppContext>>) -> Result<(), String> {
    debug!("recording invalid payload");
    context.idle_sync_metrics.record_invalid_payload();
    Ok(())
}

/// Record a state transition with timing
///
/// # Arguments
/// * `context` - Application context with idle sync metrics
/// * `from` - Previous state
/// * `to` - New state
/// * `duration_ms` - Transition duration in milliseconds
///
/// # Returns
/// Success (always succeeds, non-critical telemetry)
#[tauri::command]
pub fn record_state_transition(
    context: State<'_, Arc<AppContext>>,
    from: String,
    to: String,
    duration_ms: u64,
) -> Result<(), String> {
    debug!(from, to, duration_ms, "recording state transition");
    context.idle_sync_metrics.record_state_transition(&from, &to, duration_ms);
    Ok(())
}

/// Record an autoStartTracker rule application with validation
///
/// # Arguments
/// * `context` - Application context with idle sync metrics
/// * `rule_num` - Rule number being validated (1-based)
/// * `timer_state` - Current timer state
/// * `auto_start` - Whether auto-start was triggered
/// * `is_correct` - Whether the rule application was correct
///
/// # Returns
/// Success (always succeeds, non-critical telemetry)
#[tauri::command]
pub fn record_auto_start_tracker_rule(
    context: State<'_, Arc<AppContext>>,
    rule_num: u8,
    timer_state: String,
    auto_start: bool,
    is_correct: bool,
) -> Result<(), String> {
    debug!(rule_num, timer_state, auto_start, is_correct, "recording autoStartTracker rule");
    context.idle_sync_metrics.record_auto_start_tracker_rule(
        rule_num,
        &timer_state,
        auto_start,
        is_correct,
    );
    Ok(())
}
