//! Project management commands

use pulsearc_shared::Result;
use serde::{Deserialize, Serialize};

/// Minimal project info for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
}

/// Get list of user projects
#[tauri::command]
pub async fn get_user_projects() -> Result<Vec<Project>> {
    log::info!("get_user_projects command called");
    // TODO: Implement project fetching from database
    // For now, return empty list to prevent frontend errors
    Ok(vec![])
}
