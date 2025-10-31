//! macOS Activity Provider Implementation
//!
//! This module provides the `MacOsActivityProvider` which implements the
//! `ActivityProvider` trait using macOS Accessibility APIs.
//!
//! # Features
//!
//! - Captures active app information (name, bundle ID, window title)
//! - Fetches recent running apps
//! - Graceful degradation when Accessibility permission is denied
//! - Async-safe wrapping of synchronous macOS APIs via `spawn_blocking`
//! - Pause/resume functionality for activity tracking
//!
//! # Permission Handling
//!
//! The provider gracefully degrades to "app-only mode" when Accessibility
//! permissions are not granted:
//! - App name and bundle ID are always available (via NSWorkspace)
//! - Window titles require Accessibility permission
//! - No panics or errors on permission denial

use async_trait::async_trait;
use pulsearc_core::tracking::ports::ActivityProvider;
use pulsearc_domain::types::{
    ActivityCategory, ActivityMetadata, ConfidenceEvidence, WindowContext,
};
use pulsearc_domain::{ActivityContext, Result as DomainResult};

use super::ax_helpers;
use super::error_helpers::map_join_error;

/// macOS activity provider using Accessibility API.
///
/// Captures active app information using NSWorkspace and Accessibility APIs.
/// Gracefully degrades when permissions are not granted.
///
/// # Examples
///
/// ```rust,ignore
/// let provider = MacOsActivityProvider::new();
/// let activity = provider.get_activity().await?;
/// println!("Active app: {}", activity.active_app.app_name);
/// ```
pub struct MacOsActivityProvider {
    /// Whether activity tracking is currently paused
    paused: bool,
    /// Maximum number of recent apps to fetch
    recent_apps_limit: usize,
}

impl Default for MacOsActivityProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MacOsActivityProvider {
    /// Create a new macOS activity provider with default settings.
    ///
    /// # Default Configuration
    ///
    /// - `paused`: false (tracking enabled)
    /// - `recent_apps_limit`: 10 apps
    pub fn new() -> Self {
        Self { paused: false, recent_apps_limit: 10 }
    }

    /// Create a new macOS activity provider with custom recent apps limit.
    ///
    /// # Arguments
    ///
    /// * `recent_apps_limit` - Maximum number of recent apps to fetch
    pub fn with_recent_apps_limit(recent_apps_limit: usize) -> Self {
        Self { paused: false, recent_apps_limit }
    }

    /// Fetch active app information (synchronous, called from spawn_blocking).
    ///
    /// This is a helper method that performs synchronous AX API calls.
    /// It's designed to be called from within `tokio::task::spawn_blocking`.
    fn fetch_active_app_sync() -> DomainResult<WindowContext> {
        let app_info = ax_helpers::get_active_app_info()?;

        Ok(WindowContext {
            app_name: app_info.app_name,
            window_title: app_info.window_title.unwrap_or_else(|| String::from("Unknown")),
            bundle_id: Some(app_info.bundle_id),
            url: None,           // Enrichment (Day 2)
            url_host: None,      // Enrichment (Day 2)
            document_name: None, // Enrichment (Day 2)
            file_path: None,     // Enrichment (Day 2)
        })
    }

    /// Fetch recent running apps (synchronous, called from spawn_blocking).
    ///
    /// # Arguments
    ///
    /// * `exclude_bundle_id` - Optional bundle ID to exclude from results
    /// * `limit` - Maximum number of apps to return
    fn fetch_recent_apps_sync(
        exclude_bundle_id: Option<String>,
        limit: usize,
    ) -> DomainResult<Vec<WindowContext>> {
        let apps = ax_helpers::get_recent_apps(exclude_bundle_id.as_deref(), limit)?;

        Ok(apps
            .into_iter()
            .map(|recent| WindowContext {
                app_name: recent.app_name,
                window_title: recent.window_title.unwrap_or_else(|| String::from("Unknown")),
                bundle_id: Some(recent.bundle_id),
                url: None,
                url_host: None,
                document_name: None,
                file_path: None,
            })
            .collect())
    }
}

