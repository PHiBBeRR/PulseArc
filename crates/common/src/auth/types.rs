//! OAuth 2.0 types and structures
//!
//! Defines unified data structures for OAuth tokens, responses, and
//! configuration. This module consolidates OAuth types used across calendar,
//! SAP, and main API integrations.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// OAuth 2.0 access and refresh tokens with metadata
///
/// Unified token type that combines the best features from calendar and macOS
/// implementations:
/// - Optional refresh token (some providers don't issue them)
/// - Both expires_in (duration) and expires_at (timestamp) for flexibility
/// - ID token support for OpenID Connect
/// - Scope tracking for granted permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    /// JWT access token for API authentication
    pub access_token: String,

    /// Refresh token for obtaining new access tokens
    /// Optional because some OAuth providers don't issue refresh tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    /// ID token (JWT) containing user claims (OpenID Connect)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,

    /// Token type (always "Bearer" for OAuth 2.0)
    pub token_type: String,

    /// Access token lifetime in seconds
    pub expires_in: i64,

    /// Absolute expiration timestamp (UTC)
    /// Calculated from expires_in at token creation/retrieval time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,

    /// Granted scopes (space-separated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl TokenSet {
    /// Create a new `TokenSet` with calculated expiration time
    ///
    /// The `expires_at` timestamp is automatically calculated from
    /// `expires_in`.
    ///
    /// # Arguments
    /// * `access_token` - The access token
    /// * `refresh_token` - Optional refresh token
    /// * `id_token` - Optional ID token (OpenID Connect)
    /// * `expires_in` - Token lifetime in seconds
    /// * `scope` - Optional space-separated scopes
    #[must_use]
    pub fn new(
        access_token: String,
        refresh_token: Option<String>,
        id_token: Option<String>,
        expires_in: i64,
        scope: Option<String>,
    ) -> Self {
        let expires_at = if expires_in > 0 {
            Some(Utc::now() + chrono::Duration::seconds(expires_in))
        } else {
            None
        };

        Self {
            access_token,
            refresh_token,
            id_token,
            token_type: "Bearer".to_string(),
            expires_in,
            expires_at,
            scope,
        }
    }

    /// Check if the access token is expired or will expire within the given
    /// threshold
    ///
    /// # Arguments
    /// * `threshold_seconds` - Number of seconds before expiry to consider
    ///   expired (default recommendation: 300 = 5 minutes)
    ///
    /// # Returns
    /// `true` if the token is expired or will expire within the threshold,
    /// `false` if it's still valid beyond the threshold or if no expiry is set
    #[must_use]
    pub fn is_expired(&self, threshold_seconds: i64) -> bool {
        match self.expires_at {
            Some(expires_at) => {
                let threshold = chrono::Duration::seconds(threshold_seconds);
                Utc::now() + threshold >= expires_at
            }
            None => false, // If no expiry set, assume not expired
        }
    }

    /// Get seconds until token expiration
    ///
    /// # Returns
    /// `Some(seconds)` if expiry is set, `None` if no expiry timestamp exists
    #[must_use]
    pub fn seconds_until_expiry(&self) -> Option<i64> {
        self.expires_at.map(|expires_at| (expires_at - Utc::now()).num_seconds())
    }

    /// Update expiration timestamp based on current time and expires_in
    ///
    /// Useful when retrieving tokens from storage to recalculate expires_at
    pub fn refresh_expiry_timestamp(&mut self) {
        if self.expires_in > 0 {
            self.expires_at = Some(Utc::now() + chrono::Duration::seconds(self.expires_in));
        }
    }
}

/// OAuth token response from authorization server
///
/// Standard OAuth 2.0 token response format (RFC 6749).
/// Deserializes responses from `/oauth/token` endpoints.
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: Option<String>,
}

impl From<TokenResponse> for TokenSet {
    fn from(response: TokenResponse) -> Self {
        Self::new(
            response.access_token,
            response.refresh_token,
            response.id_token,
            response.expires_in,
            response.scope,
        )
    }
}

