# Lifecycle Management

Standardized lifecycle management patterns for async components and managers.

## Overview

The lifecycle module provides infrastructure for managing the lifecycle of async components with consistent patterns for initialization, health monitoring, and graceful shutdown. It includes thread-safe state management utilities using Arc<RwLock<T>> patterns, making it easy to build robust, concurrent managers.

## Architecture

```
lifecycle/
├── manager.rs    # AsyncManager trait and lifecycle patterns
├── state.rs      # Thread-safe state management utilities
└── mod.rs        # Public API and re-exports
```

## Features

### Manager Lifecycle
- **Standardized lifecycle** with AsyncManager trait
- **Health checks** with component-level monitoring
- **Graceful shutdown** with proper resource cleanup
- **Controller pattern** for managing multiple managers
- **Status tracking** through lifecycle stages

### State Management
- **Thread-safe patterns** with Arc<RwLock<T>>
- **Timeout support** for lock acquisition
- **Access tracking** with ManagedState
- **Atomic operations** for counters
- **Builder patterns** for complex state

## Components

### AsyncManager Trait (`manager.rs`)

Core lifecycle trait for all managers:

```rust
use agent::common::lifecycle::{AsyncManager, ManagerHealth, ManagerStatus};

#[async_trait::async_trait]
impl AsyncManager for MyManager {
    type Error = MyError;
    type Config = MyConfig;

    async fn new() -> Result<Self, Self::Error> {
        // Create with defaults
        Ok(Self { /* ... */ })
    }

    async fn with_config(config: Self::Config) -> Result<Self, Self::Error> {
        // Create with custom config
        Ok(Self { /* ... */ })
    }

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        // Initialize resources
        Ok(())
    }

    async fn health_check(&self) -> Result<ManagerHealth, Self::Error> {
        // Check component health
        Ok(ManagerHealth::healthy())
    }

    async fn shutdown(&mut self) -> Result<(), Self::Error> {
        // Clean up resources
        Ok(())
    }

    fn status(&self) -> ManagerStatus {
        self.status
    }
}
```

### SharedState (`manager.rs`)

Thread-safe state container with helper methods:

```rust
use agent::common::lifecycle::SharedState;
use std::time::Duration;

// Create shared state
let state = SharedState::new(42, "counter");

// Read operations
let value = *state.read().await;
let value = state.get().await;  // Clone (requires T: Clone)
let result = state.read_with(|v| v * 2).await;

// Write operations
let mut guard = state.write().await;
*guard = 100;

// Update with closure
state.update(|v| *v += 1).await;

// Replace entire value
let old_value = state.replace(200).await;

// Timeout support
let guard = state.read_timeout(Duration::from_secs(5)).await?;
let result = state.update_timeout(
    Duration::from_secs(5),
    |v| *v * 2
).await?;
```

### ManagedState (`state.rs`)

State container with lifecycle tracking:

```rust
use agent::common::lifecycle::ManagedState;

let state = ManagedState::new(MyData::default());

// Automatic access tracking
let guard = state.read().await;

// Query lifecycle info
let created_at = state.created_at();
let last_accessed = state.last_accessed().await;
let age = state.age().await;
let idle_time = state.idle_time().await;

// Get shared Arc for cross-thread use
let shared = state.get_shared();
```

### ManagerController (`manager.rs`)

Coordinate multiple managers with proper startup/shutdown ordering:

```rust
use agent::common::lifecycle::{ManagerController, ManagerLifecycle};

let mut controller = ManagerController::new();

// Add managers
controller.add_manager(manager1);
controller.add_manager(manager2);
controller.add_manager(manager3);

// Initialize all in order
controller.initialize_all().await?;

// Check status
let status = controller.status().await;
let statuses = controller.manager_statuses();

// Shutdown all in reverse order
controller.shutdown_all().await?;
```

## Usage

### Basic Manager Implementation

