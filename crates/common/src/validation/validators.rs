// Field Validators - Reusable validation components
use std::fmt::Display;
use std::marker::PhantomData;

#[cfg(feature = "foundation")]
use once_cell::sync::Lazy;

/// Type alias for a boxed field validator (clippy::type_complexity)
type BoxedFieldValidator<T> = Box<dyn FieldValidator<T>>;

/// Type alias for a custom validation function (clippy::type_complexity)
type CustomValidationFn<T> = Box<dyn Fn(&T) -> Result<(), String>>;

/// Trait for field validators
pub trait FieldValidator<T> {
    /// Validate a field value
    fn validate(&self, value: &T) -> Result<(), String>;
}

/// Range validator for numeric types
#[derive(Debug, Clone)]
pub struct RangeValidator<T> {
    min: Option<T>,
    max: Option<T>,
    _phantom: PhantomData<T>,
}

impl<T> Default for RangeValidator<T>
where
    T: PartialOrd + Display + Clone,
{
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> RangeValidator<T>
where
    T: PartialOrd + Display + Clone,
{
    /// Create a new range validator with no constraints
    pub fn empty() -> Self {
        Self { min: None, max: None, _phantom: PhantomData }
    }

    /// Create a new range validator with min and max values (convenience
    /// constructor)
    pub fn new(min: T, max: T) -> Self {
        Self { min: Some(min), max: Some(max), _phantom: PhantomData }
    }

    /// Set minimum value
    pub fn min(mut self, min: T) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum value
    pub fn max(mut self, max: T) -> Self {
        self.max = Some(max);
        self
    }

    /// Set both min and max
    pub fn between(mut self, min: T, max: T) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }
}

impl<T> FieldValidator<T> for RangeValidator<T>
where
    T: PartialOrd + Display + Clone,
{
    fn validate(&self, value: &T) -> Result<(), String> {
        if let Some(ref min) = self.min {
            if value < min {
                return Err(format!("Value must be at least {}", min));
            }
        }

        if let Some(ref max) = self.max {
            if value > max {
                return Err(format!("Value must not exceed {}", max));
            }
        }

        Ok(())
    }
}

/// String validator with various constraints
#[derive(Debug, Clone)]
pub struct StringValidator {
    min_length: Option<usize>,
    max_length: Option<usize>,
    pattern: Option<regex::Regex>,
    not_empty: bool,
    trim: bool,
}

impl Default for StringValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl StringValidator {
    /// Create a new string validator
    pub fn new() -> Self {
        Self { min_length: None, max_length: None, pattern: None, not_empty: false, trim: true }
    }

    /// Require non-empty string
    pub fn not_empty(mut self) -> Self {
        self.not_empty = true;
        self
    }

    /// Set minimum length
    pub fn min_length(mut self, min: usize) -> Self {
        self.min_length = Some(min);
        self
    }

    /// Set maximum length
    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Set pattern to match
    pub fn pattern(mut self, pattern: &str) -> Result<Self, regex::Error> {
        self.pattern = Some(regex::Regex::new(pattern)?);
        Ok(self)
    }

    /// Set whether to trim before validation
    pub fn trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }
}

impl FieldValidator<String> for StringValidator {
    fn validate(&self, value: &String) -> Result<(), String> {
        let val = if self.trim { value.trim() } else { value.as_str() };

        if self.not_empty && val.is_empty() {
            return Err("Value cannot be empty".to_string());
        }

        if let Some(min) = self.min_length {
            if val.len() < min {
                return Err(format!("Length must be at least {} characters", min));
            }
        }

        if let Some(max) = self.max_length {
            if val.len() > max {
                return Err(format!("Length must not exceed {} characters", max));
            }
        }

        if let Some(ref pattern) = self.pattern {
            if !pattern.is_match(val) {
                return Err(format!("Value must match pattern: {}", pattern.as_str()));
            }
        }

        Ok(())
    }
}

