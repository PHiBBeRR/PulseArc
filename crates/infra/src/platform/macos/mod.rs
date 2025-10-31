//! macOS Platform Integration
//!
//! This module provides macOS-specific platform adapters for activity tracking,
//! enrichment, and event monitoring using Accessibility APIs and NSWorkspace.
//!
//! # Modules
//!
//! - [`activity_provider`] - Implementation of `ActivityProvider` trait
//! - [`ax_helpers`] - Low-level Accessibility API bindings
//! - [`error_helpers`] - Error mapping utilities
//! - [`enrichers`] - Activity enrichment modules (browser, office apps) - Day 2
//!
//! # Platform Support
//!
//! This module is only available on macOS (target_os = "macos").
//! Non-macOS platforms should use the fallback provider.
//!
//! # Permission Requirements
//!
//! - **NSWorkspace**: No special permissions (app name, bundle ID, PID)
//! - **Accessibility API**: Requires user approval in System Settings for window titles
//!
//! The provider gracefully degrades to "app-only mode" when Accessibility
//! permissions are not granted.
//!
//! # Examples
//!
//! ```rust,ignore
//! use pulsearc_infra::platform::MacOsActivityProvider;
//! use pulsearc_core::tracking::ports::ActivityProvider;
//!
//! let provider = MacOsActivityProvider::new();
//! let activity = provider.get_activity().await?;
//! println!("Active app: {}", activity.active_app.app_name);
//! ```

pub mod activity_provider;
pub mod ax_helpers;
pub mod enrichers;
pub mod error_helpers;
pub mod event_listener;

// Re-export main types
pub use activity_provider::MacOsActivityProvider;
pub use event_listener::{MacOsEventListener, OsEventListener};
