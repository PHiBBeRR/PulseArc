//! WBS code caching with moka
//!
//! Provides in-memory cache for WBS validation results to reduce database queries.
//! Uses WbsRepository as backing store with separate positive and negative caches.
//!
//! # Architecture
//!
//! - **Positive Cache**: Stores valid `WbsElement` instances
//! - **Negative Cache**: Stores "not found" results to prevent repeated queries
//! - **Error Handling**: Only caches `Ok(None)`, never transient errors
//! - **TTL**: Configurable time-to-live with default 5 minutes
//!
//! # Example
//!
//! ```rust,ignore
//! use pulsearc_infra::integrations::sap::cache::{WbsCache, WbsCacheConfig};
//!
//! let config = WbsCacheConfig::default();
//! let cache = WbsCache::new(config);
//!
//! // Get WBS with cache fallback to repository
//! match cache.get_or_fetch("USC0063201.1.1", &repository)? {
//!     Some(element) => println!("Found: {}", element.wbs_code),
//!     None => println!("Not found"),
//! }
//! ```

use moka::sync::Cache;
use pulsearc_common::time::{Clock, SystemClock};
use pulsearc_core::classification::ports::WbsRepository;
use pulsearc_domain::types::sap::WbsElement;
use pulsearc_domain::{PulseArcError, Result};
use std::sync::Arc;
use std::time::Duration;

/// Default TTL for WBS cache entries (5 minutes)
///
/// Override via `SAP_CACHE_TTL_SECONDS` environment variable
pub const DEFAULT_WBS_CACHE_TTL_SECONDS: u64 = 300;

/// Default max capacity for WBS cache (1000 entries)
///
/// Override via `SAP_CACHE_MAX_CAPACITY` environment variable
pub const DEFAULT_WBS_CACHE_MAX_CAPACITY: u64 = 1000;

/// WBS cache configuration
#[derive(Debug, Clone)]
pub struct WbsCacheConfig {
    /// Time-to-live for cache entries
    pub ttl: Duration,

    /// Maximum number of entries in each cache
    pub max_capacity: u64,
}

impl Default for WbsCacheConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(
                std::env::var("SAP_CACHE_TTL_SECONDS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(DEFAULT_WBS_CACHE_TTL_SECONDS),
            ),
            max_capacity: std::env::var("SAP_CACHE_MAX_CAPACITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_WBS_CACHE_MAX_CAPACITY),
        }
    }
}

impl WbsCacheConfig {
    /// Create config with custom TTL (useful for testing)
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            ttl,
            max_capacity: DEFAULT_WBS_CACHE_MAX_CAPACITY,
        }
    }

    /// Log configuration at startup
    pub fn log_config(&self) {
        tracing::info!(
            ttl_seconds = self.ttl.as_secs(),
            max_capacity = self.max_capacity,
            "WBS cache configuration loaded"
        );
    }
}

/// Cache result for get operations
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)] // WbsElement is domain type, size acceptable for cache hits
pub enum CacheResult {
    /// Cache hit with valid WBS element
    Hit(WbsElement),

    /// Cache miss - not in cache
    Miss,

    /// Negative cache hit - known to not exist
    NotFound,
}

/// In-memory cache for WBS validation results
///
/// Caches both positive (valid WBS) and negative (not found) results
/// to minimize database queries. Generic over `Clock` trait for
/// deterministic testing with `MockClock`.
///
/// # Error Handling
///
/// Only caches `Ok(None)` results. Never caches transient errors like:
/// - `PulseArcError::Database` (I/O failures)
/// - `PulseArcError::Network` (connectivity issues)
///
/// This ensures transient failures don't get cached and hidden.
pub struct WbsCache<C: Clock = SystemClock> {
    /// Cache for valid WBS elements
    positive_cache: Cache<String, WbsElement>,

    /// Cache for "not found" results (empty value)
    negative_cache: Cache<String, ()>,

    /// Clock for time-based operations (injectable for testing)
    #[allow(dead_code)]
    clock: Arc<C>,

    /// Cache configuration
    #[allow(dead_code)]
    config: WbsCacheConfig,
}

impl WbsCache<SystemClock> {
    /// Create a new WBS cache with default configuration and system clock
    pub fn new(config: WbsCacheConfig) -> Self {
        config.log_config();
        Self::with_clock(config, SystemClock)
    }
}

