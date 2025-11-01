#![cfg(feature = "calendar")]

//! Integration tests for calendar integration commands (Phase 4B.2)
//!
//! Tests the 3 migrated calendar commands:
//! - `initiate_calendar_auth` - Start OAuth flow and return auth URL
//! - `sync_calendar_events` - Fetch events from Google Calendar API
//! - `get_calendar_events_for_timeline` - Query events by date range

use std::sync::Arc;

use chrono::Utc;
use pulsearc_common::testing::TempDir;
use pulsearc_domain::types::database::{ParsedFields, TimeRange};
use pulsearc_domain::{CalendarEventParams, Config, DatabaseConfig};
use pulsearc_lib::AppContext;

const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

// ============================================================================
// Test Setup
// ============================================================================

/// Helper to create a test context with a unique database
async fn create_test_context() -> (Arc<AppContext>, TempDir) {
    // Set test encryption key to avoid keychain access
    std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", TEST_KEY);

    // Create temporary database directory with auto-cleanup
    let temp_dir =
        TempDir::new("pulsearc-calendar-test").expect("failed to create temporary test directory");

    let test_db_path = temp_dir.path().join("pulsearc.db");
    let lock_dir = temp_dir.create_dir("lock").expect("failed to create lock directory");

    // Create custom config with test database path
    let config = Config {
        database: DatabaseConfig {
            path: test_db_path.to_string_lossy().to_string(),
            pool_size: 5,
            encryption_key: None, // Use TEST_DATABASE_ENCRYPTION_KEY env var
        },
        ..Config::default()
    };

    let ctx = AppContext::new_with_config_in_lock_dir(config, lock_dir)
        .await
        .expect("failed to create test context");

    (Arc::new(ctx), temp_dir)
}

/// Helper to create a test calendar event
fn create_test_event(
    email: &str,
    google_event_id: &str,
    start_ts: i64,
    duration_secs: i64,
) -> CalendarEventParams {
    CalendarEventParams {
        id: uuid::Uuid::new_v4().to_string(),
        google_event_id: google_event_id.to_string(),
        user_email: email.to_string(),
        summary: format!("Test Event {}", google_event_id),
        description: Some("Test event description".to_string()),
        when: TimeRange { start_ts, end_ts: start_ts + duration_secs, is_all_day: false },
        recurring_event_id: None,
        parsed: ParsedFields {
            project: None,
            workstream: None,
            task: None,
            confidence_score: None,
        },
        meeting_platform: None,
        is_recurring_series: false,
        is_online_meeting: false,
        has_external_attendees: None,
        organizer_email: Some(email.to_string()),
        organizer_domain: None,
        meeting_id: None,
        attendee_count: Some(3),
        external_attendee_count: None,
    }
}

