//! Mock implementations of common traits
//!
//! Provides mock objects for testing purposes.

// Allow missing error/panic docs for test mocks - they are designed to be simple
// and errors are clearly indicated by their return types
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[cfg(feature = "platform")]
use async_trait::async_trait;

#[cfg(feature = "platform")]
use crate::auth::{
    KeychainTrait, OAuthClient, OAuthClientError, OAuthClientTrait, OAuthConfig, TokenSet,
};
#[cfg(feature = "platform")]
use crate::security::KeychainError;

// Type aliases to reduce complexity
type ResponseMap = Arc<Mutex<HashMap<String, MockHttpResponse>>>;
type ResponseSequenceMap = Arc<Mutex<HashMap<String, Vec<MockHttpResponse>>>>;
type RequestLog = Arc<Mutex<Vec<HttpRequest>>>;
type StorageData = Arc<Mutex<HashMap<String, String>>>;

/// Mock HTTP client for testing
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::mocks::MockHttpClient;
///
/// let client = MockHttpClient::new();
/// client.add_response("https://api.example.com", 200, "OK");
///
/// let response = client.get("https://api.example.com").unwrap();
/// assert_eq!(response.status, 200);
/// assert_eq!(response.body, "OK");
/// ```
#[derive(Debug, Clone)]
pub struct MockHttpClient {
    responses: ResponseMap,
    response_sequences: ResponseSequenceMap,
    requests: RequestLog,
}

/// Represents a captured HTTP request
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// The URL that was requested
    pub url: String,
    /// The request method (for future extension)
    pub method: String,
}

#[derive(Debug, Clone)]
pub struct MockHttpResponse {
    pub status: u16,
    pub body: String,
}