/// OAuth configuration for authorization servers
///
/// Supports Auth0, Google, Microsoft, and other OAuth 2.0 providers.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// Authorization server domain (e.g., "dev-abc123.us.auth0.com",
    /// "accounts.google.com")
    pub domain: String,

    /// OAuth client ID
    pub client_id: String,

    /// Redirect URI (loopback for desktop apps, deep link for mobile)
    pub redirect_uri: String,

    /// OAuth scopes to request (space-separated)
    pub scopes: Vec<String>,

    /// OAuth audience (API identifier) - Optional, used by some providers like
    /// Auth0
    pub audience: Option<String>,
}

impl OAuthConfig {
    /// Create a new OAuth configuration
    #[must_use]
    pub fn new(
        domain: String,
        client_id: String,
        redirect_uri: String,
        scopes: Vec<String>,
        audience: Option<String>,
    ) -> Self {
        Self { domain, client_id, redirect_uri, scopes, audience }
    }

    /// Get the authorization URL
    ///
    /// For most providers, this is `https://{domain}/authorize`.
    /// Override this method for providers with different URL patterns.
    #[must_use]
    pub fn authorization_url(&self) -> String {
        format!("https://{}/authorize", self.domain)
    }

    /// Get the token URL
    ///
    /// For most providers, this is `https://{domain}/oauth/token`.
    /// Override this method for providers with different URL patterns.
    #[must_use]
    pub fn token_url(&self) -> String {
        format!("https://{}/oauth/token", self.domain)
    }

    /// Get scopes as space-separated string
    #[must_use]
    pub fn scope_string(&self) -> String {
        self.scopes.join(" ")
    }
}

/// OAuth error response from authorization server
///
/// Standard OAuth 2.0 error response format (RFC 6749 §5.2).
#[derive(Debug, Deserialize)]
pub struct OAuthError {
    pub error: String,
    pub error_description: Option<String>,
}

impl fmt::Display for OAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.error_description {
            Some(desc) => write!(f, "{}: {}", self.error, desc),
            None => write!(f, "{}", self.error),
        }
    }
}

impl std::error::Error for OAuthError {}

// ============================================================================
// Conversions with Calendar Integration Types
// ============================================================================
// NOTE: These conversions are disabled because integrations are not part of
// common crate

/*
/// Convert calendar OAuthTokens to core TokenSet
///
/// This allows the core OAuth module to work with tokens from the calendar integration.
impl From<crate::integrations::calendar::types::OAuthTokens> for TokenSet {
    fn from(tokens: crate::integrations::calendar::types::OAuthTokens) -> Self {
        Self::new(
            tokens.access_token,
            tokens.refresh_token,
            None, // Calendar tokens don't have id_token
            tokens.expires_in,
            None, // Calendar tokens don't have scope
        )
    }
}

/// Convert core TokenSet to calendar OAuthTokens
///
/// This allows the core OAuth module to store tokens using the calendar's KeychainProvider.
/// Note: id_token and scope fields are not preserved in this conversion.
impl From<TokenSet> for crate::integrations::calendar::types::OAuthTokens {
    fn from(tokens: TokenSet) -> Self {
        crate::integrations::calendar::types::OAuthTokens {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_in: tokens.expires_in,
            token_type: tokens.token_type,
        }
    }
}

impl From<&TokenSet> for crate::integrations::calendar::types::OAuthTokens {
    fn from(tokens: &TokenSet) -> Self {
        crate::integrations::calendar::types::OAuthTokens {
            access_token: tokens.access_token.clone(),
            refresh_token: tokens.refresh_token.clone(),
            expires_in: tokens.expires_in,
            token_type: tokens.token_type.clone(),
        }
    }
}

/// Convert calendar provider TokenResponse to core TokenSet
impl From<crate::integrations::calendar::providers::TokenResponse> for TokenSet {
    fn from(response: crate::integrations::calendar::providers::TokenResponse) -> Self {
        TokenSet::new(
            response.access_token,
            response.refresh_token,
            response.id_token,
            response.expires_in,
            None, // Calendar TokenResponse doesn't have scope
        )
    }
}

/// Convert SAP TokenResponse to core TokenSet
impl From<crate::integrations::sap::types::TokenResponse> for TokenSet {
    fn from(response: crate::integrations::sap::types::TokenResponse) -> Self {
        TokenSet::new(
            response.access_token,
            response.refresh_token,
            response.id_token,
            response.expires_in,
            response.scope,
        )
    }
}
*/

