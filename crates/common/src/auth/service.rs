//! High-level OAuth service orchestrator
//!
//! Combines OAuth client, keychain storage, and token manager
//! into a single service for easy integration.

use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::info;

use super::client::{OAuthClient, OAuthClientError};
use super::token_manager::{TokenManager, TokenManagerError};
use super::types::{OAuthConfig, TokenSet};
use crate::auth::traits::KeychainTrait;
use crate::security::KeychainProvider;

// Note: OAuthCallbackServer is provided by integrations crate, not common
// pub use crate::integrations::calendar::core::oauth::OAuthCallbackServer;

/// Error type for OAuth service operations
#[derive(Debug)]
pub enum OAuthServiceError {
    /// Token manager error
    TokenManager(TokenManagerError),

    /// OAuth client error
    OAuthClient(OAuthClientError),

    /// Configuration error
    ConfigError(String),

    /// Browser launch failed
    BrowserError(String),
}

impl std::fmt::Display for OAuthServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TokenManager(e) => write!(f, "Token manager error: {e}"),
            Self::OAuthClient(e) => write!(f, "OAuth client error: {e}"),
            Self::ConfigError(msg) => write!(f, "Configuration error: {msg}"),
            Self::BrowserError(msg) => write!(f, "Browser launch failed: {msg}"),
        }
    }
}

impl std::error::Error for OAuthServiceError {}

impl From<TokenManagerError> for OAuthServiceError {
    fn from(err: TokenManagerError) -> Self {
        Self::TokenManager(err)
    }
}

impl From<OAuthClientError> for OAuthServiceError {
    fn from(err: OAuthClientError) -> Self {
        Self::OAuthClient(err)
    }
}

/// OAuth service for desktop authentication
///
/// High-level service that orchestrates:
/// - Browser-based OAuth PKCE flow
/// - Token storage in system keychain
/// - Automatic token refresh
/// - Authentication state management
#[derive(Clone)]
pub struct OAuthService<K = KeychainProvider>
where
    K: KeychainTrait + 'static,
{
    oauth_client: Arc<OAuthClient>,
    token_manager: Arc<TokenManager<OAuthClient, K>>,
    pending_state: Arc<RwLock<Option<String>>>,
}

