//! Integration tests for auth module
//!
//! Tests OAuth 2.0 + PKCE flow, token management, and keychain integration

#![cfg(feature = "platform")]

use std::sync::{Arc, Once};

use pulsearc_common::auth::{
    generate_code_challenge, generate_code_verifier, generate_state, validate_state, OAuthClient,
    OAuthConfig, PKCEChallenge, TokenSet,
};
use pulsearc_common::testing::{MockKeychainProvider, MockOAuthClient};

fn disable_oauth_http() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        std::env::set_var("PULSEARC_DISABLE_PROXY", "1");
        std::env::set_var("PULSEARC_OAUTH_DISABLE_HTTP", "1");
    });
}

/// Validates PKCE (Proof Key for Code Exchange) challenge generation and
/// format.
///
/// This test ensures that PKCE verifiers and challenges are generated according
/// to OAuth 2.0 PKCE specification (RFC 7636), with proper length constraints
/// and character sets. PKCE protects against authorization code interception
/// attacks.
///
/// # Test Steps
/// 1. Generate code verifier (random string, 43-128 chars, alphanumeric + `-_`)
/// 2. Generate SHA256 challenge from verifier
/// 3. Verify PKCEChallenge struct contains valid verifier, challenge, and
///    method (S256)
#[tokio::test(flavor = "multi_thread")]
async fn test_pkce_challenge_generation() {
    // Generate code verifier
    let verifier = generate_code_verifier().expect("Failed to generate verifier");
    assert!(verifier.len() >= 43 && verifier.len() <= 128);
    assert!(verifier.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_'));

    // Generate challenge from verifier
    let challenge = generate_code_challenge(&verifier).expect("Failed to generate challenge");
    assert!(!challenge.is_empty());

    // Test PKCEChallenge struct
    let pkce = PKCEChallenge::generate().expect("Failed to generate PKCE challenge");
    assert!(!pkce.code_verifier.is_empty());
    assert!(!pkce.code_challenge.is_empty());
    assert_eq!(pkce.challenge_method(), "S256");
}

/// Validates OAuth state parameter generation and validation for CSRF
/// protection.
///
/// This test ensures that state parameters are cryptographically secure random
/// strings that prevent Cross-Site Request Forgery (CSRF) attacks during OAuth
/// flows. Each state must be unique and validation must be exact match only.
///
/// # Test Steps
/// 1. Generate two state values
/// 2. Verify states are different (uniqueness)
/// 3. Verify state length meets security requirements (≥32 chars)
/// 4. Validate matching states pass validation
/// 5. Validate mismatched states fail validation
#[tokio::test(flavor = "multi_thread")]
async fn test_state_generation_and_validation() {
    let state1 = generate_state().expect("Failed to generate state1");
    let state2 = generate_state().expect("Failed to generate state2");

    // States should be different
    assert_ne!(state1, state2);

    // State should be long enough for security
    assert!(state1.len() >= 32);

    // Valid state should pass validation
    assert!(validate_state(&state1, &state1));

    // Invalid state should fail validation
    assert!(!validate_state(&state1, &state2));
    assert!(!validate_state(&state1, "invalid"));
}

/// Validates OAuth configuration creation with all required parameters.
///
/// This test ensures the OAuthConfig struct can be properly initialized with
/// all necessary OAuth 2.0 parameters including issuer, client ID, redirect
/// URI, scopes, and optional audience for API access.
///
/// # Test Steps
/// 1. Create OAuthConfig with test values
/// 2. Verify all fields are correctly assigned
/// 3. Verify scopes collection is properly stored
/// 4. Verify optional audience is preserved
#[test]
fn test_oauth_config_creation() {
    let config = OAuthConfig::new(
        "dev-test.auth0.com".to_string(),
        "test_client_id".to_string(),
        "http://localhost:8888/callback".to_string(),
        vec!["openid".to_string(), "profile".to_string()],
        Some("https://api.example.com".to_string()),
    );

    assert_eq!(config.domain, "dev-test.auth0.com");
    assert_eq!(config.client_id, "test_client_id");
    assert_eq!(config.redirect_uri, "http://localhost:8888/callback");
    assert_eq!(config.scopes.len(), 2);
    assert_eq!(config.audience, Some("https://api.example.com".to_string()));
}

/// Validates TokenSet serialization and deserialization for token storage.
///
/// This test ensures that OAuth token sets (access, refresh, ID tokens) can be
/// safely serialized to JSON for storage and deserialized for use, preserving
/// all token data and metadata including expiration and scope.
///
/// # Test Steps
/// 1. Create TokenSet with all fields (access, refresh, ID tokens, expiry,
///    scope)
/// 2. Serialize to JSON and verify content
/// 3. Deserialize from JSON
/// 4. Verify all fields match original values
#[test]
fn test_token_set_operations() {
    let token_set = TokenSet::new(
        "test_access_token".to_string(),
        Some("test_refresh_token".to_string()),
        Some("test_id_token".to_string()),
        3600,
        Some("openid profile".to_string()),
    );

    // Test serialization
    let json = serde_json::to_string(&token_set).expect("Failed to serialize");
    assert!(json.contains("test_access_token"));
    assert!(json.contains("test_refresh_token"));

    // Test deserialization
    let deserialized: TokenSet = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.access_token, token_set.access_token);
    assert_eq!(deserialized.refresh_token, token_set.refresh_token);
    assert_eq!(deserialized.expires_in, 3600);
}

