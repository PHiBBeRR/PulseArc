// Enterprise-grade Persistent Sync Queue
// Modular implementation with full feature set

pub mod compression;
mod core;
mod encryption;
mod errors;
mod maintenance;
pub mod metrics;
mod persistence;
mod types;

pub use self::compression::{CompressionAlgorithm, CompressionService};
pub use self::core::SyncQueue;
// Re-export for backward compatibility
pub use self::core::SyncQueue as Queue;
pub use self::errors::{QueueError, QueueResult};
pub use self::metrics::{QueueMetrics, QueueMetricsSnapshot};
pub use self::types::{ItemStatus, Priority, QueueConfig, SyncItem};
