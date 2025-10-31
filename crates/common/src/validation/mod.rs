// Validation Module - Enterprise-grade validation framework
use std::collections::HashMap;
use std::fmt;

mod rules;
mod validators;

pub use rules::{NamedRule, RuleBuilder, RuleSet, ValidationRule};
pub use validators::{
    CollectionValidator, CustomValidator, EmailValidator, FieldValidator, IpValidator,
    RangeValidator, StringValidator, UrlValidator,
};

/// Type alias for validation results
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Type alias for custom validation rules map (clippy::type_complexity)
type CustomRulesMap = HashMap<String, Box<dyn ValidationRule>>;

/// Validation error with detailed field-level errors
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub errors: Vec<FieldError>,
    pub context: Option<ValidationContext>,
}

impl Default for ValidationError {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationError {
    /// Create a new validation error
    pub fn new() -> Self {
        Self { errors: Vec::new(), context: None }
    }

    /// Create with a single field error
    pub fn field(field: impl Into<String>, message: impl Into<String>) -> Self {
        let mut err = Self::new();
        err.add_field_error(field, message);
        err
    }

    /// Add a field-level error
    pub fn add_field_error(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.errors.push(FieldError {
            field: field.into(),
            message: message.into(),
            code: None,
            metadata: HashMap::new(),
        });
    }

    /// Add a field error with code
    pub fn add_error_with_code(
        &mut self,
        field: impl Into<String>,
        message: impl Into<String>,
        code: impl Into<String>,
    ) {
        self.errors.push(FieldError {
            field: field.into(),
            message: message.into(),
            code: Some(code.into()),
            metadata: HashMap::new(),
        });
    }