```rust
use agent::common::lifecycle::{
    AsyncManager, ManagerHealth, ManagerStatus, SharedState
};
use std::sync::Arc;

pub struct MyManager {
    status: SharedState<ManagerStatus>,
    config: MyConfig,
    connection: Option<Connection>,
}

#[async_trait::async_trait]
impl AsyncManager for MyManager {
    type Error = MyError;
    type Config = MyConfig;

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self {
            status: SharedState::new(ManagerStatus::Created, "my_manager"),
            config: MyConfig::default(),
            connection: None,
        })
    }

    async fn with_config(config: Self::Config) -> Result<Self, Self::Error> {
        Ok(Self {
            status: SharedState::new(ManagerStatus::Created, "my_manager"),
            config,
            connection: None,
        })
    }

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        self.status.replace(ManagerStatus::Initializing).await;

        // Initialize resources
        self.connection = Some(Connection::open(&self.config)?);

        self.status.replace(ManagerStatus::Running).await;
        Ok(())
    }

    async fn health_check(&self) -> Result<ManagerHealth, Self::Error> {
        let mut health = ManagerHealth::healthy();

        // Check connection
        if let Some(conn) = &self.connection {
            if conn.is_alive() {
                health = health.with_component(
                    ComponentHealth::healthy("connection")
                );
            } else {
                health = health.with_component(
                    ComponentHealth::unhealthy("connection", "Connection lost")
                );
            }
        }

        Ok(health)
    }

    async fn shutdown(&mut self) -> Result<(), Self::Error> {
        self.status.replace(ManagerStatus::ShuttingDown).await;

        // Clean up resources
        if let Some(conn) = self.connection.take() {
            conn.close()?;
        }

        self.status.replace(ManagerStatus::Shutdown).await;
        Ok(())
    }

    fn status(&self) -> ManagerStatus {
        // Note: Can't await in sync method, use try_read
        self.status.try_read()
            .map(|guard| *guard)
            .unwrap_or(ManagerStatus::Error)
    }
}
```

### Health Checks with Components

```rust
use agent::common::lifecycle::{ManagerHealth, ComponentHealth};

async fn health_check(&self) -> Result<ManagerHealth, Self::Error> {
    let mut health = ManagerHealth::healthy();

    // Check database
    if self.db.ping().await.is_ok() {
        health = health.with_component(ComponentHealth::healthy("database"));
    } else {
        health = health.with_component(
            ComponentHealth::unhealthy("database", "Cannot reach database")
        );
    }

    // Check cache
    if self.cache.is_connected() {
        health = health.with_component(ComponentHealth::healthy("cache"));
    } else {
        // Degraded but not critical
        health = ManagerHealth::degraded(0.7, "Cache unavailable");
        health = health.with_component(
            ComponentHealth::unhealthy("cache", "Redis connection lost")
        );
    }

    Ok(health)
}
```

### State Management Patterns

#### Simple Shared State

```rust
use agent::common::lifecycle::shared_state;

// Create shared state
let counter = shared_state(0u64);

// Use in async tasks
let counter_clone = Arc::clone(&counter);
tokio::spawn(async move {
    let mut guard = counter_clone.write().await;
    *guard += 1;
});
```

#### Managed State with Tracking

```rust
use agent::common::lifecycle::ManagedState;
use std::time::Duration;

let cache = ManagedState::new(HashMap::new());

// Check if cache needs refresh
if cache.idle_time().await > Duration::from_secs(300) {
    let mut data = cache.write().await;
    refresh_cache(&mut data).await?;
}
```

#### Atomic Counter

```rust
use agent::common::lifecycle::AtomicCounter;

let requests = AtomicCounter::new(0);

// Increment atomically
let count = requests.increment().await;

// Add bulk
requests.add(10).await;

// Get current value
let total = requests.get().await;
```

#### State Builder

```rust
use agent::common::lifecycle::{StateBuilder, StateConfig};
use std::time::Duration;

let state = StateBuilder::new()
    .with_initial_value(MyData::default())
    .with_read_timeout(Duration::from_secs(5))
    .with_write_timeout(Duration::from_secs(10))
    .build()
    .expect("State should have value");

// Or use default
let state = StateBuilder::<MyData>::new()
    .build_with_default();
```

### Macros for State Access

Convenient macros for common operations:

```rust
use agent::common::lifecycle::{shared_state, read_state, write_state, update_state};

let state = shared_state(42);

// Read with automatic timeout
let value = read_state!(state, timeout: Duration::from_secs(1))?;

// Write with timeout
let mut guard = write_state!(state, timeout: Duration::from_secs(1))?;

// Update with closure
let result = update_state!(state, |v| {
    *v += 1;
    *v
});

// Conditional update
let updated = update_state_if!(state,
    |v| *v < 100,  // condition
    |v| *v += 1    // update
);
```

### Coordinating Multiple Managers

```rust
use agent::common::lifecycle::ManagerController;

// Application setup
pub async fn initialize_app() -> Result<ManagerController, AppError> {
    let mut controller = ManagerController::new();

    // Add managers in dependency order
    let db = DatabaseManager::new().await?;
    controller.add_manager(db);

    let cache = CacheManager::new().await?;
    controller.add_manager(cache);

    let api = ApiManager::new().await?;
    controller.add_manager(api);

    // Initialize all
    controller.initialize_all().await?;

    Ok(controller)
}

// Application shutdown
pub async fn shutdown_app(mut controller: ManagerController) -> Result<(), AppError> {
    // Shuts down in reverse order (API -> Cache -> DB)
    controller.shutdown_all().await?;
    Ok(())
}
```

### Implementing ManagerLifecycle

Use the macro for automatic implementation:

```rust
use agent::impl_manager_lifecycle;

struct MyManager {
    status: ManagerStatus,
    // ...
}

// Automatically implements ManagerLifecycle
impl_manager_lifecycle!(MyManager, MyError);

// Now works with ManagerController
let mut controller = ManagerController::new();
controller.add_manager(MyManager::new().await?);
```

## Lifecycle States

### ManagerStatus

```rust
pub enum ManagerStatus {
    Created,       // Created but not initialized
    Initializing,  // Currently initializing
    Running,       // Operational
    ShuttingDown,  // Shutting down
    Shutdown,      // Fully shut down
    Error,         // Error state
}
```

### Health Scores

```rust
// Perfectly healthy
let health = ManagerHealth::healthy();
assert_eq!(health.score, 1.0);

// Completely unhealthy
let health = ManagerHealth::unhealthy("Service down");
assert_eq!(health.score, 0.0);

// Degraded (0.0 to 1.0)
let health = ManagerHealth::degraded(0.7, "Partial service");
assert!(health.is_healthy);  // > 0.5 is considered healthy
```

## Graceful Shutdown Patterns

### Shutdown with Timeout

```rust
use tokio::time::timeout;
use std::time::Duration;

async fn safe_shutdown(mut manager: MyManager) -> Result<(), Error> {
    // Try graceful shutdown with timeout
    match timeout(Duration::from_secs(30), manager.shutdown()).await {
        Ok(Ok(())) => {
            tracing::info!("Manager shut down gracefully");
            Ok(())
        }
        Ok(Err(e)) => {
            tracing::error!("Shutdown error: {}", e);
            Err(e)
        }
        Err(_) => {
            tracing::warn!("Shutdown timeout, forcing cleanup");
            // Force cleanup
            Err(Error::timeout("manager_shutdown"))
        }
    }
}
```

### Shutdown Signal Handler

```rust
use tokio::signal;

pub async fn run_with_shutdown(mut manager: MyManager) -> Result<(), Error> {
    // Initialize
    manager.initialize().await?;

    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            tracing::info!("Received shutdown signal");
        }
        _ = wait_for_error(&manager) => {
            tracing::error!("Manager error detected");
        }
    }

    // Graceful shutdown
    manager.shutdown().await?;
    Ok(())
}

async fn wait_for_error(manager: &MyManager) {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        match manager.health_check().await {
            Ok(health) if !health.is_healthy => {
                tracing::warn!("Health check failed: {:?}", health);
                break;
            }
            Err(e) => {
                tracing::error!("Health check error: {}", e);
                break;
            }
            _ => {}
        }
    }
}
```

### Coordinated Shutdown

