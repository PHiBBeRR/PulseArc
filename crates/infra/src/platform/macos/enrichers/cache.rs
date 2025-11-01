//! Enrichment data caching with TTL.
//!
//! Provides a thread-safe cache for browser URLs and office document names
//! to avoid expensive AppleScript calls for recently-seen applications.
//!
//! # Cache Strategy
//! - **Key**: Application bundle ID (e.g., "com.apple.Safari")
//! - **Value**: Enrichment data (URL or document name)
//! - **TTL**: 5 minutes (configurable)
//! - **Eviction**: Time-to-live based
//!
//! # Example
//! ```rust,no_run
//! use std::time::Duration;
//!
//! use pulsearc_infra::platform::macos::enrichers::cache::EnrichmentCache;
//!
//! let cache = EnrichmentCache::new(Duration::from_secs(300));
//!
//! // Cache a browser URL
//! cache.set_browser_url("com.apple.Safari", "https://example.com");
//!
//! // Retrieve from cache (if not expired)
//! if let Some(url) = cache.get_browser_url("com.apple.Safari") {
//!     println!("Cached URL: {url}");
//! }
//! ```

use std::time::Duration;

use moka::sync::Cache;

/// Default TTL for enrichment cache entries (5 minutes).
pub const DEFAULT_ENRICHMENT_TTL: Duration = Duration::from_secs(300);

/// Cached enrichment data for an application.
///
/// This enum allows storing different types of enrichment data
/// in a single cache with type safety.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnrichmentData {
    /// Browser URL from the active tab
    BrowserUrl(String),
    /// Office document name
    OfficeDocument(String),
}

/// Thread-safe enrichment cache with TTL-based eviction.
///
/// This cache stores enrichment data (URLs, document names) keyed by
/// application bundle ID to avoid repeated expensive AppleScript calls.
#[derive(Clone)]
pub struct EnrichmentCache {
    cache: Cache<String, EnrichmentData>,
}

impl EnrichmentCache {
    /// Create a new enrichment cache with the specified TTL.
    ///
    /// # Arguments
    /// * `ttl` - Time-to-live for cache entries
    ///
    /// # Returns
    /// A new `EnrichmentCache` instance
    ///
    /// # Example
    /// ```rust
    /// use std::time::Duration;
    /// # use pulsearc_infra::platform::macos::enrichers::cache::EnrichmentCache;
    ///
    /// let cache = EnrichmentCache::new(Duration::from_secs(300));
    /// ```
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(ttl)
                .max_capacity(1000) // Reasonable limit for active apps
                .build(),
        }
    }

    /// Create a new enrichment cache with default TTL (5 minutes).
    ///
    /// # Example
    /// ```rust
    /// # use pulsearc_infra::platform::macos::enrichers::cache::EnrichmentCache;
    /// let cache = EnrichmentCache::default();
    /// ```
    #[must_use]
    pub fn default_ttl() -> Self {
        Self::new(DEFAULT_ENRICHMENT_TTL)
    }

    /// Get a browser URL from the cache.
    ///
    /// # Arguments
    /// * `bundle_id` - The browser's bundle identifier
    ///
    /// # Returns
    /// * `Some(String)` - The cached URL if present and not expired
    /// * `None` - If no cached entry exists or it expired
    pub fn get_browser_url(&self, bundle_id: &str) -> Option<String> {
        self.cache.get(bundle_id).and_then(|data| {
            if let EnrichmentData::BrowserUrl(url) = data {
                Some(url)
            } else {
                // Wrong data type cached - this shouldn't happen
                tracing::warn!(
                    bundle_id = %bundle_id,
                    "Cache returned non-URL data for browser lookup"
                );
                None
            }
        })
    }

    /// Set a browser URL in the cache.
    ///
    /// # Arguments
    /// * `bundle_id` - The browser's bundle identifier
    /// * `url` - The URL to cache
    pub fn set_browser_url(&self, bundle_id: impl Into<String>, url: impl Into<String>) {
        self.cache.insert(bundle_id.into(), EnrichmentData::BrowserUrl(url.into()));
    }

    /// Get an office document name from the cache.
    ///
    /// # Arguments
    /// * `bundle_id` - The office app's bundle identifier
    ///
    /// # Returns
    /// * `Some(String)` - The cached document name if present and not expired
    /// * `None` - If no cached entry exists or it expired
    pub fn get_office_document(&self, bundle_id: &str) -> Option<String> {
        self.cache.get(bundle_id).and_then(|data| {
            if let EnrichmentData::OfficeDocument(doc) = data {
                Some(doc)
            } else {
                tracing::warn!(
                    bundle_id = %bundle_id,
                    "Cache returned non-document data for office lookup"
                );
                None
            }
        })
    }

    /// Set an office document name in the cache.
    ///
    /// # Arguments
    /// * `bundle_id` - The office app's bundle identifier
    /// * `document` - The document name to cache
    pub fn set_office_document(&self, bundle_id: impl Into<String>, document: impl Into<String>) {
        self.cache.insert(bundle_id.into(), EnrichmentData::OfficeDocument(document.into()));
    }

    /// Clear all entries from the cache.
    pub fn clear(&self) {
        self.cache.invalidate_all();
    }

    /// Get the number of entries in the cache.
    ///
    /// Note: This triggers eviction of expired entries.
    #[must_use]
    pub fn entry_count(&self) -> u64 {
        self.cache.run_pending_tasks();
        self.cache.entry_count()
    }

    /// Remove a specific entry from the cache.
    ///
    /// # Arguments
    /// * `bundle_id` - The bundle identifier to remove
    pub fn invalidate(&self, bundle_id: &str) {
        self.cache.invalidate(bundle_id);
    }
}

