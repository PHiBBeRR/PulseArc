//! Common manager patterns and utilities for the Tauri agent
//!
//! This module provides standardized manager infrastructure that can be used
//! across all modules in the application. It includes lifecycle management,
//! Arc<RwLock<T>> utilities, and async accessor patterns.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::time::timeout;

use crate::error::{CommonError, CommonResult};

/// Standard lifecycle trait for all manager types
///
/// This trait provides a common interface for manager initialization,
/// health checking, and graceful shutdown.
#[async_trait::async_trait]
pub trait AsyncManager: Send + Sync + 'static {
    /// Error type for this manager
    type Error: std::error::Error + Send + Sync + 'static;

    /// Configuration type for this manager
    type Config: Clone + Send + Sync + 'static;

    /// Create a new manager instance with default configuration
    async fn new() -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Create a new manager instance with custom configuration
    async fn with_config(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Initialize the manager and any required resources
    async fn initialize(&mut self) -> Result<(), Self::Error>;

    /// Check if the manager is healthy and operational
    async fn health_check(&self) -> Result<ManagerHealth, Self::Error>;

    /// Gracefully shutdown the manager and release resources
    async fn shutdown(&mut self) -> Result<(), Self::Error>;

    /// Get the manager's current status
    fn status(&self) -> ManagerStatus;

    /// Get manager metadata for monitoring/debugging
    fn metadata(&self) -> ManagerMetadata {
        ManagerMetadata {
            type_name: std::any::type_name::<Self>().to_string(),
            status: self.status(),
            health: None, // Async method, can't call here
        }
    }
}

/// Manager lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagerStatus {
    /// Manager has been created but not initialized
    Created,
    /// Manager is initializing
    Initializing,
    /// Manager is running and operational
    Running,
    /// Manager is shutting down
    ShuttingDown,
    /// Manager has been shut down
    Shutdown,
    /// Manager encountered an error
    Error,
}

impl std::fmt::Display for ManagerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::Initializing => write!(f, "Initializing"),
            Self::Running => write!(f, "Running"),
            Self::ShuttingDown => write!(f, "Shutting Down"),
            Self::Shutdown => write!(f, "Shutdown"),
            Self::Error => write!(f, "Error"),
        }
    }
}

/// Manager health status
#[derive(Debug, Clone)]
pub struct ManagerHealth {
    /// Overall health status
    pub is_healthy: bool,
    /// Health score from 0.0 (unhealthy) to 1.0 (perfectly healthy)
    pub score: f64,
    /// Optional health message
    pub message: Option<String>,
    /// Individual component health checks
    pub components: Vec<ComponentHealth>,
    /// Timestamp of health check
    pub timestamp: std::time::SystemTime,
}

impl ManagerHealth {
    /// Create a healthy status
    pub fn healthy() -> Self {
        Self {
            is_healthy: true,
            score: 1.0,
            message: None,
            components: Vec::new(),
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Create an unhealthy status with a message
    pub fn unhealthy<S: Into<String>>(message: S) -> Self {
        Self {
            is_healthy: false,
            score: 0.0,
            message: Some(message.into()),
            components: Vec::new(),
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Create a degraded status with a score
    pub fn degraded<S: Into<String>>(score: f64, message: S) -> Self {
        Self {
            is_healthy: score > 0.5,
            score: score.clamp(0.0, 1.0),
            message: Some(message.into()),
            components: Vec::new(),
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Add a component health check
    pub fn with_component(mut self, component: ComponentHealth) -> Self {
        self.components.push(component);
        self
    }
}

/// Individual component health within a manager
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub name: String,
    pub is_healthy: bool,
    pub message: Option<String>,
}

impl ComponentHealth {
    pub fn healthy<S: Into<String>>(name: S) -> Self {
        Self { name: name.into(), is_healthy: true, message: None }
    }

    pub fn unhealthy<S: Into<String>, M: Into<String>>(name: S, message: M) -> Self {
        Self { name: name.into(), is_healthy: false, message: Some(message.into()) }
    }
}

/// Manager metadata for monitoring
#[derive(Debug, Clone)]
pub struct ManagerMetadata {
    pub type_name: String,
    pub status: ManagerStatus,
    pub health: Option<ManagerHealth>,
}

/// Shared state container with async RwLock
///
/// This provides a standard pattern for managing shared state in managers
/// with helper methods for common operations.
#[derive(Debug)]
pub struct SharedState<T> {
    inner: Arc<RwLock<T>>,
    name: String,
}

impl<T> Clone for SharedState<T> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner), name: self.name.clone() }
    }
}

impl<T> SharedState<T> {
    /// Create a new shared state container
    pub fn new<S: Into<String>>(value: T, name: S) -> Self {
        Self { inner: Arc::new(RwLock::new(value)), name: name.into() }
    }

