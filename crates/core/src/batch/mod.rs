//! Batch processing operations
//!
//! This module provides ports for batch queue and Dead Letter Queue (DLQ)
//! operations.

pub mod ports;

pub use ports::{BatchRepository, DlqRepository};
