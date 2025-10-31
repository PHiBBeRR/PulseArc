//! Privacy Module - Portable Core
//!
//! This module provides portable, domain-independent privacy functionality
//! including secure hashing, pattern matching, and data sanitization.

pub mod hash;
pub mod patterns;

// Re-export commonly used types
pub use hash::{HashAlgorithm, HashConfig, HashError, HashResult, SecureHasher};
pub use patterns::{PatternMatcher, PiiDetectionConfig, PiiError, PiiResult, PiiType};
