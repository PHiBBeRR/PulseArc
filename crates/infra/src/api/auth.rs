//! API authentication with OAuth token management
//!
//! Provides OAuth-based authentication for the domain API with automatic
//! token refresh and keychain storage.

use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_common::auth::{OAuthConfig, OAuthService};
use pulsearc_common::security::KeychainProvider;
use tracing::{debug, info};

use super::errors::ApiError;

/// Trait for providing access tokens
///
/// This trait allows dependency injection and testing with mock providers.
#[async_trait]
pub trait AccessTokenProvider: Send + Sync {
    /// Get a valid access token
    ///
    /// This method should handle token refresh if needed.
    async fn access_token(&self) -> Result<String, ApiError>;
}

/// API authentication service with OAuth and keychain integration
pub struct ApiAuthService {
    oauth: Arc<OAuthService>,
}

impl ApiAuthService {
    /// Create a new API auth service
    ///
    /// # Arguments
    ///
    /// * `config` - OAuth configuration
    /// * `keychain` - Keychain provider for token storage
    /// * `service_name` - Keychain service name (e.g., "PulseArc.api")
    /// * `account_name` - Keychain account name (e.g., "main")
    ///
    /// # Returns
    ///
    /// Configured auth service
    pub fn new(
        config: OAuthConfig,
        keychain: Arc<KeychainProvider>,
        service_name: String,
        account_name: String,
    ) -> Self {
        let oauth = Arc::new(OAuthService::new(
            config,
            keychain,
            service_name,
            account_name,
            300, // Refresh 5 minutes before expiry
        ));

        Self { oauth }
    }

    /// Initialize the service (load tokens from keychain)
    ///
    /// Should be called on startup.
    ///
    /// # Returns
    ///
    /// `true` if tokens were loaded, `false` if no tokens exist
    ///
    /// # Errors
    ///
    /// Returns error if keychain access fails
    pub async fn initialize(&self) -> Result<bool, ApiError> {
        self.oauth
            .initialize()
            .await
            .map_err(|e| ApiError::Auth(format!("Failed to initialize OAuth: {}", e)))
    }

    /// Start OAuth login flow
    ///
    /// Returns authorization URL that should be opened in a browser.
    ///
    /// # Returns
    ///
    /// Tuple of (authorization_url, state) for CSRF protection
    ///
    /// # Errors
    ///
    /// Returns error if authorization URL generation fails
    pub async fn start_login(&self) -> Result<(String, String), ApiError> {
        info!("Starting API authentication flow");

        self.oauth
            .start_login()
            .await
            .map_err(|e| ApiError::Auth(format!("Failed to start login: {}", e)))
    }

    /// Complete OAuth login (handle callback)
    ///
    /// Called when app receives redirect callback with authorization code.
    ///
    /// # Arguments
    ///
    /// * `code` - Authorization code from callback URL
    /// * `state` - State parameter from callback URL (CSRF protection)
    ///
    /// # Errors
    ///
    /// Returns error if token exchange fails
    pub async fn complete_login(&self, code: &str, state: &str) -> Result<(), ApiError> {
        debug!("Completing API authentication");

        self.oauth
            .complete_login(code, state)
            .await
            .map_err(|e| ApiError::Auth(format!("Failed to complete login: {}", e)))?;

        info!("API authentication successful");
        Ok(())
    }

    /// Check if user is authenticated
    pub async fn is_authenticated(&self) -> bool {
        self.oauth.is_authenticated().await
    }

    /// Logout (clear all tokens)
    ///
    /// # Errors
    ///
    /// Returns error if keychain deletion fails
    pub async fn logout(&self) -> Result<(), ApiError> {
        self.oauth.logout().await.map_err(|e| ApiError::Auth(format!("Failed to logout: {}", e)))
    }

    /// Start background auto-refresh task
    pub fn start_auto_refresh(&self) {
        self.oauth.start_auto_refresh();
    }

    /// Stop background auto-refresh task if running
    pub fn stop_auto_refresh(&self) {
        self.oauth.stop_auto_refresh();
    }
}

#[async_trait]
impl AccessTokenProvider for ApiAuthService {
    async fn access_token(&self) -> Result<String, ApiError> {
        self.oauth
            .get_access_token()
            .await
            .map_err(|e| ApiError::Auth(format!("Failed to get access token: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MockAuthProvider {
        token: String,
    }

    #[async_trait]
    impl AccessTokenProvider for MockAuthProvider {
        async fn access_token(&self) -> Result<String, ApiError> {
            Ok(self.token.clone())
        }
    }

    #[tokio::test]
    async fn test_mock_auth_provider() {
        let provider = MockAuthProvider { token: "test-token".to_string() };

        let token = provider.access_token().await.unwrap();
        assert_eq!(token, "test-token");
    }
}