impl MockHttpClient {
    /// Create a new mock HTTP client
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            response_sequences: Arc::new(Mutex::new(HashMap::new())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a mock response for a URL
    pub fn add_response(&self, url: &str, status: u16, body: &str) {
        // SAFETY: Mutex poisoning is acceptable in test mocks - if a test panics,
        // the entire test fails anyway, so we don't need to handle poisoned mutexes
        // gracefully
        let mut responses = self.responses.lock().unwrap();
        responses.insert(url.to_string(), MockHttpResponse { status, body: body.to_string() });
    }

    /// Add a response sequence for a URL (returns different responses on each
    /// call)
    ///
    /// # Examples
    ///
    /// ```
    /// use pulsearc_common::testing::mocks::MockHttpClient;
    ///
    /// let client = MockHttpClient::new();
    /// client.add_response_sequence(
    ///     "https://api.example.com",
    ///     vec![(200, "First"), (200, "Second"), (404, "Not Found")],
    /// );
    ///
    /// assert_eq!(client.get("https://api.example.com").unwrap().body, "First");
    /// assert_eq!(client.get("https://api.example.com").unwrap().body, "Second");
    /// assert_eq!(client.get("https://api.example.com").unwrap().status, 404);
    /// ```
    pub fn add_response_sequence(&self, url: &str, responses: Vec<(u16, &str)>) {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        let mut sequences = self.response_sequences.lock().unwrap();
        let sequence = responses
            .into_iter()
            .map(|(status, body)| MockHttpResponse { status, body: body.to_string() })
            .collect();
        sequences.insert(url.to_string(), sequence);
    }

    /// Simulate a GET request
    pub fn get(&self, url: &str) -> Result<MockHttpResponse, String> {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.requests
            .lock()
            .unwrap()
            .push(HttpRequest { url: url.to_string(), method: "GET".to_string() });

        // Check for response sequence first
        // SAFETY: Mutex poisoning is acceptable in test mocks
        let mut sequences = self.response_sequences.lock().unwrap();
        if let Some(sequence) = sequences.get_mut(url) {
            if !sequence.is_empty() {
                return Ok(sequence.remove(0));
            }
        }
        drop(sequences);

        // Fall back to single response
        // SAFETY: Mutex poisoning is acceptable in test mocks
        let responses = self.responses.lock().unwrap();
        responses
            .get(url)
            .cloned()
            .ok_or_else(|| format!("No response configured for URL: {}", url))
    }

    /// Get all requests that were made
    #[must_use]
    pub fn requests(&self) -> Vec<HttpRequest> {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.requests.lock().unwrap().clone()
    }

    /// Get all request URLs (for backward compatibility)
    #[must_use]
    pub fn request_urls(&self) -> Vec<String> {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.requests.lock().unwrap().iter().map(|req| req.url.clone()).collect()
    }

    /// Get the number of requests made to a URL
    #[must_use]
    pub fn request_count(&self, url: &str) -> usize {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.requests.lock().unwrap().iter().filter(|req| req.url == url).count()
    }

    /// Verify that a request was made to the given URL
    #[must_use]
    pub fn was_called(&self, url: &str) -> bool {
        self.request_count(url) > 0
    }

    /// Verify that a request was made to the given URL exactly N times
    #[must_use]
    pub fn was_called_times(&self, url: &str, times: usize) -> bool {
        self.request_count(url) == times
    }

    /// Get the last request made
    #[must_use]
    pub fn last_request(&self) -> Option<HttpRequest> {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.requests.lock().unwrap().last().cloned()
    }

    /// Clear all recorded requests
    pub fn clear_requests(&self) {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.requests.lock().unwrap().clear();
    }
}

impl Default for MockHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock storage for testing
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::mocks::MockStorage;
///
/// let storage = MockStorage::new();
/// storage.set("key1", "value1").unwrap();
///
/// let value = storage.get("key1").unwrap();
/// assert_eq!(value, Some("value1".to_string()));
/// ```
#[derive(Debug, Clone)]
pub struct MockStorage {
    data: StorageData,
}

impl MockStorage {
    /// Create a new mock storage
    pub fn new() -> Self {
        Self { data: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Set a key-value pair
    pub fn set(&self, key: &str, value: &str) -> Result<(), String> {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.data.lock().unwrap().insert(key.to_string(), value.to_string());
        Ok(())
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        Ok(self.data.lock().unwrap().get(key).cloned())
    }

    /// Delete a key
    pub fn delete(&self, key: &str) -> Result<(), String> {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.data.lock().unwrap().remove(key);
        Ok(())
    }

    /// Check if a key exists
    #[must_use]
    pub fn exists(&self, key: &str) -> bool {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.data.lock().unwrap().contains_key(key)
    }

    /// Get all keys
    #[must_use]
    pub fn keys(&self) -> Vec<String> {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.data.lock().unwrap().keys().cloned().collect()
    }

    /// Clear all data
    pub fn clear(&self) {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.data.lock().unwrap().clear();
    }

    /// Get the number of items
    #[must_use]
    pub fn len(&self) -> usize {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.data.lock().unwrap().len()
    }

    /// Check if storage is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        // SAFETY: Mutex poisoning is acceptable in test mocks
        self.data.lock().unwrap().is_empty()
    }
}

impl Default for MockStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock keychain provider that stores credentials in memory.
///
/// This implementation avoids platform keychain prompts and persists data only
/// for the lifetime of the mock, making it ideal for tests.
#[derive(Clone, Debug)]
#[cfg(feature = "platform")]
pub struct MockKeychainProvider {
    storage: StorageData,
    #[allow(dead_code)]
    _service_name: String,
}

#[cfg(feature = "platform")]
impl MockKeychainProvider {
    /// Create a new mock keychain provider with a service name for namespacing.
    pub fn new(service_name: impl Into<String>) -> Self {
        Self { storage: Arc::new(Mutex::new(HashMap::new())), _service_name: service_name.into() }
    }

    /// Store an arbitrary secret value in memory.
    pub fn set_secret(&self, key: &str, value: &str) -> Result<(), KeychainError> {
        self.storage.lock().unwrap().insert(key.to_string(), value.to_string());
        Ok(())
    }

    /// Retrieve a secret value or return `KeychainError::NotFound`.
    pub fn get_secret(&self, key: &str) -> Result<String, KeychainError> {
        self.storage.lock().unwrap().get(key).cloned().ok_or(KeychainError::NotFound)
    }

    /// Delete a secret value (idempotent).
    pub fn delete_secret(&self, key: &str) -> Result<(), KeychainError> {
        self.storage.lock().unwrap().remove(key);
        Ok(())
    }

    /// Determine whether a secret exists.
    #[must_use]
    pub fn secret_exists(&self, key: &str) -> bool {
        self.storage.lock().unwrap().contains_key(key)
    }

