//! Regression test for Issue #1: Outbox Retry Filter Bug
//!
//! **Bug**: Legacy code used `status = 'sent'` instead of `status = 'pending'`
//! **Impact**: Pending entries never get retried (data loss)
//! **Reference**: docs/issues/PHASE-3-PRE-MIGRATION-FIXES.md

#![allow(dead_code)]

#[path = "support.rs"]
mod support;

use std::collections::HashSet;

use chrono::{Duration, Utc};
use pulsearc_domain::OutboxStatus;
use pulsearc_infra::database::SqliteOutboxRepository;

#[tokio::test]
#[ignore] // Remove this once OutboxRepository is implemented in Phase 3A.1
async fn test_outbox_retry_filter_uses_pending_status() {
    let db = support::setup_outbox_db();
    let repo = SqliteOutboxRepository::new(db.manager.clone());

    let now = Utc::now().timestamp();

    let mut entry_a = support::make_outbox_entry("entry-a", OutboxStatus::Pending, now - 120);
    entry_a.retry_after = None;
    repo.insert_entry(&entry_a).await.expect("entry A should insert");

    let mut entry_b = support::make_outbox_entry("entry-b", OutboxStatus::Pending, now - 110);
    entry_b.retry_after = Some(now - Duration::hours(1).num_seconds());
    repo.insert_entry(&entry_b).await.expect("entry B should insert");

    let entry_c = support::make_outbox_entry("entry-c", OutboxStatus::Sent, now - 100);
    repo.insert_entry(&entry_c).await.expect("entry C should insert");

    let entry_d = support::make_outbox_entry("entry-d", OutboxStatus::Failed, now - 90);
    repo.insert_entry(&entry_d).await.expect("entry D should insert");

    let results =
        repo.get_pending_entries_ready_for_retry(now).await.expect("query should succeed");

    let ids: HashSet<_> = results.into_iter().map(|entry| entry.id).collect();
    assert!(
        ids.contains("entry-a") && ids.contains("entry-b"),
        "pending entries should be scheduled for retry"
    );
    assert!(
        !ids.contains("entry-c") && !ids.contains("entry-d"),
        "non-pending entries must not be retried"
    );
}

#[tokio::test]
#[ignore] // Remove this once OutboxRepository is implemented
async fn test_outbox_retry_respects_retry_after_timestamp() {
    let db = support::setup_outbox_db();
    let repo = SqliteOutboxRepository::new(db.manager.clone());

    let now = Utc::now().timestamp();

    let mut entry_future =
        support::make_outbox_entry("entry-future", OutboxStatus::Pending, now - 80);
    entry_future.retry_after = Some(now + Duration::hours(1).num_seconds());
    repo.insert_entry(&entry_future).await.expect("future retry entry should insert");

    let mut entry_past = support::make_outbox_entry("entry-past", OutboxStatus::Pending, now - 70);
    entry_past.retry_after = Some(now - Duration::minutes(30).num_seconds());
    repo.insert_entry(&entry_past).await.expect("past retry entry should insert");

    let mut entry_immediate =
        support::make_outbox_entry("entry-immediate", OutboxStatus::Pending, now - 60);
    entry_immediate.retry_after = None;
    repo.insert_entry(&entry_immediate).await.expect("immediate entry should insert");

    let results =
        repo.get_pending_entries_ready_for_retry(now).await.expect("query should succeed");

    let ids: HashSet<_> = results.into_iter().map(|entry| entry.id).collect();
    assert!(
        ids.contains("entry-past") && ids.contains("entry-immediate"),
        "entries with due retry_after should be returned"
    );
    assert!(!ids.contains("entry-future"), "entry with future retry_after must not be returned");
}