impl Default for EnrichmentCache {
    fn default() -> Self {
        Self::default_ttl()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_new_cache() {
        let cache = EnrichmentCache::new(Duration::from_secs(60));
        assert_eq!(cache.entry_count(), 0);
    }

    #[test]
    fn test_default_cache() {
        let cache = EnrichmentCache::default();
        assert_eq!(cache.entry_count(), 0);
    }

    #[test]
    fn test_set_and_get_browser_url() {
        let cache = EnrichmentCache::new(Duration::from_secs(60));

        cache.set_browser_url("com.apple.Safari", "https://example.com");
        assert_eq!(
            cache.get_browser_url("com.apple.Safari"),
            Some("https://example.com".to_string())
        );
    }

    #[test]
    fn test_set_and_get_office_document() {
        let cache = EnrichmentCache::new(Duration::from_secs(60));

        cache.set_office_document("com.microsoft.Word", "Report.docx");
        assert_eq!(
            cache.get_office_document("com.microsoft.Word"),
            Some("Report.docx".to_string())
        );
    }

    #[test]
    fn test_get_nonexistent_entry() {
        let cache = EnrichmentCache::new(Duration::from_secs(60));
        assert_eq!(cache.get_browser_url("com.apple.Safari"), None);
        assert_eq!(cache.get_office_document("com.microsoft.Word"), None);
    }

    #[test]
    fn test_type_mismatch() {
        let cache = EnrichmentCache::new(Duration::from_secs(60));

        // Set a browser URL
        cache.set_browser_url("com.apple.Safari", "https://example.com");

        // Try to get it as an office document - should return None
        assert_eq!(cache.get_office_document("com.apple.Safari"), None);

        // Original URL should still be retrievable
        assert_eq!(
            cache.get_browser_url("com.apple.Safari"),
            Some("https://example.com".to_string())
        );
    }

    #[test]
    fn test_clear() {
        let cache = EnrichmentCache::new(Duration::from_secs(60));

        cache.set_browser_url("com.apple.Safari", "https://example.com");
        cache.set_office_document("com.microsoft.Word", "Report.docx");

        assert_eq!(cache.entry_count(), 2);

        cache.clear();
        assert_eq!(cache.entry_count(), 0);

        assert_eq!(cache.get_browser_url("com.apple.Safari"), None);
        assert_eq!(cache.get_office_document("com.microsoft.Word"), None);
    }

    #[test]
    fn test_invalidate() {
        let cache = EnrichmentCache::new(Duration::from_secs(60));

        cache.set_browser_url("com.apple.Safari", "https://example.com");
        cache.set_browser_url("com.google.Chrome", "https://google.com");

        assert_eq!(cache.entry_count(), 2);

        cache.invalidate("com.apple.Safari");
        assert_eq!(cache.entry_count(), 1);

        assert_eq!(cache.get_browser_url("com.apple.Safari"), None);
        assert_eq!(
            cache.get_browser_url("com.google.Chrome"),
            Some("https://google.com".to_string())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ttl_expiration() {
        let cache = EnrichmentCache::new(Duration::from_millis(100));

        cache.set_browser_url("com.apple.Safari", "https://example.com");
        assert_eq!(
            cache.get_browser_url("com.apple.Safari"),
            Some("https://example.com".to_string())
        );

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Force eviction by checking entry count
        let _ = cache.entry_count();

        // Entry should be expired
        assert_eq!(cache.get_browser_url("com.apple.Safari"), None);
    }

    #[test]
    fn test_cache_clone() {
        let cache1 = EnrichmentCache::new(Duration::from_secs(60));
        cache1.set_browser_url("com.apple.Safari", "https://example.com");

        let cache2 = cache1.clone();
        assert_eq!(
            cache2.get_browser_url("com.apple.Safari"),
            Some("https://example.com".to_string())
        );

        // Cloned cache shares the same underlying cache
        cache2.set_browser_url("com.google.Chrome", "https://google.com");
        assert_eq!(
            cache1.get_browser_url("com.google.Chrome"),
            Some("https://google.com".to_string())
        );
    }
}
