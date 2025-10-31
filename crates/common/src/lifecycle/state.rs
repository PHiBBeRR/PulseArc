// Helper utilities for Arc<RwLock<T>> patterns and async state management
// Provides macros and utilities to simplify common async access patterns

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Type alias for shared state pattern
pub type SharedState<T> = Arc<RwLock<T>>;

/// Create a new shared state instance
pub fn shared_state<T>(value: T) -> SharedState<T> {
    Arc::new(RwLock::new(value))
}

/// Macro for reading from shared state with timeout
#[macro_export]
macro_rules! read_state {
    ($state:expr) => {
        $state.read().await
    };
    ($state:expr, timeout: $duration:expr) => {
        tokio::time::timeout($duration, $state.read())
            .await
            .map_err(|_| $crate::error::CommonError::Timeout)?
    };
}

/// Macro for writing to shared state with timeout
#[macro_export]
macro_rules! write_state {
    ($state:expr) => {
        $state.write().await
    };
    ($state:expr, timeout: $duration:expr) => {
        tokio::time::timeout($duration, $state.write())
            .await
            .map_err(|_| $crate::error::CommonError::Timeout)?
    };
}

/// Macro for safely updating state with a closure
#[macro_export]
macro_rules! update_state {
    ($state:expr, $updater:expr) => {{
        let mut guard = $state.write().await;
        $updater(&mut *guard)
    }};
    ($state:expr, $updater:expr, timeout: $duration:expr) => {{
        let mut guard = tokio::time::timeout($duration, $state.write())
            .await
            .map_err(|_| $crate::error::CommonError::Timeout)?;
        $updater(&mut *guard)
    }};
}

/// Macro for conditionally updating state
#[macro_export]
macro_rules! update_state_if {
    ($state:expr, $condition:expr, $updater:expr) => {{
        let mut guard = $state.write().await;
        if $condition(&*guard) {
            $updater(&mut *guard);
            true
        } else {
            false
        }
    }};
}

/// Macro for reading state and transforming the result
#[macro_export]
macro_rules! read_state_map {
    ($state:expr, $mapper:expr) => {{
        let guard = $state.read().await;
        $mapper(&*guard)
    }};
}

/// Macro for cloning data from state (useful for avoiding lifetime issues)
#[macro_export]
macro_rules! clone_from_state {
    ($state:expr, $field:ident) => {{
        let guard = $state.read().await;
        guard.$field.clone()
    }};
    ($state:expr, $mapper:expr) => {{
        let guard = $state.read().await;
        $mapper(&*guard).clone()
    }};
}

/// Type alias for a type-erased, thread-safe state value
type StateValue = Box<dyn std::any::Any + Send + Sync>;

/// State registry for storing multiple shared states by key
///
/// This is a simple container for type-erased shared states. For lifecycle
/// management of components with initialization/shutdown logic, use the
/// `AsyncManager` trait from the `manager` module instead.
#[derive(Debug)]
pub struct StateRegistry {
    states: std::collections::HashMap<String, StateValue>,
}

impl Default for StateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl StateRegistry {
    pub fn new() -> Self {
        Self { states: std::collections::HashMap::new() }
    }

    /// Register a shared state with a key
    pub fn register<T: Send + Sync + 'static>(&mut self, key: String, state: SharedState<T>) {
        self.states.insert(key, Box::new(state));
    }

    /// Get a shared state by key
    pub fn get<T: Send + Sync + 'static>(&self, key: &str) -> Option<&SharedState<T>> {
        self.states.get(key)?.downcast_ref()
    }
}

/// Async-safe state container with built-in lifecycle management
#[derive(Debug, Clone)]
pub struct ManagedState<T> {
    state: SharedState<T>,
    created_at: std::time::Instant,
    last_accessed: SharedState<std::time::Instant>,
}