```rust
use agent::common::lifecycle::ManagerController;

pub async fn app_with_lifecycle() -> Result<(), AppError> {
    let mut controller = ManagerController::new();

    // Setup managers
    controller.add_manager(DatabaseManager::new().await?);
    controller.add_manager(CacheManager::new().await?);
    controller.add_manager(ApiManager::new().await?);

    // Initialize
    controller.initialize_all().await?;

    // Run
    tokio::select! {
        _ = signal::ctrl_c() => {
            tracing::info!("Shutdown signal received");
        }
    }

    // Graceful shutdown (reverse order)
    controller.shutdown_all().await?;

    Ok(())
}
```

## Testing

### Unit Tests

Test individual manager lifecycle:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_lifecycle() {
        let mut manager = MyManager::new().await.unwrap();

        // Should start in Created state
        assert_eq!(manager.status(), ManagerStatus::Created);

        // Initialize
        manager.initialize().await.unwrap();
        assert_eq!(manager.status(), ManagerStatus::Running);

        // Should be healthy
        let health = manager.health_check().await.unwrap();
        assert!(health.is_healthy);
        assert_eq!(health.score, 1.0);

        // Shutdown
        manager.shutdown().await.unwrap();
        assert_eq!(manager.status(), ManagerStatus::Shutdown);
    }

    #[tokio::test]
    async fn test_manager_health_checks() {
        let manager = MyManager::new().await.unwrap();

        let health = manager.health_check().await.unwrap();

        // Check components
        assert!(health.components.iter().any(|c| c.name == "database"));
        assert!(health.components.iter().any(|c| c.name == "cache"));
    }
}
```

### Integration Tests

Test manager coordination:

```rust
#[tokio::test]
async fn test_controller_lifecycle() {
    let mut controller = ManagerController::new();

    controller.add_manager(MockManager::new("db"));
    controller.add_manager(MockManager::new("cache"));

    // Initialize
    controller.initialize_all().await.unwrap();
    assert_eq!(controller.status().await, ManagerStatus::Running);

    // Shutdown
    controller.shutdown_all().await.unwrap();
    assert_eq!(controller.status().await, ManagerStatus::Shutdown);
}
```

### Mock Managers for Testing

```rust
struct MockManager {
    name: String,
    status: ManagerStatus,
    init_count: Arc<std::sync::Mutex<u32>>,
    shutdown_count: Arc<std::sync::Mutex<u32>>,
}

#[async_trait::async_trait]
impl AsyncManager for MockManager {
    type Error = std::io::Error;
    type Config = ();

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self {
            name: "mock".to_string(),
            status: ManagerStatus::Created,
            init_count: Arc::new(std::sync::Mutex::new(0)),
            shutdown_count: Arc::new(std::sync::Mutex::new(0)),
        })
    }

    async fn with_config(_config: Self::Config) -> Result<Self, Self::Error> {
        Self::new().await
    }

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        *self.init_count.lock().unwrap() += 1;
        self.status = ManagerStatus::Running;
        Ok(())
    }

    async fn health_check(&self) -> Result<ManagerHealth, Self::Error> {
        Ok(ManagerHealth::healthy())
    }

    async fn shutdown(&mut self) -> Result<(), Self::Error> {
        *self.shutdown_count.lock().unwrap() += 1;
        self.status = ManagerStatus::Shutdown;
        Ok(())
    }

    fn status(&self) -> ManagerStatus {
        self.status
    }
}
```

## Best Practices

### 1. Always Track Status

```rust
// ✅ Good - Track status through lifecycle
pub struct MyManager {
    status: SharedState<ManagerStatus>,
}

impl MyManager {
    async fn initialize(&mut self) -> Result<(), Error> {
        self.status.replace(ManagerStatus::Initializing).await;
        // ... initialization logic
        self.status.replace(ManagerStatus::Running).await;
        Ok(())
    }
}

// ❌ Bad - No status tracking
pub struct MyManager {
    initialized: bool,
}
```

### 2. Implement Comprehensive Health Checks

```rust
// ✅ Good - Check all components
async fn health_check(&self) -> Result<ManagerHealth, Self::Error> {
    let mut health = ManagerHealth::healthy();

    // Check each dependency
    if !self.db.is_connected() {
        health = health.with_component(
            ComponentHealth::unhealthy("database", "Not connected")
        );
    }

    if !self.cache.is_available() {
        health = health.with_component(
            ComponentHealth::unhealthy("cache", "Unavailable")
        );
    }

    Ok(health)
}

