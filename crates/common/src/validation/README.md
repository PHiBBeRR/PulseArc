# Validation

Enterprise-grade validation framework for configuration settings.

## Overview

This module provides a comprehensive validation system with field-level errors, custom validators, rule sets, and detailed error reporting.

## Components

### `Validator`

The main validation coordinator that collects and manages validation errors.

```rust
use agent::core::config::validation::{Validator, ValidationResult};

let mut validator = Validator::new();

// Validate individual fields
if value < 0 {
    validator.add_error("field_name", "Value must be positive");
}

// Check if validation passed
if validator.has_errors() {
    return Err(validator.into_error());
}
```

### `ValidationError`

Detailed error type with field-level granularity:

```rust
use agent::core::config::validation::ValidationError;

let error = ValidationError::field("port", "Port must be between 1-65535");

// Access individual errors
for field_error in &error.errors {
    println!("{}: {}", field_error.field, field_error.message);
}
```

### Field Validators

Pre-built validators for common use cases:

**StringValidator**:
```rust
use agent::core::config::validation::StringValidator;

let validator = StringValidator::new()
    .min_length(3)
    .max_length(50)
    .pattern(r"^[a-zA-Z0-9_]+$")?;

validator.validate("username")?;
```

**RangeValidator**:
```rust
use agent::core::config::validation::RangeValidator;

let validator = RangeValidator::new()
    .min(1)
    .max(100);

validator.validate(50)?; // OK
validator.validate(150)?; // Error
```

**CollectionValidator**:
```rust
use agent::core::config::validation::CollectionValidator;

let validator = CollectionValidator::new()
    .min_items(1)
    .max_items(10)
    .unique();

validator.validate(&vec![1, 2, 3])?;
```

### Validation Rules

Build complex validation rule sets:

```rust
use agent::core::config::validation::{ValidationRule, RuleSet, RuleBuilder};

let rules = RuleSet::new()
    .add_rule(RuleBuilder::new()
        .field("username")
        .required()
        .min_length(3)
        .max_length(50)
        .build())
    .add_rule(RuleBuilder::new()
        .field("port")
        .required()
        .range(1, 65535)
        .build());

rules.validate(&config)?;
```

### Custom Validators

Implement custom validation logic:

```rust
use agent::core::config::validation::CustomValidator;

let validator = CustomValidator::new(|value: &String| {
    if value.contains("forbidden") {
        Err(ValidationError::field("content", "Contains forbidden word"))
    } else {
        Ok(())
    }
});

validator.validate(&input)?;
```

## Error Handling

### Field-Level Errors

Each error includes:
- **Field name**: Which field failed validation
- **Message**: Human-readable error message
- **Code**: Optional error code for programmatic handling
- **Metadata**: Additional context as key-value pairs

### Context

Add validation context for better error messages:

```rust
let mut error = ValidationError::new();
error.add_context("file", "/path/to/config.toml");
error.add_context("line", 42);
```

## Best Practices

1. **Validate Early**: Validate configuration at load time
2. **Be Specific**: Provide clear, actionable error messages
3. **Use Types**: Leverage Rust's type system alongside validation
4. **Compose Rules**: Build complex validations from simple rules
5. **Document Requirements**: Make validation rules part of documentation

## Example: Complete Configuration Validation

```rust
use agent::core::config::validation::{Validator, ValidationResult};

pub struct Config {
    pub port: u16,
    pub hostname: String,
    pub workers: usize,
}

impl Config {
    pub fn validate(&self, validator: &mut Validator) -> ValidationResult<()> {
        // Port validation
        if self.port == 0 {
            validator.add_error("port", "Port cannot be 0");
        }

        // Hostname validation
        if self.hostname.is_empty() {
            validator.add_error("hostname", "Hostname is required");
        } else if self.hostname.len() > 255 {
            validator.add_error("hostname", "Hostname too long");
        }

        // Workers validation
        if self.workers == 0 {
            validator.add_error("workers", "Must have at least 1 worker");
        } else if self.workers > 1024 {
            validator.add_warning("workers", "Very high worker count");
        }

        Ok(())
    }
}
```

## Integration

The validation framework integrates with:
- Configuration loading
- API request validation
- Policy enforcement
- Compliance checking
- Runtime reconfiguration
