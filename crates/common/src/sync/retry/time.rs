//! Time abstraction for testability
//!
//! Re-exports time abstractions from the testing module.
//! These are now located in `crate::testing::time` where they belong.

pub use crate::testing::time::{Clock, MockClock, SystemClock};
