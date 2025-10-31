# Calendar Integration

OAuth2-based calendar integration for Google Calendar and Microsoft Calendar (Outlook/365).

## Feature Flag

This module is only available when the `calendar` feature is enabled:

```toml
[dependencies]
pulsearc-infra = { path = "crates/infra", features = ["calendar"] }
```

## Prerequisites

### Google Calendar

1. Create OAuth2 client in [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
2. Enable Google Calendar API
3. Configure OAuth consent screen
4. Add authorized redirect URIs: `http://localhost:*/callback` (wildcard port)
5. Required scopes:
   - `https://www.googleapis.com/auth/calendar.readonly`
   - `openid`
   - `email`

### Microsoft Calendar

1. Register application in [Azure Portal](https://portal.azure.com/#blade/Microsoft_AAD_RegisteredApps/ApplicationsListBlade)
2. Add Microsoft Graph API permissions:
   - `Calendars.Read` (delegated)
   - `offline_access` (delegated)
3. Configure redirect URI: `http://localhost:*/callback`
4. Create client secret

## Configuration

### Environment Variables

```bash
# Google Calendar
export GOOGLE_CALENDAR_CLIENT_ID="your-client-id.apps.googleusercontent.com"
export GOOGLE_CALENDAR_CLIENT_SECRET="your-client-secret"

# Microsoft Calendar
export MICROSOFT_CALENDAR_CLIENT_ID="your-application-id"
export MICROSOFT_CALENDAR_CLIENT_SECRET="your-client-secret"

# User/Org IDs (required for sync)
export USER_ID="user-uuid"
export ORG_ID="org-uuid"
```

### Database Schema

Required tables (should already exist from legacy migration):

- `calendar_tokens`: OAuth token storage (token_ref, provider, user_email, expires_at)
- `calendar_sync_settings`: Sync configuration per user
- `calendar_events`: Parsed calendar events with metadata
- `time_entry_outbox`: Suggested time entries from calendar events

## Usage

### OAuth Flow

```rust
use pulsearc_infra::integrations::calendar::{
    OAuthCallbackServer, generate_code_verifier, generate_code_challenge,
    generate_state, exchange_code_for_tokens
};

// 1. Start loopback server
let expected_state = generate_state()?;
let server = OAuthCallbackServer::start(expected_state.clone()).await?;
let redirect_uri = server.redirect_uri();

// 2. Generate PKCE parameters
let code_verifier = generate_code_verifier()?;
let code_challenge = generate_code_challenge(&code_verifier)?;

// 3. Build authorization URL (provider-specific)
let auth_url = format!(
    "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&...",
    client_id, redirect_uri
);

// 4. Open browser, wait for callback
let auth_code = server.wait_for_code(120).await?;

// 5. Exchange code for tokens
let tokens = exchange_code_for_tokens(&auth_code, &code_verifier, &redirect_uri).await?;
```

### Calendar Sync

```rust
use pulsearc_infra::integrations::calendar::{CalendarSyncWorker, CalendarClient};

let worker = CalendarSyncWorker::new(client);
let suggestions_count = worker.perform_sync(user_email, &db_connection).await?;
```

## Provider Differences

### Google Calendar

- **API**: REST (v3)
- **Sync**: Incremental via `syncToken` parameter
- **Rate Limit**: 10 requests/second
- **410 GONE**: Sync token expired, clear and trigger full resync

### Microsoft Calendar

- **API**: Microsoft Graph (v1.0)
- **Sync**: Delta queries via `@odata.deltaLink`
- **Rate Limit**: 2000 requests/minute
- **Authentication**: Azure AD OAuth2

## Token Management

Tokens are stored in macOS Keychain:

- **Service**: `Pulsarc.calendar.{provider}`
- **Account**: `access.{token_ref}` and `refresh.{token_ref}`
- **Security**: OS-level encryption, user must approve "Always Allow" on first access

### Token Expiry

- Access tokens expire after ~1 hour
- System automatically refreshes tokens when needed (5-minute buffer)
- Refresh tokens are long-lived (no expiration for Google, 90 days for Microsoft)

## Rate Limits & Backoff

Both providers implement exponential backoff with jitter:

- Base delay: 1 second
- Max delay: 32 seconds
- Max attempts: 5
- Jitter: ±25%

## Rollback

If issues arise:

1. Disable `calendar` feature in `Cargo.toml`
2. Rebuild: `cargo build`
3. System falls back to legacy calendar adapter (if available)

## Monitoring

Key metrics to track:

- OAuth failures (token expired, refresh failed)
- Sync failures (410 GONE, rate limits)
- Event parsing errors
- Keychain access errors
- HTTP latency (p50, p95, p99)

## Troubleshooting

### "No calendar connection found"

- Check `calendar_tokens` table has entry for user_email
- Verify `token_ref` is valid UUID

### "Sync token invalid (410 GONE)"

- Normal after long periods without sync
- System automatically clears sync_token and triggers full resync

### "GOOGLE_CALENDAR_CLIENT_ID not set"

- Set environment variables before starting application
- Or configure in `.env` file (if using dotenv)

### Keychain prompts on every run

- Click "Always Allow" when prompted
- Keychain should remember the decision

## Security Notes

- Tokens are NEVER logged or displayed
- PKCE prevents auth code interception
- State parameter prevents CSRF attacks
- Keychain provides OS-level encryption
- Refresh tokens should be rotated periodically (manual process)

## Future Enhancements

- [ ] Migrate to `pulsearc-common::security::KeychainProvider` (Phase 4)
- [ ] Add calendar event write support (create/update/delete)
- [ ] Support additional providers (Apple Calendar, iCloud)
- [ ] Implement periodic scheduler integration (3C.6)
- [ ] Add metrics/telemetry exports