impl FieldValidator<&str> for StringValidator {
    fn validate(&self, value: &&str) -> Result<(), String> {
        self.validate(&value.to_string())
    }
}

/// Collection validator for vectors and other collections
pub struct CollectionValidator<T> {
    min_size: Option<usize>,
    max_size: Option<usize>,
    unique_items: bool,
    item_validator: Option<BoxedFieldValidator<T>>,
}

impl<T> Clone for CollectionValidator<T> {
    fn clone(&self) -> Self {
        Self {
            min_size: self.min_size,
            max_size: self.max_size,
            unique_items: self.unique_items,
            item_validator: None, // Can't clone trait objects, so we'll set to None
        }
    }
}

impl<T> std::fmt::Debug for CollectionValidator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollectionValidator")
            .field("min_size", &self.min_size)
            .field("max_size", &self.max_size)
            .field("unique_items", &self.unique_items)
            .field("item_validator", &"<dyn FieldValidator>")
            .finish()
    }
}

impl<T> Default for CollectionValidator<T>
where
    T: PartialEq + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CollectionValidator<T>
where
    T: PartialEq + Clone,
{
    /// Create a new collection validator
    pub fn new() -> Self {
        Self { min_size: None, max_size: None, unique_items: false, item_validator: None }
    }

    /// Set minimum size
    pub fn min_size(mut self, min: usize) -> Self {
        self.min_size = Some(min);
        self
    }

    /// Set maximum size
    pub fn max_size(mut self, max: usize) -> Self {
        self.max_size = Some(max);
        self
    }

    /// Require unique items
    pub fn unique_items(mut self) -> Self {
        self.unique_items = true;
        self
    }

    /// Set a validator for individual items in the collection
    ///
    /// This allows validation of each item in the collection using any
    /// validator that implements `FieldValidator<T>`. If an item fails
    /// validation, the error message will include the index of the failing
    /// item.
    ///
    /// # Example
    ///
    /// ```
    /// use pulsearc_common::validation::{CollectionValidator, EmailValidator};
    ///
    /// let validator: CollectionValidator<String> =
    ///     CollectionValidator::new().min_size(1).item_validator(EmailValidator::new());
    /// ```
    pub fn item_validator<V>(mut self, validator: V) -> Self
    where
        V: FieldValidator<T> + 'static,
    {
        self.item_validator = Some(Box::new(validator));
        self
    }
}

impl<T> FieldValidator<Vec<T>> for CollectionValidator<T>
where
    T: PartialEq + Clone,
{
    fn validate(&self, value: &Vec<T>) -> Result<(), String> {
        let size = value.len();

        if let Some(min) = self.min_size {
            if size < min {
                return Err(format!("Collection must contain at least {} items", min));
            }
        }

        if let Some(max) = self.max_size {
            if size > max {
                return Err(format!("Collection must not exceed {} items", max));
            }
        }

        if self.unique_items {
            let mut seen = Vec::new();
            for item in value {
                if seen.contains(item) {
                    return Err("Collection must contain unique items".to_string());
                }
                seen.push(item.clone());
            }
        }

        // Validate individual items if item_validator is set
        if let Some(ref validator) = self.item_validator {
            for (index, item) in value.iter().enumerate() {
                if let Err(e) = validator.validate(item) {
                    return Err(format!("Item at index {} failed validation: {}", index, e));
                }
            }
        }

        Ok(())
    }
}

/// Custom validator that takes a closure
pub struct CustomValidator<T> {
    validator: CustomValidationFn<T>,
}

impl<T> Clone for CustomValidator<T> {
    fn clone(&self) -> Self {
        // Cannot clone closures, so we create a new one that always returns Ok
        Self::new(|_| Ok(()))
    }
}

impl<T> std::fmt::Debug for CustomValidator<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomValidator").field("validator", &"<closure>").finish()
    }
}

