//! Feature flag service with in-memory caching.
//!
//! Provides a high-level interface to feature flags with caching for
//! performance. All database operations are delegated to the repository layer.
//!
//! # Caching Strategy
//!
//! - **Read-through**: Check cache first, query DB on miss, populate cache
//! - **Write-through invalidation**: Update DB, then remove cache entry
//!   immediately
//! - **No stale data**: Cache is invalidated on every write to prevent stale
//!   reads
//! - **Lazy loading**: Cache is populated on demand (no preloading at startup)
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use pulsearc_infra::database::DbManager;
//! use pulsearc_infra::services::FeatureFlagService;
//!
//! # async fn example() {
//! let db_manager = Arc::new(DbManager::new("app.db", 4, Some("key")).unwrap());
//! let service = FeatureFlagService::new(db_manager);
//!
//! // Check if feature is enabled
//! if service.is_enabled("new_blocks_cmd", false).await.unwrap_or(false) {
//!     // Use new infrastructure
//! }
//!
//! // Toggle feature for rollback
//! service.set_enabled("use_new_infra", false).await.unwrap();
//! # }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use pulsearc_core::feature_flags_ports::FeatureFlag;
use pulsearc_domain::Result as DomainResult;
use tokio::sync::Mutex;

use crate::database::{DbManager, SqlCipherFeatureFlagsRepository};

type FlagCache = Arc<Mutex<HashMap<String, bool>>>;

/// Feature flag service with in-memory caching.
///
/// This service wraps the repository layer and provides caching for
/// performance. The repository is kept private - external code should only
/// interact via this service.
pub struct FeatureFlagService {
    repository: Arc<SqlCipherFeatureFlagsRepository>,
    cache: FlagCache,
}

