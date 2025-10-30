//! macOS-specific platform implementations

use async_trait::async_trait;
use pulsearc_core::ActivityProvider;
use pulsearc_shared::{ActivityContext, Result};
use chrono::Utc;

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
            timestamp: Utc::now(),
            app_name: "Unknown".to_string(),
            window_title: "Unknown".to_string(),
            url: None,
            document_path: None,
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
