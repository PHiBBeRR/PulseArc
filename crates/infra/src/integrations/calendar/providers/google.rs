//! Google Calendar provider implementation

use async_trait::async_trait;
use pulsearc_domain::Result;
use reqwest::Client;
use serde::Deserialize;
use tracing::warn;

use super::traits::{
    CalendarProviderTrait, FetchEventsResponse, RawCalendarEvent, TokenRefreshResponse,
};
use crate::errors::InfraError;

const GOOGLE_CALENDAR_API_BASE: &str = "https://www.googleapis.com/calendar/v3";

/// Google Calendar provider
pub struct GoogleCalendarProvider;

#[async_trait]
impl CalendarProviderTrait for GoogleCalendarProvider {
    async fn fetch_events(
        &self,
        access_token: &str,
        calendar_id: &str,
        query_params: &[(&str, String)],
    ) -> Result<FetchEventsResponse> {
        let client = Client::new();
        let url = format!("{}/calendars/{}/events", GOOGLE_CALENDAR_API_BASE, calendar_id);

        let response =
            client.get(&url).bearer_auth(access_token).query(query_params).send().await.map_err(
                |e| {
                    InfraError(pulsearc_domain::PulseArcError::Network(format!(
                        "Google API request failed: {}",
                        e
                    )))
                },
            )?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(InfraError(pulsearc_domain::PulseArcError::Network(format!(
                "Google API error ({}): {}",
                status, error_text
            )))
            .into());
        }

        let google_response: GoogleEventsResponse = response.json().await.map_err(|e| {
            InfraError(pulsearc_domain::PulseArcError::InvalidInput(format!(
                "Failed to parse Google response: {}",
                e
            )))
        })?;

        let events = google_response
            .items
            .into_iter()
            .map(
                |GoogleCalendarEvent {
                     id,
                     summary,
                     description,
                     start,
                     end,
                     recurring_event_id,
                     hangout_link,
                     attendees,
                 }| {
                    let is_all_day = start.date.is_some();
                    let subject = summary.filter(|s| !s.trim().is_empty());

                    let start_str = start.date_time.or(start.date).unwrap_or_default();
                    let end_str = end.date_time.or(end.date).unwrap_or_default();

                    // Parse attendees with validation
                    let parsed_attendees = attendees.map(|list| {
                        list.into_iter()
                            .filter_map(|a| validate_and_log_email(&a.email, &id))
                            .collect()
                    });

                    RawCalendarEvent {
                        id,
                        subject,
                        body_preview: description,
                        start: start_str,
                        end: end_str,
                        is_all_day,
                        calendar_id: Some(calendar_id.to_string()),
                        series_master_id: recurring_event_id,
                        hangout_link,
                        has_external_attendees: None, /* Not provided by Google API in basic
                                                       * response */
                        organizer_email: None,
                        organizer_domain: None,
                        meeting_id: None,
                        attendee_count: None,
                        external_attendee_count: None,
                        attendees: parsed_attendees,
                    }
                },
            )
            .collect();

        Ok(FetchEventsResponse {
            events,
            next_page_token: google_response.next_page_token,
            delta_token: google_response.next_sync_token,
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenRefreshResponse> {
        let client_id = std::env::var("GOOGLE_CALENDAR_CLIENT_ID").map_err(|_| {
            InfraError(pulsearc_domain::PulseArcError::Auth(
                "GOOGLE_CALENDAR_CLIENT_ID not set".into(),
            ))
        })?;
        let client_secret = std::env::var("GOOGLE_CALENDAR_CLIENT_SECRET").map_err(|_| {
            InfraError(pulsearc_domain::PulseArcError::Auth(
                "GOOGLE_CALENDAR_CLIENT_SECRET not set".into(),
            ))
        })?;

        let client = Client::new();
        let response = client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| {
                InfraError(pulsearc_domain::PulseArcError::Auth(format!(
                    "Token refresh request failed: {}",
                    e
                )))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(InfraError(pulsearc_domain::PulseArcError::Auth(format!(
                "Token refresh failed ({}): {}",
                status, error_text
            )))
            .into());
        }

        let refresh_response: GoogleTokenRefreshResponse = response.json().await.map_err(|e| {
            InfraError(pulsearc_domain::PulseArcError::Auth(format!(
                "Failed to parse token response: {}",
                e
            )))
        })?;

        Ok(TokenRefreshResponse {
            access_token: refresh_response.access_token,
            expires_in: refresh_response.expires_in,
        })
    }
}

/// Validate email address and log warnings for malformed emails
///
/// Returns None only for empty emails. Malformed emails (missing @) are logged
/// but kept, as provider data is canonical.
fn validate_and_log_email(email: &str, event_id: &str) -> Option<String> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        warn!(event_id, email, "empty attendee email");
        return None;
    }
    if !trimmed.contains('@') {
        warn!(event_id, email, "attendee email missing @ symbol");
        // Keep it anyway - provider data is canonical
    }
    Some(trimmed.to_string())
}

#[derive(Debug, Deserialize)]
struct GoogleEventsResponse {
    items: Vec<GoogleCalendarEvent>,
    #[serde(rename = "nextSyncToken")]
    next_sync_token: Option<String>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleCalendarEvent {
    id: String,
    summary: Option<String>,
    description: Option<String>,
    start: EventDateTime,
    end: EventDateTime,
    #[serde(rename = "recurringEventId")]
    recurring_event_id: Option<String>,
    #[serde(rename = "hangoutLink")]
    hangout_link: Option<String>,
    attendees: Option<Vec<GoogleAttendee>>,
}

#[derive(Debug, Deserialize)]
struct EventDateTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleAttendee {
    email: String,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenRefreshResponse {
    access_token: String,
    expires_in: i64,
}
