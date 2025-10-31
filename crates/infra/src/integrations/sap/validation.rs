//! WBS code validation logic
//!
//! Validates WBS codes using three layers:
//! 1. **Format validation** (static) - Regex, length, special chars
//! 2. **Existence validation** (cached) - Via WbsCache
//! 3. **Status validation** - REL, CLSD, TECO
//!
//! # Architecture
//!
//! Returns structured `WbsValidationResult` with `WbsValidationCode` enum
//! for API stability. Consumers match on enum variants, not message strings.
//!
//! # Example
//!
//! ```rust,ignore
//! use pulsearc_infra::integrations::sap::validation::{WbsValidator, WbsValidationResult};
//!
//! let validator = WbsValidator::new(cache);
//!
//! match validator.validate("USC0063201.1.1")? {
//!     WbsValidationResult::Valid => println!("Valid!"),
//!     WbsValidationResult::Warning { code, message } => {
//!         println!("Warning ({}): {}", code.as_str(), message);
//!     }
//!     WbsValidationResult::Error { code, message } => {
//!         println!("Error ({}): {}", code.as_str(), message);
//!     }
//! }
//! ```

use std::sync::{Arc, OnceLock};

use pulsearc_common::time::{Clock, SystemClock};
use pulsearc_core::classification::ports::WbsRepository;
use pulsearc_domain::Result;
use regex::Regex;

use super::cache::WbsCache;

/// Static validation codes for API stability
///
/// These codes can be matched on by API consumers without
/// needing to parse error messages. Use `as_str()` for logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WbsValidationCode {
    /// WBS code is valid and active
    Valid,

    /// WBS code format is invalid (regex check failed)
    FormatInvalid,

    /// WBS code is empty
    Empty,

    /// WBS code exceeds max length (50 chars)
    TooLong,

    /// WBS code contains invalid special characters
    InvalidChars,

    /// WBS element status is CLSD (closed, no new entries)
    StatusClosed,

    /// WBS element status is TECO (technically complete, restricted)
    StatusTechnicallyComplete,

    /// WBS element status is unknown
    StatusUnknown,

    /// WBS code not found in cache
    NotFoundInCache,

    /// Cache entry is stale (>24 hours old)
    CacheStale,
}

impl WbsValidationCode {
    /// Get static string code for API responses
    ///
    /// These codes are stable across versions and can be
    /// used by clients for conditional logic.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Valid => "VALID",
            Self::FormatInvalid => "FORMAT_INVALID",
            Self::Empty => "EMPTY",
            Self::TooLong => "TOO_LONG",
            Self::InvalidChars => "INVALID_CHARS",
            Self::StatusClosed => "STATUS_CLOSED",
            Self::StatusTechnicallyComplete => "STATUS_TECHNICALLY_COMPLETE",
            Self::StatusUnknown => "STATUS_UNKNOWN",
            Self::NotFoundInCache => "NOT_FOUND_IN_CACHE",
            Self::CacheStale => "CACHE_STALE",
        }
    }
}

/// Validation result for WBS code validation
///
/// Structured with enum codes for API stability. Consumers should
/// match on the `code()` variant, not parse `message` strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WbsValidationResult {
    /// WBS code is valid and active
    Valid,

    /// WBS code has issues but may still be usable
    Warning { code: WbsValidationCode, message: String },

    /// WBS code is invalid and cannot be used
    Error { code: WbsValidationCode, message: String },
}

impl WbsValidationResult {
    /// Check if validation passed (Valid or Warning)
    pub fn is_ok(&self) -> bool {
        !matches!(self, Self::Error { .. })
    }

    /// Check if validation failed
    pub fn is_err(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// Get the validation code
    pub fn code(&self) -> WbsValidationCode {
        match self {
            Self::Valid => WbsValidationCode::Valid,
            Self::Warning { code, .. } | Self::Error { code, .. } => *code,
        }
    }

    /// Get the message (if any)
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::Valid => None,
            Self::Warning { message, .. } | Self::Error { message, .. } => Some(message),
        }
    }
}