impl<K> OAuthService<K>
where
    K: KeychainTrait + 'static,
{
    /// Create a new OAuth service
    ///
    /// # Arguments
    /// * `config` - OAuth configuration (domain, client_id, etc.)
    /// * `keychain` - Keychain provider for token storage
    /// * `service_name` - Keychain service name (e.g., "PulseArc.api")
    /// * `account_name` - Keychain account name (e.g., "main")
    /// * `refresh_threshold_seconds` - Refresh tokens this many seconds before
    ///   expiry (default: 300)
    ///
    /// # Examples
    /// ```no_run
    /// // Note: KeychainProvider must be created by application code
    /// use std::sync::Arc;
    ///
    /// use pulsearc_common::auth::{OAuthConfig, OAuthService};
    /// use pulsearc_common::security::KeychainProvider;
    ///
    /// let config = OAuthConfig::new(
    ///     "dev-test.us.auth0.com".to_string(),
    ///     "client_id".to_string(),
    ///     "http://localhost:3000/callback".to_string(),
    ///     vec!["openid".to_string(), "profile".to_string(), "offline_access".to_string()],
    ///     Some("https://api.pulsearc.ai".to_string()),
    /// );
    ///
    /// let keychain = Arc::new(KeychainProvider::new("PulseArc".to_string()));
    /// let service =
    ///     OAuthService::new(config, keychain, "PulseArc.api".to_string(), "main".to_string(), 300);
    /// ```
    #[must_use]
    pub fn new(
        config: OAuthConfig,
        keychain: Arc<K>,
        service_name: String,
        account_name: String,
        refresh_threshold_seconds: i64,
    ) -> Self {
        let oauth_client = OAuthClient::new(config);

        let token_manager = TokenManager::new(
            oauth_client.clone(),
            keychain,
            service_name,
            account_name,
            refresh_threshold_seconds,
        );

        Self {
            oauth_client: Arc::new(oauth_client),
            token_manager: Arc::new(token_manager),
            pending_state: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize service (load tokens from keychain)
    ///
    /// Should be called on app startup.
    ///
    /// # Returns
    /// `true` if tokens were loaded, `false` if no tokens exist
    ///
    /// # Errors
    /// Returns error if keychain access fails
    pub async fn initialize(&self) -> Result<bool, OAuthServiceError> {
        self.token_manager.initialize().await.map_err(Into::into)
    }

    /// Start OAuth login flow
    ///
    /// Generates authorization URL. Caller is responsible for opening browser.
    /// User will authenticate and be redirected to `redirect_uri`.
    ///
    /// # Returns
    /// Tuple of (authorization_url, state) where state is used for CSRF
    /// protection
    ///
    /// # Errors
    /// Returns error if PKCE challenge generation fails
    pub async fn start_login(&self) -> Result<(String, String), OAuthServiceError> {
        // Generate authorization URL with PKCE challenge
        let (auth_url, state) = self.oauth_client.generate_authorization_url().await?;

        // Store state for validation in callback
        *self.pending_state.write().await = Some(state.clone());

        info!("Generated OAuth authorization URL");

        Ok((auth_url, state))
    }

    /// Complete OAuth login (handle callback)
    ///
    /// Called when app receives redirect callback with authorization code.
    /// Validates state, exchanges code for tokens, and stores in keychain.
    ///
    /// # Arguments
    /// * `code` - Authorization code from callback URL
    /// * `state` - State parameter from callback URL (CSRF protection)
    ///
    /// # Returns
    /// `TokenSet` containing access/refresh tokens
    ///
    /// # Errors
    /// Returns error if:
    /// - State mismatch (CSRF attack)
    /// - Token exchange fails
    /// - Keychain storage fails
    pub async fn complete_login(
        &self,
        code: &str,
        state: &str,
    ) -> Result<TokenSet, OAuthServiceError> {
        // Validate state parameter
        let expected_state = self
            .pending_state
            .write()
            .await
            .take()
            .ok_or_else(|| OAuthServiceError::ConfigError("No pending login".to_string()))?;

        if expected_state != state {
            return Err(OAuthServiceError::OAuthClient(OAuthClientError::StateMismatch {
                expected: expected_state,
                received: state.to_string(),
            }));
        }

        // Exchange code for tokens
        let tokens = self.oauth_client.exchange_code_for_tokens(code, state).await?;

        // Store tokens in keychain
        self.token_manager.store_tokens(tokens.clone()).await?;

        info!("OAuth login completed successfully");

        Ok(tokens)
    }

    /// Get current access token (with auto-refresh)
    ///
    /// Primary method for retrieving access tokens.
    /// Automatically refreshes if token is expired or near expiry.
    ///
    /// # Returns
    /// Valid access token string
    ///
    /// # Errors
    /// Returns error if:
    /// - Not authenticated
    /// - Token refresh fails
    pub async fn get_access_token(&self) -> Result<String, OAuthServiceError> {
        self.token_manager.get_access_token().await.map_err(Into::into)
    }

    /// Get current token set (without auto-refresh)
    ///
    /// # Returns
    /// Current `TokenSet` or None if not authenticated
    pub async fn get_tokens(&self) -> Option<TokenSet> {
        self.token_manager.get_tokens().await
    }

    /// Check if user is authenticated
    #[must_use]
    pub async fn is_authenticated(&self) -> bool {
        self.token_manager.is_authenticated().await
    }

    /// Logout (clear all tokens)
    ///
    /// # Errors
    /// Returns error if keychain deletion fails
    pub async fn logout(&self) -> Result<(), OAuthServiceError> {
        // Clear pending state (in case logout happens during login flow)
        *self.pending_state.write().await = None;

        // Clear tokens from keychain and memory
        self.token_manager.clear_tokens().await.map_err(Into::into)
    }

    /// Start background auto-refresh task
    ///
    /// Spawns a background task that sleeps until tokens need refreshing
    /// and refreshes them automatically.
    ///
    /// # Example
    /// ```no_run
    /// # use pulsearc_common::auth::{OAuthService, OAuthConfig};
    /// # use pulsearc_common::security::KeychainProvider;
    /// # use std::sync::Arc;
    /// # async fn example() {
    /// # let config = OAuthConfig::new("dev-test.us.auth0.com".to_string(), "client".to_string(), "http://localhost".to_string(), vec![], None);
    /// # let keychain = Arc::new(KeychainProvider::new("PulseArc".to_string()));
    /// let service = OAuthService::new(config, keychain, "PulseArc.api".to_string(), "main".to_string(), 300);
    /// service.start_auto_refresh();
    /// // Auto-refresh now runs in background
    /// # }
    /// ```
    pub fn start_auto_refresh(&self) {
        let token_manager = self.token_manager.clone();
        tokio::spawn(async move {
            token_manager.start_auto_refresh().await;
        });
    }

    /// Get token manager for advanced operations
    #[must_use]
    pub fn token_manager(&self) -> Arc<TokenManager<OAuthClient, K>> {
        self.token_manager.clone()
    }

    /// Get OAuth client for advanced operations
    #[must_use]
    pub fn oauth_client(&self) -> Arc<OAuthClient> {
        self.oauth_client.clone()
    }

    /// Get seconds until token expiry
    #[must_use]
    pub async fn seconds_until_expiry(&self) -> Option<i64> {
        self.token_manager.seconds_until_expiry().await
    }

    /// Clear pending state (useful for canceling login flow)
    pub async fn clear_pending_state(&self) {
        *self.pending_state.write().await = None;
    }

    /// Check if there's a pending login flow
    #[must_use]
    pub async fn has_pending_login(&self) -> bool {
        self.pending_state.read().await.is_some()
    }
}

impl std::fmt::Debug for OAuthService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthService")
            .field("oauth_client", &"OAuthClient")
            .field("token_manager", &"TokenManager")
            .finish()
    }
}