/// Validates TokenManager integration with mocks for secure token storage.
///
/// This test ensures the full TokenManager works correctly with mocks,
/// testing the complete integration without system dependencies or HTTP
/// requests.
///
/// # Test Steps
/// 1. Create TokenManager with mock keychain and mock OAuth client
/// 2. Store token set (with long expiry to avoid auto-refresh)
/// 3. Retrieve tokens and verify values match
/// 4. Test get_access_token works correctly
/// 5. Test clear tokens
#[tokio::test(flavor = "multi_thread")]
async fn test_token_manager_with_mocks() {
    let keychain = Arc::new(MockKeychainProvider::new("PulseArcTest".to_string()));
    let oauth_client = MockOAuthClient::new();
    let service_name = "test_oauth_service";
    let account_name = "test_user";

    // Create token manager with mocks
    let token_manager = pulsearc_common::auth::TokenManager::new(
        oauth_client,
        keychain.clone(),
        service_name.to_string(),
        account_name.to_string(),
        300, // 5 minutes before expiry
    );

    // Create test token set with LONG expiry (won't trigger auto-refresh)
    let token_set = TokenSet::new(
        "test_access_123".to_string(),
        Some("test_refresh_123".to_string()),
        None,
        7200, // 2 hours - well above refresh threshold
        Some("openid profile".to_string()),
    );

    // Store tokens
    token_manager.store_tokens(token_set.clone()).await.expect("Failed to store tokens");

    // Verify we're authenticated
    assert!(token_manager.is_authenticated().await);

    // Get tokens
    let loaded = token_manager.get_tokens().await.expect("No tokens found");
    assert_eq!(loaded.access_token, token_set.access_token);
    assert_eq!(loaded.refresh_token, token_set.refresh_token);

    // Test get_access_token (won't trigger refresh because token has long expiry)
    let access_token = token_manager.get_access_token().await.expect("Failed to get access token");
    assert_eq!(access_token, "test_access_123");

    // Test clear tokens
    token_manager.clear_tokens().await.expect("Failed to clear tokens");

    // Verify cleared
    assert!(!token_manager.is_authenticated().await);
    let cleared = token_manager.get_tokens().await;
    assert!(cleared.is_none());

    // Clean up
    let _ = keychain.delete_tokens(account_name);
}

