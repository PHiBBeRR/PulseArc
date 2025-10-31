// Validation Rules - Composable validation rules
use std::fmt::Debug;
use std::sync::Arc;

use super::{ValidationContext, ValidationError, ValidationResult};

/// Type alias for a custom validation function (clippy::type_complexity)
/// Wrapped in Arc for cheap cloning via reference counting
type ValidationFn = Arc<dyn Fn(&dyn std::any::Any) -> Result<(), String> + Send + Sync>;

/// Trait for validation rules
pub trait ValidationRule: Send + Sync + Debug {
    /// Validate a value against the rule
    fn validate(
        &self,
        value: &dyn std::any::Any,
        errors: &mut ValidationError,
        context: &ValidationContext,
    ) -> ValidationResult<()>;

    /// Get rule description
    fn description(&self) -> String;

    /// Get rule name (optional, returns None for unnamed rules)
    fn name(&self) -> Option<&str> {
        None
    }

    /// Clone the rule
    fn clone_box(&self) -> Box<dyn ValidationRule>;
}

impl Clone for Box<dyn ValidationRule> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Rule set for combining multiple rules
#[derive(Debug, Clone)]
pub struct RuleSet {
    rules: Vec<Box<dyn ValidationRule>>,
    operator: RuleOperator,
}

#[derive(Debug, Clone)]
pub enum RuleOperator {
    And,
    Or,
}

impl Default for RuleSet {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleSet {
    /// Create a new rule set with AND operator (default)
    pub fn new() -> Self {
        Self { rules: Vec::new(), operator: RuleOperator::And }
    }

    /// Create a new rule set with AND operator
    pub fn all() -> Self {
        Self { rules: Vec::new(), operator: RuleOperator::And }
    }

    /// Create a new rule set with OR operator
    pub fn any() -> Self {
        Self { rules: Vec::new(), operator: RuleOperator::Or }
    }

    /// Add a rule to the set (consuming version for builder pattern)
    pub fn add_rule(mut self, rule: Box<dyn ValidationRule>) -> Self {
        self.rules.push(rule);
        self
    }

    /// Add a rule to the set (mutable version)
    pub fn add(&mut self, rule: Box<dyn ValidationRule>) {
        self.rules.push(rule);
    }

    /// Get the number of rules in the set
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Check if the rule set is empty
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Validate against all rules
    pub fn validate<T: 'static>(
        &self,
        value: &T,
        errors: &mut ValidationError,
        context: &ValidationContext,
    ) -> ValidationResult<()> {
        match self.operator {
            RuleOperator::And => {
                for rule in &self.rules {
                    rule.validate(value as &dyn std::any::Any, errors, context)?;
                }
                Ok(())
            }
            RuleOperator::Or => {
                let mut temp_errors = ValidationError::new();
                let mut any_passed = false;

                for rule in &self.rules {
                    let mut rule_errors = ValidationError::new();
                    let result =
                        rule.validate(value as &dyn std::any::Any, &mut rule_errors, context);

                    if result.is_ok() && rule_errors.is_empty() {
                        any_passed = true;
                        break;
                    } else {
                        temp_errors.merge(rule_errors);
                        if let Err(err) = result {
                            temp_errors.merge(err);
                        }
                    }
                }

                if !any_passed {
                    errors.merge(temp_errors);
                }
                Ok(())
            }
        }
    }
}

impl ValidationRule for RuleSet {
    fn validate(
        &self,
        value: &dyn std::any::Any,
        errors: &mut ValidationError,
        context: &ValidationContext,
    ) -> ValidationResult<()> {
        match self.operator {
            RuleOperator::And => {
                for rule in &self.rules {
                    rule.validate(value, errors, context)?;
                }
                Ok(())
            }
            RuleOperator::Or => {
                let mut temp_errors = ValidationError::new();
                let mut any_passed = false;

                for rule in &self.rules {
                    let mut rule_errors = ValidationError::new();
                    let result = rule.validate(value, &mut rule_errors, context);

                    if result.is_ok() && rule_errors.is_empty() {
                        any_passed = true;
                        break;
                    } else {
                        temp_errors.merge(rule_errors);
                        if let Err(err) = result {
                            temp_errors.merge(err);
                        }
                    }
                }

                if !any_passed {
                    errors.merge(temp_errors);
                }
                Ok(())
            }
        }
    }

    fn description(&self) -> String {
        match self.operator {
            RuleOperator::And => format!("All of {} rules must pass", self.rules.len()),
            RuleOperator::Or => format!("Any of {} rules must pass", self.rules.len()),
        }
    }

    fn clone_box(&self) -> Box<dyn ValidationRule> {
        Box::new(self.clone())
    }
}

/// Named rule wrapper for metadata support
#[derive(Debug, Clone)]
pub struct NamedRule {
    name: String,
    description: Option<String>,
    inner: Box<dyn ValidationRule>,
}