    /// Get a read lock on the state
    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, T> {
        self.inner.read().await
    }

    /// Get a write lock on the state
    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, T> {
        self.inner.write().await
    }

    /// Try to get a read lock without blocking
    pub fn try_read(
        &self,
    ) -> Result<tokio::sync::RwLockReadGuard<'_, T>, tokio::sync::TryLockError> {
        self.inner.try_read()
    }

    /// Try to get a write lock without blocking
    pub fn try_write(
        &self,
    ) -> Result<tokio::sync::RwLockWriteGuard<'_, T>, tokio::sync::TryLockError> {
        self.inner.try_write()
    }

    /// Get a read lock with timeout
    pub async fn read_timeout(
        &self,
        duration: Duration,
    ) -> CommonResult<tokio::sync::RwLockReadGuard<'_, T>> {
        timeout(duration, self.inner.read())
            .await
            .map_err(|_| CommonError::timeout(format!("read_lock_{}", self.name), duration))
    }

    /// Get a write lock with timeout
    pub async fn write_timeout(
        &self,
        duration: Duration,
    ) -> CommonResult<tokio::sync::RwLockWriteGuard<'_, T>> {
        timeout(duration, self.inner.write())
            .await
            .map_err(|_| CommonError::timeout(format!("write_lock_{}", self.name), duration))
    }

    /// Update the state using a closure
    pub async fn update<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.write().await;
        f(&mut *guard)
    }

    /// Update the state with timeout
    pub async fn update_timeout<F, R>(&self, duration: Duration, f: F) -> CommonResult<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.write_timeout(duration).await?;
        Ok(f(&mut *guard))
    }

    /// Read the state using a closure
    pub async fn read_with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.read().await;
        f(&*guard)
    }

    /// Read the state with timeout
    pub async fn read_with_timeout<F, R>(&self, duration: Duration, f: F) -> CommonResult<R>
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.read_timeout(duration).await?;
        Ok(f(&*guard))
    }

    /// Replace the entire state
    pub async fn replace(&self, new_value: T) -> T {
        let mut guard = self.write().await;
        std::mem::replace(&mut *guard, new_value)
    }

    /// Get the state name for debugging
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<T: Clone> SharedState<T> {
    /// Get a clone of the current state
    pub async fn get(&self) -> T {
        self.read().await.clone()
    }

    /// Get a clone of the current state with timeout
    pub async fn get_timeout(&self, duration: Duration) -> CommonResult<T> {
        let guard = self.read_timeout(duration).await?;
        Ok(guard.clone())
    }
}

impl<T: Default> Default for SharedState<T> {
    fn default() -> Self {
        Self::new(T::default(), "default")
    }
}

/// Manager lifecycle controller
///
/// Helps manage the lifecycle of multiple managers with proper startup/shutdown
/// ordering.
pub struct ManagerController {
    managers: Vec<Box<dyn ManagerLifecycle>>,
    status: SharedState<ManagerStatus>,
}

/// Simplified lifecycle trait for the controller
#[async_trait::async_trait]
pub trait ManagerLifecycle: Send + Sync {
    async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn name(&self) -> &str;
    fn status(&self) -> ManagerStatus;
}

impl ManagerController {
    /// Create a new manager controller
    pub fn new() -> Self {
        Self {
            managers: Vec::new(),
            status: SharedState::new(ManagerStatus::Created, "controller"),
        }
    }