impl<T> ManagedState<T> {
    pub fn new(value: T) -> Self {
        let now = std::time::Instant::now();
        Self { state: shared_state(value), created_at: now, last_accessed: shared_state(now) }
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, T> {
        self.update_access_time().await;
        self.state.read().await
    }

    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, T> {
        self.update_access_time().await;
        self.state.write().await
    }

    pub async fn try_read(&self) -> Option<tokio::sync::RwLockReadGuard<'_, T>> {
        self.update_access_time().await;
        self.state.try_read().ok()
    }

    pub async fn try_write(&self) -> Option<tokio::sync::RwLockWriteGuard<'_, T>> {
        self.update_access_time().await;
        self.state.try_write().ok()
    }

    async fn update_access_time(&self) {
        if let Ok(mut guard) = self.last_accessed.try_write() {
            *guard = std::time::Instant::now();
        }
    }

    pub fn created_at(&self) -> std::time::Instant {
        self.created_at
    }

    pub async fn last_accessed(&self) -> std::time::Instant {
        *self.last_accessed.read().await
    }

    pub async fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    pub async fn idle_time(&self) -> Duration {
        self.last_accessed().await.elapsed()
    }

    /// Get a clone of the underlying Arc for sharing across threads
    pub fn get_shared(&self) -> SharedState<T> {
        Arc::clone(&self.state)
    }

    /// Set a new value to the state
    pub async fn set(&mut self, value: T) {
        self.update_access_time().await;
        let mut guard = self.state.write().await;
        *guard = value;
    }

    /// Modify the state using a closure
    pub async fn modify<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        self.update_access_time().await;
        let mut guard = self.state.write().await;
        f(&mut *guard);
    }
}

impl<T: Clone> ManagedState<T> {
    /// Clone the current state value (useful for avoiding lifetime issues)
    pub async fn clone_value(&self) -> T {
        self.read().await.clone()
    }
}

/// Atomic counter with shared state
#[derive(Debug, Default)]
pub struct AtomicCounter {
    value: SharedState<u64>,
}

impl AtomicCounter {
    pub fn new(initial: u64) -> Self {
        Self { value: shared_state(initial) }
    }

    pub async fn increment(&self) -> u64 {
        let mut guard = self.value.write().await;
        *guard += 1;
        *guard
    }

    pub async fn decrement(&self) -> u64 {
        let mut guard = self.value.write().await;
        *guard = guard.saturating_sub(1);
        *guard
    }

    pub async fn get(&self) -> u64 {
        *self.value.read().await
    }

    pub async fn set(&self, value: u64) {
        *self.value.write().await = value;
    }

    pub async fn add(&self, delta: u64) -> u64 {
        let mut guard = self.value.write().await;
        *guard += delta;
        *guard
    }
}

/// Configuration for state access patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub max_concurrent_readers: Option<usize>,
    pub enable_metrics: bool,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            read_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(10),
            max_concurrent_readers: None,
            enable_metrics: false,
        }
    }
}

/// Utility for safely sharing data across async contexts without lifetime
/// issues
#[derive(Debug, Clone)]
pub struct SafeShare<T> {
    data: Arc<T>,
}

impl<T> SafeShare<T> {
    pub fn new(data: T) -> Self {
        Self { data: Arc::new(data) }
    }

    pub fn get(&self) -> &T {
        &self.data
    }

    pub fn clone_arc(&self) -> Arc<T> {
        Arc::clone(&self.data)
    }
}

impl<T: Clone> SafeShare<T> {
    pub fn clone_value(&self) -> T {
        (*self.data).clone()
    }
}

/// Builder pattern for creating complex shared state structures
pub struct StateBuilder<T> {
    initial_value: Option<T>,
    config: StateConfig,
}

impl<T> Default for StateBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> StateBuilder<T> {
    pub fn new() -> Self {
        Self { initial_value: None, config: StateConfig::default() }
    }

    pub fn with_initial_value(mut self, value: T) -> Self {
        self.initial_value = Some(value);
        self
    }

    pub fn with_config(mut self, config: StateConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.config.read_timeout = timeout;
        self
    }

