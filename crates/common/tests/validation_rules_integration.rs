//! Integration tests for advanced validation rule composition.
//!
//! These cases exercise branching rule sets, custom validators, and context
//! propagation to ensure configuration-style validation matches expected UX.

use pulsearc_common::validation::{
    RuleBuilder, RuleSet, ValidationContext, ValidationError, Validator,
};

/// Ensure `RuleSet::any` accepts values that satisfy at least one branch and
/// aggregates errors when none pass.
#[test]
fn ruleset_any_combines_branch_outcomes() {
    let mut any_rules = RuleSet::any();
    let uppercase_rule = RuleBuilder::new("uppercase_only")
        .pattern("identifier", r"^[A-Z]+$")
        .build()
        .expect("uppercase rule should build");
    any_rules.add(uppercase_rule);

    let numeric_rule = RuleBuilder::new("numeric_only")
        .pattern("identifier", r"^\d+$")
        .build()
        .expect("numeric rule should build");
    any_rules.add(numeric_rule);

    let mut validator = Validator::new();
    validator.validate_with_rule(&"SERVICE".to_string(), &any_rules).unwrap();
    assert!(!validator.has_errors(), "upper-case value should satisfy the first branch");

    validator.clear();
    validator.validate_with_rule(&"abc123".to_string(), &any_rules).unwrap();
    let errors = validator.errors();
    assert!(errors.error_count() >= 2, "when no branch passes we should surface each rule failure");
    assert!(!errors.field_errors("identifier").is_empty());
}

/// Validate that composite rule sets built via `RuleBuilder` enforce every
/// constraint in order.
#[test]
fn ruleset_builder_enforces_composite_constraints() {
    let username_rules = RuleBuilder::empty()
        .required("username")
        .pattern("username", r"^[a-z0-9_]{3,16}$")
        .build_set();

    let mut validator = Validator::new();
    validator.validate_with_rule(&"valid_handle".to_string(), &username_rules).unwrap();
    assert!(!validator.has_errors(), "lowercase handle should meet required + pattern constraints");

    validator.clear();
    validator.validate_with_rule(&"Invalid Handle".to_string(), &username_rules).unwrap();
    let errors = validator.errors();
    assert!(
        !errors.field_errors("username").is_empty(),
        "pattern rule should reject whitespace or uppercase characters"
    );
}

/// Exercise the custom rule pathway to ensure arbitrary policies can block
/// disallowed values.
#[test]
fn custom_rule_blocks_reserved_identifiers() {
    let blacklist = ["root", "admin"];
    let reserved_rule = RuleBuilder::empty()
        .custom(move |value| {
            if let Some(name) = value.downcast_ref::<String>() {
                if blacklist.iter().any(|term| name.eq_ignore_ascii_case(term)) {
                    return Err("value uses reserved identifier".to_string());
                }
            }
            Ok(())
        })
        .build_set();

    let mut validator = Validator::new();
    validator.validate_with_rule(&"operator".to_string(), &reserved_rule).unwrap();
    assert!(!validator.has_errors(), "non-reserved value should pass the custom rule");

    validator.clear();
    validator.validate_with_rule(&"Admin".to_string(), &reserved_rule).unwrap();
    let errors = validator.errors();
    assert_eq!(
        errors.field_errors("custom").len(),
        1,
        "custom rule should surface a single descriptive error"
    );
}

/// Confirm that validator finalization carries the contextual metadata forward
/// for downstream consumers.
#[test]
fn validator_finalize_attaches_context() {
    let context = ValidationContext::new().strict().stop_on_first_error();
    let mut validator = Validator::with_context(context);

    validator
        .validate_nested("profile", |profile| {
            let _ = profile.validate_not_empty("display_name", "");
            let _ = profile.validate_pattern("handle", "Bad Handle", r"^[a-z_]+$");
        })
        .unwrap();

    let err = validator.finalize().expect_err("expected validation failure");
    let ctx = err.context.as_ref().expect("context should be attached");
    assert!(ctx.strict_mode, "strict flag should survive finalization");
    assert!(ctx.stop_on_first, "stop_on_first flag should survive finalization");
    assert!(ctx.path.is_empty(), "context path should unwind after nested validation completes");

    let field_errors = err.field_errors("profile.display_name");
    assert_eq!(field_errors.len(), 1, "nested field should retain its fully-qualified path");
}

/// Guard against accidental use of `ValidationError::to_result` on empty error
/// collections.
#[test]
fn validation_error_to_result_rejects_empty_conversion() {
    let err = ValidationError::new()
        .to_result::<()>()
        .expect_err("empty validation error should not convert to Ok");

    let internal_errors = err.field_errors("_internal");
    assert_eq!(internal_errors.len(), 1);
    assert!(
        internal_errors[0].message.contains("Cannot convert empty ValidationError"),
        "helper should emit a descriptive diagnostic"
    );
}
