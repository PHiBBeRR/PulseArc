//! Activity tracking commands

use std::sync::Arc;
use std::time::Instant;

use pulsearc_domain::{ActivityContext, Result};
use tauri::State;
use tracing::info;

use crate::utils::logging::{
    error_label, log_command_execution, record_command_metric, MetricRecord,
};
use crate::AppContext;

/// Get the current activity context
#[tauri::command]
pub async fn get_activity(ctx: State<'_, Arc<AppContext>>) -> Result<ActivityContext> {
    let command_name = "tracking::get_activity";
    let implementation = "new";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, "Capturing current activity");

    let result = app_ctx.tracking_service.capture_activity().await;
    let elapsed = start.elapsed();
    let success = result.is_ok();
    let error_type = result.as_ref().err().map(error_label);

    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord { command: command_name, implementation, elapsed, success, error_type },
    )
    .await;

    result
}

/// Pause activity tracking
#[tauri::command]
pub async fn pause_tracker(ctx: State<'_, Arc<AppContext>>) -> Result<()> {
    let command_name = "tracking::pause_tracker";
    let implementation = "new";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, "Pausing activity tracker");
    let result = app_ctx.tracking_service.pause().await;
    let elapsed = start.elapsed();
    let success = result.is_ok();
    let error_type = result.as_ref().err().map(error_label);

    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord { command: command_name, implementation, elapsed, success, error_type },
    )
    .await;

    result
}

/// Resume activity tracking
#[tauri::command]
pub async fn resume_tracker(ctx: State<'_, Arc<AppContext>>) -> Result<()> {
    let command_name = "tracking::resume_tracker";
    let implementation = "new";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, "Resuming activity tracker");
    let result = app_ctx.tracking_service.resume().await;
    let elapsed = start.elapsed();
    let success = result.is_ok();
    let error_type = result.as_ref().err().map(error_label);

    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord { command: command_name, implementation, elapsed, success, error_type },
    )
    .await;

    result
}

/// Save a manual time entry
///
/// Replaces legacy `save_manual_activity` command. Creates a manual activity
/// snapshot with the provided description.
///
/// # Arguments
/// * `description` - Text description of the manual activity
///
/// # Returns
/// ID of the created activity snapshot
#[tauri::command]
pub async fn save_time_entry(
    ctx: State<'_, Arc<AppContext>>,
    description: String,
) -> std::result::Result<String, String> {
    let command_name = "tracking::save_time_entry";
    let implementation = "new";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, description, "Saving manual time entry");

    let result = app_ctx.tracking_service.save_manual_entry(&description).await;
    let elapsed = start.elapsed();
    let success = result.is_ok();
    let error_type = result.as_ref().err().map(error_label);

    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord { command: command_name, implementation, elapsed, success, error_type },
    )
    .await;

    result.map_err(|e| e.to_string())
}