    /// Store OAuth tokens under an account identifier.
    pub fn store_tokens(&self, account: &str, tokens: &TokenSet) -> Result<(), KeychainError> {
        let mut storage = self.storage.lock().unwrap();

        // Persist access token
        storage.insert(format!("access.{}", account), tokens.access_token.clone());

        // Persist refresh token when available
        if let Some(refresh_token) = tokens.refresh_token.as_ref() {
            storage.insert(format!("refresh.{}", account), refresh_token.clone());
        }

        // Persist metadata needed for reconstruction
        let metadata = serde_json::json!({
            "expires_in": tokens.expires_in,
            "token_type": tokens.token_type,
            "id_token": tokens.id_token,
            "scope": tokens.scope,
            "expires_at": tokens.expires_at.map(|dt| dt.timestamp()),
        });

        let metadata_str = serde_json::to_string(&metadata)?;
        storage.insert(format!("metadata.{}", account), metadata_str);

        Ok(())
    }

    /// Retrieve tokens for an account.
    pub fn retrieve_tokens(&self, account: &str) -> Result<TokenSet, KeychainError> {
        let storage = self.storage.lock().unwrap();

        let access_token =
            storage.get(&format!("access.{}", account)).ok_or(KeychainError::NotFound)?.clone();

        let refresh_token = storage.get(&format!("refresh.{}", account)).cloned();
        let metadata_raw =
            storage.get(&format!("metadata.{}", account)).ok_or(KeychainError::NotFound)?;

        let metadata: serde_json::Value = serde_json::from_str(metadata_raw)?;
        let expires_in = metadata.get("expires_in").and_then(|v| v.as_i64()).unwrap_or(3600);
        let expires_at = metadata
            .get("expires_at")
            .and_then(|v| v.as_i64())
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));

        Ok(TokenSet {
            access_token,
            refresh_token,
            id_token: metadata.get("id_token").and_then(|v| v.as_str()).map(ToOwned::to_owned),
            token_type: metadata
                .get("token_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Bearer")
                .to_string(),
            expires_in,
            expires_at,
            scope: metadata.get("scope").and_then(|v| v.as_str()).map(ToOwned::to_owned),
        })
    }

    /// Delete all stored tokens for an account.
    pub fn delete_tokens(&self, account: &str) -> Result<(), KeychainError> {
        let mut storage = self.storage.lock().unwrap();
        storage.remove(&format!("access.{}", account));
        storage.remove(&format!("refresh.{}", account));
        storage.remove(&format!("metadata.{}", account));
        Ok(())
    }

    /// Determine whether tokens exist for an account.
    #[must_use]
    pub fn has_tokens(&self, account: &str) -> bool {
        let storage = self.storage.lock().unwrap();
        storage.contains_key(&format!("access.{}", account))
    }

    /// Store an encryption key (alias for `set_secret`).
    pub fn store_key(&self, key_id: &str, key: &str) -> Result<(), KeychainError> {
        self.set_secret(key_id, key)
    }

    /// Retrieve an encryption key.
    pub fn retrieve_key(&self, key_id: &str) -> Result<String, KeychainError> {
        self.get_secret(key_id)
    }

    /// Get or create an encryption key of the specified size.
    pub fn get_or_create_key(
        &self,
        key_id: &str,
        key_size: usize,
    ) -> Result<String, KeychainError> {
        if let Ok(existing) = self.retrieve_key(key_id) {
            return Ok(existing);
        }

        use rand::distributions::{Alphanumeric, DistString};
        let key = Alphanumeric.sample_string(&mut rand::thread_rng(), key_size);
        self.store_key(key_id, &key)?;
        Ok(key)
    }

    /// Clear all stored credentials.
    pub fn clear_all(&self) {
        self.storage.lock().unwrap().clear();
    }
}

#[cfg(feature = "platform")]
impl Default for MockKeychainProvider {
    fn default() -> Self {
        Self::new("pulsearc-test")
    }
}

#[cfg(feature = "platform")]
#[async_trait]
impl KeychainTrait for MockKeychainProvider {
    async fn store_tokens(&self, account: &str, tokens: &TokenSet) -> Result<(), String> {
        self.store_tokens(account, tokens).map_err(|err| err.to_string())
    }

    async fn retrieve_tokens(&self, account: &str) -> Result<TokenSet, String> {
        self.retrieve_tokens(account).map_err(|err| err.to_string())
    }

