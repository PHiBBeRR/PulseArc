# Auth Module (OAuth 2.0 + PKCE)

Unified OAuth 2.0 infrastructure that powers desktop and service clients across PulseArc. The code lives in `crates/common/src/auth` and is re-exported as `pulsearc_common::auth::*`.

## Why this module exists
- Share a single, battle-tested PKCE implementation across the app surface (Tauri desktop shell, sync workers, calendar integrations).
- Offer resilient token lifecycle management with automatic refresh and secure storage.
- Provide trait-based seams so tests and platform-specific code can swap HTTP or storage layers without touching business logic.

## Feature highlights
- **PKCE (S256)** end-to-end: verifier generation, challenge hashing, and CSRF state handling.
- **Background refresh** via `TokenManager::start_auto_refresh`, tuned by a configurable refresh threshold.
- **Keychain persistence** built on `security::KeychainProvider` (macOS Keychain, Windows Credential Manager, Linux Secret Service).
- **Provider-agnostic**: works with Auth0, Google, Microsoft, or any OAuth 2.0 server that follows the authorization-code flow.
- **Mock-friendly**: `OAuthClientTrait` and `KeychainTrait` abstractions plus ready-made mocks in `pulsearc_common::testing`.

## Source map
| Path | Responsibility |
| ---- | -------------- |
| `mod.rs` | Module docs, public re-exports (`OAuthService`, `TokenManager`, PKCE utilities, etc.). |
| `types.rs` | Core data types (`TokenSet`, `TokenResponse`, `OAuthConfig`, `OAuthError`). |
| `pkce.rs` | PKCE helpers and `PKCEChallenge` struct. |
| `client.rs` | HTTP OAuth client built on `reqwest`, orchestrates authorization, code exchange, and refresh. |
| `token_manager.rs` | Token caching, refresh logic, and background worker. |
| `service.rs` | High-level facade that ties together client + token manager + keychain. |
| `keychain.rs` | OAuth-specific helpers layered on top of `security::KeychainProvider`. |
| `traits.rs` | Trait contracts for client and keychain, enabling swaps/mocks. |
| `../../tests/auth_integration.rs` | Feature-gated integration tests that exercise the full flow with mocks. |

## How the pieces fit
```
+-------------------------------------------------------------------+
| Application code (Tauri command, background worker, etc.)         |
+------------------------------+------------------------------------+
                               v
                        OAuthService (service.rs)
                               | orchestrates
                               v
                        TokenManager (token_manager.rs) --> stores --> KeychainProvider
                               | refreshes                               ^
                               v                                         |
                        OAuthClient (client.rs) <-- HTTP --> OAuth provider
                               ^                                         |
                               `- PKCE utilities (pkce.rs) provide verifier/challenge/state
```

## Quickstart
```rust
use std::sync::Arc;

