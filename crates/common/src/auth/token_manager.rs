//! Token manager with automatic refresh
//!
//! Manages OAuth token lifecycle:
//! - Token retrieval from keychain
//! - Auto-refresh before expiry (configurable threshold, default 5 min)
//! - Background refresh task
//! - Token validation

use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::{debug, error, info};

use super::client::OAuthClientError;
use super::traits::{KeychainTrait, OAuthClientTrait};
use super::types::TokenSet;

/// Error type for token manager operations
#[derive(Debug)]
pub enum TokenManagerError {
    /// Keychain operation failed
    KeychainError(String),

    /// OAuth operation failed
    OAuthError(OAuthClientError),

    /// No tokens available (not authenticated)
    NotAuthenticated,

    /// Token refresh failed
    RefreshFailed(String),

    /// No refresh token available
    NoRefreshToken,
}

impl std::fmt::Display for TokenManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeychainError(e) => write!(f, "Keychain error: {e}"),
            Self::OAuthError(e) => write!(f, "OAuth error: {e}"),
            Self::NotAuthenticated => write!(f, "Not authenticated (no tokens)"),
            Self::RefreshFailed(msg) => write!(f, "Token refresh failed: {msg}"),
            Self::NoRefreshToken => write!(f, "No refresh token available"),
        }
    }
}

impl std::error::Error for TokenManagerError {}

impl From<OAuthClientError> for TokenManagerError {
    fn from(err: OAuthClientError) -> Self {
        Self::OAuthError(err)
    }
}

impl From<String> for TokenManagerError {
    fn from(err: String) -> Self {
        Self::KeychainError(err)
    }
}

/// Token manager with auto-refresh capabilities
///
/// Manages the full token lifecycle:
/// 1. Stores tokens in system keychain via KeychainProvider trait
/// 2. Automatically refreshes tokens before expiry
/// 3. Provides thread-safe access to current tokens
/// 4. Runs background refresh task
pub struct TokenManager<C: OAuthClientTrait + 'static, K: KeychainTrait + 'static> {
    oauth_client: Arc<C>,
    keychain: Arc<K>,
    account_name: String,
    current_tokens: Arc<RwLock<Option<TokenSet>>>,
    refresh_threshold_seconds: i64,
}

impl<C: OAuthClientTrait + 'static, K: KeychainTrait + 'static> TokenManager<C, K> {
    /// Create a new token manager
    ///
    /// # Arguments
    /// * `oauth_client` - OAuth client for token refresh
    /// * `keychain` - Keychain provider for persistence
    /// * `account_name` - Keychain account name (e.g., "main" or token
    ///   reference ID)
    /// * `refresh_threshold_seconds` - Refresh tokens this many seconds before
    ///   expiry (default: 300 = 5 min)
    #[must_use]
    pub fn new(
        oauth_client: C,
        keychain: Arc<K>,
        account_name: String,
        refresh_threshold_seconds: i64,
    ) -> Self {
        Self {
            oauth_client: Arc::new(oauth_client),
            keychain,
            account_name,
            current_tokens: Arc::new(RwLock::new(None)),
            refresh_threshold_seconds,
        }
    }

    /// Initialize token manager by loading tokens from keychain
    ///
    /// Should be called on app startup. If tokens exist and are valid,
    /// they will be loaded into memory.
    ///
    /// # Errors
    /// Returns error if keychain access fails (not if tokens don't exist)
    pub async fn initialize(&self) -> Result<bool, TokenManagerError> {
        match self.keychain.retrieve_tokens(&self.account_name).await {
            Ok(tokens) => {
                *self.current_tokens.write().await = Some(tokens);
                info!("Token manager initialized with existing tokens");
                Ok(true)
            }
            Err(_) => {
                // No tokens stored yet - this is fine
                debug!("No existing tokens found in keychain");
                Ok(false)
            }
        }
    }