    async fn delete_tokens(&self, account: &str) -> Result<(), String> {
        self.delete_tokens(account).map_err(|err| err.to_string())
    }

    async fn has_tokens(&self, account: &str) -> bool {
        self.has_tokens(account)
    }
}

/// Mock OAuth client that simulates OAuth flows without network calls.
#[derive(Clone, Debug)]
#[cfg(feature = "platform")]
pub struct MockOAuthClient {
    refresh_called: Arc<Mutex<bool>>,
    refresh_token_response: Arc<Mutex<Option<TokenSet>>>,
    should_fail: Arc<Mutex<bool>>,
}

#[cfg(feature = "platform")]
impl MockOAuthClient {
    /// Create a new mock OAuth client with default state.
    pub fn new() -> Self {
        Self {
            refresh_called: Arc::new(Mutex::new(false)),
            refresh_token_response: Arc::new(Mutex::new(None)),
            should_fail: Arc::new(Mutex::new(false)),
        }
    }

    /// Configure the response returned by `refresh_access_token`.
    pub fn set_refresh_response(&self, tokens: TokenSet) {
        *self.refresh_token_response.lock().unwrap() = Some(tokens);
    }

    /// Force the refresh call to fail.
    pub fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.lock().unwrap() = should_fail;
    }

    /// Check whether refresh was called.
    #[must_use]
    pub fn was_refresh_called(&self) -> bool {
        *self.refresh_called.lock().unwrap()
    }

    /// Reset internal state.
    pub fn reset(&self) {
        *self.refresh_called.lock().unwrap() = false;
        *self.refresh_token_response.lock().unwrap() = None;
        *self.should_fail.lock().unwrap() = false;
    }

    /// Construct a real OAuth client for hybrid tests.
    pub fn real_client() -> OAuthClient {
        let config = OAuthConfig::new(
            "dev-test.us.auth0.com".to_string(),
            "test_client_id".to_string(),
            "http://localhost:8888/callback".to_string(),
            vec!["openid".to_string(), "profile".to_string()],
            Some("https://api.example.com".to_string()),
        );
        OAuthClient::new(config)
    }
}

#[cfg(feature = "platform")]
impl Default for MockOAuthClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "platform")]
#[async_trait]
impl OAuthClientTrait for MockOAuthClient {
    async fn generate_authorization_url(&self) -> Result<(String, String), OAuthClientError> {
        Ok((
            "https://mock.auth0.com/authorize?client_id=test".to_string(),
            "mock_state_123".to_string(),
        ))
    }

    async fn exchange_code_for_tokens(
        &self,
        _code: &str,
        _state: &str,
    ) -> Result<TokenSet, OAuthClientError> {
        Ok(TokenSet::new(
            "mock_access_token".to_string(),
            Some("mock_refresh_token".to_string()),
            None,
            3600,
            Some("openid profile".to_string()),
        ))
    }

    async fn refresh_access_token(
        &self,
        _refresh_token: &str,
    ) -> Result<TokenSet, OAuthClientError> {
        *self.refresh_called.lock().unwrap() = true;

        if *self.should_fail.lock().unwrap() {
            return Err(OAuthClientError::NoRefreshToken);
        }

        let response = self.refresh_token_response.lock().unwrap();
        if let Some(tokens) = response.as_ref() {
            Ok(tokens.clone())
        } else {
            Ok(TokenSet::new(
                "refreshed_access_token".to_string(),
                Some("refreshed_refresh_token".to_string()),
                None,
                3600,
                Some("openid profile".to_string()),
            ))
        }
    }

    fn redirect_uri(&self) -> &str {
        "http://localhost:8888/callback"
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for testing::mocks.
    use super::*;

    /// Validates `MockHttpClient::new` behavior for the mock http client
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `response.status` equals `200`.
    /// - Confirms `response.body` equals `"OK"`.
    /// - Confirms `client.request_count("https://example.com")` equals `1`.
    #[test]
    fn test_mock_http_client() {
        let client = MockHttpClient::new();
        client.add_response("https://example.com", 200, "OK");

        let response = client.get("https://example.com").unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "OK");

        assert_eq!(client.request_count("https://example.com"), 1);
    }

