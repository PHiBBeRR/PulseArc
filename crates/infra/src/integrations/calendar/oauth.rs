//! OAuth2 authentication orchestration for calendar providers.
//!
//! This module wraps the shared `pulsearc_common::auth` service to handle
//! PKCE-based OAuth flows, secure token storage via the keychain, and token
//! refresh for Google/Microsoft calendar integrations.

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use axum::extract::Query;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use pulsearc_common::auth::client::OAuthClientError;
use pulsearc_common::auth::service::{OAuthService, OAuthServiceError};
use pulsearc_common::auth::token_manager::TokenManagerError;
use pulsearc_common::auth::types::{OAuthConfig, TokenSet};
use pulsearc_common::security::KeychainProvider;
use pulsearc_domain::{PulseArcError, Result};
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::error;
use url::Url;

const KEYCHAIN_SERVICE_PREFIX: &str = "Pulsarc.calendar";
const DEFAULT_REDIRECT_URI: &str = "http://localhost/callback";
/// Configuration for calendar OAuth providers.
#[derive(Debug, Clone)]
pub struct CalendarOAuthSettings {
    pub provider: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub scopes: Vec<String>,
    pub audience: Option<String>,
    pub extra_authorize_params: Vec<(String, String)>,
    pub extra_token_params: Vec<(String, String)>,
    pub keychain_service_name: Option<String>,
    pub refresh_threshold_seconds: i64,
}

impl CalendarOAuthSettings {
    /// Create Google OAuth settings with sensible defaults.
    pub fn google(client_id: impl Into<String>, client_secret: Option<String>) -> Self {
        Self {
            provider: "google".to_string(),
            client_id: client_id.into(),
            client_secret,
            authorization_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_endpoint: "https://oauth2.googleapis.com/token".to_string(),
            scopes: vec![
                "https://www.googleapis.com/auth/calendar.readonly".to_string(),
                "openid".to_string(),
                "email".to_string(),
            ],
            audience: None,
            extra_authorize_params: vec![
                ("access_type".to_string(), "offline".to_string()),
                ("prompt".to_string(), "consent".to_string()),
            ],
            extra_token_params: Vec::new(),
            keychain_service_name: None,
            refresh_threshold_seconds: 300,
        }
    }
}

/// High-level OAuth manager for calendar providers.
pub struct CalendarOAuthManager {
    client_id: String,
    client_secret: Option<String>,
    authorization_endpoint: String,
    token_endpoint: String,
    scopes: Vec<String>,
    audience: Option<String>,
    extra_authorize_params: Vec<(String, String)>,
    extra_token_params: Vec<(String, String)>,
    keychain_service_name: String,
    keychain: Arc<KeychainProvider>,
    refresh_threshold_seconds: i64,
}

impl CalendarOAuthManager {
    /// Create a new manager from settings.
    pub fn new(settings: CalendarOAuthSettings) -> Self {
        let keychain_service_name = settings
            .keychain_service_name
            .clone()
            .unwrap_or_else(|| format!("{}.{}", KEYCHAIN_SERVICE_PREFIX, settings.provider));

        let keychain = Arc::new(KeychainProvider::new(&keychain_service_name));

        Self {
            client_id: settings.client_id,
            client_secret: settings.client_secret,
            authorization_endpoint: settings.authorization_endpoint,
            token_endpoint: settings.token_endpoint,
            scopes: settings.scopes,
            audience: settings.audience,
            extra_authorize_params: settings.extra_authorize_params,
            extra_token_params: settings.extra_token_params,
            keychain_service_name,
            keychain,
            refresh_threshold_seconds: settings.refresh_threshold_seconds,
        }
    }

    /// Begin the OAuth login flow for a specific calendar account.
    pub async fn start_login(&self, account_name: &str) -> Result<OAuthLoginSession> {
        let server = OAuthCallbackServer::start().await?;
        let redirect_uri = server.redirect_uri();

        let service = self.build_service(account_name, redirect_uri.clone())?;
        service.initialize().await.map_err(map_oauth_service_error)?;

        let (authorization_url, state) =
            service.start_login().await.map_err(map_oauth_service_error)?;
        server.set_expected_state(state.clone());

        Ok(OAuthLoginSession { service, server, state, authorization_url })
    }