/// Validates TokenManager auto-refresh functionality with mock OAuth client.
///
/// This test ensures TokenManager automatically refreshes tokens when they're
/// near expiry, using a mock OAuth client that doesn't make real HTTP requests.
///
/// # Test Steps
/// 1. Create TokenManager with mocks and short refresh threshold
/// 2. Store token with short expiry (below threshold)
/// 3. Call get_access_token which should trigger auto-refresh
/// 4. Verify refresh was called on mock OAuth client
/// 5. Verify we got the refreshed token
#[tokio::test(flavor = "multi_thread")]
async fn test_token_manager_auto_refresh() {
    let keychain = Arc::new(MockKeychainProvider::new("PulseArcTest".to_string()));
    let oauth_client = MockOAuthClient::new();
    let account_name = "test_user";

    // Configure mock to return specific refreshed token
    oauth_client.set_refresh_response(TokenSet::new(
        "auto_refreshed_token".to_string(),
        Some("new_refresh_token".to_string()),
        None,
        3600,
        None,
    ));

    // Create token manager with short threshold (60 seconds)
    let token_manager = pulsearc_common::auth::TokenManager::new(
        oauth_client.clone(),
        keychain.clone(),
        "test_service".to_string(),
        account_name.to_string(),
        60, // 1 minute threshold
    );

    // Store token that expires soon (30 seconds - below threshold)
    let expiring_token = TokenSet::new(
        "expiring_token".to_string(),
        Some("refresh_token".to_string()),
        None,
        30, // Expires in 30 seconds
        None,
    );

    token_manager.store_tokens(expiring_token).await.expect("Failed to store tokens");

    // Call get_access_token - should trigger auto-refresh
    let access_token = token_manager.get_access_token().await.expect("Failed to get access token");

    // Verify refresh was called
    assert!(oauth_client.was_refresh_called(), "Token refresh should have been called");

    // Verify we got the refreshed token
    assert_eq!(access_token, "auto_refreshed_token");

    // Clean up
    let _ = keychain.delete_tokens(account_name);
}

/// Validates token expiration detection logic.
///
/// This test ensures the TokenSet correctly identifies when tokens are about
/// to expire using the is_expired method with a configurable threshold.
///
/// # Test Steps
/// 1. Create token expiring in 30 seconds
/// 2. Verify is_expired(60) returns true (expires within 60 second threshold)
/// 3. Create token expiring in 3600 seconds
/// 4. Verify is_expired(60) returns false (does NOT expire within threshold)
#[tokio::test(flavor = "multi_thread")]
async fn test_token_expiration() {
    // Create token that expires soon (30 seconds)
    let expiring_token = TokenSet::new(
        "expiring_token".to_string(),
        Some("refresh_token".to_string()),
        None,
        30,
        None,
    );

    // Check if token is expired within 60 second threshold
    assert!(expiring_token.is_expired(60)); // Should be expired within 60 second threshold

    // Create token that doesn't expire soon (3600 seconds)
    let valid_token = TokenSet::new(
        "valid_token".to_string(),
        Some("refresh_token".to_string()),
        None,
        3600,
        None,
    );

    // Should NOT be expired within 60 second threshold
    assert!(!valid_token.is_expired(60));

    // But should be expired with a very large threshold (e.g., 2 hours)
    assert!(valid_token.is_expired(7200));
}