    pub fn with_write_timeout(mut self, timeout: Duration) -> Self {
        self.config.write_timeout = timeout;
        self
    }

    pub fn build(self) -> Option<ManagedState<T>> {
        self.initial_value.map(ManagedState::new)
    }
}

impl<T: Default> StateBuilder<T> {
    pub fn build_with_default(self) -> ManagedState<T> {
        ManagedState::new(self.initial_value.unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for state management utilities
    //!
    //! Tests cover shared state patterns, managed state lifecycle tracking,
    //! atomic counters, state builders, and concurrent access patterns.

    use super::*;

    /// Validates the shared state creation scenario.
    ///
    /// Assertions:
    /// - Confirms `*state.read().await` equals `42`.
    #[tokio::test]
    async fn test_shared_state_creation() {
        let state = shared_state(42);
        assert_eq!(*state.read().await, 42);
    }

    /// Validates the shared state read write scenario.
    ///
    /// Assertions:
    /// - Confirms `*read_guard` equals `20`.
    #[tokio::test]
    async fn test_shared_state_read_write() {
        let state = shared_state(10);

        {
            let mut write_guard = state.write().await;
            *write_guard = 20;
        }

        let read_guard = state.read().await;
        assert_eq!(*read_guard, 20);
    }

    /// Validates `ManagedState::new` behavior for the managed state creation
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `*state.read().await` equals `100`.
    #[tokio::test]
    async fn test_managed_state_creation() {
        let state = ManagedState::new(100);
        assert_eq!(*state.read().await, 100);
    }

    /// Tests that ManagedState tracks creation time
    #[tokio::test]
    async fn test_managed_state_tracks_created_at() {
        let state = ManagedState::new(42);
        let created = state.created_at();

        tokio::time::sleep(Duration::from_millis(10)).await;

        assert!(
            created.elapsed() >= Duration::from_millis(10),
            "Creation time should be at least 10ms ago"
        );
    }

    /// Tests that ManagedState tracks last access time
    #[tokio::test]
    async fn test_managed_state_tracks_last_accessed() {
        let state = ManagedState::new(42);

        tokio::time::sleep(Duration::from_millis(10)).await;

        // Access the state
        let _ = state.read().await;
        let last_accessed = state.last_accessed().await;

        // Should be recent
        assert!(
            last_accessed.elapsed() < Duration::from_millis(100),
            "Last access should be very recent after read"
        );
    }

    /// Validates `ManagedState::new` behavior for the managed state read
    /// updates access time scenario.
    ///
    /// Assertions:
    /// - Ensures `idle_time < Duration::from_millis(50)` evaluates to true.
    #[tokio::test]
    async fn test_managed_state_read_updates_access_time() {
        let state = ManagedState::new(42);

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Read should update access time
        let _ = state.read().await;

        let idle_time = state.idle_time().await;
        assert!(idle_time < Duration::from_millis(50));
    }

    /// Validates `ManagedState::new` behavior for the managed state write
    /// updates access time scenario.
    ///
    /// Assertions:
    /// - Ensures `idle_time < Duration::from_millis(50)` evaluates to true.
    #[tokio::test]
    async fn test_managed_state_write_updates_access_time() {
        let state = ManagedState::new(42);

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Write should update access time
        let mut guard = state.write().await;
        *guard = 100;
        drop(guard);

        let idle_time = state.idle_time().await;
        assert!(idle_time < Duration::from_millis(50));
    }

    /// Validates `ManagedState::new` behavior for the managed state try read
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `guard.is_some()` evaluates to true.
    /// - Confirms `*guard.unwrap()` equals `42`.
    #[tokio::test]
    async fn test_managed_state_try_read() {
        let state = ManagedState::new(42);

        let guard = state.try_read().await;
        assert!(guard.is_some());
        assert_eq!(*guard.unwrap(), 42);
    }

    /// Validates `ManagedState::new` behavior for the managed state try write
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `guard.is_some()` evaluates to true.
    #[tokio::test]
    async fn test_managed_state_try_write() {
        let state = ManagedState::new(42);

        let guard = state.try_write().await;
        assert!(guard.is_some());
    }

    /// Validates `ManagedState::new` behavior for the managed state age
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `age >= Duration::from_millis(10)` evaluates to true.
    #[tokio::test]
    async fn test_managed_state_age() {
        let state = ManagedState::new(42);

        tokio::time::sleep(Duration::from_millis(10)).await;

        let age = state.age().await;
        assert!(age >= Duration::from_millis(10));
    }

    /// Validates `ManagedState::new` behavior for the managed state clone value
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cloned` equals `vec![1, 2, 3]`.
    #[tokio::test]
    async fn test_managed_state_clone_value() {
        let state = ManagedState::new(vec![1, 2, 3]);

        let cloned = state.clone_value().await;
        assert_eq!(cloned, vec![1, 2, 3]);
    }

    /// Validates `ManagedState::new` behavior for the managed state get shared
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `*shared.read().await` equals `42`.
    #[tokio::test]
    async fn test_managed_state_get_shared() {
        let state = ManagedState::new(42);
        let shared = state.get_shared();

        assert_eq!(*shared.read().await, 42);
    }

    /// Validates `AtomicCounter::new` behavior for the atomic counter creation
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `counter.get().await` equals `10`.
    #[tokio::test]
    async fn test_atomic_counter_creation() {
        let counter = AtomicCounter::new(10);
        assert_eq!(counter.get().await, 10);
    }

    /// Validates `AtomicCounter::default` behavior for the atomic counter
    /// default scenario.
    ///
    /// Assertions:
    /// - Confirms `counter.get().await` equals `0`.
    #[tokio::test]
    async fn test_atomic_counter_default() {
        let counter = AtomicCounter::default();
        assert_eq!(counter.get().await, 0);
    }

    /// Validates `AtomicCounter::new` behavior for the atomic counter increment
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `result` equals `6`.
    /// - Confirms `counter.get().await` equals `6`.
    #[tokio::test]
    async fn test_atomic_counter_increment() {
        let counter = AtomicCounter::new(5);

        let result = counter.increment().await;
        assert_eq!(result, 6);
        assert_eq!(counter.get().await, 6);
    }

    /// Validates `AtomicCounter::new` behavior for the atomic counter decrement
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `result` equals `4`.
    /// - Confirms `counter.get().await` equals `4`.
    #[tokio::test]
    async fn test_atomic_counter_decrement() {
        let counter = AtomicCounter::new(5);

        let result = counter.decrement().await;
        assert_eq!(result, 4);
        assert_eq!(counter.get().await, 4);
    }

    /// Validates `AtomicCounter::new` behavior for the atomic counter decrement
    /// saturates scenario.
    ///
    /// Assertions:
    /// - Confirms `result` equals `0`.
    #[tokio::test]
    async fn test_atomic_counter_decrement_saturates() {
        let counter = AtomicCounter::new(0);

        let result = counter.decrement().await;
        assert_eq!(result, 0);
    }

    /// Validates `AtomicCounter::new` behavior for the atomic counter set
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `counter.get().await` equals `100`.
    #[tokio::test]
    async fn test_atomic_counter_set() {
        let counter = AtomicCounter::new(5);

        counter.set(100).await;
        assert_eq!(counter.get().await, 100);
    }

    /// Validates `AtomicCounter::new` behavior for the atomic counter add
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `result` equals `15`.
    /// - Confirms `counter.get().await` equals `15`.
    #[tokio::test]
    async fn test_atomic_counter_add() {
        let counter = AtomicCounter::new(5);

        let result = counter.add(10).await;
        assert_eq!(result, 15);
        assert_eq!(counter.get().await, 15);
    }

    /// Validates `Arc::new` behavior for the atomic counter concurrent
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `counter.get().await` equals `10`.
    #[tokio::test]
    async fn test_atomic_counter_concurrent() {
        let counter = Arc::new(AtomicCounter::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let c = Arc::clone(&counter);
            let handle = tokio::spawn(async move {
                c.increment().await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(counter.get().await, 10);
    }

    /// Validates `StateConfig::default` behavior for the state config default
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `config.read_timeout` equals `Duration::from_secs(5)`.
    /// - Confirms `config.write_timeout` equals `Duration::from_secs(10)`.
    /// - Confirms `config.max_concurrent_readers` equals `None`.
    /// - Ensures `!config.enable_metrics` evaluates to true.
    #[test]
    fn test_state_config_default() {
        let config = StateConfig::default();

        assert_eq!(config.read_timeout, Duration::from_secs(5));
        assert_eq!(config.write_timeout, Duration::from_secs(10));
        assert_eq!(config.max_concurrent_readers, None);
        assert!(!config.enable_metrics);
    }

    /// Validates `SafeShare::new` behavior for the safe share creation
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `*share.get()` equals `42`.
    #[test]
    fn test_safe_share_creation() {
        let share = SafeShare::new(42);
        assert_eq!(*share.get(), 42);
    }

    /// Validates `SafeShare::new` behavior for the safe share clone scenario.
    ///
    /// Assertions:
    /// - Confirms `*share1.get()` equals `*share2.get()`.
    #[test]
    fn test_safe_share_clone() {
        let share1 = SafeShare::new(42);
        let share2 = share1.clone();

        assert_eq!(*share1.get(), *share2.get());
    }

    /// Validates `SafeShare::new` behavior for the safe share clone arc
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `*arc` equals `42`.
    #[test]
    fn test_safe_share_clone_arc() {
        let share = SafeShare::new(42);
        let arc = share.clone_arc();

        assert_eq!(*arc, 42);
    }

    /// Validates `SafeShare::new` behavior for the safe share clone value
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cloned` equals `vec![1, 2, 3]`.
    #[test]
    fn test_safe_share_clone_value() {
        let share = SafeShare::new(vec![1, 2, 3]);
        let cloned = share.clone_value();

        assert_eq!(cloned, vec![1, 2, 3]);
    }

    /// Validates `StateBuilder::new` behavior for the state builder creation
    /// scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_state_builder_creation() {
        let _builder = StateBuilder::<i32>::new();
        // Builder should be created successfully
    }

    /// Validates `StateBuilder::new` behavior for the state builder with
    /// initial value scenario.
    ///
    /// Assertions:
    /// - Ensures `state.is_some()` evaluates to true.
    #[test]
    fn test_state_builder_with_initial_value() {
        let builder = StateBuilder::new().with_initial_value(42);
        let state = builder.build();

        assert!(state.is_some());
    }

    /// Validates `Duration::from_secs` behavior for the state builder with
    /// config scenario.
    ///
    /// Assertions:
    /// - Confirms `*state.read().await` equals `42`.
    #[tokio::test]
    async fn test_state_builder_with_config() {
        let config = StateConfig {
            read_timeout: Duration::from_secs(1),
            write_timeout: Duration::from_secs(2),
            max_concurrent_readers: Some(10),
            enable_metrics: true,
        };

        let builder = StateBuilder::new().with_initial_value(42).with_config(config);

        let state = builder.build().unwrap();
        assert_eq!(*state.read().await, 42);
    }

    /// Validates `StateBuilder::new` behavior for the state builder with
    /// timeouts scenario.
    ///
    /// Assertions:
    /// - Ensures `state.is_some()` evaluates to true.
    #[test]
    fn test_state_builder_with_timeouts() {
        let builder = StateBuilder::new()
            .with_initial_value(42)
            .with_read_timeout(Duration::from_secs(1))
            .with_write_timeout(Duration::from_secs(2));

        let state = builder.build();
        assert!(state.is_some());
    }

    /// Validates `StateBuilder::new` behavior for the state builder build with
    /// default scenario.
    ///
    /// Assertions:
    /// - Confirms `*state.read().await` equals `0`.
    #[tokio::test]
    async fn test_state_builder_build_with_default() {
        let builder = StateBuilder::<i32>::new();
        let state = builder.build_with_default();

        assert_eq!(*state.read().await, 0);
    }

    /// Validates `StateBuilder::new` behavior for the state builder build none
    /// without value scenario.
    ///
    /// Assertions:
    /// - Ensures `state.is_none()` evaluates to true.
    #[tokio::test]
    async fn test_state_builder_build_none_without_value() {
        let builder = StateBuilder::<i32>::new();
        let state = builder.build();

        assert!(state.is_none());
    }

    /// Validates `StateRegistry::new` behavior for the state registry creation
    /// scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_state_registry_creation() {
        let _registry = StateRegistry::new();
        // Registry should be created successfully
    }

