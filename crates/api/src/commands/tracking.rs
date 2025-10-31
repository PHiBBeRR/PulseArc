//! Activity tracking commands

use std::sync::Arc;

use pulsearc_domain::{ActivityContext, Result};
use tauri::State;

use crate::AppContext;

/// Get the current activity context
#[tauri::command]
pub async fn get_activity(ctx: State<'_, Arc<AppContext>>) -> Result<ActivityContext> {
    log::info!("get_activity command called");
    ctx.tracking_service.capture_activity().await.map(|s| s.context)
}

/// Pause activity tracking
#[tauri::command]
pub async fn pause_tracker(_ctx: State<'_, Arc<AppContext>>) -> Result<()> {
    log::info!("pause_tracker command called");
    // TODO: Implement pause functionality
    Ok(())
}

/// Resume activity tracking
#[tauri::command]
pub async fn resume_tracker(_ctx: State<'_, Arc<AppContext>>) -> Result<()> {
    log::info!("resume_tracker command called");
    // TODO: Implement resume functionality
    Ok(())
}
