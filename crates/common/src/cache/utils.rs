//! Cache utilities for monitoring, reporting, and management
//!
//! This module provides helper utilities for cache management, including
//! metrics reporting, health checks, and diagnostic tools.

use std::fmt;

#[cfg(feature = "observability")]
use tracing::{info, warn};

use super::{Cache, CacheStats};

/// Cache health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheHealth {
    /// Cache is operating normally
    Healthy,
    /// Cache hit rate is low, consider tuning
    LowHitRate,
    /// Cache is nearly full, consider increasing size
    NearCapacity,
    /// Cache has both low hit rate and near capacity
    Critical,
}

impl fmt::Display for CacheHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "Healthy"),
            Self::LowHitRate => write!(f, "Low Hit Rate"),
            Self::NearCapacity => write!(f, "Near Capacity"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

/// Cache health report with diagnostics
#[derive(Debug, Clone)]
pub struct CacheHealthReport {
    /// Overall health status
    pub health: CacheHealth,
    /// Current cache statistics
    pub stats: CacheStats,
    /// Recommendations for optimization
    pub recommendations: Vec<String>,
}

impl CacheHealthReport {
    /// Generate a health report for a cache
    ///
    /// # Thresholds
    /// - Low hit rate: < 50%
    /// - Near capacity: > 85% full
    ///
    /// # Example
    /// ```
    /// use pulsearc_common::cache::utils::CacheHealthReport;
    /// use pulsearc_common::cache::{Cache, CacheConfig};
    ///
    /// let cache: Cache<String, i32> =
    ///     Cache::new(CacheConfig::builder().max_size(100).track_metrics(true).build());
    ///
    /// let report = CacheHealthReport::new(&cache);
    /// println!("{}", report);
    /// ```
    pub fn new<K, V>(cache: &Cache<K, V>) -> Self
    where
        K: Eq + std::hash::Hash + Clone,
        V: Clone,
    {
        let stats = cache.stats();
        let mut recommendations = Vec::new();

        // Check hit rate
        let low_hit_rate = stats.hit_rate() < 0.5 && stats.total_accesses() > 100;
        if low_hit_rate {
            recommendations.push(format!(
                "Hit rate is {:.2}%. Consider increasing cache size or adjusting TTL.",
                stats.hit_rate() * 100.0
            ));
        }

        // Check capacity
        let near_capacity =
            if let Some(fill_pct) = stats.fill_percentage() { fill_pct > 0.85 } else { false };

        if near_capacity {
            recommendations.push(format!(
                "Cache is {:.1}% full. Consider increasing max_size.",
                stats.fill_percentage().unwrap() * 100.0
            ));
        }

        // Check eviction rate
        if stats.total_accesses() > 0 {
            let eviction_rate = stats.evictions as f64 / stats.total_accesses() as f64;
            if eviction_rate > 0.2 {
                recommendations.push(format!(
                    "High eviction rate: {:.2}%. Cache may be too small for workload.",
                    eviction_rate * 100.0
                ));
            }
        }

        // Check expiration rate
        if stats.total_accesses() > 0 {
            let expiration_rate = stats.expirations as f64 / stats.total_accesses() as f64;
            if expiration_rate > 0.3 {
                recommendations.push(format!(
                    "High expiration rate: {:.2}%. Consider increasing TTL.",
                    expiration_rate * 100.0
                ));
            }
        }

        let health = match (low_hit_rate, near_capacity) {
            (true, true) => CacheHealth::Critical,
            (true, false) => CacheHealth::LowHitRate,
            (false, true) => CacheHealth::NearCapacity,
            (false, false) => CacheHealth::Healthy,
        };

        Self { health, stats, recommendations }
    }

    /// Log the health report using tracing (requires `observability` feature)
    #[cfg(feature = "observability")]
    pub fn log(&self) {
        match self.health {
            CacheHealth::Healthy => {
                info!(
                    health = %self.health,
                    hit_rate = self.stats.hit_rate(),
                    size = self.stats.size,
                    "Cache health check: Healthy"
                );
            }
            CacheHealth::LowHitRate | CacheHealth::NearCapacity | CacheHealth::Critical => {
                warn!(
                    health = %self.health,
                    hit_rate = self.stats.hit_rate(),
                    size = self.stats.size,
                    max_size = ?self.stats.max_size,
                    "Cache health check: Issues detected"
                );
                for rec in &self.recommendations {
                    warn!(recommendation = %rec, "Cache optimization recommendation");
                }
            }
        }
    }
}

impl fmt::Display for CacheHealthReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Cache Health Report")?;
        writeln!(f, "===================")?;
        writeln!(f, "Status: {}", self.health)?;
        writeln!(f)?;
        writeln!(f, "Statistics:")?;
        writeln!(f, "  Size: {}/{:?}", self.stats.size, self.stats.max_size)?;
        writeln!(f, "  Hits: {}", self.stats.hits)?;
        writeln!(f, "  Misses: {}", self.stats.misses)?;
        writeln!(f, "  Hit Rate: {:.2}%", self.stats.hit_rate() * 100.0)?;
        writeln!(f, "  Evictions: {}", self.stats.evictions)?;
        writeln!(f, "  Expirations: {}", self.stats.expirations)?;
        if let Some(fill_pct) = self.stats.fill_percentage() {
            writeln!(f, "  Fill: {:.1}%", fill_pct * 100.0)?;
        }

        if !self.recommendations.is_empty() {
            writeln!(f)?;
            writeln!(f, "Recommendations:")?;
            for (i, rec) in self.recommendations.iter().enumerate() {
                writeln!(f, "  {}. {}", i + 1, rec)?;
            }
        }

        Ok(())
    }
}

