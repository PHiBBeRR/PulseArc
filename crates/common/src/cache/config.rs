//! Cache configuration types and builder patterns
//!
//! This module provides configuration types for customizing cache behavior,
//! including eviction policies, TTL settings, and size limits.

use std::time::Duration;

/// Eviction policy for cache entries when capacity is reached
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EvictionPolicy {
    /// Least Recently Used - evicts the least recently accessed entry
    #[default]
    LRU,
    /// Least Frequently Used - evicts the least frequently accessed entry
    LFU,
    /// First In First Out - evicts the oldest entry by insertion time
    FIFO,
    /// Random eviction
    Random,
    /// No automatic eviction (manual only)
    None,
}

/// Configuration for cache behavior
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries (None = unlimited)
    pub max_size: Option<usize>,

    /// Time-to-live for entries (None = no expiration)
    pub ttl: Option<Duration>,

    /// Eviction policy when max_size is reached
    pub eviction_policy: EvictionPolicy,

    /// Whether to collect detailed access metrics
    pub track_metrics: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: None,
            ttl: None,
            eviction_policy: EvictionPolicy::LRU,
            track_metrics: false,
        }
    }
}

impl CacheConfig {
    /// Create a new configuration builder
    pub fn builder() -> CacheConfigBuilder {
        CacheConfigBuilder::default()
    }

    /// Quick preset for TTL-based cache
    ///
    /// # Example
    /// ```
    /// use std::time::Duration;
    ///
    /// use pulsearc_common::cache::CacheConfig;
    ///
    /// let config = CacheConfig::ttl(Duration::from_secs(3600));
    /// ```
    pub fn ttl(duration: Duration) -> Self {
        Self {
            max_size: None,
            ttl: Some(duration),
            eviction_policy: EvictionPolicy::None,
            track_metrics: false,
        }
    }

    /// Quick preset for LRU cache
    ///
    /// # Example
    /// ```
    /// use pulsearc_common::cache::CacheConfig;
    ///
    /// let config = CacheConfig::lru(1000);
    /// ```
    pub fn lru(max_size: usize) -> Self {
        Self {
            max_size: Some(max_size),
            ttl: None,
            eviction_policy: EvictionPolicy::LRU,
            track_metrics: false,
        }
    }

    /// Combined TTL + LRU cache
    ///
    /// # Example
    /// ```
    /// use std::time::Duration;
    ///
    /// use pulsearc_common::cache::CacheConfig;
    ///
    /// let config = CacheConfig::ttl_lru(Duration::from_secs(3600), 1000);
    /// ```
    pub fn ttl_lru(ttl: Duration, max_size: usize) -> Self {
        Self {
            max_size: Some(max_size),
            ttl: Some(ttl),
            eviction_policy: EvictionPolicy::LRU,
            track_metrics: false,
        }
    }
}

/// Builder for CacheConfig with fluent API
#[derive(Debug, Default)]
pub struct CacheConfigBuilder {
    config: CacheConfig,
}

