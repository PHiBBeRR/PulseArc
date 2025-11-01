//! Health check infrastructure for AppContext components
//!
//! Provides HealthStatus and ComponentHealth types for monitoring application health.
//! Pattern adapted from pulsearc-platform's ManagerHealth infrastructure.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Overall health status of the application
///
/// # Example
/// ```no_run
/// use pulsearc_app::utils::health::{HealthStatus, ComponentHealth};
///
/// let mut status = HealthStatus::new();
/// status = status.add_component(ComponentHealth::healthy("database"));
/// status = status.add_component(ComponentHealth::unhealthy("cache", "connection timeout"));
/// status.calculate_score();
///
/// assert_eq!(status.score, 0.5);  // 1 out of 2 components healthy
/// assert!(!status.is_healthy);     // Below 0.8 threshold
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall health indicator
    pub is_healthy: bool,

    /// Health score from 0.0 (completely unhealthy) to 1.0 (fully healthy)
    ///
    /// Calculated as: (healthy_components / total_components)
    pub score: f64,

    /// Optional message describing overall health state
    pub message: Option<String>,

    /// Individual component health checks
    pub components: Vec<ComponentHealth>,

    /// Unix timestamp when health check was performed
    pub timestamp: i64,
}

impl HealthStatus {
    /// Create a new health status with default values
    ///
    /// Initial state: healthy with score 1.0, no components
    pub fn new() -> Self {
        Self {
            is_healthy: true,
            score: 1.0,
            message: None,
            components: Vec::new(),
            timestamp: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
                as i64,
        }
    }

    /// Add a component health check to the status
    ///
    /// Returns self for method chaining
    pub fn add_component(mut self, component: ComponentHealth) -> Self {
        self.components.push(component);
        self
    }

    /// Calculate overall health score based on component health
    ///
    /// Score = (healthy_components / total_components)
    /// is_healthy = (score >= 0.8)  // 80% threshold
    ///
    /// Should be called after all components have been added.
    pub fn calculate_score(&mut self) {
        if self.components.is_empty() {
            return;
        }

        let healthy_count = self.components.iter().filter(|c| c.is_healthy).count();

        self.score = healthy_count as f64 / self.components.len() as f64;
        self.is_healthy = self.score >= 0.8; // 80% threshold
    }

    /// Create an unhealthy status with a message
    ///
    /// Convenience constructor for error cases
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            is_healthy: false,
            score: 0.0,
            message: Some(message.into()),
            components: Vec::new(),
            timestamp: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
                as i64,
        }
    }
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::new()
    }
}

/// Health status of an individual component
///
/// # Example
/// ```no_run
/// use pulsearc_app::utils::health::ComponentHealth;
///
/// let db = ComponentHealth::healthy("database");
/// assert!(db.is_healthy);
///
/// let cache = ComponentHealth::unhealthy("cache", "connection refused");
/// assert!(!cache.is_healthy);
/// assert_eq!(cache.message, Some("connection refused".to_string()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component identifier (e.g., "database", "feature_flags")
    pub name: String,

    /// Whether the component is healthy
    pub is_healthy: bool,

    /// Optional message describing health state or error
    pub message: Option<String>,
}

impl ComponentHealth {
    /// Create a healthy component status
    pub fn healthy(name: impl Into<String>) -> Self {
        Self { name: name.into(), is_healthy: true, message: None }
    }

    /// Create an unhealthy component status with a message
    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self { name: name.into(), is_healthy: false, message: Some(message.into()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_new() {
        let status = HealthStatus::new();
        assert!(status.is_healthy);
        assert_eq!(status.score, 1.0);
        assert!(status.message.is_none());
        assert!(status.components.is_empty());
    }

    #[test]
    fn test_health_status_add_component() {
        let status = HealthStatus::new()
            .add_component(ComponentHealth::healthy("db"))
            .add_component(ComponentHealth::healthy("cache"));

        assert_eq!(status.components.len(), 2);
        assert_eq!(status.components[0].name, "db");
        assert_eq!(status.components[1].name, "cache");
    }

    #[test]
    fn test_calculate_score_all_healthy() {
        let mut status = HealthStatus::new()
            .add_component(ComponentHealth::healthy("db"))
            .add_component(ComponentHealth::healthy("cache"));

        status.calculate_score();

        assert_eq!(status.score, 1.0);
        assert!(status.is_healthy);
    }

    #[test]
    fn test_calculate_score_half_healthy() {
        let mut status = HealthStatus::new()
            .add_component(ComponentHealth::healthy("db"))
            .add_component(ComponentHealth::unhealthy("cache", "error"));

        status.calculate_score();

        assert_eq!(status.score, 0.5);
        assert!(!status.is_healthy); // Below 0.8 threshold
    }

    #[test]
    fn test_calculate_score_threshold() {
        let mut status = HealthStatus::new()
            .add_component(ComponentHealth::healthy("db"))
            .add_component(ComponentHealth::healthy("cache"))
            .add_component(ComponentHealth::healthy("feature_flags"))
            .add_component(ComponentHealth::healthy("tracking"))
            .add_component(ComponentHealth::unhealthy("sync", "error"));

        status.calculate_score();

        assert_eq!(status.score, 0.8); // 4/5 = 0.8
        assert!(status.is_healthy); // Exactly at threshold
    }

    #[test]
    fn test_component_health_constructors() {
        let healthy = ComponentHealth::healthy("test");
        assert!(healthy.is_healthy);
        assert_eq!(healthy.name, "test");
        assert!(healthy.message.is_none());

        let unhealthy = ComponentHealth::unhealthy("test", "failed");
        assert!(!unhealthy.is_healthy);
        assert_eq!(unhealthy.name, "test");
        assert_eq!(unhealthy.message, Some("failed".to_string()));
    }
}