#[cfg(test)]
mod tests {
    //! Unit tests for auth::types.
    use super::*;

    /// Validates `TokenSet::new` behavior for the token set creation scenario.
    ///
    /// Assertions:
    /// - Confirms `token_set.access_token` equals `"access_token_123"`.
    /// - Confirms `token_set.refresh_token` equals
    ///   `Some("refresh_token_456".to_string())`.
    /// - Confirms `token_set.id_token` equals
    ///   `Some("id_token_789".to_string())`.
    /// - Confirms `token_set.expires_in` equals `3600`.
    /// - Ensures `token_set.expires_at.is_some()` evaluates to true.
    /// - Confirms `token_set.token_type` equals `"Bearer"`.
    #[test]
    fn test_token_set_creation() {
        let token_set = TokenSet::new(
            "access_token_123".to_string(),
            Some("refresh_token_456".to_string()),
            Some("id_token_789".to_string()),
            3600,
            Some("openid profile email".to_string()),
        );

        assert_eq!(token_set.access_token, "access_token_123");
        assert_eq!(token_set.refresh_token, Some("refresh_token_456".to_string()));
        assert_eq!(token_set.id_token, Some("id_token_789".to_string()));
        assert_eq!(token_set.expires_in, 3600);
        assert!(token_set.expires_at.is_some());
        assert_eq!(token_set.token_type, "Bearer");
    }

    /// Validates `TokenSet::new` behavior for the token set without refresh
    /// token scenario.
    ///
    /// Assertions:
    /// - Ensures `token_set.refresh_token.is_none()` evaluates to true.
    /// - Confirms `token_set.access_token` equals `"access_only"`.
    #[test]
    fn test_token_set_without_refresh_token() {
        // Some providers (like implicit flow) don't issue refresh tokens
        let token_set =
            TokenSet::new("access_only".to_string(), None, None, 3600, Some("read".to_string()));

        assert!(token_set.refresh_token.is_none());
        assert_eq!(token_set.access_token, "access_only");
    }

    /// Validates `TokenSet::new` behavior for the token expiry check scenario.
    ///
    /// Assertions:
    /// - Ensures `!token_set.is_expired(300)` evaluates to true.
    /// - Ensures `token_set.is_expired(7200)` evaluates to true.
    #[test]
    fn test_token_expiry_check() {
        let token_set = TokenSet::new(
            "access".to_string(),
            Some("refresh".to_string()),
            None,
            3600, // 1 hour
            None,
        );

        // Should not be expired with 5 min threshold
        assert!(!token_set.is_expired(300));

        // Should be expired with very large threshold
        assert!(token_set.is_expired(7200)); // 2 hours
    }

    /// Validates `TokenSet::new` behavior for the token expiry no expiry set
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `!token_set.is_expired(300)` evaluates to true.
    /// - Ensures `token_set.seconds_until_expiry().is_none()` evaluates to
    ///   true.
    #[test]
    fn test_token_expiry_no_expiry_set() {
        let mut token_set = TokenSet::new(
            "access".to_string(),
            Some("refresh".to_string()),
            None,
            0, // No expiry
            None,
        );

        // Manually clear expires_at
        token_set.expires_at = None;

        // Should not be considered expired if no expiry is set
        assert!(!token_set.is_expired(300));
        assert!(token_set.seconds_until_expiry().is_none());
    }

    /// Validates `TokenSet::new` behavior for the seconds until expiry
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `seconds.is_some()` evaluates to true.
    /// - Ensures `secs > 3590 && secs <= 3600` evaluates to true.
    #[test]
    fn test_seconds_until_expiry() {
        let token_set =
            TokenSet::new("access".to_string(), Some("refresh".to_string()), None, 3600, None);

        let seconds = token_set.seconds_until_expiry();
        assert!(seconds.is_some());

        // Should be close to 3600 seconds (within a few seconds for test execution
        // time)
        let secs = seconds.unwrap();
        assert!(secs > 3590 && secs <= 3600);
    }