    /// Validates `StateRegistry::new` behavior for the state registry register
    /// and get scenario.
    ///
    /// Assertions:
    /// - Ensures `retrieved.is_some()` evaluates to true.
    #[test]
    fn test_state_registry_register_and_get() {
        let mut registry = StateRegistry::new();
        let state = shared_state(42);

        registry.register("test_state".to_string(), state);

        let retrieved = registry.get::<i32>("test_state");
        assert!(retrieved.is_some());
    }

    /// Validates `StateRegistry::new` behavior for the state registry get value
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `*retrieved.read().await` equals `42`.
    #[tokio::test]
    async fn test_state_registry_get_value() {
        let mut registry = StateRegistry::new();
        let state = shared_state(42);

        registry.register("counter".to_string(), state);

        let retrieved = registry.get::<i32>("counter").unwrap();
        assert_eq!(*retrieved.read().await, 42);
    }

    /// Validates `StateRegistry::new` behavior for the state registry get
    /// nonexistent scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_none()` evaluates to true.
    #[test]
    fn test_state_registry_get_nonexistent() {
        let registry = StateRegistry::new();
        let result = registry.get::<i32>("nonexistent");

        assert!(result.is_none());
    }

    /// Validates `StateRegistry::new` behavior for the state registry multiple
    /// states scenario.
    ///
    /// Assertions:
    /// - Ensures `registry.get::<i32>("int_state").is_some()` evaluates to
    ///   true.
    /// - Ensures `registry.get::<String>("string_state").is_some()` evaluates
    ///   to true.
    #[test]
    fn test_state_registry_multiple_states() {
        let mut registry = StateRegistry::new();

        registry.register("int_state".to_string(), shared_state(42));
        registry.register("string_state".to_string(), shared_state("hello".to_string()));

        assert!(registry.get::<i32>("int_state").is_some());
        assert!(registry.get::<String>("string_state").is_some());
    }