/// Cache metrics reporter for periodic monitoring
///
/// # Example
/// ```
/// use pulsearc_common::cache::utils::MetricsReporter;
/// use pulsearc_common::cache::{Cache, CacheConfig};
///
/// let cache: Cache<String, i32> =
///     Cache::new(CacheConfig::builder().max_size(1000).track_metrics(true).build());
///
/// let reporter = MetricsReporter::new("my_cache");
/// reporter.report(&cache);
/// ```
pub struct MetricsReporter {
    cache_name: String,
}

impl MetricsReporter {
    /// Create a new metrics reporter
    pub fn new(cache_name: impl Into<String>) -> Self {
        Self { cache_name: cache_name.into() }
    }

    /// Report current cache metrics using tracing (requires `observability`
    /// feature)
    #[cfg(feature = "observability")]
    pub fn report<K, V>(&self, cache: &Cache<K, V>)
    where
        K: Eq + std::hash::Hash + Clone,
        V: Clone,
    {
        let stats = cache.stats();
        info!(
            cache = %self.cache_name,
            size = stats.size,
            max_size = ?stats.max_size,
            hits = stats.hits,
            misses = stats.misses,
            hit_rate = format!("{:.2}%", stats.hit_rate() * 100.0),
            evictions = stats.evictions,
            expirations = stats.expirations,
            "Cache metrics report"
        );
    }

    /// Report metrics in JSON format (for structured logging)
    pub fn report_json<K, V>(&self, cache: &Cache<K, V>) -> serde_json::Value
    where
        K: Eq + std::hash::Hash + Clone,
        V: Clone,
    {
        let stats = cache.stats();
        serde_json::json!({
            "cache_name": self.cache_name,
            "size": stats.size,
            "max_size": stats.max_size,
            "hits": stats.hits,
            "misses": stats.misses,
            "hit_rate": stats.hit_rate(),
            "miss_rate": stats.miss_rate(),
            "evictions": stats.evictions,
            "expirations": stats.expirations,
            "total_accesses": stats.total_accesses(),
            "fill_percentage": stats.fill_percentage(),
        })
    }
}

/// Cache prewarming utility for loading frequently accessed data
///
/// # Example
/// ```
/// use pulsearc_common::cache::utils::CacheWarmer;
/// use pulsearc_common::cache::{Cache, CacheConfig};
///
/// let cache: Cache<String, String> = Cache::new(CacheConfig::lru(100));
///
/// let warm_data = vec![
///     ("config".to_string(), "value1".to_string()),
///     ("user_prefs".to_string(), "value2".to_string()),
/// ];
///
/// let warmer = CacheWarmer::new();
/// warmer.warm(&cache, warm_data);
/// ```
pub struct CacheWarmer;

impl CacheWarmer {
    /// Create a new cache warmer
    pub fn new() -> Self {
        Self
    }

    /// Warm cache with provided data
    pub fn warm<K, V>(&self, cache: &Cache<K, V>, data: Vec<(K, V)>)
    where
        K: Eq + std::hash::Hash + Clone,
        V: Clone,
    {
        let count = data.len();
        #[cfg(feature = "observability")]
        info!(count, "Warming cache with {} entries", count);

        for (key, value) in data {
            cache.insert(key, value);
        }

        #[cfg(feature = "observability")]
        info!(count, final_size = cache.len(), "Cache warming completed");
    }

    /// Warm cache using a loader function
    ///
    /// The loader function is called for each key and should return the value
    /// to cache.
    pub fn warm_with_loader<K, V, F>(&self, cache: &Cache<K, V>, keys: Vec<K>, mut loader: F)
    where
        K: Eq + std::hash::Hash + Clone,
        V: Clone,
        F: FnMut(&K) -> Option<V>,
    {
        let count = keys.len();
        #[cfg(feature = "observability")]
        info!(count, "Warming cache with loader for {} keys", count);

        let mut loaded_count = 0;
        for key in keys {
            if let Some(value) = loader(&key) {
                cache.insert(key, value);
                loaded_count += 1;
            }
        }

        #[cfg(feature = "observability")]
        info!(
            requested = count,
            loaded = loaded_count,
            final_size = cache.len(),
            "Cache warming with loader completed"
        );
    }
}