    /// Validates `TokenSet::new` behavior for the refresh expiry timestamp
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `token_set.expires_at.is_some()` evaluates to true.
    /// - Confirms `token_set.expires_at` differs from `original_expires_at`.
    #[test]
    fn test_refresh_expiry_timestamp() {
        let mut token_set =
            TokenSet::new("access".to_string(), Some("refresh".to_string()), None, 3600, None);

        let original_expires_at = token_set.expires_at;

        // Sleep for a moment to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(100));

        token_set.refresh_expiry_timestamp();

        // Timestamp should have been updated (will be slightly different)
        assert!(token_set.expires_at.is_some());
        assert_ne!(token_set.expires_at, original_expires_at);
    }

    /// Validates `OAuthConfig::new` behavior for the oauth config urls
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `config.authorization_url()` equals `"https://dev-test.us.auth0.com/authorize"`.
    /// - Confirms `config.token_url()` equals `"https://dev-test.us.auth0.com/oauth/token"`.
    /// - Confirms `config.scope_string()` equals `"openid profile"`.
    #[test]
    fn test_oauth_config_urls() {
        let config = OAuthConfig::new(
            "dev-test.us.auth0.com".to_string(),
            "client123".to_string(),
            "http://localhost:3000/callback".to_string(),
            vec!["openid".to_string(), "profile".to_string()],
            Some("https://api.pulsearc.ai".to_string()),
        );

        assert_eq!(config.authorization_url(), "https://dev-test.us.auth0.com/authorize");
        assert_eq!(config.token_url(), "https://dev-test.us.auth0.com/oauth/token");
        assert_eq!(config.scope_string(), "openid profile");
    }

    /// Validates the token response conversion scenario.
    ///
    /// Assertions:
    /// - Confirms `token_set.access_token` equals `"access123"`.
    /// - Confirms `token_set.refresh_token` equals
    ///   `Some("refresh456".to_string())`.
    /// - Confirms `token_set.id_token` equals `Some("id789".to_string())`.
    /// - Confirms `token_set.expires_in` equals `3600`.
    /// - Ensures `token_set.expires_at.is_some()` evaluates to true.
    #[test]
    fn test_token_response_conversion() {
        let response = TokenResponse {
            access_token: "access123".to_string(),
            refresh_token: Some("refresh456".to_string()),
            id_token: Some("id789".to_string()),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            scope: Some("openid profile".to_string()),
        };

        let token_set: TokenSet = response.into();

        assert_eq!(token_set.access_token, "access123");
        assert_eq!(token_set.refresh_token, Some("refresh456".to_string()));
        assert_eq!(token_set.id_token, Some("id789".to_string()));
        assert_eq!(token_set.expires_in, 3600);
        assert!(token_set.expires_at.is_some());
    }

    /// Validates the oauth error display scenario.
    ///
    /// Assertions:
    /// - Ensures `error_string.contains("invalid_grant")` evaluates to true.
    /// - Ensures `error_string.contains("refresh token is invalid")` evaluates
    ///   to true.
    #[test]
    fn test_oauth_error_display() {
        let error = OAuthError {
            error: "invalid_grant".to_string(),
            error_description: Some("The refresh token is invalid".to_string()),
        };

        let error_string = error.to_string();
        assert!(error_string.contains("invalid_grant"));
        assert!(error_string.contains("refresh token is invalid"));
    }

    /// Validates the oauth error without description scenario.
    ///
    /// Assertions:
    /// - Confirms `error_string` equals `"invalid_request"`.
    #[test]
    fn test_oauth_error_without_description() {
        let error = OAuthError { error: "invalid_request".to_string(), error_description: None };

        let error_string = error.to_string();
        assert_eq!(error_string, "invalid_request");
    }
}