/// WBS code validator
///
/// Validates WBS codes using three layers:
/// 1. Format validation (static, no database)
/// 2. Existence validation (via cache)
/// 3. Status validation (REL/CLSD/TECO)
pub struct WbsValidator<C: Clock = SystemClock> {
    cache: Arc<WbsCache>,
    repository: Arc<dyn WbsRepository>,
    clock: Arc<C>,
}

/// Get cached regex pattern for WBS format validation
fn wbs_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        // Match: USC0063201.1.1, P-12345-01-001, WBS-001-002
        // Must start and end with alphanumeric, can contain dots/hyphens
        Regex::new(r"^[A-Z0-9][A-Z0-9.\-]*[A-Z0-9]$").unwrap()
    })
}

/// Normalize WBS code (uppercase, trim)
pub fn normalize_wbs_code(code: &str) -> String {
    code.trim().to_uppercase()
}

/// Validate WBS code format (static, no database)
///
/// Checks:
/// - Not empty
/// - Max length 50 characters
/// - No invalid special characters (%, $, @)
/// - Must contain at least one letter
/// - Matches regex pattern
pub fn validate_wbs_format(code: &str) -> WbsValidationResult {
    let normalized = normalize_wbs_code(code);

    // Check empty
    if normalized.is_empty() {
        return WbsValidationResult::Error {
            code: WbsValidationCode::Empty,
            message: "WBS code cannot be empty".to_string(),
        };
    }

    // Check max length
    if normalized.len() > 50 {
        return WbsValidationResult::Error {
            code: WbsValidationCode::TooLong,
            message: format!(
                "WBS code exceeds maximum length of 50 characters (got {})",
                normalized.len()
            ),
        };
    }

    // Check special characters
    if normalized.contains(['%', '$', '@']) {
        return WbsValidationResult::Error {
            code: WbsValidationCode::InvalidChars,
            message: "WBS code contains invalid special characters (%, $, or @)".to_string(),
        };
    }

    // Must contain at least one letter
    if !normalized.chars().any(|c| c.is_alphabetic()) {
        return WbsValidationResult::Error {
            code: WbsValidationCode::FormatInvalid,
            message: "WBS code must contain at least one letter".to_string(),
        };
    }

    // Check regex pattern
    if !wbs_regex().is_match(&normalized) {
        return WbsValidationResult::Error {
            code: WbsValidationCode::FormatInvalid,
            message: "Invalid WBS code format. Expected format: USC0063201.1.1 or P-12345-01-001"
                .to_string(),
        };
    }

    WbsValidationResult::Valid
}

/// Validate WBS status
///
/// Status codes:
/// - REL (Released) → Valid
/// - CLSD (Closed) → Error
/// - TECO (Technically Complete) → Warning
/// - Other → Warning
pub fn validate_wbs_status(status: &str) -> WbsValidationResult {
    match status.to_uppercase().as_str() {
        "REL" => WbsValidationResult::Valid,
        "CLSD" => WbsValidationResult::Error {
            code: WbsValidationCode::StatusClosed,
            message: "This project is closed and cannot accept new time entries".to_string(),
        },
        "TECO" => WbsValidationResult::Warning {
            code: WbsValidationCode::StatusTechnicallyComplete,
            message: "This project is technically complete. Time entries may be restricted"
                .to_string(),
        },
        other => WbsValidationResult::Warning {
            code: WbsValidationCode::StatusUnknown,
            message: format!("Unknown project status: {}", other),
        },
    }
}

impl WbsValidator<SystemClock> {
    /// Create a new validator
    pub fn new(cache: Arc<WbsCache>, repository: Arc<dyn WbsRepository>) -> Self {
        Self::with_clock(cache, repository, Arc::new(SystemClock))
    }
}

impl<C: Clock> WbsValidator<C> {
    /// Create a new validator with a custom clock (useful for deterministic
    /// tests)
    pub fn with_clock(
        cache: Arc<WbsCache>,
        repository: Arc<dyn WbsRepository>,
        clock: Arc<C>,
    ) -> Self {
        Self { cache, repository, clock }
    }

