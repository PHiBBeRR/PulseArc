//! macOS-specific platform implementations

use async_trait::async_trait;
use pulsearc_core::ActivityProvider;
use pulsearc_domain::{
    types::{ActivityCategory, ActivityMetadata, ConfidenceEvidence, WindowContext},
    ActivityContext, Result,
};

/// macOS activity provider using Accessibility API
pub struct MacOsActivityProvider {
    paused: bool,
}

impl Default for MacOsActivityProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MacOsActivityProvider {
    pub fn new() -> Self {
        Self { paused: false }
    }
}

#[async_trait]
impl ActivityProvider for MacOsActivityProvider {
    async fn get_activity(&self) -> Result<ActivityContext> {
        // Placeholder implementation
        // TODO: Implement actual macOS Accessibility API calls
        Ok(ActivityContext {
            active_app: WindowContext {
                app_name: "Unknown".to_string(),
                window_title: "Unknown".to_string(),
                bundle_id: None,
                url: None,
                url_host: None,
                document_name: None,
                file_path: None,
            },
            recent_apps: vec![],
            detected_activity: "Unknown".to_string(),
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
        })
    }

    fn is_paused(&self) -> bool {
        self.paused
    }

    fn pause(&mut self) -> Result<()> {
        self.paused = true;
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        self.paused = false;
        Ok(())
    }
}
