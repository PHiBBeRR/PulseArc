//! Integration tests for utilities module (macros + serde helpers)
//!
//! Verifies the public API for `impl_status_conversions!` and the
//! `duration_millis` serde helper behave correctly when used the way
//! downstream crates do in configuration flows.

#![allow(clippy::doc_lazy_continuation)]

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use pulsearc_common::duration_millis;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

pulsearc_common::impl_status_conversions!(WorkflowStatus {
    Pending => "pending",
    Running => "running",
    Completed => "completed",
    Failed => "failed",
    Cancelled => "cancelled",
});

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct WorkflowSnapshot {
    job_id: String,
    status: String,
    #[serde(with = "duration_millis")]
    elapsed: Duration,
    #[serde(with = "duration_millis")]
    retry_backoff: Duration,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct WorkflowSummary {
    #[serde(with = "duration_millis")]
    poll_interval: Duration,
    snapshots: Vec<WorkflowSnapshot>,
    #[serde(with = "duration_millis")]
    max_execution: Duration,
    metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TimeoutWindow(
    #[serde(with = "duration_millis")] Duration,
    #[serde(with = "duration_millis")] Duration,
);

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct WindowSchedule {
    name: String,
    windows: Vec<TimeoutWindow>,
    #[serde(with = "duration_millis")]
    default_retry: Duration,
}

/// Validates `WorkflowStatus::Pending` behavior for the macro produces
/// lowercase display and roundtrip from string scenario.
///
/// Assertions:
/// - Confirms `status.to_string()` equals `expected`.
/// - Confirms `status` equals `lowercase`.
/// - Confirms `status` equals `uppercase`.
/// - Confirms `status` equals `parsed`.
/// Verifies the `impl_status_conversions!` macro normalizes display casing and
/// round-trips every workflow status from string inputs.
#[test]
fn macro_produces_lowercase_display_and_roundtrip_from_string() {
    let expectations = [
        (WorkflowStatus::Pending, "pending"),
        (WorkflowStatus::Running, "running"),
        (WorkflowStatus::Completed, "completed"),
        (WorkflowStatus::Failed, "failed"),
        (WorkflowStatus::Cancelled, "cancelled"),
    ];

    for (status, expected) in expectations {
        assert_eq!(status.to_string(), expected);

        let lowercase =
            WorkflowStatus::from_str(expected).expect("Lowercase strings should parse correctly");
        assert_eq!(status, lowercase);

        let uppercase = WorkflowStatus::from_str(expected.to_uppercase().as_str())
            .expect("Uppercase strings should parse correctly");
        assert_eq!(status, uppercase);

        let mixed = {
            let mut chars = expected.chars();
            let first = chars.next().expect("status string is non-empty").to_ascii_uppercase();
            let rest: String = chars
                .enumerate()
                .map(|(idx, ch)| if idx % 2 == 0 { ch.to_ascii_uppercase() } else { ch })
                .collect();
            format!("{first}{rest}")
        };
        let parsed =
            WorkflowStatus::from_str(&mixed).expect("Mixed case strings should parse correctly");
        assert_eq!(status, parsed);
    }
}

/// Validates `WorkflowStatus::from_str` behavior for the macro returns
/// contextual errors for unknown values scenario.
///
/// Assertions:
/// - Ensures `error.contains("Invalid WorkflowStatus: unknown-status")`
///   evaluates to true.
/// Ensures the status conversion macro returns contextual errors when unknown
/// variants are parsed.
#[test]
fn macro_returns_contextual_errors_for_unknown_values() {
    let error = WorkflowStatus::from_str("unknown-status")
        .expect_err("Unknown values should return an error");
    assert!(
        error.contains("Invalid WorkflowStatus: unknown-status"),
        "error message should contain enum name and value: {error}",
    );
}

/// Validates `HashMap::new` behavior for the workflow summary round trips
/// through multiple formats scenario.
///
/// Assertions:
/// - Ensures `json.contains("\"poll_interval\":15000")` evaluates to true.
/// - Ensures `json.contains("\"max_execution\":300000")` evaluates to true.
/// - Confirms `summary` equals `restored`.
/// - Ensures `matches!(parsed, WorkflowStatus::Running |
///   WorkflowStatus::Failed)` evaluates to true.
/// - Ensures `toml.contains("poll_interval = 15000")` evaluates to true.
/// - Ensures `toml.contains("max_execution = 300000")` evaluates to true.
/// - Confirms `summary` equals `restored_toml`.
/// Validates `WorkflowSummary` serializes and deserializes consistently across
/// JSON, TOML, and YAML representations.
#[test]
fn workflow_summary_round_trips_through_multiple_formats() {
    let mut metadata = HashMap::new();
    metadata.insert("environment".to_string(), "staging".to_string());
    metadata.insert("owner".to_string(), "scheduling-team".to_string());

    let summary = WorkflowSummary {
        poll_interval: Duration::from_secs(15),
        max_execution: Duration::from_secs(300),
        metadata,
        snapshots: vec![
            WorkflowSnapshot {
                job_id: "job-123".to_string(),
                status: WorkflowStatus::Running.to_string(),
                elapsed: Duration::from_millis(12_500),
                retry_backoff: Duration::from_secs(2),
            },
            WorkflowSnapshot {
                job_id: "job-456".to_string(),
                status: WorkflowStatus::Failed.to_string(),
                elapsed: Duration::from_secs(90),
                retry_backoff: Duration::from_millis(750),
            },
        ],
    };

    let json = serde_json::to_string(&summary).expect("JSON serialization should succeed");
    assert!(
        json.contains("\"poll_interval\":15000"),
        "poll interval should be serialized as milliseconds"
    );
    assert!(
        json.contains("\"max_execution\":300000"),
        "max execution should be serialized as milliseconds"
    );

    let restored: WorkflowSummary =
        serde_json::from_str(&json).expect("JSON deserialization should succeed");
    assert_eq!(summary, restored, "JSON round-trip should preserve data");

    for snapshot in &restored.snapshots {
        let parsed = WorkflowStatus::from_str(&snapshot.status)
            .expect("statuses should parse after round-trip");
        assert!(
            matches!(parsed, WorkflowStatus::Running | WorkflowStatus::Failed),
            "unexpected parsed status: {parsed:?}"
        );
    }

    let toml = toml::to_string(&summary).expect("TOML serialization should succeed");
    assert!(
        toml.contains("poll_interval = 15000"),
        "poll interval should be represented in milliseconds in TOML\n{toml}"
    );
    assert!(
        toml.contains("max_execution = 300000"),
        "max execution should be represented in milliseconds in TOML\n{toml}"
    );

    let restored_toml: WorkflowSummary =
        toml::from_str(&toml).expect("TOML deserialization should succeed");
    assert_eq!(summary, restored_toml, "TOML round-trip should preserve data");
}

/// Validates `Duration::from_secs` behavior for the duration millis supports
/// tuple structs and collections scenario.
///
/// Assertions:
/// - Ensures `json.contains("\"default_retry\":5000")` evaluates to true.
/// - Ensures `json.contains("[30000,120000]")` evaluates to true.
/// - Confirms `schedule` equals `deserialized`.
/// - Confirms `decoded_map.get("nightly-maintenance")` equals
///   `aggregated.get("nightly-maintenance")`.
/// Confirms the `duration_millis` helper encodes tuple structs and nested
/// collections without losing precision.
#[test]
fn duration_millis_supports_tuple_structs_and_collections() {
    let schedule = WindowSchedule {
        name: "nightly-maintenance".to_string(),
        default_retry: Duration::from_secs(5),
        windows: vec![
            TimeoutWindow(Duration::from_secs(30), Duration::from_secs(120)),
            TimeoutWindow(Duration::from_millis(750), Duration::from_secs(10)),
        ],
    };

    let json = serde_json::to_string(&schedule).expect("serialization should succeed");
    assert!(
        json.contains("\"default_retry\":5000"),
        "default retry should use millisecond representation"
    );
    assert!(
        json.contains("[30000,120000]"),
        "tuple struct should serialize both durations as milliseconds"
    );

    let deserialized: WindowSchedule =
        serde_json::from_str(&json).expect("deserialization should succeed");
    assert_eq!(schedule, deserialized, "tuple-based schedule should round-trip");

    let mut aggregated = HashMap::new();
    aggregated.insert("nightly-maintenance".to_string(), deserialized);

    let json_map = serde_json::to_string(&aggregated).expect("map serialization should succeed");
    let decoded_map: HashMap<String, WindowSchedule> =
        serde_json::from_str(&json_map).expect("map deserialization should succeed");

    assert_eq!(
        decoded_map.get("nightly-maintenance"),
        aggregated.get("nightly-maintenance"),
        "schedule embedded in a map should retain duration values"
    );
}

/// Validates the workflow summary rejects invalid duration payloads scenario.
///
/// Assertions:
/// - Ensures `error.to_string().contains("invalid type")` evaluates to true.
/// Checks that invalid duration payloads surface descriptive errors when
/// decoding `WorkflowSummary` structures.
#[test]
fn workflow_summary_rejects_invalid_duration_payloads() {
    let invalid_json = r#"
        {
            "poll_interval": "fifteen",
            "max_execution": 300000,
            "metadata": {},
            "snapshots": []
        }
    "#;

    let error = serde_json::from_str::<WorkflowSummary>(invalid_json)
        .expect_err("Invalid duration payload should fail to deserialize");
    assert!(
        error.to_string().contains("invalid type"),
        "error should reference the invalid type conversion: {error}"
    );
}