    /// Helper to get current timestamp from clock (seconds since UNIX epoch)
    fn now_timestamp(&self) -> i64 {
        (self.clock.millis_since_epoch() / 1000) as i64
    }

    /// Validate WBS code existence (uses cache)
    pub fn validate_existence(&self, code: &str) -> Result<WbsValidationResult> {
        let normalized = normalize_wbs_code(code);

        match self.cache.get_or_fetch(&normalized, &*self.repository)? {
            Some(wbs) => {
                // Check if cache entry is stale (>24 hours old)
                let now = self.now_timestamp();
                let age_hours = (now - wbs.cached_at) / 3600;

                if age_hours > 24 {
                    Ok(WbsValidationResult::Warning {
                        code: WbsValidationCode::CacheStale,
                        message: format!(
                            "WBS code found but cache is {} hours old. Consider syncing.",
                            age_hours
                        ),
                    })
                } else {
                    Ok(WbsValidationResult::Valid)
                }
            }
            None => Ok(WbsValidationResult::Error {
                code: WbsValidationCode::NotFoundInCache,
                message: "WBS code not found in cache. Please sync to get the latest codes"
                    .to_string(),
            }),
        }
    }

    /// Validate a WBS code (format + existence + status)
    ///
    /// Performs all three validation layers:
    /// 1. Format validation (fails fast on format errors)
    /// 2. Existence validation (queries cache/repository)
    /// 3. Status validation (checks REL/CLSD/TECO)
    ///
    /// Returns the first error encountered, or the highest severity result.
    pub fn validate(&self, code: &str) -> Result<WbsValidationResult> {
        // Step 1: Validate format (fail fast)
        let format_result = validate_wbs_format(code);
        if format_result.is_err() {
            return Ok(format_result);
        }

        let normalized = normalize_wbs_code(code);

        // Step 2: Check existence (via cache)
        match self.cache.get_or_fetch(&normalized, &*self.repository)? {
            Some(wbs) => {
                // Step 3: Validate status
                let status_result = validate_wbs_status(&wbs.status);
                if status_result.is_err() {
                    return Ok(status_result);
                }

                // Step 4: Check cache freshness
                let now = self.now_timestamp();
                let age_hours = (now - wbs.cached_at) / 3600;

                if age_hours > 24 {
                    // Return staleness warning if no other warnings
                    if status_result == WbsValidationResult::Valid {
                        Ok(WbsValidationResult::Warning {
                            code: WbsValidationCode::CacheStale,
                            message: format!(
                                "WBS code is valid but cache is {} hours old",
                                age_hours
                            ),
                        })
                    } else {
                        // Return status warning (e.g., TECO)
                        Ok(status_result)
                    }
                } else {
                    // All checks passed or non-blocking warning
                    Ok(status_result)
                }
            }
            None => Ok(WbsValidationResult::Error {
                code: WbsValidationCode::NotFoundInCache,
                message: "WBS code not found in cache. Please sync.".to_string(),
            }),
        }
    }