    /// Store new tokens (after successful OAuth flow)
    ///
    /// # Arguments
    /// * `tokens` - TokenSet to store
    ///
    /// # Errors
    /// Returns error if keychain storage fails
    pub async fn store_tokens(&self, tokens: TokenSet) -> Result<(), TokenManagerError> {
        // Store in keychain (now uses TokenSet directly)
        self.keychain.store_tokens(&self.account_name, &tokens).await?;

        // Update in-memory cache
        *self.current_tokens.write().await = Some(tokens);

        info!("Tokens stored successfully");

        Ok(())
    }

    /// Get current access token (with auto-refresh if needed)
    ///
    /// This is the primary method for retrieving access tokens.
    /// Automatically refreshes if token is expired or near expiry.
    ///
    /// # Returns
    /// Valid access token string
    ///
    /// # Errors
    /// Returns error if:
    /// - Not authenticated (no tokens)
    /// - Token refresh fails
    pub async fn get_access_token(&self) -> Result<String, TokenManagerError> {
        // Check if we need to refresh
        if self.should_refresh().await {
            self.refresh_tokens().await?;
        }

        // Get current token
        let tokens = self.current_tokens.read().await;
        tokens.as_ref().map(|t| t.access_token.clone()).ok_or(TokenManagerError::NotAuthenticated)
    }

    /// Get current token set (without auto-refresh)
    ///
    /// # Returns
    /// Current `TokenSet` or None if not authenticated
    pub async fn get_tokens(&self) -> Option<TokenSet> {
        self.current_tokens.read().await.clone()
    }

    /// Check if user is authenticated (has tokens)
    #[must_use]
    pub async fn is_authenticated(&self) -> bool {
        self.current_tokens.read().await.is_some()
    }

    /// Check if tokens should be refreshed
    ///
    /// Returns true if:
    /// - Tokens exist
    /// - Access token is expired or will expire within threshold
    async fn should_refresh(&self) -> bool {
        let tokens = self.current_tokens.read().await;
        match tokens.as_ref() {
            Some(t) => t.is_expired(self.refresh_threshold_seconds),
            None => false,
        }
    }

    /// Refresh access token using refresh token
    ///
    /// # Errors
    /// Returns error if refresh fails or no refresh token available
    pub async fn refresh_tokens(&self) -> Result<(), TokenManagerError> {
        // Get current refresh token
        let refresh_token = {
            let tokens = self.current_tokens.read().await;
            match tokens.as_ref() {
                Some(t) => t.refresh_token.clone().ok_or(TokenManagerError::NoRefreshToken)?,
                None => return Err(TokenManagerError::NotAuthenticated),
            }
        };

        // Execute refresh
        let new_tokens = self.oauth_client.refresh_access_token(&refresh_token).await?;

        // Store new tokens
        self.store_tokens(new_tokens).await?;

        info!("Successfully refreshed access token");

        Ok(())
    }

    /// Clear all tokens (logout)
    ///
    /// # Errors
    /// Returns error if keychain deletion fails
    pub async fn clear_tokens(&self) -> Result<(), TokenManagerError> {
        // Clear from keychain
        self.keychain.delete_tokens(&self.account_name).await?;

        // Clear from memory
        *self.current_tokens.write().await = None;

        info!("Tokens cleared (logged out)");

        Ok(())
    }