impl<T> CustomValidator<T> {
    /// Create a new custom validator
    pub fn new<F>(validator: F) -> Self
    where
        F: Fn(&T) -> Result<(), String> + 'static,
    {
        Self { validator: Box::new(validator) }
    }
}

impl<T> FieldValidator<T> for CustomValidator<T> {
    fn validate(&self, value: &T) -> Result<(), String> {
        (self.validator)(value)
    }
}

/// Static email regex pattern compiled once at first use
static EMAIL_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .expect("EMAIL_REGEX pattern is valid and well-formed")
});

/// Email validator
#[derive(Debug, Clone)]
pub struct EmailValidator;

impl Default for EmailValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl EmailValidator {
    /// Create a new email validator
    pub fn new() -> Self {
        Self
    }
}

impl FieldValidator<String> for EmailValidator {
    fn validate(&self, value: &String) -> Result<(), String> {
        if !EMAIL_REGEX.is_match(value) {
            return Err("Invalid email format".to_string());
        }

        Ok(())
    }
}

impl FieldValidator<&str> for EmailValidator {
    fn validate(&self, value: &&str) -> Result<(), String> {
        self.validate(&value.to_string())
    }
}

/// URL validator
#[derive(Debug, Clone)]
pub struct UrlValidator {
    require_https: bool,
    allowed_schemes: Vec<String>,
}

impl Default for UrlValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl UrlValidator {
    /// Create a new URL validator
    pub fn new() -> Self {
        Self {
            require_https: false,
            allowed_schemes: vec!["http".to_string(), "https".to_string()],
        }
    }

    /// Require HTTPS
    pub fn require_https(mut self) -> Self {
        self.require_https = true;
        self
    }

    /// Set allowed schemes
    pub fn allowed_schemes(mut self, schemes: Vec<String>) -> Self {
        self.allowed_schemes = schemes;
        self
    }
}

impl FieldValidator<String> for UrlValidator {
    fn validate(&self, value: &String) -> Result<(), String> {
        if let Ok(parsed) = url::Url::parse(value) {
            let scheme = parsed.scheme();

            if self.require_https && scheme != "https" {
                return Err("URL must use HTTPS".to_string());
            }

            if !self.allowed_schemes.contains(&scheme.to_string()) {
                return Err(format!("URL scheme '{}' is not allowed", scheme));
            }

            Ok(())
        } else {
            Err("Invalid URL format".to_string())
        }
    }
}

impl FieldValidator<&str> for UrlValidator {
    fn validate(&self, value: &&str) -> Result<(), String> {
        self.validate(&value.to_string())
    }
}

/// IP address validator
#[derive(Debug, Clone)]
pub struct IpValidator {
    allow_v4: bool,
    allow_v6: bool,
    allow_private: bool,
}

impl Default for IpValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl IpValidator {
    /// Create a new IP validator
    pub fn new() -> Self {
        Self { allow_v4: true, allow_v6: true, allow_private: true }
    }

    /// Only allow IPv4
    pub fn v4_only(mut self) -> Self {
        self.allow_v4 = true;
        self.allow_v6 = false;
        self
    }

    /// Only allow IPv6
    pub fn v6_only(mut self) -> Self {
        self.allow_v4 = false;
        self.allow_v6 = true;
        self
    }

    /// Disallow private IPs
    pub fn no_private(mut self) -> Self {
        self.allow_private = false;
        self
    }
}

impl FieldValidator<String> for IpValidator {
    fn validate(&self, value: &String) -> Result<(), String> {
        use std::net::IpAddr;

        let ip: IpAddr = value.parse().map_err(|_| "Invalid IP address format".to_string())?;

        match ip {
            IpAddr::V4(v4) => {
                if !self.allow_v4 {
                    return Err("IPv4 addresses are not allowed".to_string());
                }
                if !self.allow_private && v4.is_private() {
                    return Err("Private IP addresses are not allowed".to_string());
                }
            }
            IpAddr::V6(v6) => {
                if !self.allow_v6 {
                    return Err("IPv6 addresses are not allowed".to_string());
                }
                if !self.allow_private && v6.is_loopback() {
                    return Err("Loopback addresses are not allowed".to_string());
                }
            }
        }

        Ok(())
    }
}