use pulsearc_common::auth::{OAuthConfig, OAuthService};
use pulsearc_common::security::KeychainProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Describe the OAuth provider.
    let config = OAuthConfig::new(
        "dev-example.us.auth0.com".into(),
        "desktop-client-id".into(),
        "http://127.0.0.1:14251/callback".into(),
        vec!["openid".into(), "profile".into(), "offline_access".into()],
        Some("https://api.pulsearc.ai".into()),
    );

    // 2. Create the keychain provider and service facade.
    let keychain = Arc::new(KeychainProvider::new("PulseArc.oauth"));
    let service = OAuthService::new(
        config,
        keychain,
        "PulseArc.desktop".into(), // namespace inside the system keychain
        "primary-user".into(),     // account identifier
        300,                       // refresh 5 minutes before expiry
    );

    // 3. Restore persisted tokens if they exist.
    if service.initialize().await? {
        service.start_auto_refresh();
        println!("Reusing previously stored credentials");
        return Ok(());
    }

    // 4. Start the browser flow and persist the state.
    let (auth_url, state) = service.start_login().await?;
    println!("Open this URL in your browser: {auth_url}");

    // 5. Handle the callback in your app (pseudo code):
    // let (code, returned_state) = wait_for_oauth_callback();
    // let tokens = service.complete_login(&code, &returned_state).await?;

    // 6. After successful login, enable background refresh and fetch tokens.
    service.start_auto_refresh();
    let access_token = service.get_access_token().await?;
    println!("Ready to call APIs with bearer token: {access_token}");

    Ok(())
}
```

**Important runtime notes**
- `start_auto_refresh` spawns an async task; call it once per service lifetime after login or restore.
- `initialize` returns `Ok(true)` when keychain tokens were loaded, allowing the app to skip an interactive login.
- Call `logout` to revoke local state (clears pending login state and wipes keychain entries).

## Key data types
- `OAuthConfig`: issuer domain, client id, redirect URI, scopes, optional audience. Produces the canonical authorize and token URLs.
- `TokenSet`: normalized token payload persisted in memory and in the keychain. Includes `expires_in`, calculated `expires_at`, optional `refresh_token`, optional `id_token`, and granted `scope`.
- `PKCEChallenge`: tuple of verifier, challenge, and state. `generate_*` helpers are also exposed individually (`generate_code_verifier`, `generate_code_challenge`, `generate_state`, `validate_state`).

## Component deep-dive
### `OAuthClient`
- One instance per OAuth configuration; internally caches the latest PKCE challenge for state validation.
- Throws `OAuthClientError::StateMismatch` on CSRF attempts.
- Uses form POSTs to the token endpoint; parses errors into `OAuthError`.

### `TokenManager`
- Persists `TokenSet` via `KeychainTrait::store_tokens` and keeps an in-memory copy guarded by `RwLock`.
- Refreshes tokens eagerly when `seconds_until_expiry <= refresh_threshold`.
- `start_auto_refresh` loops forever; on refresh failures it waits 60 s before retrying.
- Exposes helpers: `get_tokens`, `is_authenticated`, `seconds_until_expiry`, and `clear_tokens` for logout.

### `OAuthService`
- Wraps everything in a single API for UI code.
- Tracks pending login state in memory and guarantees the state parameter is validated before exchanging codes.
- Returns `OAuthServiceError`, composed of `TokenManagerError`, `OAuthClientError`, and configuration/browser issues.

### Keychain helpers
- `keychain.rs` adds `store_tokens`, `retrieve_tokens`, `delete_tokens`, and `has_tokens` to `KeychainProvider`.
- Tokens are stored under deterministic prefixes: `access.{account}`, `refresh.{account}`, `metadata.{account}`. Metadata includes `expires_at` timestamps so restart restores the correct expiry.

### Traits for customization
- `OAuthClientTrait` lets you plug in alternate clients (e.g., device flow, custom HTTP stack, or mocks).
- `KeychainTrait` allows alternative storage backends (in-memory, encrypted file, remote secret manager). The testing crate exposes `MockKeychainProvider` implementing this trait.

## Error model
- `OAuthClientError`: network failures, invalid responses, PKCE/state mismatches, missing refresh tokens.
- `TokenManagerError`: wraps keychain issues, refresh failures, and unauthenticated access attempts.
- `OAuthServiceError`: top-level error surfaced to callers; use pattern matching to inspect root causes.
- `OAuthError`: OAuth server responses (`error` + optional `error_description`), bubbled up when providers reject requests.

## Testing
- Unit tests live beside each module (`#[cfg(test)]`). Integration coverage is in `crates/common/tests/auth_integration.rs`.
- The module depends on the `platform` feature for keychain + reqwest support. Run tests with:
  ```bash
  cargo test -p pulsearc-common --features "platform test-utils"
  cargo test -p pulsearc-common --features "platform" auth_integration
  ```
- Use `pulsearc_common::testing::{MockKeychainProvider, MockOAuthClient}` to avoid touching the real keychain or network.

## Feature flags & dependencies
- Enable the `platform` feature (which pulls `runtime` and `foundation`) to compile this module: it activates `reqwest`, `tokio`, `keyring`, `urlencoding`, and other allies.
- Background tasks require a Tokio runtime; the desktop app and back-end services already run inside one.
- The module avoids storing client secrets; OAuth providers must accept public clients that use PKCE.

## Troubleshooting tips
- `StateMismatch`: ensure the state captured during `start_login` is the same one passed to `complete_login`. Clear pending state with `clear_pending_state` if you abandon a login attempt.
- `TokenManagerError::NoRefreshToken`: provider did not issue a refresh token (common if `offline_access` scope was missing). Handle by prompting for re-authentication instead of relying on auto-refresh.
- `TokenManagerError::KeychainError`: keychain access failed -- on macOS this often means the app lacks the proper keychain entitlement; in tests, make sure `platform` feature is enabled or use mocks.

## Related modules
- `crates/common/src/security/encryption/keychain.rs`: generic secure storage used underneath the auth helpers.
- `crates/common/src/testing/mocks.rs`: mock implementations referenced above.
- `crates/common/src/security/keychain.rs`: re-exported `KeychainProvider` if you prefer `pulsearc_common::security::keychain::KeychainProvider`.

With this README you should be able to navigate, extend, and safely consume the PulseArc OAuth stack without diving through each source file first.
