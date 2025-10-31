//! OAuth 2.0 client implementation with PKCE support
//!
//! Handles browser-based authorization flow with OAuth providers, including:
//! - PKCE challenge generation
//! - Browser authorization URL building
//! - Authorization code exchange
//! - Token refresh

use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client;
use tokio::sync::Mutex;

use super::pkce::PKCEChallenge;
use super::traits::OAuthClientTrait;
use super::types::{OAuthConfig, OAuthError, TokenResponse, TokenSet};

/// Error type for OAuth client operations
#[derive(Debug)]
pub enum OAuthClientError {
    /// HTTP request failed
    RequestFailed(reqwest::Error),

    /// OAuth server returned an error
    OAuthError(OAuthError),

    /// State parameter mismatch (CSRF attack detected)
    StateMismatch { expected: String, received: String },

    /// Failed to parse response
    ParseError(String),

    /// No refresh token available
    NoRefreshToken,

    /// Invalid configuration
    ConfigError(String),

    /// PKCE challenge generation failed
    PKCEError(String),
}

impl std::fmt::Display for OAuthClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestFailed(e) => write!(f, "HTTP request failed: {e}"),
            Self::OAuthError(e) => write!(f, "OAuth error: {e}"),
            Self::StateMismatch { expected, received } => {
                write!(f, "State mismatch (CSRF): expected {expected}, received {received}")
            }
            Self::ParseError(msg) => write!(f, "Parse error: {msg}"),
            Self::NoRefreshToken => write!(f, "No refresh token available"),
            Self::ConfigError(msg) => write!(f, "Configuration error: {msg}"),
            Self::PKCEError(msg) => write!(f, "PKCE generation error: {msg}"),
        }
    }
}

impl std::error::Error for OAuthClientError {}

impl From<reqwest::Error> for OAuthClientError {
    fn from(err: reqwest::Error) -> Self {
        Self::RequestFailed(err)
    }
}

/// OAuth 2.0 client with PKCE support
///
/// Supports Auth0, Google, Microsoft, and other standard OAuth 2.0 providers.
/// Implements RFC 6749 (OAuth 2.0) and RFC 7636 (PKCE).
#[derive(Debug, Clone)]
pub struct OAuthClient {
    config: OAuthConfig,
    client: Option<Client>,
    current_challenge: Arc<Mutex<Option<PKCEChallenge>>>,
}

impl OAuthClient {
    /// Create a new OAuth client with the given configuration
    ///
    /// # Arguments
    /// * `config` - OAuth configuration (domain, client_id, redirect_uri, etc.)
    ///
    /// # Examples
    /// ```
    /// use pulsearc_common::auth::{OAuthClient, OAuthConfig};
    ///
    /// let config = OAuthConfig::new(
    ///     "dev-test.us.auth0.com".to_string(),
    ///     "client_id".to_string(),
    ///     "http://localhost:3000/callback".to_string(),
    ///     vec!["openid".to_string()],
    ///     None,
    /// );
    /// let client = OAuthClient::new(config);
    /// ```
    #[must_use]
    pub fn new(config: OAuthConfig) -> Self {
        let client = if std::env::var_os("PULSEARC_OAUTH_DISABLE_HTTP").is_some() {
            None
        } else {
            let builder = Client::builder().timeout(std::time::Duration::from_secs(30));
            let builder = if std::env::var_os("PULSEARC_DISABLE_PROXY").is_some() {
                builder.no_proxy()
            } else {
                builder
            };
            Some(builder.build().unwrap_or_else(|_| Client::new()))
        };

        Self { config, client, current_challenge: Arc::new(Mutex::new(None)) }
    }