impl NamedRule {
    /// Create a new named rule
    pub fn new(name: impl Into<String>, inner: Box<dyn ValidationRule>) -> Self {
        Self { name: name.into(), description: None, inner }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Get the name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the description
    pub fn description_text(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl ValidationRule for NamedRule {
    fn validate(
        &self,
        value: &dyn std::any::Any,
        errors: &mut ValidationError,
        context: &ValidationContext,
    ) -> ValidationResult<()> {
        self.inner.validate(value, errors, context)
    }

    fn description(&self) -> String {
        self.description.clone().unwrap_or_else(|| format!("Named rule: {}", self.name))
    }

    fn name(&self) -> Option<&str> {
        Some(&self.name)
    }

    fn clone_box(&self) -> Box<dyn ValidationRule> {
        Box::new(self.clone())
    }
}

/// Builder for creating validation rules
pub struct RuleBuilder {
    name: Option<String>,
    description: Option<String>,
    rules: Vec<Box<dyn ValidationRule>>,
}

impl Default for RuleBuilder {
    fn default() -> Self {
        Self::empty()
    }
}

impl RuleBuilder {
    /// Create a new rule builder with a name
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: Some(name.into()), description: None, rules: Vec::new() }
    }

    /// Create an empty rule builder (for building RuleSets)
    pub fn empty() -> Self {
        Self { name: None, description: None, rules: Vec::new() }
    }

    /// Set the description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a required field rule
    pub fn required(mut self, field: &str) -> Self {
        self.rules.push(Box::new(RequiredRule { field: field.to_string() }));
        self
    }

    /// Add a range rule
    pub fn range<T: 'static + PartialOrd + Clone + Debug + Send + Sync>(
        mut self,
        field: &str,
        min: T,
        max: T,
    ) -> Self {
        self.rules.push(Box::new(RangeRule { field: field.to_string(), min, max }));
        self
    }

    /// Add a pattern rule
    pub fn pattern(mut self, field: &str, pattern: &str) -> Self {
        self.rules
            .push(Box::new(PatternRule { field: field.to_string(), pattern: pattern.to_string() }));
        self
    }

    /// Add a custom rule
    pub fn custom<F>(mut self, validator: F) -> Self
    where
        F: Fn(&dyn std::any::Any) -> Result<(), String> + Send + Sync + 'static,
    {
        self.rules.push(Box::new(CustomRule { validator: Arc::new(validator) }));
        self
    }

    /// Build a named rule (for use with RuleSet)
    pub fn build(self) -> ValidationResult<Box<dyn ValidationRule>> {
        if let Some(name) = self.name {
            // Build a single named rule
            let rule_set = RuleSet { rules: self.rules, operator: RuleOperator::And };
            let mut named_rule = NamedRule::new(name, Box::new(rule_set));
            if let Some(desc) = self.description {
                named_rule = named_rule.with_description(desc);
            }
            Ok(Box::new(named_rule))
        } else {
            // Build a RuleSet
            Ok(Box::new(RuleSet { rules: self.rules, operator: RuleOperator::And }))
        }
    }

    /// Build a rule set directly
    pub fn build_set(self) -> RuleSet {
        RuleSet { rules: self.rules, operator: RuleOperator::And }
    }
}

/// Required field rule
#[derive(Debug, Clone)]
struct RequiredRule {
    field: String,
}

impl ValidationRule for RequiredRule {
    fn validate(
        &self,
        value: &dyn std::any::Any,
        errors: &mut ValidationError,
        _context: &ValidationContext,
    ) -> ValidationResult<()> {
        if let Some(s) = value.downcast_ref::<String>() {
            if s.is_empty() {
                errors.add_field_error(&self.field, format!("{} is required", self.field));
            }
        } else if let Some(opt) = value.downcast_ref::<Option<String>>() {
            if opt.is_none() {
                errors.add_field_error(&self.field, format!("{} is required", self.field));
            }
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("{} is required", self.field)
    }

    fn clone_box(&self) -> Box<dyn ValidationRule> {
        Box::new(self.clone())
    }
}

/// Range validation rule
#[derive(Debug, Clone)]
struct RangeRule<T: PartialOrd + Clone + Debug + Send + Sync> {
    field: String,
    min: T,
    max: T,
}

impl<T: 'static + PartialOrd + Clone + Debug + Send + Sync> ValidationRule for RangeRule<T> {
    fn validate(
        &self,
        value: &dyn std::any::Any,
        errors: &mut ValidationError,
        _context: &ValidationContext,
    ) -> ValidationResult<()> {
        if let Some(v) = value.downcast_ref::<T>() {
            if v < &self.min || v > &self.max {
                errors.add_field_error(
                    &self.field,
                    format!("{} must be between {:?} and {:?}", self.field, self.min, self.max),
                );
            }
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("{} must be between {:?} and {:?}", self.field, self.min, self.max)
    }

    fn clone_box(&self) -> Box<dyn ValidationRule> {
        Box::new(self.clone())
    }
}

/// Pattern validation rule
#[derive(Debug, Clone)]
struct PatternRule {
    field: String,
    pattern: String,
}

impl ValidationRule for PatternRule {
    fn validate(
        &self,
        value: &dyn std::any::Any,
        errors: &mut ValidationError,
        _context: &ValidationContext,
    ) -> ValidationResult<()> {
        if let Some(s) = value.downcast_ref::<String>() {
            if let Ok(re) = regex::Regex::new(&self.pattern) {
                if !re.is_match(s) {
                    errors.add_field_error(
                        &self.field,
                        format!("{} must match pattern: {}", self.field, self.pattern),
                    );
                }
            }
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("{} must match pattern: {}", self.field, self.pattern)
    }

    fn clone_box(&self) -> Box<dyn ValidationRule> {
        Box::new(self.clone())
    }
}

/// Custom validation rule
struct CustomRule {
    validator: ValidationFn,
}

impl std::fmt::Debug for CustomRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomRule").field("validator", &"<closure>").finish()
    }
}

impl ValidationRule for CustomRule {
    fn validate(
        &self,
        value: &dyn std::any::Any,
        errors: &mut ValidationError,
        _context: &ValidationContext,
    ) -> ValidationResult<()> {
        if let Err(msg) = (self.validator)(value) {
            errors.add_field_error("custom", msg);
        }
        Ok(())
    }

    fn description(&self) -> String {
        "Custom validation rule".to_string()
    }

    fn clone_box(&self) -> Box<dyn ValidationRule> {
        // Clone via Arc reference counting (cheap, no actual function cloning)
        Box::new(Self { validator: Arc::clone(&self.validator) })
    }
}
