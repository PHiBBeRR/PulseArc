# Lifecycle Module

Lifecycle centralizes the primitives we use to bootstrap, monitor, and shut down asynchronous services inside the PulseArc agent. The module lives under `crates/common/src/lifecycle` and is re-exported by `pulsearc_common::lifecycle::*` for convenient use across the workspace.

## Folder Layout
- `mod.rs` renders the public surface by re-exporting the manager and state submodules.
- `manager.rs` holds the `AsyncManager` trait, lifecycle status and health types, the `ManagerController`, and a higher level `SharedState<T>` wrapper with timeout helpers.
- `state.rs` focuses on ergonomic `Arc<RwLock<T>>` utilities, including macros, `ManagedState`, `AtomicCounter`, builders, and registries.

## What Problems This Solves
- Bootstrapping services in a predictable order with consistent error handling.
- Tracking liveness through `ManagerStatus`, `ManagerHealth`, and `ComponentHealth`.
- Sharing mutable data between async tasks without leaking lifetimes or deadlocking.
- Instrumenting state access and collecting timing information for diagnostics.

## Quick Start: Writing a Manager
Implement `AsyncManager` for any long-lived component that needs deterministic startup and shutdown. The trait returns a custom error type and optional configuration while exposing health hooks.

```rust
use pulsearc_common::lifecycle::{
    AsyncManager, ManagerController, ManagerHealth, ManagerStatus,
};

pub struct CacheManager {
    status: ManagerStatus,
}

#[async_trait::async_trait]
impl AsyncManager for CacheManager {
    type Error = anyhow::Error;
    type Config = ();

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self { status: ManagerStatus::Created })
    }

    async fn with_config(_config: Self::Config) -> Result<Self, Self::Error> {
        Self::new().await
    }

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        self.status = ManagerStatus::Running;
        Ok(())
    }

    async fn health_check(&self) -> Result<ManagerHealth, Self::Error> {
        Ok(ManagerHealth::healthy())
    }

    async fn shutdown(&mut self) -> Result<(), Self::Error> {
        self.status = ManagerStatus::Shutdown;
        Ok(())
    }

    fn status(&self) -> ManagerStatus {
        self.status
    }
}
```

When multiple managers must follow dependency ordering, register them on a controller and call `initialize_all()` / `shutdown_all()`:

```rust
use pulsearc_common::lifecycle::ManagerController;

let mut controller = ManagerController::new();
controller.add_manager(DatabaseManager::new().await?);
controller.add_manager(CacheManager::new().await?);
controller.add_manager(ApiGatewayManager::new().await?);

controller.initialize_all().await?;
// ... run services ...
controller.shutdown_all().await?;
```

The `impl_manager_lifecycle!` macro (defined in `manager.rs`) can bridge `AsyncManager` implementors into the `ManagerLifecycle` trait expected by the controller.

## State Utilities at a Glance
- `SharedState<T>` (struct, `manager.rs`) wraps `Arc<RwLock<T>>` and layers timeout-aware methods such as `read_timeout`, `write_timeout`, `update`, and `replace`.
- `ManagedState<T>` tracks creation and last-access timestamps while exposing ergonomic `read`, `write`, `modify`, and `clone_value` calls.
- `shared_state()` (function) is a light-weight helper for quickly creating `Arc<RwLock<T>>` values without lifecycle metadata.
- `StateRegistry` offers keyed storage for heterogeneous shared state; it is handy when wiring dynamic sub-systems.
- `AtomicCounter` provides an async-friendly u64 counter with saturating decrement semantics.
- `SafeShare<T>` stores data behind an `Arc` when thread-safe sharing without interior mutability is sufficient.
- `StateBuilder<T>` and `StateConfig` make it easy to capture timeout and metric preferences when instantiating complex managed state graphs.

### Macros
The macros in `state.rs` remove boilerplate when interacting with shared state patterns:
- `read_state!(state)` and `write_state!(state)` acquire locks and optionally support `timeout: Duration`.
- `update_state!(state, |data| { ... })` mutates a value in-place, returning the closure result.
- `update_state_if!(state, |current| predicate, |data| { ... })` short-circuits when a condition fails.
- `read_state_map!(state, |data| ...)` keeps read access scoped to a single expression.
- `clone_from_state!(state, field)` helps return owned values without manual cloning logic.

All helpers use the same error vocabulary (`CommonError`) as the rest of the `common` crate, so failures align with the tooling we rely on in higher layers.

## Usage Patterns
- Prefer `initialize()` for allocating resources (connections, threads) and `shutdown()` for graceful teardown; defer heavy work until after the controller signals `Running`.
- Aggregate component level health via `ManagerHealth::with_component(ComponentHealth::healthy("cache"))` to keep telemetry granular.
- Adopt the timeout-enabled methods (`read_timeout`, `update_timeout`) in hot paths to avoid blocking the runtime if another task holds the lock.
- Compose larger subsystems with `ManagedState` when access telemetry (creation time, idle time) is useful; fall back to the lean `SharedState<T>` wrapper for simple counters or caches.
- Use `ManagerController::manager_statuses()` to surface per-manager state in admin APIs or logging.

## Testing and Benchmarks
- Unit tests in `manager.rs` and `state.rs` validate lock semantics, health reporting, and controller coordination.
- Integration coverage under `crates/common/tests/lifecycle_integration.rs` pushes realistic concurrent access scenarios (multi-threaded Tokio runtime, mutation races, complex types).
- Micro-benchmarks in `crates/common/benches/lifecycle_bench.rs` exercise mixed read/write workloads, builder overhead, counter throughput, and controller orchestration patterns. Run them with `cargo bench --bench lifecycle_bench -p pulsearc-common --features runtime`.

## Operational Tips
- Keep `AsyncManager::status()` lock-free; compute derived data lazily through cached state structures rather than holding `RwLock` guards.
- Treat timeouts as instrumentation points: the `CommonError::timeout` variants make it straightforward to trace slow paths via logs.
- When wiring new managers into the desktop agent, run `make test` to execute the entire Rust suite and ensure lifecycle tests continue to pass.
- For frontend-to-core integration work, start the desktop stack with `make dev` to observe how managers boot within the Tauri context.

## Related Modules
- `crates/common/src/error` defines `CommonError` and `CommonResult`, the error envelope used across lifecycle helpers.
- `crates/common/src/validation` complements lifecycle with light-weight assertion utilities for manager configuration.
- The infrastructure crate (`crates/infra`) contains concrete manager implementations that rely on these patterns; use them as real-world references before introducing new abstractions.
