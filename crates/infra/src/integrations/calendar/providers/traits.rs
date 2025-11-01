//! Calendar provider trait and factory
//!
//! Defines the common interface for calendar providers and factory function.

use async_trait::async_trait;
use pulsearc_domain::Result;
use serde::{Deserialize, Serialize};

use crate::errors::InfraError;

/// Raw calendar event from provider API (before parsing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawCalendarEvent {
    pub id: String,
    pub subject: Option<String>,
    pub body_preview: Option<String>,
    pub start: String,
    pub end: String,
    pub is_all_day: bool,
    pub calendar_id: Option<String>,
    pub series_master_id: Option<String>,
    pub hangout_link: Option<String>,
    pub has_external_attendees: Option<bool>,
    pub organizer_email: Option<String>,
    pub organizer_domain: Option<String>,
    pub meeting_id: Option<String>,
    pub attendee_count: Option<i32>,
    pub external_attendee_count: Option<i32>,
    pub attendees: Option<Vec<String>>,
}

/// Response from calendar provider fetch_events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchEventsResponse {
    pub events: Vec<RawCalendarEvent>,
    pub next_page_token: Option<String>,
    pub delta_token: Option<String>,
}

/// Trait for calendar provider operations
#[async_trait]
pub trait CalendarProviderTrait: Send + Sync {
    /// Fetch events from the calendar provider
    async fn fetch_events(
        &self,
        access_token: &str,
        calendar_id: &str,
        query_params: &[(&str, String)],
    ) -> Result<FetchEventsResponse>;

    /// Refresh an access token using a refresh token
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenRefreshResponse>;
}

/// Token refresh response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshResponse {
    pub access_token: String,
    pub expires_in: i64,
}

/// Create a calendar provider instance by name
pub fn create_provider(provider: &str) -> Result<Box<dyn CalendarProviderTrait>> {
    match provider {
        "google" => Ok(Box::new(super::google::GoogleCalendarProvider)),
        "microsoft" => Ok(Box::new(super::microsoft::MicrosoftCalendarProvider::default())),
        _ => Err(InfraError(pulsearc_domain::PulseArcError::InvalidInput(format!(
            "unknown provider: {}",
            provider
        )))
        .into()),
    }
}