    /// Set validation context
    pub fn with_context(mut self, context: ValidationContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Check if there are any errors
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Get errors for a specific field
    pub fn field_errors(&self, field: &str) -> Vec<&FieldError> {
        self.errors.iter().filter(|e| e.field == field).collect()
    }

    /// Convert to a result
    pub fn to_result<T>(self) -> ValidationResult<T> {
        if self.is_empty() {
            // Return a validation error indicating this is a misuse of the API
            return Err(ValidationError::field(
                "_internal",
                "Cannot convert empty ValidationError to Result - no validation errors present",
            ));
        }
        Err(self)
    }

    /// Merge another validation error into this one
    pub fn merge(&mut self, other: ValidationError) {
        self.errors.extend(other.errors);
        if self.context.is_none() {
            self.context = other.context;
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.errors.is_empty() {
            write!(f, "Validation error with no specific field errors")?;
        } else if self.errors.len() == 1 {
            write!(f, "Validation failed: {}", self.errors[0].message)?;
        } else {
            write!(f, "Validation failed with {} errors: ", self.errors.len())?;
            for (i, error) in self.errors.iter().enumerate() {
                if i > 0 {
                    write!(f, "; ")?;
                }
                write!(f, "{}: {}", error.field, error.message)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

/// Individual field error
#[derive(Debug, Clone)]
pub struct FieldError {
    pub field: String,
    pub message: String,
    pub code: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl FieldError {
    /// Create a new field error
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self { field: field.into(), message: message.into(), code: None, metadata: HashMap::new() }
    }

    /// Add metadata to the error
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set error code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
}

/// Validation context for tracking validation state
#[derive(Debug, Clone, Default)]
pub struct ValidationContext {
    pub path: Vec<String>,
    pub strict_mode: bool,
    pub stop_on_first: bool,
    pub custom_rules: CustomRulesMap,
}

impl ValidationContext {
    /// Create a new validation context
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable strict mode
    pub fn strict(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    /// Stop on first error
    pub fn stop_on_first_error(mut self) -> Self {
        self.stop_on_first = true;
        self
    }

    /// Add path segment for nested validation
    pub fn push_path(&mut self, segment: impl Into<String>) {
        self.path.push(segment.into());
    }

    /// Remove last path segment
    pub fn pop_path(&mut self) {
        self.path.pop();
    }

    /// Get current path as string
    pub fn current_path(&self) -> String {
        self.path.join(".")
    }
}

/// Main validator struct for orchestrating validations
pub struct Validator {
    errors: ValidationError,
    context: ValidationContext,
    stopped: bool,
}

impl Validator {
    /// Create a new validator
    pub fn new() -> Self {
        Self { errors: ValidationError::new(), context: ValidationContext::new(), stopped: false }
    }

    /// Create with context
    pub fn with_context(context: ValidationContext) -> Self {
        Self { errors: ValidationError::new(), context, stopped: false }
    }

    fn should_short_circuit(&self) -> bool {
        self.context.stop_on_first && self.stopped
    }

    /// Add an error
    pub fn add_error(&mut self, field: impl Into<String>, message: impl Into<String>) {
        let field = if self.context.path.is_empty() {
            field.into()
        } else {
            format!("{}.{}", self.context.current_path(), field.into())
        };
        self.errors.add_field_error(field, message);

        if self.context.stop_on_first && !self.errors.is_empty() {
            self.stopped = true;
        }
    }

    /// Validate a field with a specific validator
    pub fn validate_field<T, V>(
        &mut self,
        field: &str,
        value: &T,
        validator: &V,
    ) -> ValidationResult<()>
    where
        V: FieldValidator<T> + ?Sized,
    {
        if self.should_short_circuit() {
            return Ok(());
        }

        if let Err(msg) = validator.validate(value) {
            self.add_error(field, msg);
        }
        Ok(())
    }

    /// Validate with a custom rule
    ///
    /// The type T must be 'static to ensure it can be safely cast to Any
    pub fn validate_with_rule<T>(
        &mut self,
        value: &T,
        rule: &dyn ValidationRule,
    ) -> ValidationResult<()>
    where
        T: std::any::Any + 'static,
    {
        if self.should_short_circuit() {
            return Ok(());
        }

        let result = rule.validate(value as &dyn std::any::Any, &mut self.errors, &self.context);
        if result.is_err() && self.context.stop_on_first {
            self.stopped = true;
        }
        result
    }

    /// Validate a numeric range
    pub fn validate_range<T>(
        &mut self,
        field: &str,
        value: T,
        min: T,
        max: T,
    ) -> ValidationResult<()>
    where
        T: PartialOrd + fmt::Display,
    {
        if self.should_short_circuit() {
            return Ok(());
        }

        if value < min || value > max {
            self.add_error(field, format!("must be between {} and {}", min, max));
        }
        Ok(())
    }

    /// Validate minimum value
    pub fn validate_min<T>(&mut self, field: &str, value: T, min: T) -> ValidationResult<()>
    where
        T: PartialOrd + fmt::Display,
    {
        if self.should_short_circuit() {
            return Ok(());
        }

        if value < min {
            self.add_error(field, format!("must be at least {}", min));
        }
        Ok(())
    }

    /// Validate maximum value
    pub fn validate_max<T>(&mut self, field: &str, value: T, max: T) -> ValidationResult<()>
    where
        T: PartialOrd + fmt::Display,
    {
        if self.should_short_circuit() {
            return Ok(());
        }

        if value > max {
            self.add_error(field, format!("must not exceed {}", max));
        }
        Ok(())
    }

    /// Validate string is not empty
    pub fn validate_not_empty(&mut self, field: &str, value: &str) -> ValidationResult<()> {
        if self.should_short_circuit() {
            return Ok(());
        }

        if value.trim().is_empty() {
            self.add_error(field, "cannot be empty");
        }
        Ok(())
    }

    /// Validate string matches pattern
    pub fn validate_pattern(
        &mut self,
        field: &str,
        value: &str,
        pattern: &str,
    ) -> ValidationResult<()> {
        if self.should_short_circuit() {
            return Ok(());
        }

        if let Ok(re) = regex::Regex::new(pattern) {
            if !re.is_match(value) {
                self.add_error(field, format!("must match pattern: {}", pattern));
            }
        } else {
            self.add_error(field, "invalid regex pattern");
        }
        Ok(())
    }

    /// Validate collection size
    pub fn validate_collection_size<T>(
        &mut self,
        field: &str,
        collection: &[T],
        min: Option<usize>,
        max: Option<usize>,
    ) -> ValidationResult<()> {
        if self.should_short_circuit() {
            return Ok(());
        }

        let size = collection.len();

        if let Some(min) = min {
            if size < min {
                self.add_error(field, format!("must contain at least {} items", min));
            }
        }

        if let Some(max) = max {
            if size > max {
                self.add_error(field, format!("must not contain more than {} items", max));
            }
        }

        Ok(())
    }

    /// Validate with nested context
    pub fn validate_nested<F>(&mut self, field: &str, f: F) -> ValidationResult<()>
    where
        F: FnOnce(&mut Validator),
    {
        if self.should_short_circuit() {
            return Ok(());
        }

        self.context.push_path(field);
        f(self);
        self.context.pop_path();
        Ok(())
    }

    /// Check if validation has errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.errors.error_count()
    }

    /// Finalize and return result
    pub fn finalize(self) -> ValidationResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.with_context(self.context))
        }
    }

    /// Get errors without consuming validator
    pub fn errors(&self) -> &ValidationError {
        &self.errors
    }

    /// Clear all errors
    pub fn clear(&mut self) {
        self.errors = ValidationError::new();
        self.stopped = false;
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}