impl FieldValidator<&str> for IpValidator {
    fn validate(&self, value: &&str) -> Result<(), String> {
        self.validate(&value.to_string())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for validation::validators.
    use super::*;

    /// Validates `RangeValidator::empty` behavior for the range validator min
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&15).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&10).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&5).is_err()` evaluates to true.
    #[test]
    fn test_range_validator_min() {
        let validator = RangeValidator::empty().min(10);

        assert!(validator.validate(&15).is_ok());
        assert!(validator.validate(&10).is_ok());
        assert!(validator.validate(&5).is_err());
    }

    /// Validates `RangeValidator::empty` behavior for the range validator max
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&50).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&100).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&150).is_err()` evaluates to true.
    #[test]
    fn test_range_validator_max() {
        let validator = RangeValidator::empty().max(100);

        assert!(validator.validate(&50).is_ok());
        assert!(validator.validate(&100).is_ok());
        assert!(validator.validate(&150).is_err());
    }

    /// Validates `RangeValidator::empty` behavior for the range validator
    /// between scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&50).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&10).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&100).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&5).is_err()` evaluates to true.
    /// - Ensures `validator.validate(&150).is_err()` evaluates to true.
    #[test]
    fn test_range_validator_between() {
        let validator = RangeValidator::empty().between(10, 100);

        assert!(validator.validate(&50).is_ok());
        assert!(validator.validate(&10).is_ok());
        assert!(validator.validate(&100).is_ok());
        assert!(validator.validate(&5).is_err());
        assert!(validator.validate(&150).is_err());
    }

    /// Validates `RangeValidator::new` behavior for the range validator new
    /// convenience scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&50).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&10).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&100).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&5).is_err()` evaluates to true.
    /// - Ensures `validator.validate(&150).is_err()` evaluates to true.
    #[test]
    fn test_range_validator_new_convenience() {
        let validator = RangeValidator::new(10, 100);

        assert!(validator.validate(&50).is_ok());
        assert!(validator.validate(&10).is_ok());
        assert!(validator.validate(&100).is_ok());
        assert!(validator.validate(&5).is_err());
        assert!(validator.validate(&150).is_err());
    }

    /// Validates `StringValidator::new` behavior for the string validator not
    /// empty scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"hello".to_string()).is_ok()` evaluates
    ///   to true.
    /// - Ensures `validator.validate(&"".to_string()).is_err()` evaluates to
    ///   true.
    /// - Ensures `validator.validate(&" ".to_string()).is_err()` evaluates to
    ///   true.
    #[test]
    fn test_string_validator_not_empty() {
        let validator = StringValidator::new().not_empty();

        assert!(validator.validate(&"hello".to_string()).is_ok());
        assert!(validator.validate(&"".to_string()).is_err());
        assert!(validator.validate(&"   ".to_string()).is_err()); // Whitespace
                                                                  // only
    }

    /// Validates `StringValidator::new` behavior for the string validator min
    /// length scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"hello".to_string()).is_ok()` evaluates
    ///   to true.
    /// - Ensures `validator.validate(&"hello world".to_string()).is_ok()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"hi".to_string()).is_err()` evaluates to
    ///   true.
    #[test]
    fn test_string_validator_min_length() {
        let validator = StringValidator::new().min_length(5);

        assert!(validator.validate(&"hello".to_string()).is_ok());
        assert!(validator.validate(&"hello world".to_string()).is_ok());
        assert!(validator.validate(&"hi".to_string()).is_err());
    }

    /// Validates `StringValidator::new` behavior for the string validator max
    /// length scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"hello".to_string()).is_ok()` evaluates
    ///   to true.
    /// - Ensures `validator.validate(&"hello world".to_string()).is_err()`
    ///   evaluates to true.
    #[test]
    fn test_string_validator_max_length() {
        let validator = StringValidator::new().max_length(10);

        assert!(validator.validate(&"hello".to_string()).is_ok());
        assert!(validator.validate(&"hello world".to_string()).is_err());
    }

    /// Validates `StringValidator::new` behavior for the string validator min
    /// and max length scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"hello".to_string()).is_ok()` evaluates
    ///   to true.
    /// - Ensures `validator.validate(&"hi".to_string()).is_err()` evaluates to
    ///   true.
    /// - Ensures `validator.validate(&"hello world".to_string()).is_err()`
    ///   evaluates to true.
    #[test]
    fn test_string_validator_min_and_max_length() {
        let validator = StringValidator::new().min_length(3).max_length(10);

        assert!(validator.validate(&"hello".to_string()).is_ok());
        assert!(validator.validate(&"hi".to_string()).is_err());
        assert!(validator.validate(&"hello world".to_string()).is_err());
    }

    /// Validates `StringValidator::new` behavior for the string validator
    /// pattern scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"hello".to_string()).is_ok()` evaluates
    ///   to true.
    /// - Ensures `validator.validate(&"Hello".to_string()).is_err()` evaluates
    ///   to true.
    /// - Ensures `validator.validate(&"hello123".to_string()).is_err()`
    ///   evaluates to true.
    #[test]
    fn test_string_validator_pattern() {
        let validator = StringValidator::new().pattern(r"^[a-z]+$").expect("Valid regex");

        assert!(validator.validate(&"hello".to_string()).is_ok());
        assert!(validator.validate(&"Hello".to_string()).is_err()); // Capital letter
        assert!(validator.validate(&"hello123".to_string()).is_err()); // Numbers
    }

    /// Validates the email validator scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"user@example.com".to_string()).is_ok()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"user.name+tag@example.co.uk".
    ///   to_string()).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&"invalid-email".to_string()).is_err()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"@example.com".to_string()).is_err()`
    ///   evaluates to true.
    #[test]
    fn test_email_validator() {
        let validator = EmailValidator;

        assert!(validator.validate(&"user@example.com".to_string()).is_ok());
        assert!(validator.validate(&"user.name+tag@example.co.uk".to_string()).is_ok());
        assert!(validator.validate(&"invalid-email".to_string()).is_err());
        assert!(validator.validate(&"@example.com".to_string()).is_err());
    }

    /// Validates `StringValidator::new` behavior for the string validator
    /// pattern alphanumeric scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"hello123".to_string()).is_ok()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"HELLO123".to_string()).is_ok()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"hello-world".to_string()).is_err()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"hello world".to_string()).is_err()`
    ///   evaluates to true.
    #[test]
    fn test_string_validator_pattern_alphanumeric() {
        let validator = StringValidator::new().pattern(r"^[a-zA-Z0-9]+$").expect("Valid regex");

        assert!(validator.validate(&"hello123".to_string()).is_ok());
        assert!(validator.validate(&"HELLO123".to_string()).is_ok());
        assert!(validator.validate(&"hello-world".to_string()).is_err()); // Hyphen
        assert!(validator.validate(&"hello world".to_string()).is_err()); // Space
    }

    /// Validates `CollectionValidator::new` behavior for the collection
    /// validator min size scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&valid_list).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&invalid_list).is_err()` evaluates to
    ///   true.
    #[test]
    fn test_collection_validator_min_size() {
        let validator = CollectionValidator::<String>::new().min_size(2);

        let valid_list = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let invalid_list = vec!["a".to_string()];

        assert!(validator.validate(&valid_list).is_ok());
        assert!(validator.validate(&invalid_list).is_err());
    }

    /// Validates `CollectionValidator::new` behavior for the collection
    /// validator max size scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&valid_list).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&invalid_list).is_err()` evaluates to
    ///   true.
    #[test]
    fn test_collection_validator_max_size() {
        let validator = CollectionValidator::<String>::new().max_size(3);

        let valid_list = vec!["a".to_string(), "b".to_string()];
        let invalid_list = vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()];

        assert!(validator.validate(&valid_list).is_ok());
        assert!(validator.validate(&invalid_list).is_err());
    }

    /// Validates `CollectionValidator::new` behavior for the collection
    /// validator unique items scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&valid_list).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&invalid_list).is_err()` evaluates to
    ///   true.
    #[test]
    fn test_collection_validator_unique_items() {
        let validator = CollectionValidator::<String>::new().unique_items();

        let valid_list = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let invalid_list = vec!["a".to_string(), "b".to_string(), "a".to_string()];

        assert!(validator.validate(&valid_list).is_ok());
        assert!(validator.validate(&invalid_list).is_err());
    }

    /// Validates `CollectionValidator::new` behavior for the collection
    /// validator not empty scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&valid_list).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&invalid_list).is_err()` evaluates to
    ///   true.
    #[test]
    fn test_collection_validator_not_empty() {
        let validator = CollectionValidator::<String>::new().min_size(1);

        let valid_list = vec!["a".to_string()];
        let invalid_list: Vec<String> = vec![];

        assert!(validator.validate(&valid_list).is_ok());
        assert!(validator.validate(&invalid_list).is_err());
    }

    /// Validates `UrlValidator::new` behavior for the url validator basic
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"https://example.com".to_string()).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&"http://example.com".to_string()).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&"not-a-url".to_string()).is_err()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"ftp://ftp.example.com".to_string()).is_err()` evaluates to true.
    #[test]
    fn test_url_validator_basic() {
        let validator = UrlValidator::new();

        assert!(validator.validate(&"https://example.com".to_string()).is_ok());
        assert!(validator.validate(&"http://example.com".to_string()).is_ok());
        assert!(validator.validate(&"not-a-url".to_string()).is_err());

        // FTP is not in default allowed schemes (only http and https)
        assert!(validator.validate(&"ftp://ftp.example.com".to_string()).is_err());
    }

    /// Validates `UrlValidator::new` behavior for the url validator require
    /// https scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"https://example.com".to_string()).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&"http://example.com".to_string()).is_err()` evaluates to true.
    /// - Ensures `validator.validate(&"ftp://example.com".to_string()).is_err()` evaluates to true.
    #[test]
    fn test_url_validator_require_https() {
        let validator = UrlValidator::new().require_https();

        assert!(validator.validate(&"https://example.com".to_string()).is_ok());
        assert!(validator.validate(&"http://example.com".to_string()).is_err());
        assert!(validator.validate(&"ftp://example.com".to_string()).is_err());
    }

    /// Validates `UrlValidator::new` behavior for the url validator allowed
    /// schemes scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"https://example.com".to_string()).is_ok()` evaluates to true.
    /// - Ensures `validator.validate(&"wss://example.com".to_string()).is_ok()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"http://example.com".to_string()).is_err()` evaluates to true.
    /// - Ensures `validator.validate(&"ftp://example.com".to_string()).is_err()` evaluates to true.
    #[test]
    fn test_url_validator_allowed_schemes() {
        let validator =
            UrlValidator::new().allowed_schemes(vec!["https".to_string(), "wss".to_string()]);

        assert!(validator.validate(&"https://example.com".to_string()).is_ok());
        assert!(validator.validate(&"wss://example.com".to_string()).is_ok());
        assert!(validator.validate(&"http://example.com".to_string()).is_err());
        assert!(validator.validate(&"ftp://example.com".to_string()).is_err());
    }

    /// Validates `IpValidator::new` behavior for the ip validator basic
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"192.168.1.1".to_string()).is_ok()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"10.0.0.1".to_string()).is_ok()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"8.8.8.8".to_string()).is_ok()` evaluates
    ///   to true.
    /// - Ensures `validator.validate(&"2001:db8::1".to_string()).is_ok()`
    ///   evaluates to true.
    #[test]
    fn test_ip_validator_basic() {
        let validator = IpValidator::new();

        assert!(validator.validate(&"192.168.1.1".to_string()).is_ok());
        assert!(validator.validate(&"10.0.0.1".to_string()).is_ok());
        assert!(validator.validate(&"8.8.8.8".to_string()).is_ok());
        assert!(validator.validate(&"2001:db8::1".to_string()).is_ok());
    }

    /// Validates `IpValidator::new` behavior for the ip validator v4 only
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"192.168.1.1".to_string()).is_ok()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"8.8.8.8".to_string()).is_ok()` evaluates
    ///   to true.
    /// - Ensures `validator.validate(&"2001:db8::1".to_string()).is_err()`
    ///   evaluates to true.
    #[test]
    fn test_ip_validator_v4_only() {
        let validator = IpValidator::new().v4_only();

        assert!(validator.validate(&"192.168.1.1".to_string()).is_ok());
        assert!(validator.validate(&"8.8.8.8".to_string()).is_ok());
        assert!(validator.validate(&"2001:db8::1".to_string()).is_err());
    }

    /// Validates `IpValidator::new` behavior for the ip validator v6 only
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"2001:db8::1".to_string()).is_ok()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"::1".to_string()).is_ok()` evaluates to
    ///   true.
    /// - Ensures `validator.validate(&"192.168.1.1".to_string()).is_err()`
    ///   evaluates to true.
    #[test]
    fn test_ip_validator_v6_only() {
        let validator = IpValidator::new().v6_only();

        assert!(validator.validate(&"2001:db8::1".to_string()).is_ok());
        assert!(validator.validate(&"::1".to_string()).is_ok());
        assert!(validator.validate(&"192.168.1.1".to_string()).is_err());
    }

    /// Validates `IpValidator::new` behavior for the ip validator no private
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"8.8.8.8".to_string()).is_ok()` evaluates
    ///   to true.
    /// - Ensures `validator.validate(&"1.1.1.1".to_string()).is_ok()` evaluates
    ///   to true.
    /// - Ensures `validator.validate(&"192.168.1.1".to_string()).is_err()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"10.0.0.1".to_string()).is_err()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"172.16.0.1".to_string()).is_err()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"::1".to_string()).is_err()` evaluates to
    ///   true.
    #[test]
    fn test_ip_validator_no_private() {
        let validator = IpValidator::new().no_private();

        // Public IPs should pass
        assert!(validator.validate(&"8.8.8.8".to_string()).is_ok());
        assert!(validator.validate(&"1.1.1.1".to_string()).is_ok());

        // Private IPv4 should fail
        assert!(validator.validate(&"192.168.1.1".to_string()).is_err());
        assert!(validator.validate(&"10.0.0.1".to_string()).is_err());
        assert!(validator.validate(&"172.16.0.1".to_string()).is_err());

        // Loopback IPv6 should fail
        assert!(validator.validate(&"::1".to_string()).is_err());
    }

    /// Validates `IpValidator::new` behavior for the ip validator invalid
    /// format scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"not-an-ip".to_string()).is_err()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"999.999.999.999".to_string()).is_err()`
    ///   evaluates to true.
    /// - Ensures `validator.validate(&"256.1.1.1".to_string()).is_err()`
    ///   evaluates to true.
    #[test]
    fn test_ip_validator_invalid_format() {
        let validator = IpValidator::new();

        assert!(validator.validate(&"not-an-ip".to_string()).is_err());
        assert!(validator.validate(&"999.999.999.999".to_string()).is_err());
        assert!(validator.validate(&"256.1.1.1".to_string()).is_err());
    }
}