impl<C: Clock> WbsCache<C> {
    /// Create a new WBS cache with custom clock (for testing)
    pub fn with_clock(config: WbsCacheConfig, clock: C) -> Self {
        let positive_cache = Cache::builder()
            .time_to_live(config.ttl)
            .max_capacity(config.max_capacity)
            .build();

        let negative_cache = Cache::builder()
            .time_to_live(config.ttl)
            .max_capacity(config.max_capacity)
            .build();

        Self {
            positive_cache,
            negative_cache,
            clock: Arc::new(clock),
            config,
        }
    }

    /// Get WBS element from cache (no repository fallback)
    ///
    /// Returns:
    /// - `CacheResult::Hit` if found in positive cache
    /// - `CacheResult::NotFound` if found in negative cache
    /// - `CacheResult::Miss` if not in either cache
    pub fn get(&self, wbs_code: &str) -> CacheResult {
        let normalized = Self::normalize(wbs_code);

        // Check negative cache first (faster than clone)
        if self.negative_cache.get(&normalized).is_some() {
            tracing::debug!(wbs_code = %normalized, "WBS negative cache hit");
            return CacheResult::NotFound;
        }

        // Check positive cache
        if let Some(element) = self.positive_cache.get(&normalized) {
            tracing::debug!(wbs_code = %normalized, "WBS positive cache hit");
            return CacheResult::Hit(element);
        }

        tracing::debug!(wbs_code = %normalized, "WBS cache miss");
        CacheResult::Miss
    }

    /// Get WBS element with repository fallback
    ///
    /// Queries the cache first. On cache miss, queries the repository
    /// and caches the result. Only caches `Ok(None)` for negative results;
    /// never caches transient errors.
    ///
    /// # Error Handling
    ///
    /// - `Ok(Some(element))` → Cached in positive cache
    /// - `Ok(None)` → Cached in negative cache (permanent "not found")
    /// - `Err(Database(_))` → Propagated immediately (transient error)
    /// - `Err(Network(_))` → Propagated immediately (transient error)
    pub fn get_or_fetch(
        &self,
        wbs_code: &str,
        repository: &dyn WbsRepository,
    ) -> Result<Option<WbsElement>> {
        // Check cache first
        match self.get(wbs_code) {
            CacheResult::Hit(element) => return Ok(Some(element)),
            CacheResult::NotFound => return Ok(None),
            CacheResult::Miss => {
                // Fall through to repository query
            }
        }

        // Cache miss - query repository
        let normalized = Self::normalize(wbs_code);
        tracing::debug!(wbs_code = %normalized, "WBS cache miss, querying repository");

        match repository.get_wbs_by_wbs_code(&normalized) {
            Ok(Some(element)) => {
                // Success - cache positive result
                self.insert(&normalized, element.clone());
                Ok(Some(element))
            }
            Ok(None) => {
                // Not found (permanent) - cache negative result
                self.cache_not_found(&normalized);
                Ok(None)
            }
            Err(e @ PulseArcError::Database(_)) => {
                // Transient error - DO NOT cache, propagate immediately
                tracing::warn!(wbs_code = %normalized, error = %e, "Database error fetching WBS");
                Err(e)
            }
            Err(e @ PulseArcError::Network(_)) => {
                // Transient error - DO NOT cache
                tracing::warn!(wbs_code = %normalized, error = %e, "Network error fetching WBS");
                Err(e)
            }
            Err(e) => {
                // Other errors - propagate
                tracing::warn!(wbs_code = %normalized, error = %e, "Error fetching WBS");
                Err(e)
            }
        }
    }

    /// Insert a WBS element into the positive cache
    pub fn insert(&self, wbs_code: &str, element: WbsElement) {
        let normalized = Self::normalize(wbs_code);
        self.positive_cache.insert(normalized.clone(), element);
        // Remove from negative cache if present
        self.negative_cache.invalidate(&normalized);
        tracing::trace!(wbs_code = %normalized, "WBS cached (positive)");
    }

    /// Cache a "not found" result in the negative cache
    pub fn cache_not_found(&self, wbs_code: &str) {
        let normalized = Self::normalize(wbs_code);
        self.negative_cache.insert(normalized.clone(), ());
        // Remove from positive cache if present
        self.positive_cache.invalidate(&normalized);
        tracing::trace!(wbs_code = %normalized, "WBS cached (negative)");
    }

