//! Serialization utilities for common data types
//!
//! This module provides reusable serde serialization and deserialization
//! utilities that are used across multiple modules in the application.

use std::time::Duration;

use serde::{Deserialize, Deserializer, Serializer};

/// Custom serialization module for Duration as milliseconds
///
/// This module provides serialize and deserialize functions for Duration
/// that convert to/from milliseconds as u64 for JSON compatibility.
///
/// # Usage
/// ```rust
/// use std::time::Duration;
///
/// use pulsearc_common::duration_millis;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Example {
///     #[serde(with = "duration_millis")]
///     timeout: Duration,
/// }
/// ```
pub mod duration_millis {
    use super::*;

    /// Serde serialization result type
    type SerializeResult<S> = Result<<S as Serializer>::Ok, <S as Serializer>::Error>;

    /// Serialize a Duration as milliseconds (u64)
    pub fn serialize<S>(duration: &Duration, serializer: S) -> SerializeResult<S>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    /// Deserialize milliseconds (u64) into a Duration
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for serialization utilities
    //!
    //! Tests cover duration_millis serialization/deserialization,
    //! round-trip conversion, and edge cases (zero, large values).

    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestStruct {
        #[serde(with = "duration_millis")]
        timeout: Duration,
        name: String,
    }

    /// Tests that Duration serializes to milliseconds as u64
    #[test]
    fn test_duration_millis_serialize() {
        let data = TestStruct { timeout: Duration::from_millis(1500), name: "test".to_string() };

        let json = serde_json::to_string(&data).expect("Should serialize valid struct");
        assert!(json.contains("1500"), "Should contain milliseconds value");
        assert!(json.contains("test"), "Should contain string field");
    }

    /// Tests that milliseconds deserialize to Duration
    #[test]
    fn test_duration_millis_deserialize() {
        let json = r#"{"timeout":2500,"name":"test"}"#;
        let data: TestStruct = serde_json::from_str(json).expect("Should deserialize valid JSON");

        assert_eq!(data.timeout, Duration::from_millis(2500));
        assert_eq!(data.name, "test");
    }

    /// Tests round-trip serialization and deserialization
    #[test]
    fn test_duration_millis_round_trip() {
        let original =
            TestStruct { timeout: Duration::from_millis(3000), name: "round_trip".to_string() };

        let json = serde_json::to_string(&original).expect("Should serialize");
        let deserialized: TestStruct = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(original, deserialized, "Round-trip should preserve data");
    }

    /// Validates `Duration::ZERO` behavior for the duration millis zero
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `json.contains("\"timeout\":0")` evaluates to true.
    /// - Confirms `deserialized.timeout` equals `Duration::ZERO`.
    #[test]
    fn test_duration_millis_zero() {
        let data = TestStruct { timeout: Duration::ZERO, name: "zero".to_string() };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"timeout\":0"));

        let deserialized: TestStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.timeout, Duration::ZERO);
    }

    /// Validates `Duration::from_secs` behavior for the duration millis large
    /// value scenario.
    ///
    /// Assertions:
    /// - Ensures `json.contains("3600000")` evaluates to true.
    /// - Confirms `deserialized.timeout` equals `Duration::from_secs(3600)`.
    #[test]
    fn test_duration_millis_large_value() {
        let data = TestStruct {
            timeout: Duration::from_secs(3600), // 1 hour
            name: "large".to_string(),
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("3600000")); // 1 hour in milliseconds

        let deserialized: TestStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.timeout, Duration::from_secs(3600));
    }

    /// Validates `Duration::from_millis` behavior for the duration millis with
    /// multiple fields scenario.
    ///
    /// Assertions:
    /// - Confirms `config` equals `deserialized`.
    #[test]
    fn test_duration_millis_with_multiple_fields() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct MultiTimeout {
            #[serde(with = "duration_millis")]
            connect_timeout: Duration,
            #[serde(with = "duration_millis")]
            read_timeout: Duration,
            retries: u32,
        }

        let config = MultiTimeout {
            connect_timeout: Duration::from_millis(5000),
            read_timeout: Duration::from_millis(30000),
            retries: 3,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: MultiTimeout = serde_json::from_str(&json).unwrap();

        assert_eq!(config, deserialized);
    }

    /// Validates the duration millis deserialize invalid json scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_duration_millis_deserialize_invalid_json() {
        let invalid_json = r#"{"timeout":"not_a_number","name":"test"}"#;
        let result: Result<TestStruct, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());
    }
}