    /// Validates `MockHttpClient::new` behavior for the mock http client
    /// missing response scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_mock_http_client_missing_response() {
        let client = MockHttpClient::new();
        let result = client.get("https://example.com");
        assert!(result.is_err());
    }

    /// Validates `MockStorage::new` behavior for the mock storage scenario.
    ///
    /// Assertions:
    /// - Confirms `value` equals `Some("value1".to_string())`.
    /// - Ensures `storage.exists("key1")` evaluates to true.
    /// - Confirms `storage.len()` equals `1`.
    /// - Ensures `!storage.exists("key1")` evaluates to true.
    /// - Ensures `storage.is_empty()` evaluates to true.
    #[test]
    fn test_mock_storage() {
        let storage = MockStorage::new();
        storage.set("key1", "value1").unwrap();

        let value = storage.get("key1").unwrap();
        assert_eq!(value, Some("value1".to_string()));

        assert!(storage.exists("key1"));
        assert_eq!(storage.len(), 1);

        storage.delete("key1").unwrap();
        assert!(!storage.exists("key1"));
        assert!(storage.is_empty());
    }

    /// Validates `MockStorage::new` behavior for the mock storage keys
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `keys.len()` equals `2`.
    /// - Ensures `keys.contains(&"key1".to_string())` evaluates to true.
    /// - Ensures `keys.contains(&"key2".to_string())` evaluates to true.
    #[test]
    fn test_mock_storage_keys() {
        let storage = MockStorage::new();
        storage.set("key1", "value1").unwrap();
        storage.set("key2", "value2").unwrap();

        let keys = storage.keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
    }

    /// Validates `MockHttpClient::new` behavior for the mock http client
    /// response sequence scenario.
    ///
    /// Assertions:
    /// - Confirms `resp1.body` equals `"First"`.
    /// - Confirms `resp2.body` equals `"Second"`.
    /// - Confirms `resp3.status` equals `404`.
    #[test]
    fn test_mock_http_client_response_sequence() {
        let client = MockHttpClient::new();
        client.add_response_sequence(
            "https://api.example.com",
            vec![(200, "First"), (200, "Second"), (404, "Not Found")],
        );

        let resp1 = client.get("https://api.example.com").unwrap();
        assert_eq!(resp1.body, "First");

        let resp2 = client.get("https://api.example.com").unwrap();
        assert_eq!(resp2.body, "Second");

        let resp3 = client.get("https://api.example.com").unwrap();
        assert_eq!(resp3.status, 404);
    }

    /// Validates `MockHttpClient::new` behavior for the mock http client was
    /// called scenario.
    ///
    /// Assertions:
    /// - Ensures `!client.was_called("https://api.example.com")` evaluates to
    ///   true.
    /// - Ensures `client.was_called("https://api.example.com")` evaluates to
    ///   true.
    #[test]
    fn test_mock_http_client_was_called() {
        let client = MockHttpClient::new();
        client.add_response("https://api.example.com", 200, "OK");

        assert!(!client.was_called("https://api.example.com"));

        let _ = client.get("https://api.example.com");
        assert!(client.was_called("https://api.example.com"));
    }

    /// Validates `MockHttpClient::new` behavior for the mock http client was
    /// called times scenario.
    ///
    /// Assertions:
    /// - Ensures `client.was_called_times("https://api.example.com", 0)`
    ///   evaluates to true.
    /// - Ensures `client.was_called_times("https://api.example.com", 1)`
    ///   evaluates to true.
    /// - Ensures `client.was_called_times("https://api.example.com", 2)`
    ///   evaluates to true.
    #[test]
    fn test_mock_http_client_was_called_times() {
        let client = MockHttpClient::new();
        client.add_response("https://api.example.com", 200, "OK");

        assert!(client.was_called_times("https://api.example.com", 0));

        let _ = client.get("https://api.example.com");
        assert!(client.was_called_times("https://api.example.com", 1));

        let _ = client.get("https://api.example.com");
        assert!(client.was_called_times("https://api.example.com", 2));
    }

    /// Validates `MockHttpClient::new` behavior for the mock http client last
    /// request scenario.
    ///
    /// Assertions:
    /// - Ensures `client.last_request().is_none()` evaluates to true.
    /// - Confirms `last.url` equals `"https://api1.example.com"`.
    /// - Confirms `last.url` equals `"https://api2.example.com"`.
    #[test]
    fn test_mock_http_client_last_request() {
        let client = MockHttpClient::new();
        client.add_response("https://api1.example.com", 200, "OK");
        client.add_response("https://api2.example.com", 200, "OK");

        assert!(client.last_request().is_none());

        let _ = client.get("https://api1.example.com");
        let last = client.last_request().unwrap();
        assert_eq!(last.url, "https://api1.example.com");

        let _ = client.get("https://api2.example.com");
        let last = client.last_request().unwrap();
        assert_eq!(last.url, "https://api2.example.com");
    }

    /// Validates `MockHttpClient::new` behavior for the mock http client
    /// request urls scenario.
    ///
    /// Assertions:
    /// - Confirms `urls.len()` equals `2`.
    /// - Confirms `urls[0]` equals `"https://api1.example.com"`.
    /// - Confirms `urls[1]` equals `"https://api2.example.com"`.
    #[test]
    fn test_mock_http_client_request_urls() {
        let client = MockHttpClient::new();
        client.add_response("https://api1.example.com", 200, "OK");
        client.add_response("https://api2.example.com", 200, "OK");

        let _ = client.get("https://api1.example.com");
        let _ = client.get("https://api2.example.com");

        let urls = client.request_urls();
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "https://api1.example.com");
        assert_eq!(urls[1], "https://api2.example.com");
    }

    /// Validates `MockKeychainProvider::new` behavior for the mock keychain
    /// store and retrieve scenario.
    ///
    /// Assertions:
    /// - Confirms `retrieved.access_token` equals `"access123"`.
    /// - Confirms `retrieved.refresh_token` equals
    ///   `Some("refresh456".to_string())`.
    #[cfg(feature = "platform")]
    #[test]
    fn test_mock_keychain_store_and_retrieve() {
        let keychain = MockKeychainProvider::new("test-service");
        let tokens = TokenSet::new(
            "access123".to_string(),
            Some("refresh456".to_string()),
            None,
            3600,
            None,
        );

        keychain.store_tokens("test_user", &tokens).unwrap();

        let retrieved = keychain.retrieve_tokens("test_user").unwrap();
        assert_eq!(retrieved.access_token, "access123");
        assert_eq!(retrieved.refresh_token, Some("refresh456".to_string()));
    }

    /// Validates `MockKeychainProvider::new` behavior for the mock keychain
    /// delete scenario.
    ///
    /// Assertions:
    /// - Ensures `keychain.has_tokens("test_user")` evaluates to true.
    /// - Ensures `!keychain.has_tokens("test_user")` evaluates to true.
    #[cfg(feature = "platform")]
    #[test]
    fn test_mock_keychain_delete() {
        let keychain = MockKeychainProvider::new("test-service");
        let tokens = TokenSet::new("access".to_string(), None, None, 3600, None);

        keychain.store_tokens("test_user", &tokens).unwrap();
        assert!(keychain.has_tokens("test_user"));

        keychain.delete_tokens("test_user").unwrap();
        assert!(!keychain.has_tokens("test_user"));
    }

    /// Validates `MockOAuthClient::new` behavior for the mock oauth client
    /// refresh scenario.
    ///
    /// Assertions:
    /// - Ensures `!client.was_refresh_called()` evaluates to true.
    /// - Confirms `tokens.access_token` equals `"refreshed_access_token"`.
    /// - Ensures `client.was_refresh_called()` evaluates to true.
    #[cfg(feature = "platform")]
    #[tokio::test]
    async fn test_mock_oauth_client_refresh() {
        let client = MockOAuthClient::new();
        assert!(!client.was_refresh_called());

        let tokens =
            client.refresh_access_token("refresh_token").await.expect("refresh should succeed");
        assert_eq!(tokens.access_token, "refreshed_access_token");
        assert!(client.was_refresh_called());
    }

    /// Validates `MockOAuthClient::new` behavior for the mock oauth client
    /// failure scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    /// - Ensures `client.was_refresh_called()` evaluates to true.
    #[cfg(feature = "platform")]
    #[tokio::test]
    async fn test_mock_oauth_client_failure() {
        let client = MockOAuthClient::new();
        client.set_should_fail(true);

        let result = client.refresh_access_token("refresh_token").await;
        assert!(result.is_err());
        assert!(client.was_refresh_called());
    }
}
