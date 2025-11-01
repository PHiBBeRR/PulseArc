//! Calendar integration commands
//!
//! Phase 4B.2: Migrated calendar integration commands using new OAuth
//! infrastructure

use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "calendar")]
use std::time::Duration;

#[cfg(feature = "calendar")]
use chrono::{DateTime, Utc};

#[cfg(feature = "calendar")]
use pulsearc_domain::{CalendarEventParams, CalendarEventRow, PulseArcError};

use pulsearc_domain::Result;
use serde::{Deserialize, Serialize};
use tauri::State;

#[cfg(feature = "calendar")]
use tauri::Emitter;

#[cfg(feature = "calendar")]
use tracing::{error, info, warn};

#[cfg(not(feature = "calendar"))]
use tracing::info;

use crate::utils::logging::{log_command_execution, record_command_metric, MetricRecord};
use crate::AppContext;

/// Calendar event for timeline display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineCalendarEvent {
    pub id: String,
    pub title: String,
    pub start_time: i64,
    pub end_time: i64,
    pub is_all_day: bool,
}

/// Initiate Google Calendar OAuth flow
///
/// Phase 4B.2: New implementation using CalendarOAuthManager
#[tauri::command]
pub async fn initiate_calendar_auth(
    ctx: State<'_, Arc<AppContext>>,
    app: tauri::AppHandle,
    email: String,
) -> std::result::Result<String, String> {
    let command_name = "calendar::initiate_calendar_auth";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, email, "Initiating calendar OAuth flow");

    // Check feature flag
    let use_new =
        ctx.feature_flags.is_enabled("new_calendar_commands", true).await.unwrap_or(false);

    let result = if use_new {
        new_initiate_calendar_auth(Arc::clone(ctx.inner()), &app, email).await
    } else {
        // Legacy implementation (call legacy command)
        Err("Legacy calendar commands not available in new crate".to_string())
    };

    let elapsed = start.elapsed();
    let success = result.is_ok();

    log_command_execution(command_name, "new", elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation: "new",
            elapsed,
            success,
            error_type: if !success { Some("oauth_initiation_failed") } else { None },
        },
    )
    .await;

    result
}

#[cfg(feature = "calendar")]
async fn new_initiate_calendar_auth(
    ctx: Arc<AppContext>,
    app: &tauri::AppHandle,
    email: String,
) -> std::result::Result<String, String> {
    // 1. Start OAuth login session (handles PKCE, state, loopback server)
    let session = ctx
        .calendar_oauth
        .start_login(&email)
        .await
        .map_err(|e| format!("Failed to start OAuth: {}", e))?;

    // 2. Get authorization URL for frontend
    let auth_url = session.authorization_url().to_string();

    // 3. Spawn background task to wait for OAuth callback
    let app_clone = app.clone();
    let ctx_clone = Arc::clone(&ctx);
    let email_clone = email.clone();

    tauri::async_runtime::spawn(async move {
        match session.finish(Duration::from_secs(300)).await {
            Ok(_tokens) => {
                info!(email = %email_clone, "Calendar OAuth succeeded");

                // Update database: ensure calendar_sync_settings row exists
                if let Err(e) = ensure_calendar_settings(&ctx_clone, &email_clone).await {
                    error!(error = %e, "Failed to create calendar settings");
                }

                // Emit success event to frontend
                let _ = app_clone.emit("calendar:connected", &email_clone);
            }
            Err(e) => {
                error!(error = %e, email = %email_clone, "Calendar OAuth failed");
                let _ = app_clone.emit("calendar:error", format!("{}", e));
            }
        }
    });

    // 4. Return auth URL immediately (frontend opens in browser)
    Ok(auth_url)
}

#[cfg(not(feature = "calendar"))]
async fn new_initiate_calendar_auth(
    _ctx: Arc<AppContext>,
    _app: &tauri::AppHandle,
    _email: String,
) -> std::result::Result<String, String> {
    Err("Calendar feature not enabled".to_string())
}