    /// Load tokens from keychain. Returns true if tokens exist.
    pub async fn initialize(&self, account_name: &str) -> Result<bool> {
        let service = self.build_service(account_name, DEFAULT_REDIRECT_URI.to_string())?;
        service.initialize().await.map_err(map_oauth_service_error)
    }

    /// Retrieve current access token (auto-refreshing as needed).
    pub async fn get_access_token(&self, account_name: &str) -> Result<String> {
        let service = self.build_service(account_name, DEFAULT_REDIRECT_URI.to_string())?;
        service.initialize().await.map_err(map_oauth_service_error)?;
        service.get_access_token().await.map_err(map_oauth_service_error)
    }

    /// Retrieve the current token set without refreshing.
    pub async fn get_tokens(&self, account_name: &str) -> Result<Option<TokenSet>> {
        let service = self.build_service(account_name, DEFAULT_REDIRECT_URI.to_string())?;
        service.initialize().await.map_err(map_oauth_service_error)?;
        Ok(service.get_tokens().await)
    }

    /// Check if an account has stored tokens.
    pub async fn is_authenticated(&self, account_name: &str) -> Result<bool> {
        let service = self.build_service(account_name, DEFAULT_REDIRECT_URI.to_string())?;
        service.initialize().await.map_err(map_oauth_service_error)?;
        Ok(service.is_authenticated().await)
    }

    /// Clear stored tokens for the account.
    pub async fn logout(&self, account_name: &str) -> Result<()> {
        let service = self.build_service(account_name, DEFAULT_REDIRECT_URI.to_string())?;
        service.logout().await.map_err(map_oauth_service_error)
    }

    /// Query seconds until current token expires.
    pub async fn seconds_until_expiry(&self, account_name: &str) -> Result<Option<i64>> {
        let service = self.build_service(account_name, DEFAULT_REDIRECT_URI.to_string())?;
        service.initialize().await.map_err(map_oauth_service_error)?;
        Ok(service.seconds_until_expiry().await)
    }

    /// Expose keychain provider (primarily for tests).
    pub fn keychain(&self) -> Arc<KeychainProvider> {
        self.keychain.clone()
    }

    fn build_service(
        &self,
        account_name: &str,
        redirect_uri: String,
    ) -> Result<OAuthService<KeychainProvider>> {
        let mut config = self.build_config(redirect_uri)?;
        config.set_client_secret(self.client_secret.clone());
        config.set_authorization_endpoint(self.authorization_endpoint.clone());
        config.set_token_endpoint(self.token_endpoint.clone());

        for (key, value) in &self.extra_authorize_params {
            config.add_authorize_param(key.clone(), value.clone());
        }

        for (key, value) in &self.extra_token_params {
            config.add_token_param(key.clone(), value.clone());
        }

        let service =
            OAuthService::new(config, self.keychain.clone(), account_name.to_string(), self.refresh_threshold_seconds);

        Ok(service)
    }

    fn build_config(&self, redirect_uri: String) -> Result<OAuthConfig> {
        let domain = extract_domain(&self.authorization_endpoint)?;
        Ok(OAuthConfig::new(
            domain,
            self.client_id.clone(),
            redirect_uri,
            self.scopes.clone(),
            self.audience.clone(),
        ))
    }
}

/// Represents an in-flight OAuth login.
pub struct OAuthLoginSession {
    service: OAuthService<KeychainProvider>,
    server: OAuthCallbackServer,
    state: String,
    authorization_url: String,
}

impl OAuthLoginSession {
    /// Authorization URL to open in the user's browser.
    pub fn authorization_url(&self) -> &str {
        &self.authorization_url
    }

    /// Redirect URI supplied to the provider.
    pub fn redirect_uri(&self) -> String {
        self.server.redirect_uri()
    }

