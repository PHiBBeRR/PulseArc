//! SAP OAuth authentication wrapper
//!
//! This module provides a thin wrapper around
//! `pulsearc-common::auth::OAuthService` with SAP-specific configuration for
//! Auth0 authentication.
//!
//! # Usage
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use pulsearc_common::security::KeychainProvider;
//! use pulsearc_infra::integrations::sap::auth::{create_sap_oauth_config, SapAuthService};
//! use pulsearc_infra::integrations::sap::client::{AccessTokenProvider, SapClient};
//!
//! # async fn example(
//! #     wbs_repo: Arc<dyn pulsearc_core::classification::ports::WbsRepository>
//! # ) -> Result<(), Box<dyn std::error::Error>> {
//! // Create SAP OAuth configuration
//! let config = create_sap_oauth_config(
//!     "your_sap_client_id".to_string(),
//!     "http://localhost:8888/callback".to_string(),
//! );
//!
//! // Create auth service
//! let keychain = Arc::new(KeychainProvider::new("PulseArc".to_string()));
//! let auth_service = Arc::new(SapAuthService::new(
//!     config,
//!     keychain,
//!     "PulseArc.sap".to_string(),
//!     "sap".to_string(),
//!     300, // Refresh 5 minutes before expiry
//! ));
//!
//! // Initialize auth
//! auth_service.initialize().await?;
//!
//! // Use with SAP client
//! let client = SapClient::new(
//!     "https://sap-connector.example.com".to_string(),
//!     wbs_repo,
//!     "user@example.com".to_string(),
//!     auth_service, // Implements AccessTokenProvider
//! )?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use pulsearc_common::auth::{OAuthConfig, OAuthService};
use pulsearc_domain::Result;

use crate::integrations::sap::client::AccessTokenProvider;

/// Type alias for SAP OAuth service
///
/// This is the same as `pulsearc_common::auth::OAuthService` but provides
/// a clearer type name in SAP-specific contexts.
pub type SapAuthService = OAuthService;

/// Create SAP-specific OAuth configuration for Auth0
///
/// Configures OAuth for the SAP connector API with Auth0 as the identity
/// provider.
///
/// # Arguments
///
/// * `client_id` - OAuth client ID from Auth0 application
/// * `redirect_uri` - Redirect URI for OAuth callback (e.g., "http://localhost:8888/callback")
///
/// # Returns
///
/// An `OAuthConfig` configured for SAP with:
/// - Auth0 domain: `dev-q6f8uk0tlxem2tpc.us.auth0.com`
/// - Audience: `https://sap-connector.pulsarc.local`
/// - Scopes: `timesheet:write`, `wbs:read`, `openid`, `profile`,
///   `offline_access`
///
/// # Example
///
/// ```
/// use pulsearc_infra::integrations::sap::auth::create_sap_oauth_config;
///
/// let config = create_sap_oauth_config(
///     "my_client_id".to_string(),
///     "http://localhost:8888/callback".to_string(),
/// );
///
/// assert_eq!(config.domain, "dev-q6f8uk0tlxem2tpc.us.auth0.com");
/// assert_eq!(config.audience.as_deref(), Some("https://sap-connector.pulsarc.local"));
/// ```
pub fn create_sap_oauth_config(client_id: String, redirect_uri: String) -> OAuthConfig {
    OAuthConfig::new(
        "dev-q6f8uk0tlxem2tpc.us.auth0.com".to_string(),
        client_id,
        redirect_uri,
        vec![
            "timesheet:write".to_string(),
            "wbs:read".to_string(),
            "openid".to_string(),
            "profile".to_string(),
            "offline_access".to_string(),
        ],
        Some("https://sap-connector.pulsarc.local".to_string()),
    )
}

/// Allow `SapAuthService` (and `Arc<SapAuthService>`) to be used anywhere an
/// `AccessTokenProvider` is required.
#[async_trait]
impl AccessTokenProvider for SapAuthService {
    async fn access_token(&self) -> Result<String> {
        self.get_access_token().await.map_err(|e| {
            pulsearc_domain::PulseArcError::Auth(format!("Failed to get SAP access token: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_config_with_correct_domain() {
        let config = create_sap_oauth_config(
            "test_client_id".to_string(),
            "http://localhost:8888/callback".to_string(),
        );

        assert_eq!(config.domain, "dev-q6f8uk0tlxem2tpc.us.auth0.com");
    }

    #[test]
    fn creates_config_with_correct_audience_and_scopes() {
        let config = create_sap_oauth_config(
            "test_client_id".to_string(),
            "http://localhost:8888/callback".to_string(),
        );

        // Verify audience
        assert_eq!(config.audience.as_deref(), Some("https://sap-connector.pulsarc.local"));

        // Verify all required scopes are present
        let scopes = &config.scopes;
        assert!(scopes.contains(&"timesheet:write".to_string()));
        assert!(scopes.contains(&"wbs:read".to_string()));
        assert!(scopes.contains(&"openid".to_string()));
        assert!(scopes.contains(&"profile".to_string()));
        assert!(scopes.contains(&"offline_access".to_string()));
        assert_eq!(scopes.len(), 5);
    }

    #[test]
    fn creates_config_with_provided_client_id_and_redirect() {
        let config = create_sap_oauth_config(
            "my_custom_client_id".to_string(),
            "http://example.com/callback".to_string(),
        );

        assert_eq!(config.client_id, "my_custom_client_id");
        assert_eq!(config.redirect_uri, "http://example.com/callback");
    }
}
