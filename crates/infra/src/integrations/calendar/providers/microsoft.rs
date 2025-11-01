//! Microsoft Calendar provider implementation

use async_trait::async_trait;
use pulsearc_domain::Result;
use reqwest::Client;
use serde::Deserialize;
use tracing::warn;

use super::traits::{
    CalendarProviderTrait, FetchEventsResponse, RawCalendarEvent, TokenRefreshResponse,
};
use crate::errors::InfraError;

const MICROSOFT_GRAPH_API_BASE: &str = "https://graph.microsoft.com/v1.0";
const OUTLOOK_TIMEZONE_HEADER: &str = r#"outlook.timezone="UTC""#;
const OUTLOOK_MAX_PAGE_SIZE_HEADER: &str = r#"odata.maxpagesize=50"#;
const OUTLOOK_ID_TYPE_HEADER: &str = r#"IdType="ImmutableId""#;

/// Microsoft Calendar provider
#[derive(Clone)]
pub struct MicrosoftCalendarProvider {
    client: Client,
}

impl MicrosoftCalendarProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

impl Default for MicrosoftCalendarProvider {
    fn default() -> Self {
        Self::new(Client::new())
    }
}

#[async_trait]
impl CalendarProviderTrait for MicrosoftCalendarProvider {
    async fn fetch_events(
        &self,
        access_token: &str,
        calendar_id: &str,
        query_params: &[(&str, String)],
    ) -> Result<FetchEventsResponse> {
        let url = if calendar_id.eq_ignore_ascii_case("primary") {
            format!("{}/me/calendarView/delta", MICROSOFT_GRAPH_API_BASE)
        } else {
            format!("{}/me/calendars/{}/calendarView/delta", MICROSOFT_GRAPH_API_BASE, calendar_id)
        };

        let response = self
            .client
            .get(&url)
            .bearer_auth(access_token)
            .header("Prefer", OUTLOOK_TIMEZONE_HEADER)
            .header("Prefer", OUTLOOK_MAX_PAGE_SIZE_HEADER)
            .header("Prefer", OUTLOOK_ID_TYPE_HEADER)
            .query(query_params)
            .send()
            .await
            .map_err(|e| {
                InfraError(pulsearc_domain::PulseArcError::Network(format!(
                    "Microsoft API request failed: {}",
                    e
                )))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(InfraError(pulsearc_domain::PulseArcError::Network(format!(
                "Microsoft API error ({}): {}",
                status, error_text
            )))
            .into());
        }

        let ms_response: MicrosoftEventsResponse = response.json().await.map_err(|e| {
            InfraError(pulsearc_domain::PulseArcError::InvalidInput(format!(
                "Failed to parse Microsoft response: {}",
                e
            )))
        })?;

        let events = ms_response
            .value
            .into_iter()
            .map(
                |MicrosoftCalendarEvent {
                     id,
                     subject,
                     body_preview,
                     start,
                     end,
                     is_all_day,
                     series_master_id,
                     calendar_id: calendar_opt,
                     online_meeting,
                     attendees,
                 }| {
                    let subject = subject.filter(|s| !s.trim().is_empty());
                    let calendar_id = calendar_opt.or_else(|| Some(calendar_id.to_owned()));

                    // Parse attendees with validation
                    let parsed_attendees = attendees.map(|list| {
                        list.into_iter()
                            .filter_map(|a| validate_and_log_email(&a.email_address.address, &id))
                            .collect()
                    });

                    RawCalendarEvent {
                        id,
                        subject,
                        body_preview,
                        start: normalise_event_time(&start),
                        end: normalise_event_time(&end),
                        is_all_day,
                        calendar_id,
                        series_master_id,
                        hangout_link: online_meeting.and_then(|meeting| meeting.join_url),
                        has_external_attendees: None,
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
            next_page_token: ms_response.next_link,
            delta_token: ms_response.delta_link,
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenRefreshResponse> {
        let client_id = std::env::var("MICROSOFT_CALENDAR_CLIENT_ID").map_err(|_| {
            InfraError(pulsearc_domain::PulseArcError::Auth(
                "MICROSOFT_CALENDAR_CLIENT_ID not set".into(),
            ))
        })?;
        let client_secret = std::env::var("MICROSOFT_CALENDAR_CLIENT_SECRET").map_err(|_| {
            InfraError(pulsearc_domain::PulseArcError::Auth(
                "MICROSOFT_CALENDAR_CLIENT_SECRET not set".into(),
            ))
        })?;

        let response = self
            .client
            .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
            .form(&[
                ("client_id", client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
                ("scope", "Calendars.Read offline_access"),
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

        let refresh_response: MicrosoftTokenRefreshResponse =
            response.json().await.map_err(|e| {
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

#[derive(Debug, Deserialize)]
struct MicrosoftEventsResponse {
    value: Vec<MicrosoftCalendarEvent>,
    #[serde(rename = "@odata.nextLink")]
    next_link: Option<String>,
    #[serde(rename = "@odata.deltaLink")]
    delta_link: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MicrosoftCalendarEvent {
    id: String,
    subject: Option<String>,
    #[serde(rename = "bodyPreview")]
    body_preview: Option<String>,
    start: EventDateTime,
    end: EventDateTime,
    #[serde(rename = "isAllDay")]
    is_all_day: bool,
    #[serde(rename = "seriesMasterId")]
    series_master_id: Option<String>,
    #[serde(rename = "calendarId")]
    calendar_id: Option<String>,
    #[serde(rename = "onlineMeeting")]
    online_meeting: Option<OnlineMeeting>,
    attendees: Option<Vec<MicrosoftAttendee>>,
}

#[derive(Debug, Deserialize)]
struct EventDateTime {
    #[serde(rename = "dateTime")]
    date_time: String,
    #[serde(rename = "timeZone")]
    time_zone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MicrosoftTokenRefreshResponse {
    access_token: String,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct OnlineMeeting {
    #[serde(rename = "joinUrl")]
    join_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MicrosoftAttendee {
    #[serde(rename = "emailAddress")]
    email_address: EmailAddress,
}

#[derive(Debug, Deserialize)]
struct EmailAddress {
    address: String,
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

fn normalise_event_time(event: &EventDateTime) -> String {
    let value = event.date_time.trim();
    if value.ends_with('Z') {
        value.to_owned()
    } else if event.time_zone.as_deref().map(|tz| tz.eq_ignore_ascii_case("utc")).unwrap_or(false) {
        format!("{value}Z")
    } else {
        value.to_owned()
    }
}
