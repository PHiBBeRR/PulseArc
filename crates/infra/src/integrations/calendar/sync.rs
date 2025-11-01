//! Calendar sync worker
//!
//! Orchestrates periodic synchronization of calendar events from providers,
//! parsing them into time entry suggestions.

use std::sync::Arc;

use chrono::Utc;
use pulsearc_core::calendar_ports::SyncStatus;
use pulsearc_core::tracking::ports::CalendarEventRepository;
use pulsearc_core::OutboxQueue;
use pulsearc_domain::types::database::{ParsedFields, TimeRange};
use pulsearc_domain::{
    parse_event_title, CalendarEventParams, ParsedEventTitle, PulseArcError, Result,
};
use tracing::{debug, error, info, instrument, warn};
use url::{form_urlencoded, Url};
use uuid::Uuid;

use super::client::CalendarClient;
use super::platform::detect_meeting_platform;
use super::providers::RawCalendarEvent;
use super::types::{CalendarEvent, CalendarSyncSettings};

type QueryParam = (&'static str, String);

/// Calendar sync worker
pub struct CalendarSyncWorker {
    client: CalendarClient,
    calendar_repo: Arc<dyn CalendarEventRepository>,
    #[allow(dead_code)] // TODO: Use for suggestion generation
    outbox_queue: Arc<dyn OutboxQueue>,
    pool: Arc<pulsearc_common::storage::SqlCipherPool>,
}

impl CalendarSyncWorker {
    /// Create a new sync worker instance
    pub fn new(
        client: CalendarClient,
        calendar_repo: Arc<dyn CalendarEventRepository>,
        outbox_queue: Arc<dyn OutboxQueue>,
        pool: Arc<pulsearc_common::storage::SqlCipherPool>,
    ) -> Self {
        Self { client, calendar_repo, outbox_queue, pool }
    }

    /// Perform calendar synchronization for a specific user
    ///
    /// 1. Get sync settings from database
    /// 2. Build request params (initial vs incremental sync)
    /// 3. Fetch events from provider API
    /// 4. Parse event titles
    /// 5. Save to calendar_events table
    /// 6. Generate time entry suggestions
    /// 7. Update sync token
    #[instrument(skip(self), fields(user_email))]
    pub async fn perform_sync(&self, user_email: &str) -> Result<SyncStatus> {
        info!(user_email, "starting calendar sync");

        // Get sync settings
        let settings = self.get_sync_settings(user_email).await?;

        if !settings.enabled {
            debug!(user_email, "sync disabled for user");
            return Ok(SyncStatus { last_sync: None, events_synced: 0, success: true });
        }

        // Build query parameters based on sync token presence
        let provider = self.client.provider();
        let query_params = self.build_query_params(provider, &settings)?;

        // Fetch events from provider, following pagination when required
        let mut all_raw_events = Vec::new();
        let mut page_cursor: Option<String> = None;
        let mut latest_delta_token: Option<String> = None;

        loop {
            let mut paged_params = query_params.clone();
            if let Some(ref token) = page_cursor {
                if let Some(param) = Self::pagination_param(token) {
                    paged_params.push(param);
                } else {
                    warn!(
                        user_email,
                        token, "unsupported pagination token format; stopping pagination loop"
                    );
                    break;
                }
            }

            let response = match self.client.fetch_events("primary", &paged_params).await {
                Ok(resp) => resp,
                Err(e) => {
                    error!(user_email, error = %e, "failed to fetch calendar events");

                    // Check for 410 GONE (sync token invalid)
                    if format!("{:?}", e).contains("410") {
                        warn!(user_email, "sync token invalid (410 GONE), clearing for retry");
                        self.clear_sync_token(user_email).await?;
                    }

                    return Err(e);
                }
            };

            latest_delta_token = response.delta_token.or(latest_delta_token);
            page_cursor = response.next_page_token;

            all_raw_events.extend(response.events);

            if page_cursor.is_none() {
                break;
            }
        }

        // Parse events
        let parsed_events = self.parse_raw_events(all_raw_events, user_email).await?;

        // Save to database
        let saved_count = self.save_calendar_events(&parsed_events, user_email).await?;

        // Generate time entry suggestions
        let suggestions_count =
            self.generate_suggestions(&parsed_events, user_email, &settings).await?;

        // Update sync token only when provider returns a delta token
        if let Some(sync_token) = latest_delta_token {
            self.update_sync_token(user_email, &sync_token).await?;
        } else {
            debug!(
                user_email,
                "provider returned no delta token after sync; leaving existing token unchanged"
            );
        }

        let last_sync = Some(Utc::now());

        info!(user_email, saved_count, suggestions_count, "calendar sync completed successfully");

        Ok(SyncStatus { last_sync, events_synced: saved_count, success: true })
    }

    /// Get sync settings from database
    async fn get_sync_settings(&self, user_email: &str) -> Result<CalendarSyncSettings> {
        let conn = self.pool.get_sqlcipher_connection().map_err(|e| {
            crate::errors::InfraError(pulsearc_domain::PulseArcError::Database(format!(
                "Failed to get database connection: {}",
                e
            )))
        })?;

        let result = conn.query_row(
            "SELECT enabled, sync_interval_minutes, include_all_day_events,
                    min_event_duration_minutes, lookback_hours, lookahead_hours,
                    excluded_calendar_ids, sync_token, last_sync_epoch
             FROM calendar_sync_settings
             WHERE user_email = ?1",
            &[&user_email as &dyn rusqlite::ToSql],
            |row| {
                Ok(CalendarSyncSettings {
                    enabled: row.get(0)?,
                    sync_interval_minutes: row.get(1)?,
                    include_all_day_events: row.get(2)?,
                    min_event_duration_minutes: row.get(3)?,
                    lookback_hours: row.get(4)?,
                    lookahead_hours: row.get(5)?,
                    excluded_calendar_ids: row
                        .get::<_, String>(6)?
                        .split(',')
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                        .collect(),
                    sync_token: row.get(7)?,
                    last_sync_epoch: row.get(8)?,
                })
            },
        );

        result.map_err(|e| {
            use pulsearc_common::storage::error::StorageError;
            match e {
                StorageError::Query(msg) if msg.contains("no rows") => {
                    crate::errors::InfraError(pulsearc_domain::PulseArcError::NotFound(format!(
                        "Calendar sync settings not found for user: {}",
                        user_email
                    )))
                    .into()
                }
                StorageError::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => {
                    crate::errors::InfraError(pulsearc_domain::PulseArcError::NotFound(format!(
                        "Calendar sync settings not found for user: {}",
                        user_email
                    )))
                    .into()
                }
                other => crate::errors::InfraError::from(other).into(),
            }
        })
    }

    /// Build query parameters for API request
    fn build_query_params(
        &self,
        provider: &str,
        settings: &CalendarSyncSettings,
    ) -> Result<Vec<QueryParam>> {
        if provider.eq_ignore_ascii_case("microsoft") {
            return self.build_microsoft_query_params(settings);
        }

        self.build_google_query_params(settings)
    }
    fn build_google_query_params(
        &self,
        settings: &CalendarSyncSettings,
    ) -> Result<Vec<QueryParam>> {
        if let Some(ref sync_token) = settings.sync_token {
            Ok(vec![
                ("syncToken", sync_token.clone()),
                (
                    "fields",
                    "items(id,summary,description,start,end,recurringEventId,hangoutLink),nextPageToken,nextSyncToken"
                        .to_string(),
                ),
            ])
        } else {
            let now = Utc::now();
            let time_min = now - chrono::Duration::hours(settings.lookback_hours as i64);
            let time_max = now + chrono::Duration::hours(settings.lookahead_hours as i64);

            Ok(vec![
                ("singleEvents", "true".to_string()),
                ("orderBy", "startTime".to_string()),
                ("timeMin", time_min.to_rfc3339()),
                ("timeMax", time_max.to_rfc3339()),
                ("timeZone", "UTC".to_string()),
                (
                    "fields",
                    "items(id,summary,description,start,end,recurringEventId,hangoutLink),nextPageToken,nextSyncToken"
                        .to_string(),
                ),
            ])
        }
    }

    fn build_microsoft_query_params(
        &self,
        settings: &CalendarSyncSettings,
    ) -> Result<Vec<QueryParam>> {
        if let Some(ref sync_token) = settings.sync_token {
            return self.parse_microsoft_delta_params(sync_token);
        }

        let now = Utc::now();
        let time_min = now - chrono::Duration::hours(settings.lookback_hours as i64);
        let time_max = now + chrono::Duration::hours(settings.lookahead_hours as i64);

        Ok(vec![("startDateTime", time_min.to_rfc3339()), ("endDateTime", time_max.to_rfc3339())])
    }

    fn parse_microsoft_delta_params(&self, token: &str) -> Result<Vec<QueryParam>> {
        if token.trim().is_empty() {
            return Err(crate::errors::InfraError(pulsearc_domain::PulseArcError::InvalidInput(
                "empty Microsoft delta token".into(),
            ))
            .into());
        }

        if let Ok(url) = Url::parse(token) {
            return self.collect_microsoft_query_pairs(url.query());
        }

        if let Some(idx) = token.find('?') {
            return self.collect_microsoft_query_pairs(Some(&token[idx + 1..]));
        }

        // Treat as bare token value
        Ok(vec![("$deltatoken", token.to_string())])
    }

    fn collect_microsoft_query_pairs(&self, query: Option<&str>) -> Result<Vec<QueryParam>> {
        let Some(query) = query else {
            return Err(crate::errors::InfraError(pulsearc_domain::PulseArcError::InvalidInput(
                "Microsoft delta token missing query parameters".into(),
            ))
            .into());
        };

        let mut params = Vec::new();
        for (key, value) in form_urlencoded::parse(query.as_bytes()) {
            let mapped_key = match key.as_ref() {
                "$deltatoken" => Some("$deltatoken"),
                "$skiptoken" => Some("$skiptoken"),
                "startDateTime" => Some("startDateTime"),
                "endDateTime" => Some("endDateTime"),
                "$top" => Some("$top"),
                other => {
                    warn!(
                        microsoft_param = other,
                        "ignoring unsupported Microsoft delta parameter"
                    );
                    None
                }
            };

            if let Some(k) = mapped_key {
                params.push((k, value.into_owned()));
            }
        }

        if params.is_empty() {
            return Err(crate::errors::InfraError(pulsearc_domain::PulseArcError::InvalidInput(
                "Microsoft delta token contained no supported parameters".into(),
            ))
            .into());
        }

        Ok(params)
    }

    /// Parse raw events into CalendarEvent structs
    async fn parse_raw_events(
        &self,
        raw_events: Vec<RawCalendarEvent>,
        user_email: &str,
    ) -> Result<Vec<CalendarEvent>> {
        let mut parsed = Vec::new();

        for raw in raw_events {
            let calendar_event = self.convert_raw_event(raw, user_email)?;
            parsed.push(calendar_event);
        }

        Ok(parsed)
    }

    /// Convert RawCalendarEvent to CalendarEvent with parsing
    fn convert_raw_event(&self, raw: RawCalendarEvent, _user_email: &str) -> Result<CalendarEvent> {
        // Parse timestamps
        let (start_ts, end_ts, is_all_day) = if raw.is_all_day {
            let start_ts = self.parse_all_day_timestamp(&raw.start, "start")?;
            let end_ts = self.parse_all_day_timestamp(&raw.end, "end")?;
            (start_ts, end_ts, true)
        } else {
            let start_ts = self.parse_event_timestamp(&raw.start, "start")?;
            let end_ts = self.parse_event_timestamp(&raw.end, "end")?;
            (start_ts, end_ts, false)
        };

        // Parse event title
        let parsed = if let Some(ref subject) = raw.subject {
            if !subject.is_empty() {
                parse_event_title(subject)
            } else {
                ParsedEventTitle {
                    project: Some("General".to_string()),
                    workstream: None,
                    task: Some("untitled event".to_string()),
                    confidence: 0.5,
                }
            }
        } else {
            ParsedEventTitle {
                project: Some("General".to_string()),
                workstream: None,
                task: Some("untitled event".to_string()),
                confidence: 0.5,
            }
        };

        // Detect meeting platform
        let meeting_platform = detect_meeting_platform(
            raw.subject.as_deref(),
            raw.body_preview.as_deref(),
            raw.hangout_link.as_deref(),
        );

        // Calculate derived fields before moving values
        let is_recurring_series = raw.series_master_id.is_some();
        let is_online_meeting = raw.hangout_link.is_some() || meeting_platform.is_some();

        Ok(CalendarEvent {
            id: raw.id,
            summary: raw.subject,
            description: raw.body_preview,
            start: start_ts,
            end: end_ts,
            calendar_id: raw.calendar_id.unwrap_or_else(|| "primary".to_string()),
            is_all_day,
            recurring_event_id: raw.series_master_id,
            original_start_time: None,
            parsed_project: parsed.project,
            parsed_workstream: parsed.workstream,
            parsed_task: parsed.task,
            parsed_confidence: parsed.confidence,
            meeting_platform,
            is_recurring_series,
            is_online_meeting,
            has_external_attendees: raw.has_external_attendees,
            organizer_email: raw.organizer_email,
            organizer_domain: raw.organizer_domain,
            meeting_id: raw.meeting_id,
            attendee_count: raw.attendee_count,
            external_attendee_count: raw.external_attendee_count,
        })
    }

    fn parse_all_day_timestamp(&self, value: &str, field: &str) -> Result<i64> {
        let date = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|e| {
            PulseArcError::InvalidInput(format!("Invalid all-day {field} date '{}': {e}", value))
        })?;

        let midnight = date.and_hms_opt(0, 0, 0).ok_or_else(|| {
            PulseArcError::InvalidInput(format!(
                "Invalid all-day {field} date '{}': could not derive midnight",
                value
            ))
        })?;

        Ok(midnight.and_utc().timestamp())
    }

    fn parse_event_timestamp(&self, value: &str, field: &str) -> Result<i64> {
        let trimmed = value.trim();
        let has_explicit_timezone = trimmed.ends_with('Z')
            || trimmed
                .rfind('T')
                .is_some_and(|idx| trimmed[idx + 1..].chars().any(|c| matches!(c, '+' | '-')));

        let candidate =
            if has_explicit_timezone { trimmed.to_string() } else { format!("{trimmed}Z") };

        chrono::DateTime::parse_from_rfc3339(&candidate)
            .map(|dt| dt.with_timezone(&Utc).timestamp())
            .map_err(|e| {
                PulseArcError::InvalidInput(format!("Invalid {field} timestamp '{}': {e}", value))
            })
    }

    fn pagination_param(token: &str) -> Option<QueryParam> {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return None;
        }

        const SKIPTOKEN: &str = "$skiptoken=";
        const ENCODED_SKIPTOKEN: &str = "%24skiptoken=";
        const PAGETOKEN: &str = "pageToken=";

        fn extract(after_marker: &str, marker: &str) -> Option<String> {
            let start = after_marker.find(marker)? + marker.len();
            let remainder = &after_marker[start..];
            let value = remainder.split('&').next().unwrap_or_default();
            if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        }

        if let Some(value) =
            extract(trimmed, SKIPTOKEN).or_else(|| extract(trimmed, ENCODED_SKIPTOKEN))
        {
            return Some(("$skiptoken", value));
        }

        if let Some(value) = extract(trimmed, PAGETOKEN) {
            return Some(("pageToken", value));
        }

        Some(("pageToken", trimmed.to_string()))
    }

    /// Save calendar events to database
    async fn save_calendar_events(
        &self,
        events: &[CalendarEvent],
        user_email: &str,
    ) -> Result<usize> {
        let mut saved_count = 0;

        for event in events {
            let params = CalendarEventParams {
                id: Uuid::now_v7().to_string(),
                google_event_id: event.id.clone(),
                user_email: user_email.to_string(),
                summary: event.summary.clone().unwrap_or_else(|| "Untitled Event".to_string()),
                description: event.description.clone(),
                when: TimeRange {
                    start_ts: event.start,
                    end_ts: event.end,
                    is_all_day: event.is_all_day,
                },
                recurring_event_id: event.recurring_event_id.clone(),
                parsed: ParsedFields {
                    project: event.parsed_project.clone(),
                    workstream: event.parsed_workstream.clone(),
                    task: event.parsed_task.clone(),
                    confidence_score: Some(f64::from(event.parsed_confidence.clamp(0.0, 1.0))),
                },
                meeting_platform: event.meeting_platform.clone(),
                is_recurring_series: event.is_recurring_series,
                is_online_meeting: event.is_online_meeting,
                has_external_attendees: event.has_external_attendees,
                organizer_email: event.organizer_email.clone(),
                organizer_domain: event.organizer_domain.clone(),
                meeting_id: event.meeting_id.clone(),
                attendee_count: event.attendee_count,
                external_attendee_count: event.external_attendee_count,
            };

            match self.calendar_repo.insert_calendar_event(params).await {
                Ok(_) => saved_count += 1,
                Err(e) => {
                    error!(
                        event_id = %event.id,
                        error = %e,
                        "failed to save calendar event"
                    );
                    // Continue processing other events
                }
            }
        }

        Ok(saved_count)
    }

    /// Generate time entry suggestions from calendar events
    async fn generate_suggestions(
        &self,
        events: &[CalendarEvent],
        _user_email: &str,
        settings: &CalendarSyncSettings,
    ) -> Result<usize> {
        let mut suggestions_count = 0;

        for event in events {
            // Filter by settings
            if !settings.include_all_day_events && event.is_all_day {
                continue;
            }

            let duration_minutes = ((event.end - event.start) / 60).max(1) as u32;
            if duration_minutes < settings.min_event_duration_minutes {
                continue;
            }

            // Generate suggestion (stub for now)
            // TODO: Port suggestion generation logic from
            // legacy/api/src/integrations/calendar/suggestions.rs
            debug!(
                event_id = %event.id,
                duration_minutes,
                "generated suggestion from calendar event"
            );

            suggestions_count += 1;
        }

        Ok(suggestions_count)
    }

    /// Update sync token in database
    async fn update_sync_token(&self, user_email: &str, sync_token: &str) -> Result<()> {
        let conn = self.pool.get_sqlcipher_connection().map_err(|e| {
            crate::errors::InfraError(pulsearc_domain::PulseArcError::Database(format!(
                "Failed to get database connection: {}",
                e
            )))
        })?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE calendar_sync_settings
                 SET sync_token = ?1, last_sync_epoch = ?2, updated_at = ?3
                 WHERE user_email = ?4",
            [&sync_token as &dyn rusqlite::ToSql, &now, &now, &user_email].as_ref(),
        )
        .map_err(crate::errors::InfraError::from)?;

        debug!(user_email, "updated sync token");

        Ok(())
    }

    /// Clear sync token (triggered by 410 GONE)
    async fn clear_sync_token(&self, user_email: &str) -> Result<()> {
        let conn = self.pool.get_sqlcipher_connection().map_err(|e| {
            crate::errors::InfraError(pulsearc_domain::PulseArcError::Database(format!(
                "Failed to get database connection: {}",
                e
            )))
        })?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE calendar_sync_settings
                 SET sync_token = NULL, last_sync_epoch = NULL, updated_at = ?1
                 WHERE user_email = ?2",
            [&now as &dyn rusqlite::ToSql, &user_email].as_ref(),
        )
        .map_err(crate::errors::InfraError::from)?;

        debug!(user_email, "cleared sync token");

        Ok(())
    }
}

/// Helper functions for sync request building
///
/// Calculate exponential backoff delay with jitter
pub fn calculate_backoff(attempt: u32) -> u64 {
    let base_delay = 1000u64; // 1 second in milliseconds
    let max_delay = 32000u64; // 32 seconds max

    let delay = base_delay * 2u64.pow(attempt.min(5));
    let capped_delay = delay.min(max_delay);

    // Add ±25% jitter
    use rand::Rng;
    let jitter_range = (capped_delay as f64 * 0.25) as u64;
    let mut rng = rand::thread_rng();
    let jitter = rng.gen_range(0..=jitter_range * 2) as i64 - jitter_range as i64;

    (capped_delay as i64 + jitter).max(0) as u64
}