/// Ensure calendar_sync_settings row exists for user
#[cfg(feature = "calendar")]
async fn ensure_calendar_settings(ctx: &Arc<AppContext>, email: &str) -> Result<()> {
    let db = ctx.db.clone();
    let email = email.to_string();

    tokio::task::spawn_blocking(move || -> Result<()> {
        let conn = db.get_connection()?;
        let now = Utc::now().timestamp();
        let id = uuid::Uuid::new_v4().to_string();
        let idempotency_key = uuid::Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO calendar_sync_settings
             (id, user_email, enabled, created_at, updated_at, idempotency_key)
             VALUES (?1, ?2, 1, ?3, ?3, ?4)
             ON CONFLICT(user_email) DO NOTHING",
            rusqlite::params![id, email, now, idempotency_key],
        )
        .map_err(|e| {
            PulseArcError::Database(format!("Failed to insert calendar settings: {}", e))
        })?;

        Ok(())
    })
    .await
    .map_err(|e| PulseArcError::Internal(format!("Task join error: {}", e)))??;

    Ok(())
}

/// Manually trigger calendar sync
///
/// Phase 4B.2: New implementation using CalendarOAuthManager and
/// CalendarEventRepository
#[tauri::command]
pub async fn sync_calendar_events(
    ctx: State<'_, Arc<AppContext>>,
    email: String,
) -> std::result::Result<usize, String> {
    let command_name = "calendar::sync_calendar_events";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, email, "Syncing calendar events");

    // Check feature flag
    let use_new =
        ctx.feature_flags.is_enabled("new_calendar_commands", true).await.unwrap_or(false);

    let result = if use_new {
        new_sync_calendar_events(Arc::clone(ctx.inner()), email).await
    } else {
        Err("Legacy calendar commands not available in new crate".to_string())
    };

    let elapsed = start.elapsed();
    let success = result.is_ok();

    log_command_execution(command_name, "new", elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation: "new",
            elapsed,
            success,
            error_type: if !success { Some("sync_failed") } else { None },
        },
    )
    .await;

    result
}

#[cfg(feature = "calendar")]
async fn new_sync_calendar_events(
    ctx: Arc<AppContext>,
    email: String,
) -> std::result::Result<usize, String> {
    // 1. Ensure user is authenticated (has tokens)
    let is_authed = ctx
        .calendar_oauth
        .is_authenticated(&email)
        .await
        .map_err(|e| format!("Auth check failed: {}", e))?;

    if !is_authed {
        return Err("Calendar not connected. Please authenticate first.".to_string());
    }

    // 2. Get valid access token (auto-refreshes if expired)
    let access_token = ctx
        .calendar_oauth
        .get_access_token(&email)
        .await
        .map_err(|e| format!("Failed to get access token: {}", e))?;

    // 3. Fetch events from Google Calendar API
    let now = Utc::now();
    let start = now - chrono::Duration::days(14); // Lookback 2 weeks
    let end = now + chrono::Duration::days(7); // Lookahead 1 week

    let events = fetch_google_calendar_events(&email, &access_token, start, end)
        .await
        .map_err(|e| format!("Failed to fetch events: {}", e))?;

    // 4. Save to database via repository (loop through events)
    let event_count = events.len();
    for event in events {
        ctx.calendar_events
            .insert_calendar_event(event)
            .await
            .map_err(|e| format!("Failed to save event: {}", e))?;
    }

    // 5. Update last_sync_epoch in calendar_sync_settings
    update_last_sync_timestamp(&ctx, &email)
        .await
        .map_err(|e| format!("Failed to update sync timestamp: {}", e))?;

    info!(email, event_count, "Calendar sync completed");
    Ok(event_count)
}

#[cfg(not(feature = "calendar"))]
async fn new_sync_calendar_events(
    _ctx: Arc<AppContext>,
    _email: String,
) -> std::result::Result<usize, String> {
    Err("Calendar feature not enabled".to_string())
}

