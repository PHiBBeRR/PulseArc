//! Feature flags commands for Phase 4 rollback control

use std::sync::Arc;
use std::time::Instant;

use tauri::State;
use tracing::info;

use crate::utils::logging::{log_command_execution, log_feature_flag_check, record_command_metric};
use crate::AppContext;

#[tauri::command]
pub async fn is_feature_enabled(
    ctx: State<'_, Arc<AppContext>>,
    flag: String,
    default: bool,
) -> Result<bool, String> {
    let command_name = "feature_flags::is_feature_enabled";
    let implementation = "new";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    let result = app_ctx
        .feature_flags
        .is_enabled(&flag, default)
        .await
        .map_err(|e| format!("Failed to check feature flag: {e}"));

    let elapsed = start.elapsed();
    let success = result.is_ok();

    log_command_execution(command_name, implementation, elapsed, success);

    if let Ok(is_enabled) = &result {
        log_feature_flag_check(&flag, *is_enabled, false);
    }

    let error_type = if success { None } else { Some("feature_flag_error") };

    record_command_metric(&app_ctx, command_name, implementation, elapsed, success, error_type)
        .await;

    result
}

#[tauri::command]
pub async fn toggle_feature_flag(
    ctx: State<'_, Arc<AppContext>>,
    flag: String,
    enabled: bool,
) -> Result<(), String> {
    let command_name = "feature_flags::toggle_feature_flag";
    let implementation = "new";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(
        command = command_name,
        flag = %flag,
        enabled,
        "Toggling feature flag"
    );

    let result = app_ctx
        .feature_flags
        .set_enabled(&flag, enabled)
        .await
        .map_err(|e| format!("Failed to toggle feature flag: {e}"));

    let elapsed = start.elapsed();
    let success = result.is_ok();
    let error_type = if success { None } else { Some("feature_flag_error") };

    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(&app_ctx, command_name, implementation, elapsed, success, error_type)
        .await;

    result
}

#[tauri::command]
pub async fn list_feature_flags(
    ctx: State<'_, Arc<AppContext>>,
) -> Result<Vec<serde_json::Value>, String> {
    let command_name = "feature_flags::list_feature_flags";
    let implementation = "new";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    let result = app_ctx
        .feature_flags
        .list_all()
        .await
        .map(|flags| {
            flags
                .into_iter()
                .map(|flag| {
                    serde_json::json!({
                        "flag_name": flag.flag_name,
                        "enabled": flag.enabled,
                        "description": flag.description,
                        "updated_at": flag.updated_at,
                    })
                })
                .collect()
        })
        .map_err(|e| format!("Failed to list feature flags: {e}"));

    let elapsed = start.elapsed();
    let success = result.is_ok();
    let error_type = if success { None } else { Some("feature_flag_error") };

    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(&app_ctx, command_name, implementation, elapsed, success, error_type)
        .await;

    result
}