/// Validates OAuth authorization URL generation with PKCE parameters.
///
/// This test ensures the OAuthClient generates properly formatted authorization
/// URLs with all required OAuth 2.0 + PKCE parameters correctly encoded. This
/// URL initiates the user authentication flow in a web browser.
///
/// # Test Steps
/// 1. Create OAuthClient with test configuration
/// 2. Generate PKCE challenge and state
/// 3. Build authorization URL
/// 4. Verify URL contains all required parameters: client_id, redirect_uri,
///    response_type, state, code_challenge, code_challenge_method, scope,
///    audience
/// 5. Verify parameters are properly URL-encoded
#[tokio::test(flavor = "multi_thread")]
async fn test_oauth_client_authorization_url() {
    disable_oauth_http();
    let config = OAuthConfig::new(
        "dev-test.auth0.com".to_string(),
        "test_client_id".to_string(),
        "http://localhost:8888/callback".to_string(),
        vec!["openid".to_string(), "profile".to_string()],
        Some("https://api.example.com".to_string()),
    );

    let client = OAuthClient::new(config);

    let (auth_url, state) =
        client.generate_authorization_url().await.expect("Failed to generate URL");

    // Verify URL contains required parameters
    assert!(auth_url.contains("client_id=test_client_id"));
    assert!(auth_url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A8888%2Fcallback"));
    assert!(auth_url.contains("response_type=code"));
    assert!(auth_url.contains(&format!("state={}", state)));
    assert!(auth_url.contains("code_challenge="));
    assert!(auth_url.contains("code_challenge_method=S256"));
    assert!(auth_url.contains("scope=openid"));
    assert!(auth_url.contains("audience=https%3A%2F%2Fapi.example.com"));
}

/// Validates graceful handling of missing refresh tokens.
///
/// This test ensures TokenSet correctly handles cases where tokens don't
/// include a refresh token (some OAuth flows don't provide them).
///
/// # Test Steps
/// 1. Create TokenSet WITHOUT refresh_token (set to None)
/// 2. Store and retrieve from mock keychain
/// 3. Verify refresh_token field is None
#[tokio::test(flavor = "multi_thread")]
async fn test_missing_refresh_token() {
    let keychain = MockKeychainProvider::new("PulseArcTest".to_string());

    // Create token without refresh token
    let token_set = TokenSet::new(
        "access_only".to_string(),
        None, // No refresh token
        None,
        3600,
        None,
    );

    // Store tokens
    keychain.store_tokens("test_user", &token_set).expect("Failed to store tokens");

    // Retrieve and verify no refresh token
    let tokens = keychain.retrieve_tokens("test_user").expect("Failed to retrieve tokens");
    assert_eq!(tokens.access_token, "access_only");
    assert!(tokens.refresh_token.is_none());

    // Clean up
    keychain.delete_tokens("test_user").unwrap();
}

/// Validates thread-safe concurrent token access from multiple async tasks.
///
/// This test ensures MockKeychainProvider is safe for concurrent use by
/// multiple async tasks, verifying that simultaneous reads/writes don't cause
/// data races.
///
/// # Test Steps
/// 1. Create MockKeychainProvider (wrapped in Arc for sharing)
/// 2. Store initial token set
/// 3. Spawn 10 concurrent Tokio tasks
/// 4. Each task retrieves tokens simultaneously
/// 5. Verify all tasks successfully retrieve the same token
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_token_access() {
    let keychain = Arc::new(MockKeychainProvider::new("PulseArcTest".to_string()));

    // Save initial token
    let token_set = TokenSet::new(
        "concurrent_token".to_string(),
        Some("concurrent_refresh".to_string()),
        None,
        3600,
        None,
    );

    keychain.store_tokens("test_user", &token_set).expect("Failed to store tokens");

    // Spawn multiple tasks that access tokens concurrently
    let mut handles = vec![];
    for _ in 0..10 {
        let kc = Arc::clone(&keychain);
        let handle = tokio::spawn(async move {
            let tokens = kc.retrieve_tokens("test_user");
            assert!(tokens.is_ok());
            tokens.expect("Tokens should be present").access_token
        });
        handles.push(handle);
    }

    // Wait for all tasks and verify all got the same token
    for handle in handles {
        let token = handle.await.expect("Task should complete");
        assert_eq!(token, "concurrent_token");
    }

    // Clean up
    keychain.delete_tokens("test_user").unwrap();
}