/// Fetch events from Google Calendar API
#[cfg(feature = "calendar")]
async fn fetch_google_calendar_events(
    email: &str,
    access_token: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<CalendarEventParams>> {
    let client = reqwest::Client::new();
    let url = "https://www.googleapis.com/calendar/v3/calendars/primary/events";

    let response = client
        .get(url)
        .bearer_auth(access_token)
        .query(&[
            ("timeMin", start.to_rfc3339()),
            ("timeMax", end.to_rfc3339()),
            ("maxResults", "250".to_string()),
            ("singleEvents", "true".to_string()),
        ])
        .send()
        .await
        .map_err(|e| PulseArcError::Network(format!("Google API request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(PulseArcError::Network(format!(
            "Google API returned {}: {}",
            response.status(),
            response.text().await.unwrap_or_default()
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| PulseArcError::InvalidInput(format!("Failed to parse response: {}", e)))?;

    // Parse events from JSON
    parse_google_events(email, &json)
}

/// Parse Google Calendar API response into domain events
#[cfg(feature = "calendar")]
fn parse_google_events(email: &str, json: &serde_json::Value) -> Result<Vec<CalendarEventParams>> {
    let items = json["items"]
        .as_array()
        .ok_or_else(|| PulseArcError::InvalidInput("Missing 'items' array".to_string()))?;

    let events: Vec<CalendarEventParams> = items
        .iter()
        .filter_map(|item| {
            // Parse each event (handle errors gracefully)
            parse_single_event(email, item).ok()
        })
        .collect();

    Ok(events)
}

/// Parse a single Google Calendar event
#[cfg(feature = "calendar")]
fn parse_single_event(email: &str, item: &serde_json::Value) -> Result<CalendarEventParams> {
    use pulsearc_domain::types::database::{ParsedFields, TimeRange};

    let google_event_id = item["id"]
        .as_str()
        .ok_or_else(|| PulseArcError::InvalidInput("Missing event id".to_string()))?
        .to_string();

    let summary = item["summary"].as_str().unwrap_or("(No title)").to_string();

    let description = item["description"].as_str().map(String::from);

    // Parse start/end times
    let start_str = item["start"]["dateTime"]
        .as_str()
        .or_else(|| item["start"]["date"].as_str())
        .ok_or_else(|| PulseArcError::InvalidInput("Missing start time".to_string()))?;

    let end_str = item["end"]["dateTime"]
        .as_str()
        .or_else(|| item["end"]["date"].as_str())
        .ok_or_else(|| PulseArcError::InvalidInput("Missing end time".to_string()))?;

    let is_all_day = item["start"]["date"].is_string();

    let start_dt = DateTime::parse_from_rfc3339(start_str)
        .map_err(|e| PulseArcError::InvalidInput(format!("Invalid start time: {}", e)))?
        .with_timezone(&Utc);

    let end_dt = DateTime::parse_from_rfc3339(end_str)
        .map_err(|e| PulseArcError::InvalidInput(format!("Invalid end time: {}", e)))?
        .with_timezone(&Utc);

    Ok(CalendarEventParams {
        id: uuid::Uuid::new_v4().to_string(),
        google_event_id,
        user_email: email.to_string(),
        summary,
        description,
        when: TimeRange { start_ts: start_dt.timestamp(), end_ts: end_dt.timestamp(), is_all_day },
        recurring_event_id: item["recurringEventId"].as_str().map(String::from),
        parsed: ParsedFields {
            project: None,
            workstream: None,
            task: None,
            confidence_score: None,
        },
        meeting_platform: None,
        is_recurring_series: item["recurringEventId"].is_string(),
        is_online_meeting: item["hangoutLink"].is_string() || item["conferenceData"].is_object(),
        has_external_attendees: None,
        organizer_email: item["organizer"]["email"].as_str().map(String::from),
        organizer_domain: None,
        meeting_id: None,
        attendee_count: item["attendees"].as_array().map(|a| a.len() as i32),
        external_attendee_count: None,
    })
}

/// Update last_sync_epoch in calendar_sync_settings
#[cfg(feature = "calendar")]
async fn update_last_sync_timestamp(ctx: &Arc<AppContext>, email: &str) -> Result<()> {
    let db = ctx.db.clone();
    let email = email.to_string();

    tokio::task::spawn_blocking(move || -> Result<()> {
        let conn = db.get_connection()?;
        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE calendar_sync_settings SET last_sync_epoch = ?1, updated_at = ?1 WHERE user_email = ?2",
            rusqlite::params![now, email],
        )
        .map_err(|e| PulseArcError::Database(format!("Failed to update sync timestamp: {}", e)))?;

        Ok(())
    })
    .await
    .map_err(|e| PulseArcError::Internal(format!("Task join error: {}", e)))??;

    Ok(())
}

/// Get calendar events for timeline within date range
///
/// Phase 4B.2: New implementation using CalendarEventRepository
#[tauri::command]
pub async fn get_calendar_events_for_timeline(
    ctx: State<'_, Arc<AppContext>>,
    start_date: i64,
    end_date: i64,
) -> Result<Vec<TimelineCalendarEvent>> {
    let command_name = "calendar::get_calendar_events_for_timeline";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, start_date, end_date, "Fetching calendar events for timeline");

    // Check feature flag
    let use_new =
        ctx.feature_flags.is_enabled("new_calendar_commands", true).await.unwrap_or(false);

    let result = if use_new {
        new_get_calendar_events_for_timeline(Arc::clone(ctx.inner()), start_date, end_date).await
    } else {
        // Legacy returns empty
        Ok(vec![])
    };

    let elapsed = start.elapsed();
    let success = result.is_ok();

    log_command_execution(command_name, "new", elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation: "new",
            elapsed,
            success,
            error_type: if !success { Some("query_failed") } else { None },
        },
    )
    .await;

    result
}

