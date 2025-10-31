//! SAP integration helpers.
//!
//! This module is only compiled when the `sap` feature is enabled. It provides
//! a forwarder that converts outbox entries into the lightweight structures
//! expected by the SAP client port while enforcing data hygiene safeguards.

use chrono::{DateTime, Utc};
use log::warn;
use pulsearc_core::sap_ports::TimeEntry as SapTimeEntry;
use pulsearc_domain::{Result, TimeEntryOutbox};
use serde_json::Value;

/// Converts outbox entries into SAP-ready payloads.
pub struct SapForwarder;

impl SapForwarder {
    /// Create a new forwarder instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Prepare an SAP time entry from an outbox record.
    ///
    /// The forwarder prefers explicit values from `payload_json` but gracefully
    /// falls back to the outbox record (or sensible defaults) to avoid the
    /// legacy anti-patterns documented in Phase 3 pre-migration fixes.
    pub fn prepare_entry(&self, entry: &TimeEntryOutbox) -> Result<SapTimeEntry> {
        let payload = Self::parse_payload(entry);

        let date = self.resolve_date(entry, &payload);
        let duration_hours = payload
            .get("duration")
            .and_then(Value::as_f64)
            .map(|seconds| (seconds / 3600.0) as f32)
            .unwrap_or(0.0);

        let description = payload
            .get("note")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| entry.description.clone())
            .unwrap_or_default();

        let wbs_code = payload
            .get("wbs_code")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| entry.wbs_code.clone())
            .unwrap_or_default();

        Ok(SapTimeEntry { wbs_code, description, duration_hours, date })
    }

    fn parse_payload(entry: &TimeEntryOutbox) -> Value {
        serde_json::from_str(&entry.payload_json).unwrap_or_else(|err| {
            warn!(
                "Failed to parse payload_json for entry {}: {} – defaulting to empty object",
                entry.id, err
            );
            Value::Null
        })
    }

    fn resolve_date(&self, entry: &TimeEntryOutbox, payload: &Value) -> String {
        if let Some(date) = payload.get("date").and_then(Value::as_str) {
            return date.to_string();
        }

        self.derive_date_from_created_at(entry)
    }

    fn derive_date_from_created_at(&self, entry: &TimeEntryOutbox) -> String {
        if let Some(created_at) = DateTime::<Utc>::from_timestamp(entry.created_at, 0) {
            let derived = created_at.format("%Y-%m-%d").to_string();
            warn!(
                "Missing date field for entry {}, deriving from created_at: {}",
                entry.id, derived
            );
            derived
        } else {
            let now = Utc::now();
            let derived = now.format("%Y-%m-%d").to_string();
            warn!(
                "Missing date field and invalid created_at for entry {}; falling back to current time: {}",
                entry.id, derived
            );
            derived
        }
    }
}