    /// Wait for the OAuth callback, exchange the code for tokens, and persist
    /// them.
    pub async fn finish(self, timeout: Duration) -> Result<TokenSet> {
        let code = self.server.wait_for_code(timeout).await?;
        let tokens = self
            .service
            .complete_login(&code, &self.state)
            .await
            .map_err(map_oauth_service_error)?;

        self.service.start_auto_refresh();

        self.server.shutdown().await?;
        Ok(tokens)
    }
}

/// OAuth callback data captured by loopback server.
#[derive(Debug, Clone)]
pub struct OAuthCallbackData {
    pub code: String,
    pub state: String,
}

/// Loopback HTTP server that receives OAuth redirect callbacks.
pub struct OAuthCallbackServer {
    port: u16,
    callback_data: Arc<StdMutex<Option<OAuthCallbackData>>>,
    expected_state: Arc<StdMutex<Option<String>>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    handle: Option<JoinHandle<()>>,
}

impl OAuthCallbackServer {
    /// Start the loopback server on an ephemeral port.
    pub async fn start() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|err| {
            PulseArcError::Network(format!("failed to bind OAuth loopback server: {err}"))
        })?;

        let port = listener
            .local_addr()
            .map_err(|err| PulseArcError::Network(format!("failed to determine port: {err}")))?
            .port();

        let callback_data = Arc::new(StdMutex::new(None));
        let expected_state = Arc::new(StdMutex::new(None));

        let callback_data_clone = callback_data.clone();
        let expected_state_clone = expected_state.clone();

        let app = Router::new().route(
            "/callback",
            get(move |query: Query<HashMap<String, String>>| {
                handle_oauth_callback(
                    query,
                    callback_data_clone.clone(),
                    expected_state_clone.clone(),
                )
            }),
        );

        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let handle = tokio::spawn(async move {
            if let Err(err) = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
            {
                error!("OAuth callback server error: {}", err);
            }
        });

        Ok(Self {
            port,
            callback_data,
            expected_state,
            shutdown_tx: Some(shutdown_tx),
            handle: Some(handle),
        })
    }

    /// Redirect URI used in the authorization request.
    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}/callback", self.port)
    }

    /// Configure expected OAuth state for CSRF validation.
    pub fn set_expected_state(&self, state: String) {
        let mut guard = self.expected_state.lock().expect("expected_state poisoned");
        *guard = Some(state);
    }

    /// Await the OAuth callback with a timeout.
    pub async fn wait_for_code(&self, timeout: Duration) -> Result<String> {
        {
            let guard = self.expected_state.lock().expect("expected_state poisoned");
            if guard.is_none() {
                return Err(PulseArcError::Config(
                    "OAuth expected state not configured".to_string(),
                ));
            }
        }

        let deadline = Instant::now() + timeout;

        loop {
            {
                let data_guard = self.callback_data.lock().expect("callback_data poisoned");
                if let Some(data) = data_guard.clone() {
                    return Ok(data.code);
                }
            }

            if Instant::now() > deadline {
                return Err(PulseArcError::Network(
                    "OAuth callback timeout waiting for authorization code".into(),
                ));
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Shut down the loopback server gracefully.
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.handle.take() {
            if let Err(err) = handle.await {
                if err.is_panic() {
                    return Err(PulseArcError::Internal(format!(
                        "OAuth callback server panicked: {err}"
                    )));
                }
            }
        }

        Ok(())
    }
}

impl Drop for OAuthCallbackServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            if !handle.is_finished() {
                handle.abort();
            }
        }
    }
}