#[cfg(feature = "calendar")]
async fn new_get_calendar_events_for_timeline(
    ctx: Arc<AppContext>,
    start_date: i64,
    end_date: i64,
) -> Result<Vec<TimelineCalendarEvent>> {
    // 1. Get all connected user emails from calendar_tokens
    let emails = get_connected_user_emails(&ctx).await?;

    if emails.is_empty() {
        warn!("No calendar connected, returning empty array");
        return Ok(vec![]);
    }

    info!("Querying calendar events for {} connected provider(s)", emails.len());

    // 2. Query events for each email and merge
    let mut all_events: Vec<CalendarEventRow> = Vec::new();
    for email in emails {
        let events = ctx
            .calendar_events
            .get_calendar_events_by_time_range(&email, start_date, end_date)
            .await?;
        all_events.extend(events);
    }

    // 3. Sort by start_ts
    all_events.sort_by_key(|e| e.start_ts);

    // 4. Map to timeline format
    Ok(all_events
        .into_iter()
        .map(|e| TimelineCalendarEvent {
            id: e.id,
            title: e.summary,
            start_time: e.start_ts,
            end_time: e.end_ts,
            is_all_day: e.is_all_day,
        })
        .collect())
}

#[cfg(not(feature = "calendar"))]
async fn new_get_calendar_events_for_timeline(
    _ctx: Arc<AppContext>,
    _start_date: i64,
    _end_date: i64,
) -> Result<Vec<TimelineCalendarEvent>> {
    Ok(vec![])
}

/// Get all connected user emails from calendar_tokens
#[cfg(feature = "calendar")]
async fn get_connected_user_emails(ctx: &Arc<AppContext>) -> Result<Vec<String>> {
    let db = ctx.db.clone();
    let now = Utc::now().timestamp();

    tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
        let conn = db.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT DISTINCT user_email FROM calendar_tokens WHERE expires_at > ?1")
            .map_err(|e| PulseArcError::Database(format!("Failed to prepare statement: {}", e)))?;

        let emails =
            stmt.query_map(&[&now as &dyn rusqlite::ToSql], |row| row.get::<_, String>(0))
                .map_err(|e| PulseArcError::Database(format!("Failed to query tokens: {}", e)))?;

        Ok(emails)
    })
    .await
    .map_err(|e| PulseArcError::Internal(format!("Task join error: {}", e)))?
}