    /// Add a manager to the controller
    pub fn add_manager<M: ManagerLifecycle + 'static>(&mut self, manager: M) {
        self.managers.push(Box::new(manager));
    }

    /// Initialize all managers in order
    pub async fn initialize_all(&mut self) -> CommonResult<()> {
        self.status.replace(ManagerStatus::Initializing).await;

        for (i, manager) in self.managers.iter_mut().enumerate() {
            tracing::info!("Initializing manager {}: {}", i, manager.name());

            if let Err(e) = manager.initialize().await {
                tracing::error!("Failed to initialize manager {}: {}", manager.name(), e);
                self.status.replace(ManagerStatus::Error).await;
                return Err(CommonError::internal_with_context(
                    e.to_string(),
                    format!("manager_init_{}", manager.name()),
                ));
            }
        }

        self.status.replace(ManagerStatus::Running).await;
        Ok(())
    }

    /// Shutdown all managers in reverse order
    pub async fn shutdown_all(&mut self) -> CommonResult<()> {
        self.status.replace(ManagerStatus::ShuttingDown).await;

        // Shutdown in reverse order
        for (i, manager) in self.managers.iter_mut().enumerate().rev() {
            tracing::info!("Shutting down manager {}: {}", i, manager.name());

            if let Err(e) = manager.shutdown().await {
                tracing::error!("Failed to shutdown manager {}: {}", manager.name(), e);
                // Continue shutting down other managers even if one fails
            }
        }

        self.status.replace(ManagerStatus::Shutdown).await;
        Ok(())
    }

    /// Get controller status
    pub async fn status(&self) -> ManagerStatus {
        self.status.get().await
    }

    /// Get status of all managers
    pub fn manager_statuses(&self) -> Vec<(String, ManagerStatus)> {
        self.managers.iter().map(|m| (m.name().to_string(), m.status())).collect()
    }
}

