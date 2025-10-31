//! Regression test for Issue #2: SAP Forwarder Hard-Coded Date
//!
//! **Bug**: Legacy code used hard-coded fallback date "2025-10-22"
//! **Impact**: Data corruption in SAP system, idempotency collisions
//! **Reference**: docs/issues/PHASE-3-PRE-MIGRATION-FIXES.md

#![allow(dead_code)]

#[path = "support.rs"]
mod support;

#[cfg(feature = "sap")]
use chrono::Utc;
#[cfg(feature = "sap")]
use log::Level;
#[cfg(feature = "sap")]
use pulsearc_domain::OutboxStatus;
#[cfg(feature = "sap")]
use pulsearc_infra::integrations::sap::SapForwarder;

#[tokio::test]
#[ignore] // Remove this once SAP forwarder is implemented in Phase 3C.2
#[cfg(feature = "sap")]
async fn test_sap_forwarder_derives_date_from_created_at() {
    let log_handle = support::init_test_logger();

    let mut entry = support::make_outbox_entry("sap-derive", OutboxStatus::Pending, 1_730_419_200);
    entry.payload_json = r#"{"duration": 3600, "note": "Test entry"}"#.to_string();

    let forwarder = SapForwarder::new();
    let result = forwarder.prepare_entry(&entry).expect("forwarder should succeed");

    assert_eq!(result.date, "2025-11-01", "Date should be derived from created_at");
    assert!(
        log_handle.contains(Level::Warn, "deriving from created_at"),
        "Missing date should produce a warning"
    );
}

#[tokio::test]
#[ignore] // Remove this once SAP forwarder is implemented
#[cfg(feature = "sap")]
async fn test_sap_forwarder_uses_payload_date_when_present() {
    let log_handle = support::init_test_logger();

    let mut entry =
        support::make_outbox_entry("sap-explicit", OutboxStatus::Pending, 1_730_419_200);
    entry.payload_json = r#"{"date": "2025-10-30", "duration": 3600}"#.to_string();

    let forwarder = SapForwarder::new();
    let result = forwarder.prepare_entry(&entry).expect("forwarder should succeed");

    assert_eq!(result.date, "2025-10-30", "Payload date should take precedence");
    assert!(
        !log_handle.contains(Level::Warn, "deriving from created_at"),
        "Explicit date should not produce a fallback warning"
    );
}

#[tokio::test]
#[ignore] // Remove this once SAP forwarder is implemented
#[cfg(feature = "sap")]
async fn test_sap_forwarder_handles_invalid_created_at() {
    let log_handle = support::init_test_logger();

    let mut entry = support::make_outbox_entry("sap-invalid-ts", OutboxStatus::Pending, -1);
    entry.payload_json.clear();

    let forwarder = SapForwarder::new();
    let result = forwarder.prepare_entry(&entry).expect("forwarder should succeed");

    let before = Utc::now().format("%Y-%m-%d").to_string();
    let after = Utc::now().format("%Y-%m-%d").to_string();
    assert!(result.date == before || result.date == after, "Fallback should use current UTC date");
    assert!(
        log_handle.contains(Level::Warn, "invalid created_at"),
        "Invalid timestamp should emit warning"
    );
}
