//! Calendar HTTP client with token management
//!
//! Handles OAuth token retrieval, automatic refresh, and HTTP requests to
//! calendar APIs.

use std::sync::Arc;

use pulsearc_domain::Result;
use tracing::debug;

use super::oauth::CalendarOAuthManager;
use super::providers::{create_provider, FetchEventsResponse};

/// Calendar API client with token management
#[derive(Clone)]
pub struct CalendarClient {
    account_name: String,
    provider: String,
    oauth_manager: Arc<CalendarOAuthManager>,
}

impl CalendarClient {
    /// Create a new calendar client
    ///
    /// # Arguments
    /// * `account_name` - Account identifier (usually user email)
    /// * `provider` - Provider name ("google" or "microsoft")
    /// * `oauth_manager` - OAuth manager for token retrieval/refresh
    pub fn new(
        account_name: String,
        provider: String,
        oauth_manager: Arc<CalendarOAuthManager>,
    ) -> Result<Self> {
        Ok(Self { account_name, provider, oauth_manager })
    }

    /// Fetch calendar events using provider-specific API
    ///
    /// Automatically retrieves/refreshes access token via OAuth manager.
    pub async fn fetch_events(
        &self,
        calendar_id: &str,
        query_params: &[(&str, String)],
    ) -> Result<FetchEventsResponse> {
        let access_token = self.oauth_manager.get_access_token(&self.account_name).await?;

        debug!(
            provider = %self.provider,
            calendar_id,
            "fetching events from calendar API"
        );

        let provider = create_provider(&self.provider)?;
        provider.fetch_events(&access_token, calendar_id, query_params).await
    }

    /// Return the configured provider identifier (`"google"`, `"microsoft"`,
    /// …).
    pub fn provider(&self) -> &str {
        &self.provider
    }
}
