//! Regression test for Issue #3: Outbox Status Parsing Panic
//!
//! **Bug**: Legacy code used `parse().unwrap()` - panics on invalid status
//! **Impact**: One bad row crashes entire outbox pipeline
//! **Reference**: docs/issues/PHASE-3-PRE-MIGRATION-FIXES.md

#![allow(dead_code)]

#[path = "support.rs"]
mod support;

use chrono::Utc;
use pulsearc_domain::OutboxStatus;
use pulsearc_infra::database::SqliteOutboxRepository;
use tracing::Level;

#[tokio::test]
async fn test_outbox_status_parsing_handles_invalid_values() {
    let db = support::setup_outbox_db();
    let repo = SqliteOutboxRepository::new(db.manager.clone());
    let log_handle = support::init_test_logger();

    let now = Utc::now().timestamp();

    for suffix in ['a', 'b', 'c', 'd'] {
        let entry =
            support::make_outbox_entry(&format!("entry-{}", suffix), OutboxStatus::Pending, now);
        repo.insert_entry(&entry).await.expect("seed entry should insert");
    }

    let conn = db.manager.get_connection().expect("connection should be available");
    conn.execute("UPDATE time_entry_outbox SET status = 'unknown' WHERE id = 'entry-a'", [])
        .expect("update should succeed");
    conn.execute("UPDATE time_entry_outbox SET status = 'retrying' WHERE id = 'entry-b'", [])
        .expect("update should succeed");
    conn.execute("UPDATE time_entry_outbox SET status = 'PENDING' WHERE id = 'entry-c'", [])
        .expect("update should succeed");
    conn.execute("UPDATE time_entry_outbox SET status = '' WHERE id = 'entry-d'", [])
        .expect("update should succeed");

    let entries = repo.list_all_entries().await.expect("listing entries should succeed");

    for entry in entries.iter().filter(|entry| entry.id.starts_with("entry-")) {
        assert_eq!(
            entry.status,
            OutboxStatus::Pending,
            "Invalid statuses should map to Pending: {} => {:?}",
            entry.id,
            entry.status
        );
    }

    let warnings = log_handle.entries();
    assert!(
        warnings.iter().filter(|(level, _)| *level == Level::WARN).count() >= 3,
        "Invalid statuses should emit warnings ({warnings:?})"
    );
}

#[tokio::test]
async fn test_outbox_status_parsing_handles_valid_values() {
    let db = support::setup_outbox_db();
    let repo = SqliteOutboxRepository::new(db.manager.clone());
    let log_handle = support::init_test_logger();

    let now = Utc::now().timestamp();

    let statuses = [
        ("entry-pending", OutboxStatus::Pending),
        ("entry-sent", OutboxStatus::Sent),
        ("entry-failed", OutboxStatus::Failed),
        ("entry-dismissed", OutboxStatus::Dismissed),
    ];

    for (id, status) in statuses {
        let entry = support::make_outbox_entry(id, status, now);
        repo.insert_entry(&entry).await.expect("valid entry insert should succeed");
    }

    let entries = repo.list_all_entries().await.expect("listing entries should succeed");

    for entry in entries.iter().filter(|entry| entry.id.starts_with("entry-")) {
        match entry.id.as_str() {
            "entry-pending" => assert_eq!(entry.status, OutboxStatus::Pending),
            "entry-sent" => assert_eq!(entry.status, OutboxStatus::Sent),
            "entry-failed" => assert_eq!(entry.status, OutboxStatus::Failed),
            "entry-dismissed" => assert_eq!(entry.status, OutboxStatus::Dismissed),
            _ => {}
        }
    }

    assert!(
        !log_handle.contains(Level::WARN, "Invalid outbox status"),
        "Valid statuses must not emit warnings"
    );
}

#[test]
fn test_outbox_status_string_parse_error_handling() {
    // Unit test for OutboxStatus::from_str error handling
    //
    // This can be implemented immediately if OutboxStatus implements FromStr

    // Valid cases
    assert_eq!("pending".parse::<OutboxStatus>().ok(), Some(OutboxStatus::Pending));
    assert_eq!("sent".parse::<OutboxStatus>().ok(), Some(OutboxStatus::Sent));

    // Invalid cases should return Err (not panic)
    assert!("invalid".parse::<OutboxStatus>().is_err());
    assert!("".parse::<OutboxStatus>().is_err());

    // Note: Repository code should handle these errors gracefully
    // by defaulting to Pending and logging warnings
}
