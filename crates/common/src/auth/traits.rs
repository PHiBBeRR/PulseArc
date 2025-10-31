//! Traits for OAuth and keychain operations
//!
//! These traits enable dependency injection and testing by abstracting
//! external dependencies (OAuth servers, system keychain).

use async_trait::async_trait;

use super::client::OAuthClientError;
use super::types::TokenSet;

/// Trait for OAuth client operations
///
/// This trait abstracts OAuth operations to enable testing with mock
/// implementations and to support different OAuth providers or configurations.
#[async_trait]
pub trait OAuthClientTrait: Send + Sync {
    /// Generate authorization URL for browser-based login
    ///
    /// # Returns
    /// Tuple of (authorization_url, state) where state must be validated in
    /// callback
    ///
    /// # Errors
    /// Returns error if PKCE challenge generation fails
    async fn generate_authorization_url(&self) -> Result<(String, String), OAuthClientError>;

    /// Exchange authorization code for tokens
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
    /// Returns error if state mismatch, token exchange fails, or response
    /// parsing fails
    async fn exchange_code_for_tokens(
        &self,
        code: &str,
        state: &str,
    ) -> Result<TokenSet, OAuthClientError>;

    /// Refresh access token using refresh token
    ///
    /// # Arguments
    /// * `refresh_token` - Refresh token from previous authorization
    ///
    /// # Returns
    /// New `TokenSet` with updated access token and possibly new refresh token
    ///
    /// # Errors
    /// Returns error if refresh fails or token is invalid/revoked
    async fn refresh_access_token(&self, refresh_token: &str)
        -> Result<TokenSet, OAuthClientError>;

    /// Get the configured redirect URI
    fn redirect_uri(&self) -> &str;
}

/// Trait for keychain operations
///
/// This trait abstracts keychain/credential storage to enable testing with
/// mock implementations and to support different storage backends.
#[async_trait]
pub trait KeychainTrait: Send + Sync {
    /// Store OAuth tokens
    ///
    /// # Arguments
    /// * `account` - Account identifier (e.g., user email or token reference)
    /// * `tokens` - OAuth tokens to store
    ///
    /// # Errors
    /// Returns error if storage fails
    async fn store_tokens(&self, account: &str, tokens: &TokenSet) -> Result<(), String>;

    /// Retrieve OAuth tokens
    ///
    /// # Arguments
    /// * `account` - Account identifier
    ///
    /// # Returns
    /// The stored OAuth tokens
    ///
    /// # Errors
    /// Returns error if tokens don't exist or retrieval fails
    async fn retrieve_tokens(&self, account: &str) -> Result<TokenSet, String>;

    /// Delete OAuth tokens
    ///
    /// # Arguments
    /// * `account` - Account identifier
    ///
    /// # Errors
    /// Returns error if deletion fails
    async fn delete_tokens(&self, account: &str) -> Result<(), String>;

    /// Check if OAuth tokens exist for the given account
    ///
    /// # Arguments
    /// * `account` - Account identifier
    ///
    /// # Returns
    /// `true` if tokens exist, `false` otherwise
    async fn has_tokens(&self, account: &str) -> bool;
}
