//! PulseArc - macOS Time Tracking Application
//!
//! Main entry point for the Tauri application.

use std::sync::Arc;

use pulsearc_lib::AppContext;
use tauri::window::{Effect, EffectState, EffectsBuilder};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
// Allow std::process::exit in tauri::generate_context! macro - it's part of Tauri's
// standard initialization and handles graceful shutdown internally
#[allow(clippy::disallowed_methods)]
pub fn run() {
    // Initialize logging FIRST so we can see .env loading
    env_logger::init();

    // Load environment variables from .env file
    match dotenvy::dotenv() {
        Ok(path) => log::info!("Loaded .env from: {:?}", path),
        Err(e) => log::warn!("Could not load .env file: {}", e),
    }

    // Verify encryption key is available
    match std::env::var("DATABASE_ENCRYPTION_KEY") {
        Ok(key) => log::info!("DATABASE_ENCRYPTION_KEY loaded ({} chars)", key.len()),
        Err(_) => log::warn!("DATABASE_ENCRYPTION_KEY not found in environment"),
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            log::info!("PulseArc starting...");

            // Initialize application context
            let ctx = tauri::async_runtime::block_on(AppContext::new())?;
            let ctx_arc = Arc::new(ctx);

            // Manage feature flags service separately for command access
            app.manage(ctx_arc.feature_flags.clone());
            app.manage(ctx_arc);

            // Set native macOS window blur effects
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_effects(
                    EffectsBuilder::new()
                        .effect(Effect::UnderWindowBackground) // macOS native blur
                        .state(EffectState::Active)
                        .radius(40.0) // Corner radius for main timer
                        .build(),
                );
                log::info!("Applied native window effects to main window");
            }

            log::info!("PulseArc initialized successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Activity tracking
            pulsearc_lib::get_activity,
            pulsearc_lib::pause_tracker,
            pulsearc_lib::resume_tracker,
            // Projects
            pulsearc_lib::get_user_projects,
            // Suggestions & proposed blocks
            pulsearc_lib::get_dismissed_suggestions,
            pulsearc_lib::get_proposed_blocks,
            pulsearc_lib::get_outbox_status,
            // Calendar integration
            pulsearc_lib::get_calendar_events_for_timeline,
            // Database commands (Phase 4A.1)
            pulsearc_lib::get_database_stats,
            pulsearc_lib::get_recent_snapshots,
            pulsearc_lib::vacuum_database,
            pulsearc_lib::get_database_health,
            // Feature flags (Phase 4)
            pulsearc_lib::is_feature_enabled,
            pulsearc_lib::toggle_feature_flag,
            pulsearc_lib::list_feature_flags,
            // Health check (Phase 4.1.6)
            pulsearc_lib::get_app_health,
            // User profile commands (Phase 4A.2)
            pulsearc_lib::get_user_profile,
            pulsearc_lib::upsert_user_profile,
            // Window commands (Phase 4A.3)
            pulsearc_lib::animate_window_resize,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}
