//! Application constants
//!
//! Centralized location for all domain-level constants used throughout the
//! application.

// Configuration constants
pub const DEFAULT_CACHE_DURATION_MS: u64 = 500;
pub const MAX_TITLE_LENGTH: usize = 50;
pub const MAX_PROJECT_NAME_LENGTH: usize = 30;
pub const MAX_STACKOVERFLOW_TOPIC_LENGTH: usize = 100;
pub const TITLE_TRUNCATE_SUFFIX: &str = "...";
pub const FORCE_INITIAL_FETCH_SECS: u64 = 10;

// Burst mode for TTFD measurement (event-driven only)
pub const REFRESH_VISIBLE_BURST_MS: u64 = 75;
pub const VISIBLE_BURST_DURATION_MS: u64 = 3000;

// Enrichment worker configuration
pub const ENRICHMENT_THROTTLE_MS: u64 = 750;
pub const APPLESCRIPT_TIMEOUT_MS: u64 = 200; // Used by enrichers (browser/office apps)
pub const ENRICHMENT_WORKER_QUEUE: usize = 1;

// Event emission configuration
pub const EVENT_ACTIVITY_UPDATED: &str = "activity-context-updated";
pub const MAX_EVENT_FAILURES: usize = 5;