    /// Validates `Duration::from_secs` behavior for the state config
    /// serialization scenario.
    ///
    /// Assertions:
    /// - Confirms `deserialized.read_timeout` equals `config.read_timeout`.
    /// - Confirms `deserialized.write_timeout` equals `config.write_timeout`.
    /// - Confirms `deserialized.max_concurrent_readers` equals
    ///   `config.max_concurrent_readers`.
    /// - Confirms `deserialized.enable_metrics` equals `config.enable_metrics`.
    #[tokio::test]
    async fn test_state_config_serialization() {
        let config = StateConfig {
            read_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(10),
            max_concurrent_readers: Some(100),
            enable_metrics: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: StateConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.read_timeout, config.read_timeout);
        assert_eq!(deserialized.write_timeout, config.write_timeout);
        assert_eq!(deserialized.max_concurrent_readers, config.max_concurrent_readers);
        assert_eq!(deserialized.enable_metrics, config.enable_metrics);
    }

    /// Validates `Arc::new` behavior for the managed state concurrent access
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `*state.read().await` equals `10`.
    #[tokio::test]
    async fn test_managed_state_concurrent_access() {
        let state = Arc::new(ManagedState::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let s = Arc::clone(&state);
            let handle = tokio::spawn(async move {
                let mut guard = s.write().await;
                *guard += 1;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(*state.read().await, 10);
    }
}
