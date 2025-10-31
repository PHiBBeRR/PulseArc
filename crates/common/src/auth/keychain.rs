//! OAuth token storage helpers layered on top of `KeychainProvider`.
//!
//! # Module Layering
//!
//! This module provides OAuth-specific token storage that builds on the generic
//! keychain provider:
//!
//! - **`security::encryption::keychain`**: Generic secret storage via platform
//!   keychain (macOS Keychain, Windows Credential Manager, Linux Secret
//!   Service)
//! - **`auth::keychain`** (this module): OAuth token-specific helpers for
//!   storing/retrieving access tokens, refresh tokens, and metadata
//! - **`security::keychain`**: Convenience re-export of
//!   `security::encryption::keychain`
//!
//! This layering keeps token-specific logic alongside the auth crate while the
//! security crate exposes the generic keychain integration. This prevents the
//! security module from depending on auth types while preserving the existing
//! public API.

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde_json::json;
use tracing::debug;

use crate::auth::traits::KeychainTrait;
use crate::auth::types::TokenSet;
use crate::security::{KeychainError, KeychainProvider};

const ACCESS_PREFIX: &str = "access.";
const REFRESH_PREFIX: &str = "refresh.";
const METADATA_PREFIX: &str = "metadata.";

impl KeychainProvider {
    /// Persist OAuth tokens in the platform keychain.
    pub fn store_tokens(&self, account: &str, tokens: &TokenSet) -> Result<(), KeychainError> {
        debug!(account = %account, "Storing OAuth tokens");

        self.set_secret(&format!("{}{}", ACCESS_PREFIX, account), &tokens.access_token)?;

        if let Some(refresh) = &tokens.refresh_token {
            self.set_secret(&format!("{}{}", REFRESH_PREFIX, account), refresh)?;
        }

        let metadata = json!({
            "expires_in": tokens.expires_in,
            "token_type": tokens.token_type,
            "id_token": tokens.id_token,
            "scope": tokens.scope,
            "expires_at": tokens.expires_at.map(|dt| dt.timestamp()),
        });

        let metadata_key = format!("{}{}", METADATA_PREFIX, account);
        let metadata_str = serde_json::to_string(&metadata)?;
        self.set_secret(&metadata_key, &metadata_str)?;

        debug!(account = %account, "OAuth tokens stored");
        Ok(())
    }

    /// Retrieve OAuth tokens for the specified account.
    pub fn retrieve_tokens(&self, account: &str) -> Result<TokenSet, KeychainError> {
        debug!(account = %account, "Retrieving OAuth tokens");

        let access_token = self.get_secret(&format!("{}{}", ACCESS_PREFIX, account))?;

        let refresh_token = match self.get_secret(&format!("{}{}", REFRESH_PREFIX, account)) {
            Ok(value) => Some(value),
            Err(KeychainError::NotFound) => None,
            Err(other) => return Err(other),
        };

        let metadata_str = self.get_secret(&format!("{}{}", METADATA_PREFIX, account))?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;

        let expires_in = metadata.get("expires_in").and_then(|v| v.as_i64()).unwrap_or(3600);
        let expires_at = metadata
            .get("expires_at")
            .and_then(|v| v.as_i64())
            .and_then(|ts| Utc.timestamp_opt(ts, 0).single());

        Ok(TokenSet {
            access_token,
            refresh_token,
            id_token: metadata.get("id_token").and_then(|v| v.as_str()).map(String::from),
            token_type: metadata
                .get("token_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Bearer")
                .to_string(),
            expires_in,
            expires_at,
            scope: metadata.get("scope").and_then(|v| v.as_str()).map(String::from),
        })
    }