async fn handle_oauth_callback(
    Query(params): Query<HashMap<String, String>>,
    callback_data: Arc<StdMutex<Option<OAuthCallbackData>>>,
    expected_state: Arc<StdMutex<Option<String>>>,
) -> Html<&'static str> {
    let code = params.get("code").cloned();
    let state = params.get("state").cloned();
    let expected = expected_state.lock().expect("expected_state poisoned").clone();

    match (code, state, expected) {
        (Some(code), Some(state), Some(expected_state)) if state == expected_state => {
            let mut guard = callback_data.lock().expect("callback_data poisoned");
            *guard = Some(OAuthCallbackData { code, state });

            Html(
                r#"<!DOCTYPE html>
<html>
<head><title>Authorization Complete</title></head>
<body><h1>Authorization Successful</h1><p>You can close this window.</p></body>
</html>"#,
            )
        }
        _ => Html(
            r#"<!DOCTYPE html>
<html>
<head><title>Authorization Failed</title></head>
<body><h1>Authorization Failed</h1><p>Invalid or unexpected callback parameters.</p></body>
</html>"#,
        ),
    }
}

fn extract_domain(endpoint: &str) -> Result<String> {
    let url = Url::parse(endpoint)
        .map_err(|err| PulseArcError::Config(format!("invalid OAuth endpoint URL: {err}")))?;

    url.host_str()
        .map(|host| host.to_string())
        .ok_or_else(|| PulseArcError::Config("OAuth endpoint missing host".to_string()))
}

fn map_oauth_service_error(err: OAuthServiceError) -> PulseArcError {
    match err {
        OAuthServiceError::TokenManager(inner) => map_token_manager_error(inner),
        OAuthServiceError::OAuthClient(inner) => map_oauth_client_error(inner),
        OAuthServiceError::ConfigError(msg) => PulseArcError::Config(msg),
        OAuthServiceError::BrowserError(msg) => PulseArcError::Platform(msg),
    }
}

fn map_token_manager_error(err: TokenManagerError) -> PulseArcError {
    match err {
        TokenManagerError::KeychainError(msg) => PulseArcError::Security(msg),
        TokenManagerError::OAuthError(inner) => map_oauth_client_error(inner),
        TokenManagerError::NotAuthenticated => {
            PulseArcError::Auth("calendar account not authenticated".to_string())
        }
        TokenManagerError::RefreshFailed(msg) => PulseArcError::Auth(msg),
        TokenManagerError::NoRefreshToken => {
            PulseArcError::Auth("missing refresh token for calendar account".to_string())
        }
    }
}

fn map_oauth_client_error(err: OAuthClientError) -> PulseArcError {
    match err {
        OAuthClientError::RequestFailed(e) => PulseArcError::Network(e.to_string()),
        OAuthClientError::OAuthError(e) => PulseArcError::Auth(e.to_string()),
        OAuthClientError::StateMismatch { expected, received } => PulseArcError::Security(format!(
            "OAuth state mismatch (expected {expected}, received {received})"
        )),
        OAuthClientError::ParseError(msg) => PulseArcError::InvalidInput(msg),
        OAuthClientError::NoRefreshToken => PulseArcError::Auth("no refresh token issued".into()),
        OAuthClientError::ConfigError(msg) => PulseArcError::Config(msg),
        OAuthClientError::PKCEError(msg) => PulseArcError::Security(msg),
    }
}

/// Generate token reference ID for keychain storage.
pub fn generate_token_reference_id() -> Result<String> {
    Ok(uuid::Uuid::now_v7().to_string())
}

/// Extract email from ID token (JWT).
pub fn extract_email_from_id_token(id_token: &str) -> Result<String> {
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() != 3 {
        return Err(PulseArcError::InvalidInput("invalid ID token format".into()));
    }

    let payload_bytes = URL_SAFE_NO_PAD.decode(parts[1]).map_err(|err| {
        PulseArcError::InvalidInput(format!("failed to decode ID token payload: {err}"))
    })?;
    let payload_str = String::from_utf8(payload_bytes).map_err(|err| {
        PulseArcError::InvalidInput(format!("invalid UTF-8 in ID token payload: {err}"))
    })?;

    let payload: serde_json::Value = serde_json::from_str(&payload_str).map_err(|err| {
        PulseArcError::InvalidInput(format!("failed to parse ID token payload: {err}"))
    })?;

    payload
        .get("email")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| PulseArcError::InvalidInput("email claim missing from ID token".into()))
}

/// OAuth token response (legacy compatibility helper).
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
}
