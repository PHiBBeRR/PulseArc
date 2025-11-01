//! Project management commands

use std::sync::Arc;
use std::time::Instant;

use pulsearc_domain::Result;
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::info;

use crate::utils::logging::{log_command_execution, record_command_metric, MetricRecord};
use crate::AppContext;

/// Minimal project info for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
}

/// Get list of user projects
#[tauri::command]
pub async fn get_user_projects(ctx: State<'_, Arc<AppContext>>) -> Result<Vec<Project>> {
    let command_name = "projects::get_user_projects";
    let implementation = "new";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, "Fetching user projects");
    // TODO: Implement project fetching from database
    // For now, return empty list to prevent frontend errors
    let result = Ok(vec![]);
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
