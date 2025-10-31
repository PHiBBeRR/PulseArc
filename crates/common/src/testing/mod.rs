//! Testing utilities and helpers
//!
//! This module provides comprehensive testing utilities including:
//! - **[`assertions`]**: Custom assertions for complex scenarios
//! - **[`async_utils`]**: Async test utilities and helpers
//! - **[`builders`]**: Test data builders with fluent API
//! - **[`fixtures`]**: Test fixture generators (with deterministic seeded
//!   variants)
//! - **[`matchers`]**: Custom matchers for assertions
//! - **[`mocks`]**: Mock implementations of common traits
//! - **[`temp`]**: Temporary file/directory helpers
//! - **[`time`]**: Time mocking utilities (re-exported from sync)
//!
//! ## Usage
//!
//! ```rust
//! # #[cfg(feature = "runtime")]
//! # {
//! use pulsearc_common::testing::{MockClock, TestBuilder};
//!
//! // In your tests:
//! fn test_with_mock_time() {
//!     let clock = MockClock::new();
//!     clock.advance(std::time::Duration::from_secs(5));
//!     // ... test with controlled time
//! }
//!
//! fn test_error_message() {
//!     let result: Result<(), String> = Err("Connection timeout".to_string());
//!     pulsearc_common::assert_error_contains!(result, "timeout");
//! }
//! # }
//! ```

pub mod assertions;
pub mod async_utils;
pub mod builders;
pub mod fixtures;
pub mod matchers;
pub mod mocks;
pub mod temp;
pub mod time;

// Re-export commonly used items
// Note: Macros exported with #[macro_export] are available at crate root
// Re-export assertion functions (not macros)
pub use assertions::{
    assert_approx_eq, assert_contains_all, assert_duration_in_range, assert_sorted,
};
pub use async_utils::{poll_until, retry_async, timeout_ok};
pub use builders::{StringBuilder, TestBuilder};
pub use fixtures::{random_email, random_string, random_u64};
pub use matchers::{contains_string, is_error, is_ok, matches_pattern};
pub use mocks::{MockHttpClient, MockStorage};
#[cfg(feature = "platform")]
pub use mocks::{MockKeychainProvider, MockOAuthClient};
pub use temp::{TempDir, TempFile};
pub use time::{Clock, MockClock, SystemClock};
