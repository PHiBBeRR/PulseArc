/// SAP integration module (feature-gated)
///
/// This module provides SAP GraphQL client and time entry forwarding functionality.
/// Only compiled when the `sap` feature is enabled.
///
/// # Architecture
///
/// - **Client**: `SapClient` - GraphQL client for sap-connector API
/// - **Forwarder**: `SapForwarder` - Converts outbox entries to SAP format
/// - **WBS Validation**: Integrated with `WbsRepository` from Phase 2
///
/// # Usage
///
/// ```no_run
/// use std::sync::Arc;
/// use pulsearc_infra::integrations::sap::SapClient;
/// use pulsearc_core::sap_ports::{SapClient as SapClientTrait, TimeEntry};
///
/// # async fn example(wbs_repo: Arc<dyn pulsearc_core::classification::ports::WbsRepository>) -> pulsearc_domain::Result<()> {
/// // Create SAP client
/// let client = SapClient::new(
///     "http://localhost:3000".to_string(),
///     wbs_repo,
///     "user@example.com".to_string(),
///     Arc::new(MyAccessTokenProvider::default()),
/// )?;
///
/// // Forward a time entry
/// let entry = TimeEntry {
///     wbs_code: "USC0063201.1.1".to_string(),
///     description: "Development work".to_string(),
///     duration_hours: 2.5,
///     date: "2025-10-31".to_string(),
/// };
///
/// let entry_id = client.forward_entry(&entry).await?;
/// println!("Submitted entry: {}", entry_id);
///
/// // Validate WBS code
/// let is_valid = client.validate_wbs("USC0063201.1.1").await?;
/// assert!(is_valid);
/// # Ok(())
/// # }
/// ```
///
/// # Authentication
///
/// Access tokens are supplied via an `AccessTokenProvider` implementation.
/// Production deployments should wire this to the OAuth token manager so tokens
/// refresh automatically.
///
/// # GraphQL API
///
/// Communicates with sap-connector GraphQL API:
/// - `submitTimeEntries` mutation - Submit time entries
/// - Health check endpoint at `/health`
///
/// # Error Handling
///
/// - Network errors: Retried automatically by `HttpClient`
/// - GraphQL errors: Returned with correlation IDs for tracing
/// - Missing token: Fails fast with `PulseArcError::Config`
pub mod client;
pub mod forwarder;

pub use client::{AccessTokenProvider, SapClient};
pub use forwarder::SapForwarder;