#[async_trait]
impl ActivityProvider for MacOsActivityProvider {
    /// Get the current activity context.
    ///
    /// Fetches active app info and recent apps using macOS Accessibility APIs.
    /// All blocking AX API calls are wrapped in `spawn_blocking` for async safety.
    ///
    /// # Returns
    ///
    /// * `Ok(ActivityContext)` - Current activity context
    /// * `Err(PulseArcError)` - If activity fetch fails
    ///
    /// # Behavior
    ///
    /// - Returns immediately if tracking is paused (placeholder context)
    /// - Uses `spawn_blocking` for all synchronous AX API calls
    /// - Gracefully degrades on permission denial (app-only mode)
    /// - Window titles are "Unknown" when AX permission is denied
    /// - Recent apps list may be empty on errors (non-fatal)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let provider = MacOsActivityProvider::new();
    /// let activity = provider.get_activity().await?;
    ///
    /// tracing::info!(
    ///     app = %activity.active_app.app_name,
    ///     bundle_id = ?activity.active_app.bundle_id,
    ///     recent_count = activity.recent_apps.len(),
    ///     "Captured activity"
    /// );
    /// ```
    async fn get_activity(&self) -> DomainResult<ActivityContext> {
        if self.paused {
            tracing::debug!(
                paused = true,
                status = "placeholder_return",
                "Activity tracking paused; returning placeholder context"
            );
            return Ok(ActivityContext {
                active_app: WindowContext {
                    app_name: "Paused".to_string(),
                    window_title: "Tracking Paused".to_string(),
                    bundle_id: None,
                    url: None,
                    url_host: None,
                    document_name: None,
                    file_path: None,
                },
                recent_apps: vec![],
                detected_activity: "Paused".to_string(),
                work_type: None,
                activity_category: ActivityCategory::Administrative,
                billable_confidence: 0.0,
                suggested_client: None,
                suggested_matter: None,
                suggested_task_code: None,
                extracted_metadata: ActivityMetadata::default(),
                evidence: ConfidenceEvidence::default(),
                calendar_event: None,
                location: None,
                temporal_context: None,
                classification: None,
            });
        }

        // Spawn blocking for active app (synchronous AX APIs)
        let active_app = tokio::task::spawn_blocking(Self::fetch_active_app_sync)
            .await
            .map_err(map_join_error)??; // Flatten Result<Result<T>>

        // Spawn blocking for recent apps (non-fatal if fails)
        let exclude_bundle_id = active_app.bundle_id.clone();
        let limit = self.recent_apps_limit;

        let recent_apps = tokio::task::spawn_blocking(move || {
            Self::fetch_recent_apps_sync(exclude_bundle_id, limit)
        })
        .await
        .map_err(map_join_error)?
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to fetch recent apps - continuing with empty list");
            vec![]
        });

        tracing::debug!(
            app = %active_app.app_name,
            bundle_id = ?active_app.bundle_id,
            has_window_title = !active_app.window_title.is_empty() && active_app.window_title != "Unknown",
            recent_count = recent_apps.len(),
            "Fetched activity context"
        );

        Ok(ActivityContext {
            active_app,
            recent_apps,
            detected_activity: "Captured".to_string(), // Classification (Phase 4)
            work_type: None,                           // Classification (Phase 4)
            activity_category: ActivityCategory::Administrative, // Default
            billable_confidence: 0.0,                  // Classification (Phase 4)
            suggested_client: None,                    // Classification (Phase 4)
            suggested_matter: None,                    // Classification (Phase 4)
            suggested_task_code: None,                 // Classification (Phase 4)
            extracted_metadata: ActivityMetadata::default(), // Enrichment (Day 2)
            evidence: ConfidenceEvidence::default(),   // Classification (Phase 4)
            calendar_event: None,                      // Integration (future)
            location: None,                            // Integration (future)
            temporal_context: None,                    // Integration (future)
            classification: None,                      // Classification (Phase 4)
        })
    }

    /// Check if activity tracking is paused.
    ///
    /// # Returns
    ///
    /// * `true` - Tracking is paused
    /// * `false` - Tracking is active
    fn is_paused(&self) -> bool {
        self.paused
    }

    /// Pause activity tracking.
    ///
    /// When paused, `get_activity()` returns a placeholder context without
    /// querying macOS APIs.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut provider = MacOsActivityProvider::new();
    /// provider.pause()?;
    /// assert!(provider.is_paused());
    /// ```
    fn pause(&mut self) -> DomainResult<()> {
        if !self.paused {
            tracing::info!(paused = true, op = "pause_tracking", "Pausing activity tracking");
            self.paused = true;
        }
        Ok(())
    }

    /// Resume activity tracking.
    ///
    /// Re-enables activity tracking after being paused.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut provider = MacOsActivityProvider::new();
    /// provider.pause()?;
    /// provider.resume()?;
    /// assert!(!provider.is_paused());
    /// ```
    fn resume(&mut self) -> DomainResult<()> {
        if self.paused {
            tracing::info!(paused = false, op = "resume_tracking", "Resuming activity tracking");
            self.paused = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_provider_not_paused() {
        let provider = MacOsActivityProvider::new();
        assert!(!provider.is_paused());
    }

    #[test]
    fn test_pause_resume() {
        let mut provider = MacOsActivityProvider::new();

        // Initially not paused
        assert!(!provider.is_paused());

        // Pause
        provider.pause().unwrap();
        assert!(provider.is_paused());

        // Resume
        provider.resume().unwrap();
        assert!(!provider.is_paused());
    }

    #[test]
    fn test_with_recent_apps_limit() {
        let provider = MacOsActivityProvider::with_recent_apps_limit(5);
        assert_eq!(provider.recent_apps_limit, 5);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_activity_when_paused() {
        let mut provider = MacOsActivityProvider::new();
        provider.pause().unwrap();

        let result = provider.get_activity().await;
        assert!(result.is_ok());

        let activity = result.unwrap();
        assert_eq!(activity.active_app.app_name, "Paused");
        assert_eq!(activity.recent_apps.len(), 0);
    }

    // Platform-specific test (requires macOS)
    #[cfg(target_os = "macos")]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_activity_basic() {
        let provider = MacOsActivityProvider::new();
        let result = provider.get_activity().await;

        // Should succeed even without AX permission (app-only mode)
        assert!(result.is_ok());

        let activity = result.unwrap();
        // Should have an active app (at least the test runner)
        assert!(!activity.active_app.app_name.is_empty());
        assert!(activity.active_app.bundle_id.is_some());
    }
}
