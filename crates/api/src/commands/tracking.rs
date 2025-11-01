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
        MetricRecord {
            command: command_name,
            implementation,
            elapsed,
            success,
            error_type,
        },
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
    // TODO: Implement pause functionality
    let result = Ok(());
    let elapsed = start.elapsed();

    log_command_execution(command_name, implementation, elapsed, true);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation,
            elapsed,
            success: true,
            error_type: None,
        },
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
    // TODO: Implement resume functionality
    let result = Ok(());
    let elapsed = start.elapsed();

    log_command_execution(command_name, implementation, elapsed, true);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation,
            elapsed,
            success: true,
            error_type: None,
        },
    )
    .await;

    result
}