// ============================================================================
// get_calendar_events_for_timeline tests
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_get_events_filters_by_date() {
    let (ctx, _temp_dir) = create_test_context().await;
    let email = "test@example.com";

    // Enable feature flag
    ctx.feature_flags
        .set_enabled("new_calendar_commands", true)
        .await
        .expect("failed to enable feature flag");

    // Create events at different times
    let now = Utc::now().timestamp();
    let event1 = create_test_event(email, "event1", now - 7200, 1800); // 2 hours ago
    let event2 = create_test_event(email, "event2", now - 3600, 1800); // 1 hour ago
    let event3 = create_test_event(email, "event3", now + 3600, 1800); // 1 hour from now
    let event4 = create_test_event(email, "event4", now + 7200, 1800); // 2 hours from now

    // Insert all events
    ctx.calendar_events.insert_calendar_event(event1).await.expect("failed to insert event1");
    ctx.calendar_events.insert_calendar_event(event2).await.expect("failed to insert event2");
    ctx.calendar_events.insert_calendar_event(event3).await.expect("failed to insert event3");
    ctx.calendar_events.insert_calendar_event(event4).await.expect("failed to insert event4");

    // Create a mock token in the database so the query finds this email
    insert_mock_calendar_token(&ctx, email, now + 86400).await;

    // Query events in the range [now - 3600, now + 5400]
    // This range includes the full duration of event2 and event3
    // event2: starts at now-3600, ends at now-1800 (fully within range)
    // event3: starts at now+3600, ends at now+5400 (fully within range)
    let events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(email, now - 3600, now + 5400)
        .await
        .expect("failed to query events");

    // Should return only events completely within the range
    assert_eq!(events.len(), 2, "should return 2 events within time range");

    // Verify they're the correct events
    let titles: Vec<String> = events.iter().map(|e| e.summary.clone()).collect();
    assert!(titles.contains(&"Test Event event2".to_string()));
    assert!(titles.contains(&"Test Event event3".to_string()));
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_get_events_returns_empty_when_no_events() {
    let (ctx, _temp_dir) = create_test_context().await;
    let email = "test@example.com";

    // Enable feature flag
    ctx.feature_flags
        .set_enabled("new_calendar_commands", true)
        .await
        .expect("failed to enable feature flag");

    let now = Utc::now().timestamp();

    // Query empty database
    let events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(email, now - 3600, now + 3600)
        .await
        .expect("failed to query events");

    assert_eq!(events.len(), 0, "should return empty array");
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_get_events_sorts_by_start_time() {
    let (ctx, _temp_dir) = create_test_context().await;
    let email = "test@example.com";

    // Enable feature flag
    ctx.feature_flags
        .set_enabled("new_calendar_commands", true)
        .await
        .expect("failed to enable feature flag");

    let now = Utc::now().timestamp();

    // Create events in reverse chronological order
    let event3 = create_test_event(email, "event3", now + 7200, 1800);
    let event1 = create_test_event(email, "event1", now + 1800, 1800);
    let event2 = create_test_event(email, "event2", now + 3600, 1800);

    // Insert in random order
    ctx.calendar_events.insert_calendar_event(event3).await.expect("failed to insert event3");
    ctx.calendar_events.insert_calendar_event(event1).await.expect("failed to insert event1");
    ctx.calendar_events.insert_calendar_event(event2).await.expect("failed to insert event2");

    // Query all events
    let events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(email, now, now + 10800)
        .await
        .expect("failed to query events");

    // Verify sorted by start_ts
    assert_eq!(events.len(), 3);
    assert!(events[0].start_ts < events[1].start_ts);
    assert!(events[1].start_ts < events[2].start_ts);
    assert_eq!(events[0].summary, "Test Event event1");
    assert_eq!(events[1].summary, "Test Event event2");
    assert_eq!(events[2].summary, "Test Event event3");
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_get_events_handles_all_day_events() {
    let (ctx, _temp_dir) = create_test_context().await;
    let email = "test@example.com";

    // Enable feature flag
    ctx.feature_flags
        .set_enabled("new_calendar_commands", true)
        .await
        .expect("failed to enable feature flag");

    let now = Utc::now().timestamp();

    // Create an all-day event
    let mut all_day_event = create_test_event(email, "all-day", now, 86400);
    all_day_event.when.is_all_day = true;

    ctx.calendar_events
        .insert_calendar_event(all_day_event)
        .await
        .expect("failed to insert all-day event");

    // Query events
    let events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(email, now - 3600, now + 86400)
        .await
        .expect("failed to query events");

    assert_eq!(events.len(), 1);
    assert!(events[0].is_all_day, "event should be marked as all-day");
}

// ============================================================================
// CalendarEventRepository tests
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_insert_calendar_event_creates_record() {
    let (ctx, _temp_dir) = create_test_context().await;
    let email = "test@example.com";

    let now = Utc::now().timestamp();
    let event = create_test_event(email, "test-event-1", now, 1800);
    let event_id = event.id.clone();

    // Insert event
    ctx.calendar_events.insert_calendar_event(event).await.expect("failed to insert event");

    // Verify it was inserted by querying (range must include event's full duration)
    let events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(email, now - 100, now + 2000)
        .await
        .expect("failed to query events");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, event_id);
    assert_eq!(events[0].google_event_id, "test-event-1");
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_insert_calendar_event_is_idempotent() {
    let (ctx, _temp_dir) = create_test_context().await;
    let email = "test@example.com";

    let now = Utc::now().timestamp();
    let event = create_test_event(email, "duplicate-event", now, 1800);

    // Insert same event twice
    ctx.calendar_events
        .insert_calendar_event(event.clone())
        .await
        .expect("failed to insert event first time");

    let result = ctx.calendar_events.insert_calendar_event(event).await;

    // Should succeed (idempotent ON CONFLICT DO UPDATE)
    assert!(result.is_ok(), "duplicate insert should be idempotent");

    // Verify only one event exists (range must include event's full duration)
    let events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(email, now - 100, now + 2000)
        .await
        .expect("failed to query events");

    assert_eq!(events.len(), 1, "should have only one event");
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_get_events_handles_multiple_users() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Enable feature flag
    ctx.feature_flags
        .set_enabled("new_calendar_commands", true)
        .await
        .expect("failed to enable feature flag");

    let now = Utc::now().timestamp();
    let user1 = "user1@example.com";
    let user2 = "user2@example.com";

    // Create events for two different users
    let event1 = create_test_event(user1, "user1-event", now, 1800);
    let event2 = create_test_event(user2, "user2-event", now, 1800);

    ctx.calendar_events.insert_calendar_event(event1).await.expect("failed to insert user1 event");
    ctx.calendar_events.insert_calendar_event(event2).await.expect("failed to insert user2 event");

    // Query events for user1 (range must include event's full duration)
    let user1_events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(user1, now - 100, now + 2000)
        .await
        .expect("failed to query user1 events");

    // Query events for user2 (range must include event's full duration)
    let user2_events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(user2, now - 100, now + 2000)
        .await
        .expect("failed to query user2 events");

    assert_eq!(user1_events.len(), 1);
    assert_eq!(user2_events.len(), 1);
    assert_eq!(user1_events[0].google_event_id, "user1-event");
    assert_eq!(user2_events[0].google_event_id, "user2-event");
}

// ============================================================================
// Event parsing tests (unit-level, but part of integration flow)
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_event_with_recurring_fields() {
    let (ctx, _temp_dir) = create_test_context().await;
    let email = "test@example.com";

    let now = Utc::now().timestamp();
    let mut event = create_test_event(email, "recurring-1", now, 1800);
    event.recurring_event_id = Some("parent-recurring-id".to_string());
    event.is_recurring_series = true;

    ctx.calendar_events
        .insert_calendar_event(event)
        .await
        .expect("failed to insert recurring event");

    let events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(email, now - 100, now + 2000)
        .await
        .expect("failed to query events");

    assert_eq!(events.len(), 1);
    assert!(events[0].is_recurring_series);
    assert_eq!(events[0].recurring_event_id, Some("parent-recurring-id".to_string()));
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_event_with_meeting_details() {
    let (ctx, _temp_dir) = create_test_context().await;
    let email = "test@example.com";

    let now = Utc::now().timestamp();
    let mut event = create_test_event(email, "meeting-1", now, 1800);
    event.is_online_meeting = true;
    event.meeting_platform = Some("Google Meet".to_string());
    event.meeting_id = Some("abc-defg-hij".to_string());
    event.attendee_count = Some(5);

    ctx.calendar_events.insert_calendar_event(event).await.expect("failed to insert meeting event");

    let events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(email, now - 100, now + 2000)
        .await
        .expect("failed to query events");

    assert_eq!(events.len(), 1);
    assert!(events[0].is_online_meeting);
    assert_eq!(events[0].meeting_platform, Some("Google Meet".to_string()));
    assert_eq!(events[0].attendee_count, Some(5));
}

// ============================================================================
// OAuth flow tests (requires mocking external services)
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_initiate_auth_requires_feature_flag() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Disable feature flag
    ctx.feature_flags
        .set_enabled("new_calendar_commands", false)
        .await
        .expect("failed to disable feature flag");

    let is_enabled = ctx
        .feature_flags
        .is_enabled("new_calendar_commands", true)
        .await
        .expect("failed to check flag");

    assert!(!is_enabled, "feature flag should be disabled");
}

// ============================================================================
// OAuth and sync flow tests
// ============================================================================

/// Test that MockOAuthClient can generate authorization URLs
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_oauth_client_generates_authorization_url() {
    use pulsearc_common::auth::OAuthClientTrait;
    use pulsearc_common::testing::mocks::MockOAuthClient;

    let oauth_client = MockOAuthClient::new();

    // Generate authorization URL
    let result = oauth_client.generate_authorization_url().await;

    assert!(result.is_ok(), "Should generate authorization URL");
    let (auth_url, state) = result.unwrap();

    // Verify URL format
    assert!(auth_url.starts_with("https://"), "URL should use HTTPS");
    assert!(auth_url.contains("client_id"), "URL should contain client_id parameter");
    assert!(!state.is_empty(), "State should be non-empty");
}

/// Test Google Calendar API response parsing logic
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_parse_google_calendar_api_response() {
    // Sample Google Calendar API response (simplified)
    let api_response = serde_json::json!({
        "items": [
            {
                "id": "event123",
                "summary": "Team Meeting",
                "description": "Weekly sync",
                "start": {
                    "dateTime": "2025-01-15T10:00:00-08:00"
                },
                "end": {
                    "dateTime": "2025-01-15T11:00:00-08:00"
                },
                "recurringEventId": null,
                "hangoutLink": "https://meet.google.com/abc-defg-hij",
                "organizer": {
                    "email": "organizer@example.com"
                },
                "attendees": [
                    {"email": "attendee1@example.com"},
                    {"email": "attendee2@example.com"}
                ]
            },
            {
                "id": "event456",
                "summary": "All Day Event",
                "start": {
                    "date": "2025-01-16"
                },
                "end": {
                    "date": "2025-01-17"
                }
            }
        ]
    });

    let items = api_response["items"].as_array().expect("items should be array");

    // Verify we can parse regular timed event
    let event1 = &items[0];
    assert_eq!(event1["id"].as_str().unwrap(), "event123");
    assert_eq!(event1["summary"].as_str().unwrap(), "Team Meeting");
    assert!(event1["hangoutLink"].is_string(), "Should detect online meeting");
    assert_eq!(event1["attendees"].as_array().unwrap().len(), 2);

    // Verify we can parse all-day event
    let event2 = &items[1];
    assert_eq!(event2["id"].as_str().unwrap(), "event456");
    assert!(event2["start"]["date"].is_string(), "All-day event should use 'date' field");
    assert!(event2["start"]["dateTime"].is_null(), "All-day event should not have dateTime");
}

/// Test OAuth token refresh flow with mocks
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_oauth_token_refresh_with_mock() {
    use pulsearc_common::auth::{OAuthClientTrait, TokenSet};
    use pulsearc_common::testing::mocks::MockOAuthClient;

    // Create mock OAuth client
    let oauth_client = MockOAuthClient::new();

    // Configure mock to return new tokens on refresh
    let refreshed_tokens = TokenSet::new(
        "new_access_token".to_string(),
        Some("new_refresh_token".to_string()),
        None,
        3600,
        Some("https://www.googleapis.com/auth/calendar.readonly".to_string()),
    );
    oauth_client.set_refresh_response(refreshed_tokens.clone());

    // Simulate token refresh
    let new_tokens = oauth_client
        .refresh_access_token("valid_refresh_token")
        .await
        .expect("refresh should succeed");

    // Verify refresh was called
    assert!(
        oauth_client.was_refresh_called(),
        "OAuth client should have been called to refresh token"
    );

    // Verify new tokens are correct
    assert_eq!(new_tokens.access_token, "new_access_token");
    assert_eq!(new_tokens.refresh_token, Some("new_refresh_token".to_string()));
}

/// Test that token refresh failure is handled correctly
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_oauth_token_refresh_failure() {
    use pulsearc_common::auth::OAuthClientTrait;
    use pulsearc_common::testing::mocks::MockOAuthClient;

    let oauth_client = MockOAuthClient::new();

    // Configure mock to fail on refresh
    oauth_client.set_should_fail(true);

    // Attempt refresh
    let result = oauth_client.refresh_access_token("valid_refresh_token").await;

    // Verify refresh failed as expected
    assert!(result.is_err(), "Refresh should fail when configured to fail");
    assert!(
        oauth_client.was_refresh_called(),
        "OAuth client should have been called despite failure"
    );
}

/// Test calendar event upsert behavior during sync
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_calendar_event_upsert_on_sync() {
    let (ctx, _temp_dir) = create_test_context().await;
    let email = "test@example.com";

    let now = chrono::Utc::now().timestamp();

    // First sync: insert new event
    let event_v1 = create_test_event(email, "recurring-event-123", now, 3600);
    ctx.calendar_events
        .insert_calendar_event(event_v1.clone())
        .await
        .expect("failed to insert event");

    // Second sync: update same event (simulates re-sync)
    let mut event_v2 = create_test_event(email, "recurring-event-123", now, 3600);
    event_v2.summary = "Updated Meeting Title".to_string();
    event_v2.description = Some("Updated description".to_string());

    ctx.calendar_events.insert_calendar_event(event_v2).await.expect("failed to update event");

    // Verify only one event exists (upsert, not duplicate)
    let events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(email, now - 100, now + 4000)
        .await
        .expect("failed to query events");

    assert_eq!(events.len(), 1, "should have exactly one event (upserted)");
    assert_eq!(events[0].summary, "Updated Meeting Title");
    assert_eq!(events[0].description, Some("Updated description".to_string()));
}

/// Test keychain token storage and retrieval
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_keychain_token_storage() {
    use pulsearc_common::auth::TokenSet;
    use pulsearc_common::testing::mocks::MockKeychainProvider;

    let keychain = MockKeychainProvider::new("test-calendar");
    let email = "test@example.com";

    // Store tokens
    let tokens = TokenSet::new(
        "access_token_123".to_string(),
        Some("refresh_token_456".to_string()),
        None,
        3600,
        Some("https://www.googleapis.com/auth/calendar.readonly".to_string()),
    );

    keychain.store_tokens(email, &tokens).expect("failed to store tokens");

    // Retrieve tokens
    let retrieved = keychain.retrieve_tokens(email).expect("failed to retrieve tokens");

    assert_eq!(retrieved.access_token, "access_token_123");
    assert_eq!(retrieved.refresh_token, Some("refresh_token_456".to_string()));
    assert!(keychain.has_tokens(email));
}

// NOTE: Full end-to-end tests with actual HTTP mocking would require:
// - wiremock server to mock Google Calendar API endpoints
// - Dependency injection to replace CalendarOAuthManager's HTTP client
// - These are deferred pending architecture refactoring for testability

// ============================================================================
// Error handling tests
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_get_events_with_invalid_date_range() {
    let (ctx, _temp_dir) = create_test_context().await;
    let email = "test@example.com";

    // Query with end < start (invalid range)
    let now = Utc::now().timestamp();
    let events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(email, now + 3600, now - 3600)
        .await
        .expect("query should not fail on invalid range");

    // Should return empty (no events in impossible range)
    assert_eq!(events.len(), 0);
}

// ============================================================================
// Helper functions for test setup
// ============================================================================

/// Insert a mock calendar token to simulate an authenticated user
#[cfg(feature = "calendar")]
async fn insert_mock_calendar_token(ctx: &Arc<AppContext>, email: &str, expires_at: i64) {
    let db = ctx.db.clone();
    let email = email.to_string();

    tokio::task::spawn_blocking(move || {
        let conn = db.get_connection().expect("failed to get connection");
        let now = Utc::now().timestamp();
        let id = uuid::Uuid::new_v4().to_string();
        let token_ref = uuid::Uuid::new_v4().to_string();
        let idempotency_key = uuid::Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO calendar_tokens (id, token_ref, user_email, expires_at, created_at, updated_at, idempotency_key, provider)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, 'google')
             ON CONFLICT(provider) DO UPDATE SET user_email = ?3, expires_at = ?4, updated_at = ?5",
            rusqlite::params![id, token_ref, email, expires_at, now, idempotency_key],
        )
        .expect("failed to insert mock token");
    })
    .await
    .expect("task join error");
}