impl FeatureFlagService {
    /// Create a new feature flag service.
    ///
    /// The repository is created internally and kept private.
    /// All database operations are handled through the repository layer.
    pub fn new(db: Arc<DbManager>) -> Self {
        Self {
            repository: Arc::new(SqlCipherFeatureFlagsRepository::new(db)),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if a feature flag is enabled (cached).
    ///
    /// Returns the `default` value if the flag doesn't exist in the database.
    /// Uses read-through caching: checks cache first, queries DB on miss.
    ///
    /// # Performance
    ///
    /// - Cached queries: <1ms (in-memory lookup)
    /// - Database queries: <5ms (spawn_blocking + SQLite query)
    ///
    /// # Arguments
    ///
    /// * `flag_name` - The unique identifier for the feature flag
    /// * `default` - The value to return if the flag doesn't exist
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pulsearc_infra::services::FeatureFlagService;
    /// # async fn example(service: &FeatureFlagService) {
    /// // Check if new blocks command is enabled (default to false)
    /// let enabled = service.is_enabled("new_blocks_cmd", false).await.unwrap_or(false);
    /// # }
    /// ```
    pub async fn is_enabled(&self, flag_name: &str, default: bool) -> DomainResult<bool> {
        // Check cache first (fast path)
        {
            let cache = self.cache.lock().await;
            if let Some(&enabled) = cache.get(flag_name) {
                return Ok(enabled);
            }
        }

        // Cache miss - query DB (inside spawn_blocking in repository)
        let enabled = self.repository.is_enabled(flag_name, default).await?;

        // Populate cache for next time
        {
            let mut cache = self.cache.lock().await;
            cache.insert(flag_name.to_string(), enabled);
        }

        Ok(enabled)
    }

    /// Set a feature flag's enabled status.
    ///
    /// Creates the flag if it doesn't exist (upsert semantics).
    /// Invalidates the cache entry immediately to prevent stale reads.
    ///
    /// # Cache Invalidation
    ///
    /// This method uses **write-through cache invalidation**:
    /// 1. Update the database first (inside spawn_blocking in repository)
    /// 2. Remove the cache entry immediately
    ///
    /// This ensures the cache never holds stale data longer than a single
    /// toggle operation.
    ///
    /// # Arguments
    ///
    /// * `flag_name` - The unique identifier for the feature flag
    /// * `enabled` - The new enabled status
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pulsearc_infra::services::FeatureFlagService;
    /// # async fn example(service: &FeatureFlagService) {
    /// // Disable new infrastructure for quick rollback
    /// service.set_enabled("use_new_infra", false).await.unwrap();
    /// # }
    /// ```
    pub async fn set_enabled(&self, flag_name: &str, enabled: bool) -> DomainResult<()> {
        // Update DB first (inside spawn_blocking in repository)
        self.repository.set_enabled(flag_name, enabled).await?;

        // Invalidate cache entry immediately (write-through invalidation)
        {
            let mut cache = self.cache.lock().await;
            cache.remove(flag_name);
        }

        Ok(())
    }

    /// List all feature flags (uncached - always fresh).
    ///
    /// Returns all flags currently in the database, including their current
    /// state and metadata. This method bypasses the cache to ensure freshness.
    ///
    /// Useful for admin UI or debugging.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pulsearc_infra::services::FeatureFlagService;
    /// # async fn example(service: &FeatureFlagService) {
    /// let all_flags = service.list_all().await.unwrap();
    /// for flag in all_flags {
    ///     println!("{}: {}", flag.flag_name, flag.enabled);
    /// }
    /// # }
    /// ```
    pub async fn list_all(&self) -> DomainResult<Vec<FeatureFlag>> {
        // Always query DB for freshness (bypasses cache)
        self.repository.list_all().await
    }

    /// Clear the entire cache.
    ///
    /// Useful for testing or manual cache refresh. In production, the cache
    /// should stay warm as flags are queried and updated normally.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pulsearc_infra::services::FeatureFlagService;
    /// # async fn example(service: &FeatureFlagService) {
    /// // Force cache refresh (for testing/debugging)
    /// service.clear_cache().await;
    /// # }
    /// ```
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cache_hit_after_first_query() {
        let (service, _mgr, _dir) = setup().await;

        // First query - cache miss, populates cache
        let enabled =
            service.is_enabled("new_blocks_cmd", false).await.expect("first query succeeded");
        assert!(enabled);

        // Second query - cache hit (fast path)
        let enabled =
            service.is_enabled("new_blocks_cmd", false).await.expect("second query succeeded");
        assert!(enabled);

        // Verify cache contains the entry
        let cache = service.cache.lock().await;
        assert!(cache.contains_key("new_blocks_cmd"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cache_invalidation_on_write() {
        let (service, _mgr, _dir) = setup().await;

        // Populate cache
        service.is_enabled("new_blocks_cmd", false).await.expect("query succeeded");

        // Verify cache is populated
        {
            let cache = service.cache.lock().await;
            assert!(cache.contains_key("new_blocks_cmd"));
        }

        // Update flag - should invalidate cache
        service.set_enabled("new_blocks_cmd", false).await.expect("update succeeded");

        // Verify cache was invalidated
        {
            let cache = service.cache.lock().await;
            assert!(
                !cache.contains_key("new_blocks_cmd"),
                "cache should be invalidated after write"
            );
        }

        // Next query should return updated value
        let enabled = service.is_enabled("new_blocks_cmd", true).await.expect("query succeeded");
        assert!(!enabled, "should return updated value from DB");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_clear_cache() {
        let (service, _mgr, _dir) = setup().await;

        // Populate cache with multiple entries
        service.is_enabled("new_blocks_cmd", false).await.unwrap();
        service.is_enabled("use_new_infra", false).await.unwrap();

        // Verify cache has entries
        {
            let cache = service.cache.lock().await;
            assert_eq!(cache.len(), 2);
        }

        // Clear cache
        service.clear_cache().await;

        // Verify cache is empty
        {
            let cache = service.cache.lock().await;
            assert_eq!(cache.len(), 0);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_list_all_bypasses_cache() {
        let (service, _mgr, _dir) = setup().await;

        // Populate cache with one entry
        service.is_enabled("new_blocks_cmd", false).await.unwrap();

        // list_all should return all flags, not just cached ones
        let all_flags = service.list_all().await.expect("list_all succeeded");
        assert_eq!(all_flags.len(), 2, "should return all flags from DB");

        // Cache should still only have one entry
        let cache = service.cache.lock().await;
        assert_eq!(cache.len(), 1, "list_all should not populate cache");
    }

    /// Set up a test service with fresh database.
    async fn setup() -> (FeatureFlagService, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("flags.db");

        let mgr =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        mgr.run_migrations().expect("migrations run");

        let service = FeatureFlagService::new(mgr.clone());
        (service, mgr, temp_dir)
    }
}