impl Default for ManagerController {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility macros for common manager patterns
#[macro_export]
macro_rules! impl_manager_lifecycle {
    ($manager_type:ty, $error_type:ty) => {
        #[async_trait::async_trait]
        impl $crate::manager::ManagerLifecycle for $manager_type {
            async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                <Self as $crate::manager::AsyncManager>::initialize(self)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }

            async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                <Self as $crate::manager::AsyncManager>::shutdown(self)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            }

            fn name(&self) -> &str {
                std::any::type_name::<Self>()
            }

            fn status(&self) -> $crate::manager::ManagerStatus {
                <Self as $crate::manager::AsyncManager>::status(self)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    //! Unit tests for manager patterns and lifecycle utilities
    //!
    //! Tests cover SharedState operations, ManagerController lifecycle,
    //! manager health checks, and concurrent access patterns.

    use super::*;

    /// Validates `SharedState::new` behavior for the shared state creation
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `state.name()` equals `"test_counter"`.
    /// - Confirms `*state.read().await` equals `42`.
    #[tokio::test]
    async fn test_shared_state_creation() {
        let state = SharedState::new(42, "test_counter");
        assert_eq!(state.name(), "test_counter");
        assert_eq!(*state.read().await, 42);
    }

    /// Validates `SharedState::new` behavior for the shared state read write
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `*read_guard` equals `20`.
    #[tokio::test]
    async fn test_shared_state_read_write() {
        let state = SharedState::new(10, "counter");

        {
            let mut write_guard = state.write().await;
            *write_guard = 20;
        }

        let read_guard = state.read().await;
        assert_eq!(*read_guard, 20);
    }

    /// Validates `SharedState::new` behavior for the shared state try read
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `guard.is_ok()` evaluates to true.
    /// - Confirms `*guard.unwrap()` equals `5`.
    #[tokio::test]
    async fn test_shared_state_try_read() {
        let state = SharedState::new(5, "test");

        let guard = state.try_read();
        assert!(guard.is_ok());
        assert_eq!(*guard.unwrap(), 5);
    }

    /// Validates `SharedState::new` behavior for the shared state try write
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `guard.is_ok()` evaluates to true.
    #[tokio::test]
    async fn test_shared_state_try_write() {
        let state = SharedState::new(5, "test");

        let guard = state.try_write();
        assert!(guard.is_ok());
    }

    /// Tests that read operations complete successfully within timeout
    #[tokio::test]
    async fn test_shared_state_read_timeout() {
        let state = SharedState::new(100, "test");
        let timeout = Duration::from_millis(100);

        let result = state.read_timeout(timeout).await;
        assert!(result.is_ok(), "Read should complete within timeout");
        let guard = result.expect("Read lock should be acquired");
        assert_eq!(*guard, 100);
    }

    /// Tests that write operations complete successfully within timeout
    #[tokio::test]
    async fn test_shared_state_write_timeout() {
        let state = SharedState::new(100, "test");
        let timeout = Duration::from_millis(100);

        let result = state.write_timeout(timeout).await;
        assert!(result.is_ok(), "Write should complete within timeout");
    }

    /// Validates `SharedState::new` behavior for the shared state update
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `result` equals `15`.
    /// - Confirms `*state.read().await` equals `15`.
    #[tokio::test]
    async fn test_shared_state_update() {
        let state = SharedState::new(10, "counter");

        let result = state
            .update(|value| {
                *value += 5;
                *value
            })
            .await;

        assert_eq!(result, 15);
        assert_eq!(*state.read().await, 15);
    }

    /// Validates `SharedState::new` behavior for the shared state update
    /// timeout scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `result.unwrap()` equals `20`.
    #[tokio::test]
    async fn test_shared_state_update_timeout() {
        let state = SharedState::new(10, "counter");
        let timeout = Duration::from_millis(100);

        let result = state
            .update_timeout(timeout, |value| {
                *value *= 2;
                *value
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 20);
    }

    /// Validates `SharedState::new` behavior for the shared state read with
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `length` equals `5`.
    #[tokio::test]
    async fn test_shared_state_read_with() {
        let state = SharedState::new(String::from("hello"), "text");

        let length = state.read_with(|s| s.len()).await;
        assert_eq!(length, 5);
    }

    /// Validates `SharedState::new` behavior for the shared state read with
    /// timeout scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `result.unwrap()` equals `"TEST"`.
    #[tokio::test]
    async fn test_shared_state_read_with_timeout() {
        let state = SharedState::new(String::from("test"), "text");
        let timeout = Duration::from_millis(100);

        let result = state.read_with_timeout(timeout, |s| s.to_uppercase()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "TEST");
    }

    /// Validates `SharedState::new` behavior for the shared state replace
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `old_value` equals `10`.
    /// - Confirms `*state.read().await` equals `20`.
    #[tokio::test]
    async fn test_shared_state_replace() {
        let state = SharedState::new(10, "value");

        let old_value = state.replace(20).await;
        assert_eq!(old_value, 10);
        assert_eq!(*state.read().await, 20);
    }

    /// Validates `SharedState::new` behavior for the shared state get clone
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cloned` equals `vec![1, 2, 3]`.
    #[tokio::test]
    async fn test_shared_state_get_clone() {
        let state = SharedState::new(vec![1, 2, 3], "vector");

        let cloned = state.get().await;
        assert_eq!(cloned, vec![1, 2, 3]);
    }

    /// Validates `SharedState::new` behavior for the shared state get timeout
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `result.unwrap()` equals `42`.
    #[tokio::test]
    async fn test_shared_state_get_timeout() {
        let state = SharedState::new(42, "number");
        let timeout = Duration::from_millis(100);

        let result = state.get_timeout(timeout).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    /// Validates `SharedState::default` behavior for the shared state default
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `*state.read().await` equals `0`.
    #[tokio::test]
    async fn test_shared_state_default() {
        let state: SharedState<i32> = SharedState::default();
        assert_eq!(*state.read().await, 0);
    }

    /// Validates `ManagerStatus::Created` behavior for the manager status
    /// display scenario.
    ///
    /// Assertions:
    /// - Confirms `ManagerStatus::Created.to_string()` equals `"Created"`.
    /// - Confirms `ManagerStatus::Initializing.to_string()` equals
    ///   `"Initializing"`.
    /// - Confirms `ManagerStatus::Running.to_string()` equals `"Running"`.
    /// - Confirms `ManagerStatus::ShuttingDown.to_string()` equals `"Shutting
    ///   Down"`.
    /// - Confirms `ManagerStatus::Shutdown.to_string()` equals `"Shutdown"`.
    /// - Confirms `ManagerStatus::Error.to_string()` equals `"Error"`.
    #[test]
    fn test_manager_status_display() {
        assert_eq!(ManagerStatus::Created.to_string(), "Created");
        assert_eq!(ManagerStatus::Initializing.to_string(), "Initializing");
        assert_eq!(ManagerStatus::Running.to_string(), "Running");
        assert_eq!(ManagerStatus::ShuttingDown.to_string(), "Shutting Down");
        assert_eq!(ManagerStatus::Shutdown.to_string(), "Shutdown");
        assert_eq!(ManagerStatus::Error.to_string(), "Error");
    }

    /// Validates `ManagerStatus::Created` behavior for the manager status
    /// equality scenario.
    ///
    /// Assertions:
    /// - Confirms `ManagerStatus::Created` equals `ManagerStatus::Created`.
    /// - Confirms `ManagerStatus::Running` differs from
    ///   `ManagerStatus::Shutdown`.
    #[test]
    fn test_manager_status_equality() {
        assert_eq!(ManagerStatus::Created, ManagerStatus::Created);
        assert_ne!(ManagerStatus::Running, ManagerStatus::Shutdown);
    }

    /// Validates `ManagerHealth::healthy` behavior for the manager health
    /// healthy scenario.
    ///
    /// Assertions:
    /// - Ensures `health.is_healthy` evaluates to true.
    /// - Confirms `health.score` equals `1.0`.
    /// - Confirms `health.message` equals `None`.
    /// - Ensures `health.components.is_empty()` evaluates to true.
    #[test]
    fn test_manager_health_healthy() {
        let health = ManagerHealth::healthy();
        assert!(health.is_healthy);
        assert_eq!(health.score, 1.0);
        assert_eq!(health.message, None);
        assert!(health.components.is_empty());
    }

    /// Validates `ManagerHealth::unhealthy` behavior for the manager health
    /// unhealthy scenario.
    ///
    /// Assertions:
    /// - Ensures `!health.is_healthy` evaluates to true.
    /// - Confirms `health.score` equals `0.0`.
    /// - Confirms `health.message` equals `Some("service
    ///   unavailable".to_string())`.
    #[test]
    fn test_manager_health_unhealthy() {
        let health = ManagerHealth::unhealthy("service unavailable");
        assert!(!health.is_healthy);
        assert_eq!(health.score, 0.0);
        assert_eq!(health.message, Some("service unavailable".to_string()));
    }

    /// Validates `ManagerHealth::degraded` behavior for the manager health
    /// degraded scenario.
    ///
    /// Assertions:
    /// - Ensures `health.is_healthy` evaluates to true.
    /// - Confirms `health.score` equals `0.7`.
    /// - Confirms `health.message` equals `Some("partial
    ///   service".to_string())`.
    /// - Ensures `!unhealthy.is_healthy` evaluates to true.
    #[test]
    fn test_manager_health_degraded() {
        let health = ManagerHealth::degraded(0.7, "partial service");
        assert!(health.is_healthy); // > 0.5
        assert_eq!(health.score, 0.7);
        assert_eq!(health.message, Some("partial service".to_string()));

        let unhealthy = ManagerHealth::degraded(0.3, "mostly down");
        assert!(!unhealthy.is_healthy); // <= 0.5
    }

    /// Validates `ComponentHealth::healthy` behavior for the manager health
    /// with component scenario.
    ///
    /// Assertions:
    /// - Confirms `health.components.len()` equals `1`.
    /// - Confirms `health.components[0].name` equals `"database"`.
    /// - Ensures `health.components[0].is_healthy` evaluates to true.
    #[test]
    fn test_manager_health_with_component() {
        let component = ComponentHealth::healthy("database");
        let health = ManagerHealth::healthy().with_component(component);

        assert_eq!(health.components.len(), 1);
        assert_eq!(health.components[0].name, "database");
        assert!(health.components[0].is_healthy);
    }

    /// Validates `ComponentHealth::healthy` behavior for the component health
    /// healthy scenario.
    ///
    /// Assertions:
    /// - Confirms `component.name` equals `"cache"`.
    /// - Ensures `component.is_healthy` evaluates to true.
    /// - Confirms `component.message` equals `None`.
    #[test]
    fn test_component_health_healthy() {
        let component = ComponentHealth::healthy("cache");
        assert_eq!(component.name, "cache");
        assert!(component.is_healthy);
        assert_eq!(component.message, None);
    }

    /// Validates `ComponentHealth::unhealthy` behavior for the component health
    /// unhealthy scenario.
    ///
    /// Assertions:
    /// - Confirms `component.name` equals `"storage"`.
    /// - Ensures `!component.is_healthy` evaluates to true.
    /// - Confirms `component.message` equals `Some("disk full".to_string())`.
    #[test]
    fn test_component_health_unhealthy() {
        let component = ComponentHealth::unhealthy("storage", "disk full");
        assert_eq!(component.name, "storage");
        assert!(!component.is_healthy);
        assert_eq!(component.message, Some("disk full".to_string()));
    }

    /// Validates `ManagerController::new` behavior for the manager controller
    /// creation scenario.
    ///
    /// Assertions:
    /// - Confirms `controller.status().await` equals `ManagerStatus::Created`.
    #[tokio::test]
    async fn test_manager_controller_creation() {
        let controller = ManagerController::new();
        assert_eq!(controller.status().await, ManagerStatus::Created);
    }

    /// Validates `ManagerController::default` behavior for the manager
    /// controller default scenario.
    ///
    /// Assertions:
    /// - Confirms `controller.status().await` equals `ManagerStatus::Created`.
    #[tokio::test]
    async fn test_manager_controller_default() {
        let controller = ManagerController::default();
        assert_eq!(controller.status().await, ManagerStatus::Created);
    }

    // Mock manager for testing
    struct MockManager {
        status: ManagerStatus,
        init_count: Arc<std::sync::Mutex<u32>>,
        shutdown_count: Arc<std::sync::Mutex<u32>>,
    }

    #[async_trait::async_trait]
    impl ManagerLifecycle for MockManager {
        async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let mut count = self.init_count.lock().unwrap();
            *count += 1;
            self.status = ManagerStatus::Running;
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let mut count = self.shutdown_count.lock().unwrap();
            *count += 1;
            self.status = ManagerStatus::Shutdown;
            Ok(())
        }

        fn name(&self) -> &str {
            "MockManager"
        }

        fn status(&self) -> ManagerStatus {
            self.status
        }
    }

    /// Validates `ManagerController::new` behavior for the manager controller
    /// add manager scenario.
    ///
    /// Assertions:
    /// - Confirms `statuses.len()` equals `1`.
    /// - Confirms `statuses[0].0` equals `"MockManager"`.
    #[tokio::test]
    async fn test_manager_controller_add_manager() {
        let mut controller = ManagerController::new();
        let init_count = Arc::new(std::sync::Mutex::new(0));
        let shutdown_count = Arc::new(std::sync::Mutex::new(0));

        let manager = MockManager {
            status: ManagerStatus::Created,
            init_count: Arc::clone(&init_count),
            shutdown_count: Arc::clone(&shutdown_count),
        };

        controller.add_manager(manager);
        let statuses = controller.manager_statuses();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].0, "MockManager");
    }

