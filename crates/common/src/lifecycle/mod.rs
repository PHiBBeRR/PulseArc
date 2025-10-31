//! Lifecycle management utilities for async components
//!
//! This module provides standardized lifecycle management patterns including:
//! - **[`manager`]**: Async component lifecycle management with health checks
//! - **[`state`]**: Thread-safe state management with Arc<RwLock<T>> patterns

pub mod manager;
pub mod state;

// Re-export commonly used types and traits for convenience
pub use manager::{
    AsyncManager, ComponentHealth, ManagerController, ManagerHealth, ManagerLifecycle,
    ManagerMetadata, ManagerStatus, SharedState,
};
pub use state::{
    shared_state, AtomicCounter, ManagedState, SafeShare, SharedState as AsyncSharedState,
    StateBuilder, StateConfig, StateRegistry,
};