impl Default for CacheWarmer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CacheConfig;

    /// Validates `Cache::new` behavior for the health report healthy scenario.
    ///
    /// Assertions:
    /// - Confirms `report.health` equals `CacheHealth::Healthy`.
    #[test]
    fn test_health_report_healthy() {
        let cache: Cache<String, i32> =
            Cache::new(CacheConfig::builder().max_size(100).track_metrics(true).build());

        // Populate with good hit rate
        for i in 0..50 {
            cache.insert(format!("key{}", i), i);
        }

        // Generate hits
        for i in 0..50 {
            let _ = cache.get(&format!("key{}", i));
        }

        let report = CacheHealthReport::new(&cache);
        assert_eq!(report.health, CacheHealth::Healthy);
    }

    /// Validates `Cache::new` behavior for the health report low hit rate
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `report.health` equals `CacheHealth::LowHitRate`.
    /// - Ensures `!report.recommendations.is_empty()` evaluates to true.
    #[test]
    fn test_health_report_low_hit_rate() {
        let cache: Cache<String, i32> =
            Cache::new(CacheConfig::builder().max_size(100).track_metrics(true).build());

        // Populate
        for i in 0..10 {
            cache.insert(format!("key{}", i), i);
        }

        // Generate hits (to reach > 100 total accesses threshold)
        for i in 0..10 {
            let _ = cache.get(&format!("key{}", i));
        }

        // Generate lots of misses to push hit rate below 50%
        for i in 100..250 {
            let _ = cache.get(&format!("key{}", i));
        }

        let report = CacheHealthReport::new(&cache);
        assert_eq!(report.health, CacheHealth::LowHitRate);
        assert!(!report.recommendations.is_empty());
    }

    /// Validates `Cache::new` behavior for the health report near capacity
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `report.health` equals `CacheHealth::NearCapacity`.
    #[test]
    fn test_health_report_near_capacity() {
        let cache: Cache<String, i32> =
            Cache::new(CacheConfig::builder().max_size(100).track_metrics(true).build());

        // Fill cache to 90%
        for i in 0..90 {
            cache.insert(format!("key{}", i), i);
        }

        // Generate some hits to avoid low hit rate
        for i in 0..90 {
            let _ = cache.get(&format!("key{}", i));
        }

        let report = CacheHealthReport::new(&cache);
        assert_eq!(report.health, CacheHealth::NearCapacity);
    }

    /// Validates `Cache::new` behavior for the health report display scenario.
    ///
    /// Assertions:
    /// - Ensures `display.contains("Cache Health Report")` evaluates to true.
    /// - Ensures `display.contains("Status:")` evaluates to true.
    /// - Ensures `display.contains("Statistics:")` evaluates to true.
    #[test]
    fn test_health_report_display() {
        let cache: Cache<String, i32> =
            Cache::new(CacheConfig::builder().max_size(100).track_metrics(true).build());

        let report = CacheHealthReport::new(&cache);
        let display = format!("{}", report);
        assert!(display.contains("Cache Health Report"));
        assert!(display.contains("Status:"));
        assert!(display.contains("Statistics:"));
    }

    /// Validates `Cache::new` behavior for the metrics reporter scenario.
    ///
    /// Assertions:
    /// - Confirms `json["cache_name"]` equals `"test_cache"`.
    /// - Confirms `json["size"]` equals `1`.
    /// - Confirms `json["hits"]` equals `1`.
    #[test]
    fn test_metrics_reporter() {
        let cache: Cache<String, i32> =
            Cache::new(CacheConfig::builder().max_size(100).track_metrics(true).build());

        cache.insert("key".to_string(), 42);
        let _ = cache.get(&"key".to_string());

        let reporter = MetricsReporter::new("test_cache");
        reporter.report(&cache);

        let json = reporter.report_json(&cache);
        assert_eq!(json["cache_name"], "test_cache");
        assert_eq!(json["size"], 1);
        assert_eq!(json["hits"], 1);
    }

    /// Validates `Cache::new` behavior for the cache warmer scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len()` equals `3`.
    /// - Confirms `cache.get(&"key1".to_string())` equals
    ///   `Some("value1".to_string())`.
    #[test]
    fn test_cache_warmer() {
        let cache: Cache<String, String> = Cache::new(CacheConfig::lru(100));

        let warm_data = vec![
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
            ("key3".to_string(), "value3".to_string()),
        ];

        let warmer = CacheWarmer::new();
        warmer.warm(&cache, warm_data);

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
    }

    /// Validates `Cache::new` behavior for the cache warmer with loader
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len()` equals `3`.
    /// - Confirms `cache.get(&"key1".to_string())` equals `Some(42)`.
    #[test]
    fn test_cache_warmer_with_loader() {
        let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(100));

        let keys = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];

        let warmer = CacheWarmer::new();
        warmer.warm_with_loader(&cache, keys, |key| {
            // Simulate loading from database
            if key.starts_with("key") {
                Some(42)
            } else {
                None
            }
        });

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&"key1".to_string()), Some(42));
    }
}