    /// Start background auto-refresh task
    ///
    /// Wakes up only when tokens need refreshing (no polling).
    /// Sleeps until refresh threshold is reached, then refreshes tokens.
    /// Runs indefinitely until the app shuts down.
    ///
    /// # Example
    /// ```no_run
    /// # use pulsearc_common::auth::{TokenManager, OAuthClientTrait, KeychainTrait};
    /// # async fn example<C: OAuthClientTrait, K: KeychainTrait>(
    /// #     token_manager: std::sync::Arc<TokenManager<C, K>>
    /// # ) {
    /// tokio::spawn(async move {
    ///     token_manager.start_auto_refresh().await;
    /// });
    /// # }
    /// ```
    pub async fn start_auto_refresh(self: Arc<Self>) {
        use tokio::time::{sleep, sleep_until, Instant};

        info!("Starting token auto-refresh background task");

        loop {
            // Calculate next wake time based on token expiry
            let wake_duration = {
                let tokens = self.current_tokens.read().await;
                match tokens.as_ref() {
                    Some(t) => {
                        // Calculate seconds until we should refresh
                        // (expiry time - refresh threshold)
                        match t.seconds_until_expiry() {
                            Some(seconds_until_expiry) => {
                                let seconds_until_refresh =
                                    seconds_until_expiry - self.refresh_threshold_seconds;

                                if seconds_until_refresh <= 0 {
                                    // Already need to refresh - do it immediately
                                    Duration::from_secs(0)
                                } else {
                                    // Sleep until refresh is needed
                                    Duration::from_secs(seconds_until_refresh as u64)
                                }
                            }
                            None => {
                                // No expiry set - check again in 60 seconds
                                Duration::from_secs(60)
                            }
                        }
                    }
                    None => {
                        // Not authenticated - check again in 60 seconds
                        Duration::from_secs(60)
                    }
                }
            };

            // Sleep until refresh is needed
            if wake_duration.as_secs() > 0 {
                debug!(
                    "Auto-refresh: Sleeping for {} seconds until next check",
                    wake_duration.as_secs()
                );
                sleep_until(Instant::now() + wake_duration).await;
            }

            // Check if we're still authenticated (user might have logged out during sleep)
            if !self.is_authenticated().await {
                continue;
            }

            // Refresh tokens if needed
            if self.should_refresh().await {
                info!("Auto-refresh: Token expiring soon, refreshing...");

                if let Err(e) = self.refresh_tokens().await {
                    error!("Auto-refresh failed: {e}");
                    // Retry after 60 seconds on failure
                    sleep(Duration::from_secs(60)).await;
                }
            }
        }
    }

    /// Get seconds until token expiry
    ///
    /// # Returns
    /// Number of seconds until expiry, or None if not authenticated
    #[must_use]
    pub async fn seconds_until_expiry(&self) -> Option<i64> {
        let tokens = self.current_tokens.read().await;
        tokens.as_ref().and_then(|t| t.seconds_until_expiry())
    }

    /// Get the refresh threshold in seconds
    #[must_use]
    pub fn refresh_threshold(&self) -> i64 {
        self.refresh_threshold_seconds
    }
}

#[cfg(all(test, feature = "platform"))]
mod tests {
    //! Unit tests for auth::token_manager.
    use std::sync::Once;

    use super::*;
    use crate::auth::{OAuthClient, OAuthConfig};
    use crate::testing::MockKeychainProvider;