    /// Validate multiple WBS codes in batch
    ///
    /// Returns a vector of tuples with normalized code and validation result.
    /// Continues validation even if some codes fail.
    #[allow(clippy::type_complexity)] // Return type is clear: list of (code, validation result)
    pub fn validate_batch(&self, codes: Vec<String>) -> Vec<(String, Result<WbsValidationResult>)> {
        codes
            .into_iter()
            .map(|code| {
                let normalized = normalize_wbs_code(&code);
                let result = self.validate(&code);
                (normalized, result)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use pulsearc_common::time::{MockClock, SystemClock};
    use pulsearc_domain::types::sap::WbsElement;

    use super::*;
    use crate::integrations::sap::cache::{WbsCache, WbsCacheConfig};

    /// Mock WBS repository for testing
    struct MockWbsRepository {
        valid_codes: Vec<String>,
        /// Track query count
        query_count: Mutex<usize>,
    }

    impl MockWbsRepository {
        fn new(valid_codes: Vec<String>) -> Self {
            Self { valid_codes, query_count: Mutex::new(0) }
        }
    }

    impl WbsRepository for MockWbsRepository {
        fn get_wbs_by_wbs_code(&self, wbs_code: &str) -> Result<Option<WbsElement>> {
            *self.query_count.lock().unwrap() += 1;

            if self.valid_codes.contains(&wbs_code.to_string()) {
                Ok(Some(WbsElement {
                    wbs_code: wbs_code.to_string(),
                    project_def: "TEST-001".to_string(),
                    project_name: Some("Test Project".to_string()),
                    description: Some("Test Description".to_string()),
                    status: "REL".to_string(),
                    cached_at: chrono::Utc::now().timestamp(),
                    opportunity_id: None,
                    deal_name: None,
                    target_company_name: None,
                    counterparty: None,
                    industry: None,
                    region: None,
                    amount: None,
                    stage_name: None,
                    project_code: None,
                }))
            } else {
                Ok(None)
            }
        }

        fn count_active_wbs(&self) -> Result<i64> {
            Ok(self.valid_codes.len() as i64)
        }

        fn get_last_sync_timestamp(&self) -> Result<Option<i64>> {
            Ok(Some(chrono::Utc::now().timestamp()))
        }

        fn load_common_projects(&self, _limit: usize) -> Result<Vec<WbsElement>> {
            Ok(vec![])
        }

        fn fts5_search_keyword(&self, _keyword: &str, _limit: usize) -> Result<Vec<WbsElement>> {
            Ok(vec![])
        }

        fn get_wbs_by_project_def(&self, _project_def: &str) -> Result<Option<WbsElement>> {
            Ok(None)
        }
    }

    #[test]
    fn test_validate_format_valid_codes() {
        assert_eq!(validate_wbs_format("USC0063201.1.1"), WbsValidationResult::Valid);
        assert_eq!(validate_wbs_format("P-12345-01-001"), WbsValidationResult::Valid);
        assert_eq!(validate_wbs_format("WBS-001-002"), WbsValidationResult::Valid);
        assert_eq!(validate_wbs_format("A1"), WbsValidationResult::Valid);
    }

    #[test]
    fn test_validate_format_invalid_codes() {
        // Empty
        let result = validate_wbs_format("");
        assert!(result.is_err());
        assert_eq!(result.code(), WbsValidationCode::Empty);

        // Too long
        let result = validate_wbs_format(&"A".repeat(51));
        assert!(result.is_err());
        assert_eq!(result.code(), WbsValidationCode::TooLong);

        // Invalid special chars
        let result = validate_wbs_format("USC%001");
        assert!(result.is_err());
        assert_eq!(result.code(), WbsValidationCode::InvalidChars);

        let result = validate_wbs_format("USC$001");
        assert!(result.is_err());
        assert_eq!(result.code(), WbsValidationCode::InvalidChars);

        // No letters (numbers only)
        let result = validate_wbs_format("123456");
        assert!(result.is_err());
        assert_eq!(result.code(), WbsValidationCode::FormatInvalid);

        // Invalid start/end characters
        let result = validate_wbs_format(".USC001");
        assert!(result.is_err());
        assert_eq!(result.code(), WbsValidationCode::FormatInvalid);

        let result = validate_wbs_format("USC001.");
        assert!(result.is_err());
        assert_eq!(result.code(), WbsValidationCode::FormatInvalid);
    }

    #[test]
    fn test_validate_status() {
        // REL (released) → Valid
        assert_eq!(validate_wbs_status("REL"), WbsValidationResult::Valid);

        // CLSD (closed) → Error
        let result = validate_wbs_status("CLSD");
        assert!(result.is_err());
        assert_eq!(result.code(), WbsValidationCode::StatusClosed);

        // TECO (technically complete) → Warning
        let result = validate_wbs_status("TECO");
        assert!(!result.is_err());
        assert_eq!(result.code(), WbsValidationCode::StatusTechnicallyComplete);

        // Unknown status → Warning
        let result = validate_wbs_status("UNKNOWN");
        assert!(!result.is_err());
        assert_eq!(result.code(), WbsValidationCode::StatusUnknown);
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize_wbs_code(" usc001 "), "USC001");
        assert_eq!(normalize_wbs_code("usc001"), "USC001");
        assert_eq!(normalize_wbs_code("USC001"), "USC001");
        assert_eq!(normalize_wbs_code("  usc0063201.1.1  "), "USC0063201.1.1");
    }

    #[test]
    fn test_full_validation_valid_code() {
        let repo = Arc::new(MockWbsRepository::new(vec!["USC0063201.1.1".to_string()]));
        let config = WbsCacheConfig::with_ttl(std::time::Duration::from_secs(60));
        let cache = Arc::new(WbsCache::<SystemClock>::new(config));
        let validator = WbsValidator::new(cache, repo);

        let result = validator.validate("USC0063201.1.1").unwrap();
        assert!(result.is_ok());
        assert_eq!(result.code(), WbsValidationCode::Valid);
    }

    #[test]
    fn test_full_validation_not_found() {
        let repo = Arc::new(MockWbsRepository::new(vec![]));
        let config = WbsCacheConfig::with_ttl(std::time::Duration::from_secs(60));
        let cache = Arc::new(WbsCache::<SystemClock>::new(config));
        let validator = WbsValidator::new(cache, repo);

        let result = validator.validate("INVALID-CODE").unwrap();
        assert!(result.is_err());
        assert_eq!(result.code(), WbsValidationCode::NotFoundInCache);
    }

    #[test]
    fn test_full_validation_format_error() {
        let repo = Arc::new(MockWbsRepository::new(vec![]));
        let config = WbsCacheConfig::with_ttl(std::time::Duration::from_secs(60));
        let cache = Arc::new(WbsCache::<SystemClock>::new(config));
        let validator = WbsValidator::new(cache, repo);

        // Empty code should fail format validation
        let result = validator.validate("").unwrap();
        assert!(result.is_err());
        assert_eq!(result.code(), WbsValidationCode::Empty);

        // Invalid special chars should fail format validation
        let result = validator.validate("USC%001").unwrap();
        assert!(result.is_err());
        assert_eq!(result.code(), WbsValidationCode::InvalidChars);
    }

    #[test]
    fn test_batch_validation() {
        let repo = Arc::new(MockWbsRepository::new(vec![
            "USC0063201.1.1".to_string(),
            "USC0063201.1.2".to_string(),
        ]));
        let config = WbsCacheConfig::with_ttl(std::time::Duration::from_secs(60));
        let cache = Arc::new(WbsCache::<SystemClock>::new(config));
        let validator = WbsValidator::new(cache, repo);

        let codes = vec![
            "USC0063201.1.1".to_string(),
            "USC0063201.1.2".to_string(),
            "INVALID-CODE".to_string(),
            "".to_string(), // Format error
        ];

        let results = validator.validate_batch(codes);
        assert_eq!(results.len(), 4);

        // First two should be valid
        assert!(results[0].1.as_ref().unwrap().is_ok());
        assert!(results[1].1.as_ref().unwrap().is_ok());

        // Third should be not found error
        assert!(results[2].1.as_ref().unwrap().is_err());
        assert_eq!(results[2].1.as_ref().unwrap().code(), WbsValidationCode::NotFoundInCache);

        // Fourth should be format error (empty)
        assert!(results[3].1.as_ref().unwrap().is_err());
        assert_eq!(results[3].1.as_ref().unwrap().code(), WbsValidationCode::Empty);
    }

    #[test]
    fn test_full_validation_warns_when_cache_stale_with_mock_clock() {
        const WBS_CODE: &str = "USC0063201.1.1";

        let clock = Arc::new(MockClock::new());
        clock.set_elapsed(Duration::from_secs(48 * 3600));

        let current_secs = (clock.millis_since_epoch() / 1_000) as i64;
        let stale_element = WbsElement {
            wbs_code: WBS_CODE.to_string(),
            project_def: "TEST-001".to_string(),
            project_name: Some("Test Project".to_string()),
            description: Some("Stale cache entry".to_string()),
            status: "REL".to_string(),
            cached_at: current_secs - (25 * 3600),
            opportunity_id: None,
            deal_name: None,
            target_company_name: None,
            counterparty: None,
            industry: None,
            region: None,
            amount: None,
            stage_name: None,
            project_code: None,
        };

        struct StaleRepo {
            element: WbsElement,
        }

        impl WbsRepository for StaleRepo {
            fn get_wbs_by_wbs_code(&self, _wbs_code: &str) -> Result<Option<WbsElement>> {
                Ok(Some(self.element.clone()))
            }

            fn count_active_wbs(&self) -> Result<i64> {
                Ok(1)
            }

            fn get_last_sync_timestamp(&self) -> Result<Option<i64>> {
                Ok(Some(self.element.cached_at))
            }

            fn load_common_projects(&self, _limit: usize) -> Result<Vec<WbsElement>> {
                Ok(vec![])
            }

            fn fts5_search_keyword(
                &self,
                _keyword: &str,
                _limit: usize,
            ) -> Result<Vec<WbsElement>> {
                Ok(vec![])
            }

            fn get_wbs_by_project_def(&self, _project_def: &str) -> Result<Option<WbsElement>> {
                Ok(None)
            }
        }

        let repo: Arc<dyn WbsRepository> = Arc::new(StaleRepo { element: stale_element });
        let cache = Arc::new(WbsCache::new(WbsCacheConfig::with_ttl(Duration::from_secs(60))));
        let validator = WbsValidator::with_clock(cache, repo, clock.clone());

        let result = validator.validate(WBS_CODE).expect("validation should succeed");
        assert!(result.is_ok());
        assert_eq!(result.code(), WbsValidationCode::CacheStale);
    }

    #[test]
    fn test_validation_result_methods() {
        let valid = WbsValidationResult::Valid;
        assert!(valid.is_ok());
        assert!(!valid.is_err());
        assert_eq!(valid.code(), WbsValidationCode::Valid);
        assert_eq!(valid.message(), None);

        let warning = WbsValidationResult::Warning {
            code: WbsValidationCode::CacheStale,
            message: "Cache is stale".to_string(),
        };
        assert!(warning.is_ok());
        assert!(!warning.is_err());
        assert_eq!(warning.code(), WbsValidationCode::CacheStale);
        assert_eq!(warning.message(), Some("Cache is stale"));

        let error = WbsValidationResult::Error {
            code: WbsValidationCode::StatusClosed,
            message: "Project is closed".to_string(),
        };
        assert!(!error.is_ok());
        assert!(error.is_err());
        assert_eq!(error.code(), WbsValidationCode::StatusClosed);
        assert_eq!(error.message(), Some("Project is closed"));
    }

    #[test]
    fn test_validation_code_as_str() {
        assert_eq!(WbsValidationCode::Valid.as_str(), "VALID");
        assert_eq!(WbsValidationCode::FormatInvalid.as_str(), "FORMAT_INVALID");
        assert_eq!(WbsValidationCode::Empty.as_str(), "EMPTY");
        assert_eq!(WbsValidationCode::TooLong.as_str(), "TOO_LONG");
        assert_eq!(WbsValidationCode::InvalidChars.as_str(), "INVALID_CHARS");
        assert_eq!(WbsValidationCode::StatusClosed.as_str(), "STATUS_CLOSED");
        assert_eq!(
            WbsValidationCode::StatusTechnicallyComplete.as_str(),
            "STATUS_TECHNICALLY_COMPLETE"
        );
        assert_eq!(WbsValidationCode::StatusUnknown.as_str(), "STATUS_UNKNOWN");
        assert_eq!(WbsValidationCode::NotFoundInCache.as_str(), "NOT_FOUND_IN_CACHE");
        assert_eq!(WbsValidationCode::CacheStale.as_str(), "CACHE_STALE");
    }
}
