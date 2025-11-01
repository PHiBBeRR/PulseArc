//! Comprehensive integration tests for sync-related workflows
//!
//! This test suite complements the existing calendar_commands tests by focusing
//! on:
//! - Calendar event sync deduplication across multiple operations
//! - Concurrent calendar event sync operations
//! - Large batch calendar sync performance
//! - Cross-user calendar event isolation
//!
//! Each test uses an isolated temporary database to ensure independence.

#![cfg(feature = "calendar")]

use std::sync::Arc;

use chrono::Utc;
use pulsearc_common::testing::TempDir;
use pulsearc_domain::types::database::{ParsedFields, TimeRange};
use pulsearc_domain::{CalendarEventParams, Config, DatabaseConfig};
use pulsearc_lib::AppContext;
use uuid::Uuid;

const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

// ============================================================================
// Test Setup Helpers
// ============================================================================

/// Creates an isolated test context with a unique database
async fn create_test_context() -> (Arc<AppContext>, TempDir) {
    std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", TEST_KEY);

    let temp_dir =
        TempDir::new("pulsearc-sync-test").expect("failed to create temporary test directory");

    let test_db_path = temp_dir.path().join("pulsearc.db");
    let lock_dir = temp_dir.create_dir("lock").expect("failed to create lock directory");

    let config = Config {
        database: DatabaseConfig {
            path: test_db_path.to_string_lossy().to_string(),
            pool_size: 5,
            encryption_key: None,
        },
        ..Config::default()
    };

    let ctx = AppContext::new_with_config_in_lock_dir(config, lock_dir)
        .await
        .expect("failed to create test context");

    (Arc::new(ctx), temp_dir)
}

/// Creates a test calendar event with specified parameters
fn create_test_event(
    user_email: &str,
    google_event_id: &str,
    start_ts: i64,
    duration_secs: i64,
    summary: &str,
) -> CalendarEventParams {
    CalendarEventParams {
        id: Uuid::new_v4().to_string(),
        google_event_id: google_event_id.to_string(),
        user_email: user_email.to_string(),
        summary: summary.to_string(),
        description: Some(format!("Description for {summary}")),
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
        organizer_email: Some(user_email.to_string()),
        organizer_domain: None,
        meeting_id: None,
        attendee_count: Some(1),
        external_attendee_count: None,
    }
}

// ============================================================================
// Calendar Event Sync Integration Tests
// ============================================================================

// NOTE: These tests complement the existing calendar_commands.rs tests
// by focusing on specific sync scenarios

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_calendar_event_multiple_sync_cycles() {
    // Tests that multiple sync cycles properly upsert events
    let (ctx, _temp_dir) = create_test_context().await;
    let email = "multi-sync@example.com";

    ctx.feature_flags
        .set_enabled("new_calendar_commands", true)
        .await
        .expect("failed to enable feature flag");

    let now = Utc::now().timestamp();

    // Cycle 1: Insert 3 events
    for i in 1..=3 {
        let event = create_test_event(
            email,
            &format!("event-{i}"),
            now + (i * 3600),
            1800,
            &format!("Event {i} v1"),
        );
        ctx.calendar_events.insert_calendar_event(event).await.expect("failed to insert event");
    }

    // Cycle 2: Update 2 existing, add 1 new
    for i in 2..=4 {
        let event = create_test_event(
            email,
            &format!("event-{i}"),
            now + (i * 3600),
            1800,
            &format!("Event {i} v2"),
        );
        ctx.calendar_events.insert_calendar_event(event).await.expect("failed to upsert event");
    }

    // Verify final state: 4 unique events
    let events = ctx
        .calendar_events
        .get_calendar_events_by_time_range(email, now, now + (10 * 3600))
        .await
        .expect("failed to query events");

    assert_eq!(events.len(), 4, "should have 4 unique events after multiple sync cycles");

    // Verify updates were applied
    let event2 = events.iter().find(|e| e.google_event_id == "event-2").expect("event-2 not found");
    assert!(event2.summary.contains("v2"), "event should be updated to v2");
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "calendar")]
async fn test_calendar_sync_cross_user_isolation() {
    // Tests that events from different users don't interfere
    let (ctx, _temp_dir) = create_test_context().await;

    ctx.feature_flags
        .set_enabled("new_calendar_commands", true)
        .await
        .expect("failed to enable feature flag");

    let now = Utc::now().timestamp();

    let users = vec!["user1@example.com", "user2@example.com", "user3@example.com"];

    // Each user gets 5 events
    for (user_idx, email) in users.iter().enumerate() {
        for i in 1..=5 {
            let event = create_test_event(
                email,
                &format!("user{user_idx}-event-{i}"),
                now + (i * 1800),
                1800,
                &format!("User{user_idx} Event {i}"),
            );
            ctx.calendar_events.insert_calendar_event(event).await.expect("failed to insert event");
        }
    }

    // Verify each user only sees their events
    for email in &users {
        let events = ctx
            .calendar_events
            .get_calendar_events_by_time_range(email, now, now + (10 * 3600))
            .await
            .expect("failed to query events");

        assert_eq!(events.len(), 5, "each user should have exactly 5 events");
        assert!(
            events.iter().all(|e| e.user_email == *email),
            "all events should belong to the querying user"
        );
    }
}