// ❌ Bad - Always returns healthy
async fn health_check(&self) -> Result<ManagerHealth, Self::Error> {
    Ok(ManagerHealth::healthy())
}
```

### 3. Clean Up Resources on Shutdown

```rust
// ✅ Good - Clean up all resources
async fn shutdown(&mut self) -> Result<(), Self::Error> {
    self.status.replace(ManagerStatus::ShuttingDown).await;

    // Stop background tasks
    if let Some(task) = self.background_task.take() {
        task.abort();
    }

    // Close connections
    if let Some(conn) = self.connection.take() {
        conn.close().await?;
    }

    self.status.replace(ManagerStatus::Shutdown).await;
    Ok(())
}

// ❌ Bad - Leak resources
async fn shutdown(&mut self) -> Result<(), Self::Error> {
    self.status.replace(ManagerStatus::Shutdown).await;
    Ok(())
}
```

### 4. Use Timeouts for Lock Acquisition

```rust
// ✅ Good - Use timeouts to prevent deadlocks
let guard = state.read_timeout(Duration::from_secs(5)).await?;

// ❌ Bad - Can deadlock
let guard = state.read().await;  // Hangs forever if deadlocked
```

### 5. Handle Shutdown Errors Gracefully

```rust
// ✅ Good - Continue shutdown even on errors
async fn shutdown_all(&mut self) -> Result<(), Error> {
    let mut errors = Vec::new();

    for manager in &mut self.managers {
        if let Err(e) = manager.shutdown().await {
            tracing::error!("Shutdown error: {}", e);
            errors.push(e);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::multiple(errors))
    }
}

// ❌ Bad - Abort on first error
async fn shutdown_all(&mut self) -> Result<(), Error> {
    for manager in &mut self.managers {
        manager.shutdown().await?;  // Stops on first error
    }
    Ok(())
}
```

### 6. Proper Dependency Order

```rust
// ✅ Good - Initialize in dependency order
controller.add_manager(database);  // First
controller.add_manager(cache);     // Depends on database
controller.add_manager(api);       // Depends on cache
// Shutdown happens in reverse: api -> cache -> database

// ❌ Bad - Wrong order
controller.add_manager(api);       // Fails without dependencies
controller.add_manager(cache);
controller.add_manager(database);
```

## Performance Considerations

### Lock Contention

```rust
// Use try_read/try_write for non-critical reads
if let Ok(guard) = state.try_read() {
    process_data(&*guard);
}

// Use timeout for critical operations
let guard = state.read_timeout(Duration::from_millis(100)).await?;
```

### Memory Usage

- SharedState: Minimal overhead (Arc + RwLock)
- ManagedState: Additional tracking data (~48 bytes)
- ManagerController: Vec of trait objects (small)

### Thread Safety

All types are `Send + Sync` and safe for concurrent use:

```rust
let state = Arc::new(SharedState::new(data, "shared"));

// Safe to clone across threads
let state_clone = Arc::clone(&state);
tokio::spawn(async move {
    state_clone.write().await;
});
```

## Troubleshooting

### Deadlocks

Use timeouts and proper lock ordering:

```rust
// Always acquire locks in the same order
let guard1 = state1.read_timeout(Duration::from_secs(5)).await?;
let guard2 = state2.read_timeout(Duration::from_secs(5)).await?;
```

### Shutdown Hangs

Add timeouts to shutdown operations:

```rust
use tokio::time::timeout;

timeout(Duration::from_secs(30), controller.shutdown_all()).await??;
```

### Status Out of Sync

Use `try_read` in `status()` method:

```rust
fn status(&self) -> ManagerStatus {
    self.status.try_read()
        .map(|guard| *guard)
        .unwrap_or(ManagerStatus::Error)
}
```

## See Also

- [common/](../) - Common utilities module
- [common/error/](../error/) - Error handling
- [common/validation/](../validation/) - Validation patterns
- [storage/](../../storage/) - Storage manager example