    /// Validates `ManagerController::new` behavior for the manager controller
    /// initialize all scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `controller.status().await` equals `ManagerStatus::Running`.
    /// - Confirms `*init_count.lock().unwrap()` equals `1`.
    #[tokio::test]
    async fn test_manager_controller_initialize_all() {
        let mut controller = ManagerController::new();
        let init_count = Arc::new(std::sync::Mutex::new(0));
        let shutdown_count = Arc::new(std::sync::Mutex::new(0));

        let manager = MockManager {
            status: ManagerStatus::Created,
            init_count: Arc::clone(&init_count),
            shutdown_count: Arc::clone(&shutdown_count),
        };

        controller.add_manager(manager);
        let result = controller.initialize_all().await;

        assert!(result.is_ok());
        assert_eq!(controller.status().await, ManagerStatus::Running);
        assert_eq!(*init_count.lock().unwrap(), 1);
    }

    /// Validates `ManagerController::new` behavior for the manager controller
    /// shutdown all scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Confirms `controller.status().await` equals `ManagerStatus::Shutdown`.
    /// - Confirms `*shutdown_count.lock().unwrap()` equals `1`.
    #[tokio::test]
    async fn test_manager_controller_shutdown_all() {
        let mut controller = ManagerController::new();
        let init_count = Arc::new(std::sync::Mutex::new(0));
        let shutdown_count = Arc::new(std::sync::Mutex::new(0));

        let manager = MockManager {
            status: ManagerStatus::Running,
            init_count: Arc::clone(&init_count),
            shutdown_count: Arc::clone(&shutdown_count),
        };

        controller.add_manager(manager);
        let result = controller.shutdown_all().await;

        assert!(result.is_ok());
        assert_eq!(controller.status().await, ManagerStatus::Shutdown);
        assert_eq!(*shutdown_count.lock().unwrap(), 1);
    }

    /// Validates `ManagerController::new` behavior for the manager controller
    /// multiple managers scenario.
    ///
    /// Assertions:
    /// - Confirms `statuses.len()` equals `3`.
    #[tokio::test]
    async fn test_manager_controller_multiple_managers() {
        let mut controller = ManagerController::new();

        for _ in 0..3 {
            let manager = MockManager {
                status: ManagerStatus::Created,
                init_count: Arc::new(std::sync::Mutex::new(0)),
                shutdown_count: Arc::new(std::sync::Mutex::new(0)),
            };
            controller.add_manager(manager);
        }

        let statuses = controller.manager_statuses();
        assert_eq!(statuses.len(), 3);
    }
}
