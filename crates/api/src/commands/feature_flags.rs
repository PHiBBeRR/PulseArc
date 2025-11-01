//! Feature flags commands for Phase 4 rollback control

use pulsearc_infra::FeatureFlagService;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn is_feature_enabled(
    flag: String,
    default: bool,
    service: State<'_, Arc<FeatureFlagService>>,
) -> Result<bool, String> {
    service
        .is_enabled(&flag, default)
        .await
        .map_err(|e| format!("Failed to check feature flag: {e}"))
}

#[tauri::command]
pub async fn toggle_feature_flag(
    flag: String,
    enabled: bool,
    service: State<'_, Arc<FeatureFlagService>>,
) -> Result<(), String> {
    service
        .set_enabled(&flag, enabled)
        .await
        .map_err(|e| format!("Failed to toggle feature flag: {e}"))
}

#[tauri::command]
pub async fn list_feature_flags(
    service: State<'_, Arc<FeatureFlagService>>,
) -> Result<Vec<serde_json::Value>, String> {
    service
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
        .map_err(|e| format!("Failed to list feature flags: {e}"))
}