impl CacheConfigBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum number of entries
    pub fn max_size(mut self, size: usize) -> Self {
        self.config.max_size = Some(size);
        self
    }

    /// Set time-to-live for entries
    pub fn ttl(mut self, duration: Duration) -> Self {
        self.config.ttl = Some(duration);
        self
    }

    /// Set eviction policy
    pub fn eviction_policy(mut self, policy: EvictionPolicy) -> Self {
        self.config.eviction_policy = policy;
        self
    }

    /// Enable or disable metrics tracking
    pub fn track_metrics(mut self, enabled: bool) -> Self {
        self.config.track_metrics = enabled;
        self
    }

    /// Build the configuration
    pub fn build(self) -> CacheConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for cache::config.
    use super::*;

    /// Validates `EvictionPolicy::default` behavior for the eviction policy
    /// default scenario.
    ///
    /// Assertions:
    /// - Confirms `EvictionPolicy::default()` equals `EvictionPolicy::LRU`.
    #[test]
    fn test_eviction_policy_default() {
        assert_eq!(EvictionPolicy::default(), EvictionPolicy::LRU);
    }

    /// Validates `CacheConfig::default` behavior for the cache config default
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `config.max_size.is_none()` evaluates to true.
    /// - Ensures `config.ttl.is_none()` evaluates to true.
    /// - Confirms `config.eviction_policy` equals `EvictionPolicy::LRU`.
    /// - Ensures `!config.track_metrics` evaluates to true.
    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(config.max_size.is_none());
        assert!(config.ttl.is_none());
        assert_eq!(config.eviction_policy, EvictionPolicy::LRU);
        assert!(!config.track_metrics);
    }

    /// Validates `Duration::from_secs` behavior for the cache config ttl preset
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `config.max_size.is_none()` evaluates to true.
    /// - Confirms `config.ttl` equals `Some(ttl)`.
    /// - Confirms `config.eviction_policy` equals `EvictionPolicy::None`.
    /// - Ensures `!config.track_metrics` evaluates to true.
    #[test]
    fn test_cache_config_ttl_preset() {
        let ttl = Duration::from_secs(3600);
        let config = CacheConfig::ttl(ttl);

        assert!(config.max_size.is_none());
        assert_eq!(config.ttl, Some(ttl));
        assert_eq!(config.eviction_policy, EvictionPolicy::None);
        assert!(!config.track_metrics);
    }

    /// Validates `CacheConfig::lru` behavior for the cache config lru preset
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `config.max_size` equals `Some(1000)`.
    /// - Ensures `config.ttl.is_none()` evaluates to true.
    /// - Confirms `config.eviction_policy` equals `EvictionPolicy::LRU`.
    /// - Ensures `!config.track_metrics` evaluates to true.
    #[test]
    fn test_cache_config_lru_preset() {
        let config = CacheConfig::lru(1000);

        assert_eq!(config.max_size, Some(1000));
        assert!(config.ttl.is_none());
        assert_eq!(config.eviction_policy, EvictionPolicy::LRU);
        assert!(!config.track_metrics);
    }

    /// Validates `Duration::from_secs` behavior for the cache config ttl lru
    /// preset scenario.
    ///
    /// Assertions:
    /// - Confirms `config.max_size` equals `Some(1000)`.
    /// - Confirms `config.ttl` equals `Some(ttl)`.
    /// - Confirms `config.eviction_policy` equals `EvictionPolicy::LRU`.
    /// - Ensures `!config.track_metrics` evaluates to true.
    #[test]
    fn test_cache_config_ttl_lru_preset() {
        let ttl = Duration::from_secs(3600);
        let config = CacheConfig::ttl_lru(ttl, 1000);

        assert_eq!(config.max_size, Some(1000));
        assert_eq!(config.ttl, Some(ttl));
        assert_eq!(config.eviction_policy, EvictionPolicy::LRU);
        assert!(!config.track_metrics);
    }

    /// Validates `CacheConfig::builder` behavior for the cache config builder
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `config.max_size` equals `Some(500)`.
    /// - Confirms `config.ttl` equals `Some(Duration::from_secs(1800))`.
    /// - Confirms `config.eviction_policy` equals `EvictionPolicy::LFU`.
    /// - Ensures `config.track_metrics` evaluates to true.
    #[test]
    fn test_cache_config_builder() {
        let config = CacheConfig::builder()
            .max_size(500)
            .ttl(Duration::from_secs(1800))
            .eviction_policy(EvictionPolicy::LFU)
            .track_metrics(true)
            .build();

        assert_eq!(config.max_size, Some(500));
        assert_eq!(config.ttl, Some(Duration::from_secs(1800)));
        assert_eq!(config.eviction_policy, EvictionPolicy::LFU);
        assert!(config.track_metrics);
    }

    /// Validates `CacheConfig::builder` behavior for the cache config builder
    /// partial scenario.
    ///
    /// Assertions:
    /// - Confirms `config.max_size` equals `Some(100)`.
    /// - Ensures `config.ttl.is_none()` evaluates to true.
    /// - Confirms `config.eviction_policy` equals `EvictionPolicy::LRU`.
    /// - Ensures `!config.track_metrics` evaluates to true.
    #[test]
    fn test_cache_config_builder_partial() {
        let config = CacheConfig::builder().max_size(100).build();

        assert_eq!(config.max_size, Some(100));
        assert!(config.ttl.is_none());
        assert_eq!(config.eviction_policy, EvictionPolicy::LRU);
        assert!(!config.track_metrics);
    }

    /// Validates `EvictionPolicy::LRU` behavior for the eviction policy
    /// variants scenario.
    ///
    /// Assertions:
    /// - Confirms `config.eviction_policy` equals `policy`.
    #[test]
    fn test_eviction_policy_variants() {
        let policies = vec![
            EvictionPolicy::LRU,
            EvictionPolicy::LFU,
            EvictionPolicy::FIFO,
            EvictionPolicy::Random,
            EvictionPolicy::None,
        ];

        for policy in policies {
            let config = CacheConfig::builder().eviction_policy(policy).build();
            assert_eq!(config.eviction_policy, policy);
        }
    }
}
