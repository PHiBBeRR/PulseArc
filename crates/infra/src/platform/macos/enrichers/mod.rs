//! Application enrichment modules for macOS.
//!
//! Provides specialized enrichment for different application types:
//! - **Browser**: URL extraction from active tabs
//! - **Office**: Document name extraction from productivity apps
//! - **Cache**: TTL-based caching to reduce AppleScript overhead
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │     Activity Provider                   │
//! └──────────┬──────────────────────────────┘
//!            │
//!            ├──► Browser Enricher ──► Cache
//!            │         │
//!            │         └──► AppleScript Helpers
//!            │
//!            └──► Office Enricher ──► Cache
//!                      │
//!                      └──► AppleScript Helpers
//! ```
//!
//! # Example
//! ```rust,no_run
//! use pulsearc_infra::platform::macos::enrichers::cache::EnrichmentCache;
//! use pulsearc_infra::platform::macos::enrichers::{browser, office};
//!
//! let cache = EnrichmentCache::default();
//!
//! // Check cache first
//! if let Some(url) = cache.get_browser_url("com.apple.Safari") {
//!     println!("Cached URL: {url}");
//! } else {
//!     // Fetch fresh data
//!     if let Some(url) = browser::get_browser_url_sync("com.apple.Safari", "Safari") {
//!         cache.set_browser_url("com.apple.Safari", &url);
//!         println!("Fresh URL: {url}");
//!     }
//! }
//! ```

pub mod applescript_helpers;
pub mod browser;
pub mod cache;
pub mod office;

// Re-export commonly used items
pub use cache::{EnrichmentCache, EnrichmentData, DEFAULT_ENRICHMENT_TTL};