    fn disable_oauth_http() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            std::env::set_var("PULSEARC_DISABLE_PROXY", "1");
            std::env::set_var("PULSEARC_OAUTH_DISABLE_HTTP", "1");
        });
    }

    fn create_test_manager() -> TokenManager<OAuthClient, MockKeychainProvider> {
        disable_oauth_http();
        let config = OAuthConfig::new(
            "dev-test.us.auth0.com".to_string(),
            "test_client".to_string(),
            "http://localhost:3000/callback".to_string(),
            vec!["openid".to_string()],
            None,
        );

        let oauth_client = OAuthClient::new(config);

        // Use in-memory keychain mock for deterministic tests
        let test_service = format!("PulseArcTest.oauth.{}", uuid::Uuid::new_v4());
        let keychain = Arc::new(MockKeychainProvider::new(test_service));

        TokenManager::new(oauth_client, keychain, "test.account".to_string(), 300)
    }

    /// Validates the token manager creation scenario.
    ///
    /// Assertions:
    /// - Ensures `!manager.is_authenticated().await` evaluates to true.
    #[tokio::test]
    async fn test_token_manager_creation() {
        let manager = create_test_manager();
        assert!(!manager.is_authenticated().await);
    }

    /// Validates `TokenSet::new` behavior for the store and retrieve tokens
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `manager.is_authenticated().await` evaluates to true.
    /// - Ensures `retrieved.is_some()` evaluates to true.
    /// - Confirms `retrieved.as_ref().map(|t| &t.access_token)` equals
    ///   `Some(&"access_token".to_string())`.
    #[tokio::test]
    async fn test_store_and_retrieve_tokens() {
        let manager = create_test_manager();

        let tokens = TokenSet::new(
            "access_token".to_string(),
            Some("refresh_token".to_string()),
            None,
            3600,
            None,
        );

        // Store tokens
        manager.store_tokens(tokens).await.unwrap();

        // Verify authentication
        assert!(manager.is_authenticated().await);

        // Get tokens
        let retrieved = manager.get_tokens().await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.as_ref().map(|t| &t.access_token), Some(&"access_token".to_string()));

        // Cleanup
        manager.clear_tokens().await.unwrap();
    }

    /// Validates `TokenSet::new` behavior for the clear tokens scenario.
    ///
    /// Assertions:
    /// - Ensures `!manager.is_authenticated().await` evaluates to true.
    #[tokio::test]
    async fn test_clear_tokens() {
        let manager = create_test_manager();

        // Store tokens
        let tokens =
            TokenSet::new("access".to_string(), Some("refresh".to_string()), None, 3600, None);
        manager.store_tokens(tokens).await.unwrap();

        // Clear
        manager.clear_tokens().await.unwrap();

        // Verify cleared
        assert!(!manager.is_authenticated().await);
    }

    /// Validates `TokenManagerError::NotAuthenticated` behavior for the not
    /// authenticated error scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(result, Err(TokenManagerError::NotAuthenticated))`
    ///   evaluates to true.
    #[tokio::test]
    async fn test_not_authenticated_error() {
        let manager = create_test_manager();

        // Try to get token when not authenticated
        let result = manager.get_access_token().await;
        assert!(matches!(result, Err(TokenManagerError::NotAuthenticated)));
    }

    /// Validates `TokenSet::new` behavior for the should refresh logic
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `!manager.should_refresh().await` evaluates to true.
    /// - Ensures `manager.should_refresh().await` evaluates to true.
    #[tokio::test]
    async fn test_should_refresh_logic() {
        let manager = create_test_manager();

        // No tokens - should not refresh
        assert!(!manager.should_refresh().await);

        // Store tokens with short expiry
        let tokens = TokenSet::new(
            "access".to_string(),
            Some("refresh".to_string()),
            None,
            60, // 1 minute - within 5 min threshold
            None,
        );
        manager.store_tokens(tokens).await.unwrap();

        // Should refresh (expires within threshold)
        assert!(manager.should_refresh().await);

        // Cleanup
        manager.clear_tokens().await.unwrap();
    }

    /// Validates `TokenSet::new` behavior for the no refresh token error
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(result, Err(TokenManagerError::NoRefreshToken))`
    ///   evaluates to true.
    #[tokio::test]
    async fn test_no_refresh_token_error() {
        let manager = create_test_manager();

        // Store tokens without refresh token
        let tokens = TokenSet::new("access".to_string(), None, None, 60, None);
        manager.store_tokens(tokens).await.unwrap();

        // Try to refresh
        let result = manager.refresh_tokens().await;
        assert!(matches!(result, Err(TokenManagerError::NoRefreshToken)));

        // Cleanup
        manager.clear_tokens().await.unwrap();
    }

    /// Validates the refresh threshold getter scenario.
    ///
    /// Assertions:
    /// - Confirms `manager.refresh_threshold()` equals `300`.
    #[tokio::test]
    async fn test_refresh_threshold_getter() {
        let manager = create_test_manager();
        assert_eq!(manager.refresh_threshold(), 300);
    }
}
