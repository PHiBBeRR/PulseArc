//! Feature flag service with in-memory caching and fallback awareness.
//!
//! Provides a high-level interface to database-backed feature flags with
//! caching for performance. The service exposes both a simple `is_enabled`
//! helper (for existing call-sites) and an `evaluate` method that reports when
//! the default fallback value was used, enabling precise observability.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_core::feature_flags_ports::{FeatureFlag, FeatureFlagEvaluation, FeatureFlagsPort};
use pulsearc_domain::Result as DomainResult;
use tokio::sync::Mutex;

use crate::database::{DbManager, SqlCipherFeatureFlagsRepository};

type FlagCache = Arc<Mutex<HashMap<String, FeatureFlagEvaluation>>>;

/// Feature flag service with in-memory caching.
///
/// The repository is kept internal; consumers interact via the service to gain
/// caching and fallback context.
pub struct FeatureFlagService {
    repository: Arc<SqlCipherFeatureFlagsRepository>,
    cache: FlagCache,
}

impl FeatureFlagService {
    /// Create a new feature flag service.
    pub fn new(db: Arc<DbManager>) -> Self {
        Self {
            repository: Arc::new(SqlCipherFeatureFlagsRepository::new(db)),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Evaluate a feature flag, returning both the computed value and whether
    /// the default fallback was used.
    pub async fn evaluate(
        &self,
        flag_name: &str,
        default: bool,
    ) -> DomainResult<FeatureFlagEvaluation> {
        // Fast path: cache hit.
        {
            let cache = self.cache.lock().await;
            if let Some(entry) = cache.get(flag_name) {
                return Ok(*entry);
            }
        }

        // Cache miss: fetch from repository (which handles spawn_blocking).
        let evaluation = self.repository.evaluate(flag_name, default).await?;

        // Populate cache with the evaluation result.
        {
            let mut cache = self.cache.lock().await;
            cache.insert(flag_name.to_owned(), evaluation);
        }

        Ok(evaluation)
    }

    /// Check if a feature flag is enabled, using read-through caching.
    pub async fn is_enabled(&self, flag_name: &str, default: bool) -> DomainResult<bool> {
        self.evaluate(flag_name, default).await.map(|evaluation| evaluation.enabled)
    }

    /// Set a feature flag's enabled status (write-through invalidation).
    pub async fn set_enabled(&self, flag_name: &str, enabled: bool) -> DomainResult<()> {
        self.repository.set_enabled(flag_name, enabled).await?;

        // Invalidate cache entry immediately to avoid stale reads.
        {
            let mut cache = self.cache.lock().await;
            cache.remove(flag_name);
        }

        Ok(())
    }

    /// List all feature flags (always hits the repository for freshness).
    pub async fn list_all(&self) -> DomainResult<Vec<FeatureFlag>> {
        self.repository.list_all().await
    }

    /// Clear the in-memory cache (useful for tests or manual refresh).
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }
}

#[async_trait]
impl FeatureFlagsPort for FeatureFlagService {
    async fn evaluate(
        &self,
        flag_name: &str,
        default: bool,
    ) -> DomainResult<FeatureFlagEvaluation> {
        <FeatureFlagService>::evaluate(self, flag_name, default).await
    }

    async fn set_enabled(&self, flag_name: &str, enabled: bool) -> DomainResult<()> {
        <FeatureFlagService>::set_enabled(self, flag_name, enabled).await
    }

    async fn list_all(&self) -> DomainResult<Vec<FeatureFlag>> {
        <FeatureFlagService>::list_all(self).await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::database::DbManager;

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn cache_hit_after_miss() {
        let (service, _mgr, _dir) = setup().await;

        // First call populates cache.
        let first = service.evaluate("new_blocks_cmd", false).await.expect("initial evaluation");
        assert!(first.enabled);
        assert!(!first.fallback_used);

        // Second call should be served from cache.
        let second = service.evaluate("new_blocks_cmd", false).await.expect("cached evaluation");
        assert!(second.enabled);
        assert!(!second.fallback_used);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cache_invalidation_on_write() {
        let (service, _mgr, _dir) = setup().await;

        // Populate cache.
        service.evaluate("new_blocks_cmd", false).await.expect("initial evaluation");

        // Ensure cache entry exists.
        assert!(service.cache.lock().await.contains_key("new_blocks_cmd"));

        // Update flag (write-through invalidation).
        service.set_enabled("new_blocks_cmd", false).await.expect("update succeeded");

        // Cache entry should be gone.
        assert!(
            !service.cache.lock().await.contains_key("new_blocks_cmd"),
            "cache entry should be invalidated after write"
        );

        // Subsequent evaluation reflects new value.
        let evaluation =
            service.evaluate("new_blocks_cmd", true).await.expect("evaluation after update");
        assert!(!evaluation.enabled);
        assert!(!evaluation.fallback_used);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn clear_cache_empties_store() {
        let (service, _mgr, _dir) = setup().await;

        service.evaluate("new_blocks_cmd", false).await.expect("evaluation");
        service.evaluate("use_new_infra", false).await.expect("evaluation");
        assert_eq!(service.cache.lock().await.len(), 2);

        service.clear_cache().await;
        assert!(service.cache.lock().await.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn list_all_bypasses_cache() {
        let (service, _mgr, _dir) = setup().await;

        // Populate cache with one entry.
        service.evaluate("new_blocks_cmd", false).await.expect("evaluation");

        // list_all should still return all flags from the repository.
        let all_flags = service.list_all().await.expect("list_all");
        assert!(
            all_flags.len() >= 2,
            "bootstrap migrations should surface at least the default feature flags"
        );

        // Cache should still only have the one entry.
        assert_eq!(service.cache.lock().await.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fallback_flag_detected() {
        let (service, _mgr, _dir) = setup().await;

        // Use a flag that doesn't exist yet.
        let evaluation = service.evaluate("brand_new_flag", true).await.expect("evaluation");
        assert!(evaluation.enabled);
        assert!(evaluation.fallback_used);

        // After setting the flag it should no longer use fallback.
        service.set_enabled("brand_new_flag", false).await.expect("set flag");

        let evaluation = service.evaluate("brand_new_flag", true).await.expect("evaluation");
        assert!(!evaluation.enabled);
        assert!(!evaluation.fallback_used);
    }

    async fn setup() -> (FeatureFlagService, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("flags.db");

        let manager =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        manager.run_migrations().expect("migrations executed");

        let service = FeatureFlagService::new(manager.clone());
        (service, manager, temp_dir)
    }
}