    /// Generate authorization URL for browser-based login
    ///
    /// Opens the system browser with OAuth provider's authorization page.
    /// User will be redirected to `redirect_uri` after authentication.
    ///
    /// # Returns
    /// Tuple of (authorization_url, state) where state must be validated in
    /// callback
    ///
    /// # Errors
    /// Returns error if PKCE challenge generation fails
    ///
    /// # Examples
    /// ```
    /// # use pulsearc_common::auth::OAuthClient;
    /// # use pulsearc_common::auth::OAuthConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = OAuthConfig::new("dev-test.us.auth0.com".to_string(), "client".to_string(), "http://localhost".to_string(), vec![], None);
    /// let client = OAuthClient::new(config);
    /// let (url, state) = client.generate_authorization_url().await?;
    /// // Open URL in browser, save state for validation
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_authorization_url(&self) -> Result<(String, String), OAuthClientError> {
        // Generate new PKCE challenge
        let challenge =
            PKCEChallenge::generate().map_err(|e| OAuthClientError::PKCEError(e.to_string()))?;
        let state = challenge.state.clone();

        // Store challenge for later token exchange
        *self.current_challenge.lock().await = Some(challenge.clone());

        // Build authorization URL with query parameters
        let scope_string = self.config.scope_string();

        let mut params = vec![
            ("response_type".to_string(), "code".to_string()),
            ("client_id".to_string(), self.config.client_id.clone()),
            ("redirect_uri".to_string(), self.config.redirect_uri.clone()),
            ("scope".to_string(), scope_string),
            ("state".to_string(), state.clone()),
            ("code_challenge".to_string(), challenge.code_challenge.clone()),
            ("code_challenge_method".to_string(), challenge.challenge_method().to_string()),
        ];

        // Add audience if configured (for API access)
        if let Some(audience) = &self.config.audience {
            params.push(("audience".to_string(), audience.clone()));
        }

        params.extend(self.config.extra_authorize_params().iter().cloned());

        let query_string = params
            .iter()
            .map(|(k, v)| format!("{k}={}", urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!("{}?{}", self.config.authorization_url(), query_string);

        Ok((url, state))
    }

    /// Exchange authorization code for tokens
    ///
    /// Called after user completes browser authorization and is redirected
    /// back. Validates state parameter and exchanges authorization code for
    /// access/refresh tokens.
    ///
    /// # Arguments
    /// * `code` - Authorization code from redirect callback
    /// * `state` - State parameter from redirect (for CSRF validation)
    ///
    /// # Returns
    /// `TokenSet` containing access_token, refresh_token (if issued), and
    /// metadata
    ///
    /// # Errors
    /// Returns error if:
    /// - State mismatch (CSRF attack)
    /// - Token exchange fails
    /// - Response parsing fails
    pub async fn exchange_code_for_tokens(
        &self,
        code: &str,
        state: &str,
    ) -> Result<TokenSet, OAuthClientError> {
        // Retrieve and validate challenge
        let challenge =
            self.current_challenge.lock().await.take().ok_or_else(|| {
                OAuthClientError::ConfigError("No PKCE challenge found".to_string())
            })?;

        // Validate state parameter (CSRF protection)
        if challenge.state != state {
            return Err(OAuthClientError::StateMismatch {
                expected: challenge.state,
                received: state.to_string(),
            });
        }

        // Prepare token exchange request
        let mut request_body = vec![
            ("grant_type".to_string(), "authorization_code".to_string()),
            ("client_id".to_string(), self.config.client_id.clone()),
            ("code".to_string(), code.to_string()),
            ("redirect_uri".to_string(), self.config.redirect_uri.clone()),
            ("code_verifier".to_string(), challenge.code_verifier.clone()),
        ];

        if let Some(secret) = self.config.client_secret() {
            request_body.push(("client_secret".to_string(), secret.to_string()));
        }

        request_body.extend(self.config.extra_token_params().iter().cloned());

        // Execute token exchange
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| OAuthClientError::ConfigError("HTTP client disabled".to_string()))?;
        let response = client.post(self.config.token_url()).form(&request_body).send().await?;

        // Handle OAuth errors
        if !response.status().is_success() {
            let error: OAuthError =
                response.json().await.map_err(|e| OAuthClientError::ParseError(e.to_string()))?;
            return Err(OAuthClientError::OAuthError(error));
        }

        // Parse token response
        let token_response: TokenResponse =
            response.json().await.map_err(|e| OAuthClientError::ParseError(e.to_string()))?;

        Ok(token_response.into())
    }

    /// Refresh access token using refresh token
    ///
    /// Used for obtaining new access tokens without user interaction.
    /// Should be called before current access token expires (typically 5 min
    /// before).
    ///
    /// # Arguments
    /// * `refresh_token` - Refresh token from previous authorization
    ///
    /// # Returns
    /// New `TokenSet` with updated access token and possibly new refresh token
    ///
    /// # Errors
    /// Returns error if:
    /// - No refresh token provided
    /// - Refresh fails
    /// - Token is invalid/revoked
    pub async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenSet, OAuthClientError> {
        if refresh_token.is_empty() {
            return Err(OAuthClientError::NoRefreshToken);
        }

        // Prepare refresh request
        let mut params = vec![
            ("grant_type".to_string(), "refresh_token".to_string()),
            ("client_id".to_string(), self.config.client_id.clone()),
            ("refresh_token".to_string(), refresh_token.to_string()),
        ];

        if let Some(secret) = self.config.client_secret() {
            params.push(("client_secret".to_string(), secret.to_string()));
        }

        params.extend(self.config.extra_token_params().iter().cloned());

        // Execute refresh
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| OAuthClientError::ConfigError("HTTP client disabled".to_string()))?;
        let response = client.post(self.config.token_url()).form(&params).send().await?;

        // Handle errors
        if !response.status().is_success() {
            let error: OAuthError =
                response.json().await.map_err(|e| OAuthClientError::ParseError(e.to_string()))?;
            return Err(OAuthClientError::OAuthError(error));
        }

        // Parse response
        let token_response: TokenResponse =
            response.json().await.map_err(|e| OAuthClientError::ParseError(e.to_string()))?;

        Ok(token_response.into())
    }

