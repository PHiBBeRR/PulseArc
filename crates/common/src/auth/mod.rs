//! Core OAuth 2.0 + PKCE Infrastructure
//!
//! This module provides a unified OAuth 2.0 implementation with PKCE support
//! for desktop and mobile applications. It consolidates OAuth functionality
//! used across calendar, SAP, and main API integrations.
//!
//! # Features
//!
//! - **PKCE Flow**: RFC 7636 compliant Proof Key for Code Exchange
//! - **Token Management**: Automatic token refresh with configurable thresholds
//! - **Keychain Storage**: Secure token storage via platform-specific keychains
//! - **Background Refresh**: Intelligent auto-refresh that sleeps until needed
//! - **Multi-Provider**: Supports Auth0, Google, Microsoft, and custom OAuth
//!   servers
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │  OAuthService   │  High-level orchestrator
//! └────────┬────────┘
//!          │
//!          ├──► OAuthClient        (HTTP OAuth flows)
//!          ├──► TokenManager       (Token lifecycle + auto-refresh)
//!          │         │
//!          │         └──► KeychainProvider  (Platform-specific storage)
//!          │
//!          └──► PKCE utilities     (Challenge generation)
//! ```
//!
//! # Usage Example
//!
//! ```no_run
//! // Note: KeychainProvider must be created by application code
//! use std::sync::Arc;
//!
//! use pulsearc_common::auth::{OAuthConfig, OAuthService};
//! use pulsearc_common::security::KeychainProvider;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Configure OAuth provider
//!     let config = OAuthConfig::new(
//!         "dev-abc123.us.auth0.com".to_string(),
//!         "your_client_id".to_string(),
//!         "http://localhost:8888/callback".to_string(),
//!         vec!["openid".to_string(), "profile".to_string(), "offline_access".to_string()],
//!         Some("https://api.pulsearc.ai".to_string()),
//!     );
//!
//!     // Create service
//!     let keychain = Arc::new(KeychainProvider::new("PulseArc".to_string()));
//!     let service = OAuthService::new(
//!         config,
//!         keychain,
//!         "PulseArc.api".to_string(),
//!         "main".to_string(),
//!         300, // Refresh 5 minutes before expiry
//!     );
//!
//!     // Initialize (load existing tokens)
//!     service.initialize().await?;
//!
//!     // Start login flow
//!     let (auth_url, state) = service.start_login().await?;
//!     println!("Open this URL in your browser: {}", auth_url);
//!
//!     // ... user authorizes in browser, app receives callback ...
//!
//!     // Complete login
//!     let tokens = service.complete_login("authorization_code", &state).await?;
//!     println!("Logged in! Access token expires in {} seconds", tokens.expires_in);
//!
//!     // Start background auto-refresh
//!     service.start_auto_refresh();
//!
//!     // Get access token (auto-refreshes if needed)
//!     let access_token = service.get_access_token().await?;
//!     println!("Access token: {}", access_token);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Module Organization
//!
//! - **[`types`]**: Core OAuth types (`TokenSet`, `OAuthConfig`, `OAuthError`)
//! - **[`pkce`]**: PKCE challenge generation and validation
//! - **[`client`]**: OAuth HTTP client for authorization and token exchange
//! - **[`token_manager`]**: Token lifecycle management with auto-refresh
//! - **[`service`]**: High-level OAuth service orchestrator
//!
//! # Security Features
//!
//! - **PKCE**: Prevents authorization code interception
//! - **State Validation**: CSRF protection with cryptographic randomness
//! - **Secure Storage**: Platform keychain integration (macOS Keychain, Windows
//!   Credential Manager)
//! - **No Client Secrets**: Safe for desktop/mobile apps
//! - **Constant-Time Comparison**: Prevents timing attacks on state validation
//!
//! # Platform Support
//!
//! - **macOS**: Uses macOS Keychain Services via `keyring` crate
//! - **Windows**: Uses Windows Credential Manager via `keyring` crate
//! - **Linux**: Uses Secret Service API via `keyring` crate

pub mod client;
mod keychain;
pub mod pkce;
pub mod service;
pub mod token_manager;
pub mod traits;
pub mod types;

// Re-export commonly used types and functions
pub use client::{OAuthClient, OAuthClientError};
pub use pkce::PKCEChallenge;
// Re-export PKCE utility functions
pub use pkce::{generate_code_challenge, generate_code_verifier, generate_state, validate_state};
pub use service::{OAuthService, OAuthServiceError};
pub use token_manager::{TokenManager, TokenManagerError};
pub use traits::{KeychainTrait, OAuthClientTrait};
pub use types::{OAuthConfig, OAuthError, TokenResponse, TokenSet};

// Re-export OAuth callback server from calendar integration
// Note: OAuthCallbackServer is provided by integrations crate, not common
// pub use service::OAuthCallbackServer;
