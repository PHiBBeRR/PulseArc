//! Platform-specific implementations
//!
//! This module provides platform-specific adapters for activity tracking.
//!
//! # Platform Support
//!
//! - **macOS**: Full support via Accessibility APIs and NSWorkspace
//! - **Other platforms**: Fallback stub (returns platform error)

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::MacOsActivityProvider;

// Fallback stub for non-macOS platforms (Day 6)
#[cfg(not(target_os = "macos"))]
pub mod fallback {
    use async_trait::async_trait;
    use pulsearc_core::tracking::ports::ActivityProvider;
    use pulsearc_domain::{ActivityContext, PulseArcError, Result as DomainResult};

    /// Fallback activity provider for unsupported platforms.
    ///
    /// This stub implementation returns a platform error on all operations.
    pub struct FallbackActivityProvider;

    impl FallbackActivityProvider {
        #[allow(clippy::new_without_default)]
        pub fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl ActivityProvider for FallbackActivityProvider {
        async fn get_activity(&self) -> DomainResult<ActivityContext> {
            Err(PulseArcError::Platform("Activity tracking is only supported on macOS".to_string()))
        }

        fn is_paused(&self) -> bool {
            false
        }

        fn pause(&mut self) -> DomainResult<()> {
            Err(PulseArcError::Platform("Activity tracking is only supported on macOS".to_string()))
        }

        fn resume(&mut self) -> DomainResult<()> {
            Err(PulseArcError::Platform("Activity tracking is only supported on macOS".to_string()))
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub use fallback::FallbackActivityProvider;