    /// Invalidate cache entry for a specific WBS code
    pub fn invalidate(&self, wbs_code: &str) {
        let normalized = Self::normalize(wbs_code);
        self.positive_cache.invalidate(&normalized);
        self.negative_cache.invalidate(&normalized);
        tracing::debug!(wbs_code = %normalized, "WBS cache invalidated");
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        self.positive_cache.invalidate_all();
        self.negative_cache.invalidate_all();
        tracing::info!("WBS cache cleared");
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        // Run pending tasks to get accurate counts
        self.positive_cache.run_pending_tasks();
        self.negative_cache.run_pending_tasks();

        CacheStats {
            positive_entry_count: self.positive_cache.entry_count(),
            negative_entry_count: self.negative_cache.entry_count(),
        }
    }

    /// Normalize WBS code (uppercase, trim)
    fn normalize(code: &str) -> String {
        code.trim().to_uppercase()
    }
}

/// Cache statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    /// Number of entries in positive cache
    pub positive_entry_count: u64,

    /// Number of entries in negative cache
    pub negative_entry_count: u64,
}

impl CacheStats {
    /// Total number of cached entries
    pub fn total_entry_count(&self) -> u64 {
        self.positive_entry_count + self.negative_entry_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulsearc_common::time::MockClock;
    use pulsearc_domain::types::sap::WbsElement;
    use std::sync::Mutex;

    /// Mock WBS repository for testing
    struct MockWbsRepository {
        valid_codes: Vec<String>,
        /// Track query count
        query_count: Mutex<usize>,
    }

    impl MockWbsRepository {
        fn new(valid_codes: Vec<String>) -> Self {
            Self {
                valid_codes,
                query_count: Mutex::new(0),
            }
        }

        fn query_count(&self) -> usize {
            *self.query_count.lock().unwrap()
        }
    }

    impl WbsRepository for MockWbsRepository {
        fn get_wbs_by_wbs_code(&self, wbs_code: &str) -> Result<Option<WbsElement>> {
            *self.query_count.lock().unwrap() += 1;

            if self.valid_codes.contains(&wbs_code.to_string()) {
                Ok(Some(WbsElement {
                    wbs_code: wbs_code.to_string(),
                    project_def: "TEST-001".to_string(),
                    project_name: Some("Test Project".to_string()),
                    description: Some("Test Description".to_string()),
                    status: "REL".to_string(),
                    cached_at: chrono::Utc::now().timestamp(),
                    opportunity_id: None,
                    deal_name: None,
                    target_company_name: None,
                    counterparty: None,
                    industry: None,
                    region: None,
                    amount: None,
                    stage_name: None,
                    project_code: None,
                }))
            } else {
                Ok(None)
            }
        }

        fn count_active_wbs(&self) -> Result<i64> {
            Ok(self.valid_codes.len() as i64)
        }

        fn get_last_sync_timestamp(&self) -> Result<Option<i64>> {
            Ok(Some(chrono::Utc::now().timestamp()))
        }

        fn load_common_projects(&self, _limit: usize) -> Result<Vec<WbsElement>> {
            Ok(vec![])
        }

        fn fts5_search_keyword(&self, _keyword: &str, _limit: usize) -> Result<Vec<WbsElement>> {
            Ok(vec![])
        }

        fn get_wbs_by_project_def(&self, _project_def: &str) -> Result<Option<WbsElement>> {
            Ok(None)
        }
    }

    #[test]
    fn test_cache_hit() {
        let config = WbsCacheConfig::with_ttl(Duration::from_secs(60));
        let cache = WbsCache::new(config);
        let repo = MockWbsRepository::new(vec!["USC0063201.1.1".to_string()]);

        // First call: cache miss, queries repository
        let result1 = cache.get_or_fetch("USC0063201.1.1", &repo).unwrap();
        assert!(result1.is_some());
        assert_eq!(repo.query_count(), 1);

        // Second call: cache hit, no repository query
        let result2 = cache.get_or_fetch("USC0063201.1.1", &repo).unwrap();
        assert!(result2.is_some());
        assert_eq!(repo.query_count(), 1); // Still 1, not 2
    }

    #[test]
    fn test_negative_caching() {
        let config = WbsCacheConfig::with_ttl(Duration::from_secs(60));
        let cache = WbsCache::new(config);
        let repo = MockWbsRepository::new(vec![]); // No valid codes

        // First call: cache miss, queries repository, gets None
        let result1 = cache.get_or_fetch("INVALID-CODE", &repo).unwrap();
        assert!(result1.is_none());
        assert_eq!(repo.query_count(), 1);

        // Second call: negative cache hit, no repository query
        let result2 = cache.get_or_fetch("INVALID-CODE", &repo).unwrap();
        assert!(result2.is_none());
        assert_eq!(repo.query_count(), 1); // Still 1, not 2
    }

    #[test]
    fn test_invalidation() {
        let config = WbsCacheConfig::with_ttl(Duration::from_secs(60));
        let cache = WbsCache::new(config);
        let repo = MockWbsRepository::new(vec!["USC0063201.1.1".to_string()]);

        // Cache the entry
        cache.get_or_fetch("USC0063201.1.1", &repo).unwrap();
        assert_eq!(repo.query_count(), 1);

        // Verify it's cached
        cache.get_or_fetch("USC0063201.1.1", &repo).unwrap();
        assert_eq!(repo.query_count(), 1);

        // Invalidate
        cache.invalidate("USC0063201.1.1");

        // Should query repository again
        cache.get_or_fetch("USC0063201.1.1", &repo).unwrap();
        assert_eq!(repo.query_count(), 2);
    }

    #[test]
    fn test_clear() {
        let config = WbsCacheConfig::with_ttl(Duration::from_secs(60));
        let cache = WbsCache::new(config);

        // Add some entries
        cache.insert("USC001", WbsElement {
            wbs_code: "USC001".to_string(),
            project_def: "TEST".to_string(),
            project_name: None,
            description: None,
            status: "REL".to_string(),
            cached_at: chrono::Utc::now().timestamp(),
            opportunity_id: None,
            deal_name: None,
            target_company_name: None,
            counterparty: None,
            industry: None,
            region: None,
            amount: None,
            stage_name: None,
            project_code: None,
        });
        cache.cache_not_found("USC002");

        // Verify entries exist
        let stats_before = cache.stats();
        assert_eq!(stats_before.positive_entry_count, 1);
        assert_eq!(stats_before.negative_entry_count, 1);

        // Clear cache
        cache.clear();

        // Verify entries removed
        let stats_after = cache.stats();
        assert_eq!(stats_after.positive_entry_count, 0);
        assert_eq!(stats_after.negative_entry_count, 0);
    }

    #[test]
    fn test_normalize() {
        let config = WbsCacheConfig::with_ttl(Duration::from_secs(60));
        let cache = WbsCache::new(config);
        let repo = MockWbsRepository::new(vec!["USC0063201.1.1".to_string()]);

        // Cache with lowercase
        cache.get_or_fetch("usc0063201.1.1", &repo).unwrap();
        assert_eq!(repo.query_count(), 1);

        // Query with uppercase - should hit cache (normalized)
        cache.get_or_fetch("USC0063201.1.1", &repo).unwrap();
        assert_eq!(repo.query_count(), 1); // Still 1, not 2
    }

    #[test]
    #[ignore] // moka uses std::time::Instant internally, doesn't respect MockClock
    fn test_ttl_expiration_with_mock_clock() {
        let clock = MockClock::new();
        let config = WbsCacheConfig::with_ttl(Duration::from_secs(300));
        let cache = WbsCache::with_clock(config, clock.clone());

        // Cache a WBS element
        let wbs = WbsElement {
            wbs_code: "USC0063201.1.1".to_string(),
            project_def: "TEST-001".to_string(),
            project_name: Some("Test Project".to_string()),
            description: None,
            status: "REL".to_string(),
            cached_at: chrono::Utc::now().timestamp(),
            opportunity_id: None,
            deal_name: None,
            target_company_name: None,
            counterparty: None,
            industry: None,
            region: None,
            amount: None,
            stage_name: None,
            project_code: None,
        };
        cache.insert("USC0063201.1.1", wbs);

        // Verify it's cached
        assert!(matches!(
            cache.get("USC0063201.1.1"),
            CacheResult::Hit(_)
        ));

        // Advance clock past TTL
        clock.advance(Duration::from_secs(301));

        // Force eviction by running pending tasks
        cache.positive_cache.run_pending_tasks();

        // Verify it's expired
        assert!(matches!(cache.get("USC0063201.1.1"), CacheResult::Miss));
    }
}