#[cfg(all(test, feature = "platform"))]
mod tests {
    //! Unit tests for auth::service.
    use std::sync::Once;

    use super::*;
    use crate::testing::MockKeychainProvider;

    fn disable_proxy() {
        static INIT: Once = Once::new();
        INIT.call_once(|| std::env::set_var("PULSEARC_DISABLE_PROXY", "1"));
    }

    fn create_test_service() -> OAuthService<MockKeychainProvider> {
        disable_proxy();
        let config = OAuthConfig::new(
            "dev-test.us.auth0.com".to_string(),
            "test_client".to_string(),
            "http://localhost:3000/callback".to_string(),
            vec!["openid".to_string(), "offline_access".to_string()],
            Some("https://api.pulsearc.ai".to_string()),
        );

        // Use in-memory keychain mock for deterministic tests
        let test_service = format!("PulseArcTest.oauth.{}", uuid::Uuid::new_v4());
        let keychain = Arc::new(MockKeychainProvider::new(test_service));

        OAuthService::new(
            config,
            keychain,
            "test.service".to_string(),
            "test.account".to_string(),
            300,
        )
    }

    /// Validates the oauth service creation scenario.
    ///
    /// Assertions:
    /// - Ensures `!service.is_authenticated().await` evaluates to true.
    #[tokio::test]
    async fn test_oauth_service_creation() {
        let service = create_test_service();
        assert!(!service.is_authenticated().await);
    }

    /// Validates the start login flow scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Ensures `url.starts_with("https://dev-test.us.auth0.com/authorize?")`
    ///   evaluates to true.
    /// - Ensures `!state.is_empty()` evaluates to true.
    /// - Ensures `service.has_pending_login().await` evaluates to true.
    #[tokio::test]
    async fn test_start_login_flow() {
        let service = create_test_service();

        let result = service.start_login().await;
        assert!(result.is_ok());

        let (url, state) = result.unwrap();
        assert!(url.starts_with("https://dev-test.us.auth0.com/authorize?"));
        assert!(!state.is_empty());

        // Should have pending state
        assert!(service.has_pending_login().await);
    }

    /// Validates `OAuthServiceError::ConfigError` behavior for the complete
    /// login no pending state scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(result, Err(OAuthServiceError::ConfigError(_)))`
    ///   evaluates to true.
    #[tokio::test]
    async fn test_complete_login_no_pending_state() {
        let service = create_test_service();

        // Try to complete login without starting it
        let result = service.complete_login("code123", "state456").await;
        assert!(matches!(result, Err(OAuthServiceError::ConfigError(_))));
    }

    /// Validates `TokenSet::new` behavior for the logout scenario.
    ///
    /// Assertions:
    /// - Ensures `!service.is_authenticated().await` evaluates to true.
    #[tokio::test]
    async fn test_logout() {
        let service = create_test_service();

        // Store some tokens
        let tokens =
            TokenSet::new("access".to_string(), Some("refresh".to_string()), None, 3600, None);
        service.token_manager.store_tokens(tokens).await.unwrap();

        // Logout
        service.logout().await.unwrap();

        // Should not be authenticated
        assert!(!service.is_authenticated().await);
    }

    /// Validates the clear pending state scenario.
    ///
    /// Assertions:
    /// - Ensures `service.has_pending_login().await` evaluates to true.
    /// - Ensures `!service.has_pending_login().await` evaluates to true.
    #[tokio::test]
    async fn test_clear_pending_state() {
        let service = create_test_service();

        // Start login
        service.start_login().await.unwrap();
        assert!(service.has_pending_login().await);

        // Clear pending state
        service.clear_pending_state().await;
        assert!(!service.has_pending_login().await);
    }

    /// Validates `OAuthServiceError::TokenManager` behavior for the get access
    /// token not authenticated scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!( result,
    ///   Err(OAuthServiceError::TokenManager(TokenManagerError::NotAuthenticated))
    ///   )` evaluates to true.
    #[tokio::test]
    async fn test_get_access_token_not_authenticated() {
        let service = create_test_service();

        let result = service.get_access_token().await;
        assert!(matches!(
            result,
            Err(OAuthServiceError::TokenManager(TokenManagerError::NotAuthenticated))
        ));
    }
}