    /// Delete OAuth tokens associated with the given account.
    pub fn delete_tokens(&self, account: &str) -> Result<(), KeychainError> {
        debug!(account = %account, "Deleting OAuth tokens");

        let _ = self.delete_secret(&format!("{}{}", ACCESS_PREFIX, account));
        let _ = self.delete_secret(&format!("{}{}", REFRESH_PREFIX, account));
        let _ = self.delete_secret(&format!("{}{}", METADATA_PREFIX, account));

        Ok(())
    }

    /// Determine whether an access token exists for the account.
    #[must_use]
    pub fn has_tokens(&self, account: &str) -> bool {
        self.secret_exists(&format!("{}{}", ACCESS_PREFIX, account))
    }
}

#[async_trait]
impl KeychainTrait for KeychainProvider {
    async fn store_tokens(&self, account: &str, tokens: &TokenSet) -> Result<(), String> {
        self.store_tokens(account, tokens).map_err(|e| e.to_string())
    }

    async fn retrieve_tokens(&self, account: &str) -> Result<TokenSet, String> {
        self.retrieve_tokens(account).map_err(|e| e.to_string())
    }

    async fn delete_tokens(&self, account: &str) -> Result<(), String> {
        self.delete_tokens(account).map_err(|e| e.to_string())
    }

    async fn has_tokens(&self, account: &str) -> bool {
        self.has_tokens(account)
    }
}

#[cfg(all(test, feature = "platform"))]
mod tests {
    //! Tests focus on token-specific behaviour; general secret handling lives
    //! in security tests.
    use super::*;
    use crate::testing::MockKeychainProvider;

    type TestKeychain = MockKeychainProvider;

    fn test_service_name() -> String {
        format!("PulseArcTestTokens.{}", uuid::Uuid::new_v4())
    }

    fn sample_tokens() -> TokenSet {
        TokenSet::new(
            "test_access".to_string(),
            Some("test_refresh".to_string()),
            Some("id".to_string()),
            3600,
            Some("openid profile".to_string()),
        )
    }

    /// Validates `TestKeychain::new` behavior for the store and retrieve tokens
    /// roundtrip scenario.
    ///
    /// Assertions:
    /// - Confirms `retrieved.access_token` equals `tokens.access_token`.
    /// - Confirms `retrieved.refresh_token` equals `tokens.refresh_token`.
    /// - Confirms `retrieved.token_type` equals `tokens.token_type`.
    /// - Confirms `retrieved.scope` equals `tokens.scope`.
    #[test]
    fn store_and_retrieve_tokens_roundtrip() {
        let service = test_service_name();
        let keychain = TestKeychain::new(&service);
        let account = "auth@example.com";
        let tokens = sample_tokens();

        keychain.store_tokens(account, &tokens).unwrap();
        let retrieved = keychain.retrieve_tokens(account).unwrap();

        assert_eq!(retrieved.access_token, tokens.access_token);
        assert_eq!(retrieved.refresh_token, tokens.refresh_token);
        assert_eq!(retrieved.token_type, tokens.token_type);
        assert_eq!(retrieved.scope, tokens.scope);

        keychain.delete_tokens(account).unwrap();
    }

    /// Validates `TestKeychain::new` behavior for the delete tokens is
    /// idempotent scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn delete_tokens_is_idempotent() {
        let service = test_service_name();
        let keychain = TestKeychain::new(&service);
        let account = "delete@example.com";

        keychain.delete_tokens(account).unwrap();

        keychain.store_tokens(account, &sample_tokens()).unwrap();
        keychain.delete_tokens(account).unwrap();
        keychain.delete_tokens(account).unwrap();
    }

    /// Validates `TestKeychain::new` behavior for the retrieve missing tokens
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(result, Err(KeychainError::NotFound))` evaluates to
    ///   true.
    #[test]
    fn retrieve_missing_tokens() {
        let service = test_service_name();
        let keychain = TestKeychain::new(&service);
        let account = "missing@example.com";

        let result = keychain.retrieve_tokens(account);
        assert!(matches!(result, Err(KeychainError::NotFound)));
    }
}
