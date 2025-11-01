//! Feature flags port for runtime rollback control.
//!
//! Provides a database-backed feature flag system to enable gradual rollout
//! and quick rollback of Phase 4 command rewiring. Flags persist across app
//! restarts, suitable for macOS GUI applications.
//!
//! # Example
//!
//! ```no_run
//! use pulsearc_core::FeatureFlagsPort;
//!
//! async fn should_use_new_blocks(flags: &impl FeatureFlagsPort) -> bool {
//!     flags.is_enabled("new_blocks_cmd", false).await.unwrap_or(false)
//! }
//! ```

use async_trait::async_trait;
use pulsearc_domain::Result;

/// Feature flag data transfer object.
#[derive(Debug, Clone)]
pub struct FeatureFlag {
    /// Unique flag identifier (e.g., "new_blocks_cmd")
    pub flag_name: String,
    /// Whether the flag is currently enabled
    pub enabled: bool,
    /// Human-readable description of the flag's purpose
    pub description: Option<String>,
    /// Timestamp when the flag was last modified (Unix epoch seconds)
    pub updated_at: i64,
}

/// Result of evaluating a feature flag.
///
/// Provides both the effective enabled state and whether the value came from
/// the caller-provided default (meaning the flag is currently missing from the
/// backing store).
#[derive(Debug, Clone, Copy)]
pub struct FeatureFlagEvaluation {
    /// Whether the flag should be considered enabled.
    pub enabled: bool,
    /// Whether the returned value originated from the supplied fallback.
    pub fallback_used: bool,
}

/// Port for querying and managing feature flags.
///
/// Feature flags enable runtime control of feature rollout without code
/// changes. All operations are database-backed for persistence across restarts.
#[async_trait]
pub trait FeatureFlagsPort: Send + Sync {
    /// Evaluate a feature flag and report whether the fallback was used.
    ///
    /// Returns [`FeatureFlagEvaluation`] containing both the effective enabled
    /// value and whether the caller-provided default was returned because the
    /// flag is missing from storage.
    ///
    /// # Arguments
    /// * `flag_name` - The unique identifier for the feature flag
    /// * `default` - The value to return if the flag doesn't exist
    async fn evaluate(&self, flag_name: &str, default: bool) -> Result<FeatureFlagEvaluation>;

    /// Check if a feature flag is enabled.
    ///
    /// Returns the `default` value if the flag doesn't exist in the database.
    /// This allows graceful handling of new flags before they're added to the
    /// schema.
    ///
    /// # Arguments
    /// * `flag_name` - The unique identifier for the feature flag
    /// * `default` - The value to return if the flag doesn't exist
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pulsearc_core::FeatureFlagsPort;
    /// # async fn example(flags: &impl FeatureFlagsPort) {
    /// // Check if new blocks command is enabled (default to false)
    /// let enabled = flags.is_enabled("new_blocks_cmd", false).await.unwrap();
    /// if enabled {
    ///     // Use new block builder infrastructure
    /// } else {
    ///     // Fall back to legacy code
    /// }
    /// # }
    /// ```
    async fn is_enabled(&self, flag_name: &str, default: bool) -> Result<bool> {
        self.evaluate(flag_name, default).await.map(|evaluation| evaluation.enabled)
    }

    /// Set a feature flag's enabled status.
    ///
    /// Creates the flag if it doesn't exist (upsert semantics).
    /// Updates `updated_at` timestamp automatically.
    ///
    /// # Arguments
    /// * `flag_name` - The unique identifier for the feature flag
    /// * `enabled` - The new enabled status
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pulsearc_core::FeatureFlagsPort;
    /// # async fn example(flags: &impl FeatureFlagsPort) {
    /// // Disable new infrastructure for quick rollback
    /// flags.set_enabled("use_new_infra", false).await.unwrap();
    /// # }
    /// ```
    async fn set_enabled(&self, flag_name: &str, enabled: bool) -> Result<()>;

    /// List all feature flags ordered by name.
    ///
    /// Returns all flags currently in the database, including their current
    /// state and metadata. Useful for admin UI or debugging.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pulsearc_core::FeatureFlagsPort;
    /// # async fn example(flags: &impl FeatureFlagsPort) {
    /// let all_flags = flags.list_all().await.unwrap();
    /// for flag in all_flags {
    ///     println!("{}: {}", flag.flag_name, flag.enabled);
    /// }
    /// # }
    /// ```
    async fn list_all(&self) -> Result<Vec<FeatureFlag>>;
}
