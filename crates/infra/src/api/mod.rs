//! Domain API client for PulseArc
//!
//! This module provides HTTP-based API client for domain sync operations.
//! It handles authentication, command execution, batch forwarding, and
//! scheduling for the remote PulseArc API.
//!
//! # Architecture
//!
//! - Uses Phase 3A HttpClient (no direct reqwest)
//! - OAuth authentication with token refresh
//! - Circuit breaker + retry for resilience
//! - Batch forwarding with partial success handling
//! - Background scheduler for periodic sync
//!
//! # Compliance
//!
//! - **CLAUDE.md §3**: Structured tracing only (no println!)
//! - **CLAUDE.md §5**: Timeout on all external calls
//! - **CLAUDE.md §9**: No secrets in code, keychain for credentials
//! - **Performance**: p99 < 500ms target

pub mod auth;
pub mod client;
pub mod commands;
pub mod errors;
pub mod forwarder;
pub mod scheduler;

pub use auth::{create_api_oauth_config, AccessTokenProvider, ApiAuthService};
pub use client::{ApiClient, ApiClientConfig};
pub use commands::ApiCommands;
pub use errors::{ApiError, ApiErrorCategory};
pub use forwarder::{ApiForwarder, BatchSubmissionResult, ForwarderConfig};
pub use scheduler::{ApiScheduler, SchedulerConfig};