    /// Get the configured redirect URI
    #[must_use]
    pub fn redirect_uri(&self) -> &str {
        &self.config.redirect_uri
    }

    /// Get a reference to the OAuth configuration
    #[must_use]
    pub fn config(&self) -> &OAuthConfig {
        &self.config
    }
}

// Implement OAuthClientTrait for OAuthClient
#[async_trait]
impl OAuthClientTrait for OAuthClient {
    async fn generate_authorization_url(&self) -> Result<(String, String), OAuthClientError> {
        self.generate_authorization_url().await
    }

    async fn exchange_code_for_tokens(
        &self,
        code: &str,
        state: &str,
    ) -> Result<TokenSet, OAuthClientError> {
        self.exchange_code_for_tokens(code, state).await
    }

    async fn refresh_access_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenSet, OAuthClientError> {
        self.refresh_access_token(refresh_token).await
    }

    fn redirect_uri(&self) -> &str {
        self.redirect_uri()
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for auth::client.
    use std::sync::Once;

    use super::*;

    fn disable_oauth_http() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            std::env::set_var("PULSEARC_DISABLE_PROXY", "1");
            std::env::set_var("PULSEARC_OAUTH_DISABLE_HTTP", "1");
        });
    }

    fn create_test_config() -> OAuthConfig {
        disable_oauth_http();
        OAuthConfig::new(
            "dev-test.us.auth0.com".to_string(),
            "test_client_id".to_string(),
            "http://localhost:3000/callback".to_string(),
            vec!["openid".to_string(), "profile".to_string()],
            Some("https://api.pulsearc.ai".to_string()),
        )
    }

    /// Validates `OAuthClient::new` behavior for the generate authorization url
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Ensures `url.starts_with("https://dev-test.us.auth0.com/authorize?")`
    ///   evaluates to true.
    /// - Ensures `url.contains("response_type=code")` evaluates to true.
    /// - Ensures `url.contains("client_id=test_client_id")` evaluates to true.
    /// - Ensures `url.contains("code_challenge=")` evaluates to true.
    /// - Ensures `url.contains("code_challenge_method=S256")` evaluates to
    ///   true.
    /// - Ensures `url.contains(&format!("state={state}"))` evaluates to true.
    /// - Ensures `url.contains("audience=https%3A%2F%2Fapi.pulsearc.ai")`
    ///   evaluates to true.
    #[tokio::test]
    async fn test_generate_authorization_url() {
        let config = create_test_config();
        let client = OAuthClient::new(config);

        let result = client.generate_authorization_url().await;
        assert!(result.is_ok());

        let (url, state) = result.unwrap();

        assert!(url.starts_with("https://dev-test.us.auth0.com/authorize?"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=test_client_id"));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains(&format!("state={state}")));
        assert!(url.contains("audience=https%3A%2F%2Fapi.pulsearc.ai"));
    }

    /// Validates `OAuthClient::new` behavior for the state validation scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_ok()` evaluates to true.
    /// - Ensures `matches!(result, Err(OAuthClientError::StateMismatch { ..
    ///   }))` evaluates to true.
    #[tokio::test]
    async fn test_state_validation() {
        let config = create_test_config();
        let client = OAuthClient::new(config);

        let result = client.generate_authorization_url().await;
        assert!(result.is_ok());

        // Attempt exchange with wrong state
        let result = client.exchange_code_for_tokens("test_code", "wrong_state").await;

        assert!(matches!(result, Err(OAuthClientError::StateMismatch { .. })));
    }

    /// Validates `OAuthClient::new` behavior for the oauth client creation
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `client.redirect_uri()` equals `"http://localhost:3000/callback"`.
    #[test]
    fn test_oauth_client_creation() {
        let config = create_test_config();
        let client = OAuthClient::new(config);

        assert_eq!(client.redirect_uri(), "http://localhost:3000/callback");
    }

    /// Validates `OAuthClient::new` behavior for the oauth client config access
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `client.config().client_id` equals `"test_client_id"`.
    /// - Confirms `client.config().domain` equals `"dev-test.us.auth0.com"`.
    #[test]
    fn test_oauth_client_config_access() {
        let config = create_test_config();
        let client = OAuthClient::new(config);

        assert_eq!(client.config().client_id, "test_client_id");
        assert_eq!(client.config().domain, "dev-test.us.auth0.com");
    }

    /// Validates `OAuthClient::new` behavior for the refresh with empty token
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(result, Err(OAuthClientError::NoRefreshToken))`
    ///   evaluates to true.
    #[tokio::test]
    async fn test_refresh_with_empty_token() {
        let config = create_test_config();
        let client = OAuthClient::new(config);

        let result = client.refresh_access_token("").await;
        assert!(matches!(result, Err(OAuthClientError::NoRefreshToken)));
    }
}
