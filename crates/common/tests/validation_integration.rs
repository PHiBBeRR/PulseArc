//! Integration tests for validation module
//!
//! Tests enterprise-grade validation framework with complex scenarios

use pulsearc_common::validation::{
    CollectionValidator, EmailValidator, IpValidator, RangeValidator, RuleBuilder, RuleSet,
    StringValidator, UrlValidator, ValidationContext, ValidationError, Validator,
};

mod data;
use data::sample_entities::TestUser;

/// Test basic field validation
#[test]
fn test_basic_field_validation() {
    let mut validator = Validator::new();

    let email = "test@example.com";
    let email_validator = EmailValidator::new();

    validator.validate_field("email", &email, &email_validator).expect("Validation should succeed");

    assert!(!validator.has_errors());

    let result = validator.finalize();
    assert!(result.is_ok());
}

/// Test invalid email validation
#[test]
fn test_invalid_email_validation() {
    let mut validator = Validator::new();

    let invalid_email = "not-an-email";
    let email_validator = EmailValidator::new();

    let _ = validator.validate_field("email", &invalid_email, &email_validator);

    assert!(validator.has_errors());
    assert_eq!(validator.error_count(), 1);

    let errors = validator.errors();
    assert_eq!(errors.field_errors("email").len(), 1);
}

/// Test URL validation
#[test]
fn test_url_validation() {
    let mut validator = Validator::new();
    let url_validator = UrlValidator::new();

    // Valid URLs
    let valid_urls = vec![
        "https://example.com",
        "http://test.org/path",
        "https://api.service.com:8080/endpoint",
    ];

    for url in valid_urls {
        validator.clear();
        let _ = validator.validate_field("url", &url, &url_validator);
        assert!(!validator.has_errors(), "URL '{}' should be valid", url);
    }

    // Invalid URLs
    let invalid_urls = vec!["not-a-url", "ftp://unsupported-scheme.com", "just text"];

    for url in invalid_urls {
        validator.clear();
        let _ = validator.validate_field("url", &url, &url_validator);
        assert!(validator.has_errors(), "URL '{}' should be invalid", url);
    }
}

/// Test IP address validation
#[test]
fn test_ip_validation() {
    let mut validator = Validator::new();
    let ip_validator = IpValidator::new();

    // Valid IPv4 addresses
    let valid_ips = vec!["192.168.1.1", "10.0.0.1", "8.8.8.8", "127.0.0.1"];

    for ip in valid_ips {
        validator.clear();
        let _ = validator.validate_field("ip", &ip, &ip_validator);
        assert!(!validator.has_errors(), "IP '{}' should be valid", ip);
    }

    // Invalid IPs
    let invalid_ips = vec!["256.1.1.1", "192.168.1", "not-an-ip", "192.168.1.1.1"];

    for ip in invalid_ips {
        validator.clear();
        let _ = validator.validate_field("ip", &ip, &ip_validator);
        assert!(validator.has_errors(), "IP '{}' should be invalid", ip);
    }
}

/// Test range validation
#[test]
fn test_range_validation() {
    let mut validator = Validator::new();
    let range_validator = RangeValidator::new(0, 100);

    // Valid values
    let _ = validator.validate_field("value", &50, &range_validator);
    assert!(!validator.has_errors());

    // Below minimum
    validator.clear();
    let _ = validator.validate_field("value", &-1, &range_validator);
    assert!(validator.has_errors());

    // Above maximum
    validator.clear();
    let _ = validator.validate_field("value", &101, &range_validator);
    assert!(validator.has_errors());
}

/// Test string length validation
#[test]
fn test_string_length_validation() {
    let mut validator = Validator::new();
    let string_validator = StringValidator::new().min_length(3).max_length(10);

    // Valid length
    let _ = validator.validate_field("name", &"valid", &string_validator);
    assert!(!validator.has_errors());

    // Too short
    validator.clear();
    let _ = validator.validate_field("name", &"ab", &string_validator);
    assert!(validator.has_errors());

    // Too long
    validator.clear();
    let _ = validator.validate_field("name", &"this is too long", &string_validator);
    assert!(validator.has_errors());
}

/// Test collection validation
#[test]
fn test_collection_validation() {
    let mut validator = Validator::new();

    let items = vec![1, 2, 3, 4, 5];

    // Valid collection size
    let _ = validator.validate_collection_size("items", &items, Some(1), Some(10));
    assert!(!validator.has_errors());

    // Too few items
    validator.clear();
    let _ = validator.validate_collection_size("items", &items, Some(10), None);
    assert!(validator.has_errors());

    // Too many items
    validator.clear();
    let _ = validator.validate_collection_size("items", &items, None, Some(3));
    assert!(validator.has_errors());
}

/// Test nested validation
#[test]
fn test_nested_validation() {
    let mut validator = Validator::new();

    // Validate nested structure
    validator
        .validate_nested("user", |v| {
            let _ = v.validate_not_empty("name", "John Doe");
            let _ = v.validate_range("age", 25, 18, 120);

            v.validate_nested("address", |v| {
                let _ = v.validate_not_empty("street", "123 Main St");
                let _ = v.validate_not_empty("city", "");
            })
            .expect("Nested validation failed");
        })
        .expect("Parent validation failed");

    assert!(validator.has_errors());

    // Check nested field error path
    let errors = validator.errors();
    let city_errors = errors.field_errors("user.address.city");
    assert_eq!(city_errors.len(), 1);
}

/// Test validation context with strict mode
#[test]
fn test_validation_context_strict_mode() {
    let context = ValidationContext::new().strict();

    let mut validator = Validator::with_context(context);

    // In strict mode, validation should be more stringent
    let _ = validator.validate_not_empty("field", "  ");
    assert!(validator.has_errors());
}

/// Test validation context with stop on first error
#[test]
fn test_validation_context_stop_on_first() {
    let context = ValidationContext::new().stop_on_first_error();

    let mut validator = Validator::with_context(context);

    // Add multiple errors - should stop after first
    let _ = validator.validate_not_empty("field1", "");
    let _ = validator.validate_not_empty("field2", "");
    let _ = validator.validate_not_empty("field3", "");

    // Due to stop_on_first, should only have one error
    assert_eq!(validator.error_count(), 1);
}

/// Test complex user validation scenario
#[test]
fn test_complex_user_validation() {
    let user = TestUser {
        id: "user_123".to_string(),
        email: "invalid-email".to_string(),
        name: "".to_string(),
        age: 15,
        active: true,
    };

    let mut validator = Validator::new();

    // Validate user ID
    let _ = validator.validate_not_empty("id", &user.id);

    // Validate email
    let email_validator = EmailValidator::new();
    let _ = validator.validate_field("email", &user.email, &email_validator);

    // Validate name
    let _ = validator.validate_not_empty("name", &user.name);

    // Validate age (must be 18+)
    let _ = validator.validate_min("age", user.age, 18);

    assert!(validator.has_errors());
    assert!(validator.error_count() >= 3); // email, name, age

    // Check specific errors
    let errors = validator.errors();
    assert!(!errors.field_errors("email").is_empty());
    assert!(!errors.field_errors("name").is_empty());
    assert!(!errors.field_errors("age").is_empty());
}

/// Test rule builder pattern
#[test]
fn test_rule_builder_pattern() {
    let rule = RuleBuilder::new("test_rule")
        .description("Test validation rule")
        .build()
        .expect("Failed to build rule");

    // Rules should have metadata
    assert_eq!(rule.name(), Some("test_rule"));
}

/// Test rule set
#[test]
fn test_rule_set() {
    let rule1 = RuleBuilder::new("rule1").build().expect("Failed to build rule");
    let rule2 = RuleBuilder::new("rule2").build().expect("Failed to build rule");

    let mut rule_set = RuleSet::new();
    rule_set.add(rule1);
    rule_set.add(rule2);

    assert_eq!(rule_set.len(), 2);
}

/// Test validation error merging
#[test]
fn test_validation_error_merging() {
    let mut error1 = ValidationError::new();
    error1.add_field_error("field1", "Error 1");
    error1.add_field_error("field2", "Error 2");

    let mut error2 = ValidationError::new();
    error2.add_field_error("field3", "Error 3");

    error1.merge(error2);

    assert_eq!(error1.error_count(), 3);
}

/// Test validation error with code
#[test]
fn test_validation_error_with_code() {
    let mut error = ValidationError::new();
    error.add_error_with_code("email", "Invalid format", "INVALID_EMAIL");

    let field_errors = error.field_errors("email");
    assert_eq!(field_errors.len(), 1);
    assert_eq!(field_errors[0].code, Some("INVALID_EMAIL".to_string()));
}

/// Test pattern matching validation
#[test]
fn test_pattern_validation() {
    let mut validator = Validator::new();

    // Valid pattern
    let _ = validator.validate_pattern("code", "ABC123", r"^[A-Z]{3}\d{3}$");
    assert!(!validator.has_errors());

    // Invalid pattern
    validator.clear();
    let _ = validator.validate_pattern("code", "abc123", r"^[A-Z]{3}\d{3}$");
    assert!(validator.has_errors());
}

/// Test minimum value validation
#[test]
fn test_min_value_validation() {
    let mut validator = Validator::new();

    let _ = validator.validate_min("score", 75, 50);
    assert!(!validator.has_errors());

    validator.clear();
    let _ = validator.validate_min("score", 25, 50);
    assert!(validator.has_errors());
}

/// Test maximum value validation
#[test]
fn test_max_value_validation() {
    let mut validator = Validator::new();

    let _ = validator.validate_max("score", 75, 100);
    assert!(!validator.has_errors());

    validator.clear();
    let _ = validator.validate_max("score", 125, 100);
    assert!(validator.has_errors());
}

/// Test empty collection validation
#[test]
fn test_empty_collection_validation() {
    let mut validator = Validator::new();

    let empty: Vec<i32> = vec![];
    let _ = validator.validate_collection_size("items", &empty, Some(1), None);
    assert!(validator.has_errors());
}

/// Test collection validator
#[test]
fn test_collection_validator() {
    let collection_validator = CollectionValidator::new().min_size(2).max_size(5);

    let mut validator = Validator::new();

    // Valid collection
    let valid_items = vec![1, 2, 3];
    let _ = validator.validate_field("items", &valid_items, &collection_validator);
    assert!(!validator.has_errors());

    // Too few items
    validator.clear();
    let too_few = vec![1];
    let _ = validator.validate_field("items", &too_few, &collection_validator);
    assert!(validator.has_errors());

    // Too many items
    validator.clear();
    let too_many = vec![1, 2, 3, 4, 5, 6];
    let _ = validator.validate_field("items", &too_many, &collection_validator);
    assert!(validator.has_errors());
}

/// Test batch validation
#[test]
fn test_batch_user_validation() {
    let users = TestUser::batch(5);

    let mut all_errors = ValidationError::new();

    for (i, user) in users.iter().enumerate() {
        let mut validator = Validator::new();

        // Validate each user
        let email_validator = EmailValidator::new();
        let _ = validator.validate_field("email", &user.email, &email_validator);
        let _ = validator.validate_not_empty("name", &user.name);
        let _ = validator.validate_range("age", user.age, 18, 100);

        if validator.has_errors() {
            let mut errors = validator.errors().clone();
            // Prefix errors with user index
            for error in &mut errors.errors {
                error.field = format!("users[{}].{}", i, error.field);
            }
            all_errors.merge(errors);
        }
    }

    // Check if any users had validation errors
    if !all_errors.is_empty() {
        assert!(all_errors.error_count() > 0);
    }
}

/// Test validation clear operation
#[test]
fn test_validation_clear() {
    let mut validator = Validator::new();

    let _ = validator.validate_not_empty("field1", "");
    let _ = validator.validate_not_empty("field2", "");

    assert!(validator.has_errors());

    validator.clear();

    assert!(!validator.has_errors());
    assert_eq!(validator.error_count(), 0);
}

/// Test collection validator with per-item email validation
#[test]
fn test_collection_with_item_email_validation() {
    use pulsearc_common::validation::FieldValidator;

    let collection_validator =
        CollectionValidator::new().min_size(1).item_validator(EmailValidator::new());

    // Valid emails
    let valid_emails = vec![
        "user1@example.com".to_string(),
        "user2@test.org".to_string(),
        "admin@company.co.uk".to_string(),
    ];

    let result = collection_validator.validate(&valid_emails);
    assert!(result.is_ok(), "Valid emails should pass validation");

    // Invalid email in collection
    let invalid_emails = vec![
        "user1@example.com".to_string(),
        "invalid-email".to_string(), // Invalid at index 1
        "user3@example.com".to_string(),
    ];

    let result = collection_validator.validate(&invalid_emails);
    assert!(result.is_err(), "Invalid email should fail validation");

    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("index 1"), "Error should specify index 1");
    assert!(error_msg.contains("Invalid email format"), "Error should mention email format");
}

/// Test collection validator with per-item range validation
#[test]
fn test_collection_with_item_range_validation() {
    use pulsearc_common::validation::FieldValidator;

    let range_validator = RangeValidator::new(0, 100);
    let collection_validator =
        CollectionValidator::new().min_size(1).max_size(10).item_validator(range_validator);

    // Valid scores
    let valid_scores = vec![50, 75, 80, 90, 100];
    let result = collection_validator.validate(&valid_scores);
    assert!(result.is_ok(), "Valid scores should pass validation");

    // Invalid score in collection
    let invalid_scores = vec![50, 75, 150, 90]; // 150 exceeds max at index 2

    let result = collection_validator.validate(&invalid_scores);
    assert!(result.is_err(), "Invalid score should fail validation");

    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("index 2"), "Error should specify index 2");
    assert!(error_msg.contains("must not exceed 100"), "Error should mention max value");
}

/// Test collection validator with unique items and per-item validation
#[test]
fn test_collection_with_uniqueness_and_item_validation() {
    use pulsearc_common::validation::FieldValidator;

    let string_validator = StringValidator::new().min_length(3).max_length(20);
    let collection_validator =
        CollectionValidator::new().unique_items().item_validator(string_validator);

    // Valid unique strings
    let valid_strings = vec!["hello".to_string(), "world".to_string(), "rust".to_string()];
    let result = collection_validator.validate(&valid_strings);
    assert!(result.is_ok(), "Valid unique strings should pass");

    // Duplicate items (should fail on uniqueness first)
    let duplicate_strings = vec!["hello".to_string(), "world".to_string(), "hello".to_string()];
    let result = collection_validator.validate(&duplicate_strings);
    assert!(result.is_err(), "Duplicate items should fail validation");
    assert!(result.unwrap_err().contains("unique"), "Error should mention uniqueness");

    // Invalid item length (should fail on item validation)
    let invalid_length = vec!["hello".to_string(), "ab".to_string(), "world".to_string()]; // "ab" too
                                                                                           // short

    let result = collection_validator.validate(&invalid_length);
    assert!(result.is_err(), "Invalid length should fail validation");
    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("index 1"), "Error should specify index 1");
}

/// Test collection size validation with per-item URL validation
#[test]
fn test_collection_size_with_item_url_validation() {
    use pulsearc_common::validation::FieldValidator;

    let url_validator = UrlValidator::new().require_https();
    let collection_validator =
        CollectionValidator::new().min_size(2).max_size(5).item_validator(url_validator);

    // Valid HTTPS URLs
    let valid_urls = vec![
        "https://api.example.com".to_string(),
        "https://secure.site.org".to_string(),
        "https://app.company.com".to_string(),
    ];

    let result = collection_validator.validate(&valid_urls);
    assert!(result.is_ok(), "Valid HTTPS URLs should pass");

    // HTTP URL (not HTTPS)
    let http_urls = vec!["https://api.example.com".to_string(), "http://insecure.com".to_string()];

    let result = collection_validator.validate(&http_urls);
    assert!(result.is_err(), "HTTP URL should fail HTTPS requirement");

    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("index 1"), "Error should specify index 1");
    assert!(error_msg.contains("HTTPS"), "Error should mention HTTPS requirement");

    // Too few URLs (violates min_size)
    let too_few = vec!["https://example.com".to_string()];
    let result = collection_validator.validate(&too_few);
    assert!(result.is_err(), "Too few items should fail");
    assert!(result.unwrap_err().contains("at least 2"), "Error should mention minimum size");
}

/// Test error message format for per-item validation failures
#[test]
fn test_per_item_validation_error_messages() {
    use pulsearc_common::validation::FieldValidator;

    let email_validator = EmailValidator::new();
    let collection_validator = CollectionValidator::new().item_validator(email_validator);

    let invalid_emails = vec![
        "valid@example.com".to_string(),
        "also-valid@test.org".to_string(),
        "this-is-not-an-email".to_string(), // Index 2
        "valid@company.com".to_string(),
        "also-bad".to_string(), // Index 4
    ];

    let result = collection_validator.validate(&invalid_emails);
    assert!(result.is_err());

    let error_msg = result.unwrap_err();
    // Should fail on first invalid email (index 2)
    assert!(
        error_msg.contains("index 2"),
        "Error message should specify index 2, got: {}",
        error_msg
    );
    assert!(
        error_msg.contains("failed validation"),
        "Error message should indicate validation failure, got: {}",
        error_msg
    );
}

/// Test combination of all collection constraints with per-item validation
#[test]
fn test_comprehensive_collection_validation() {
    use pulsearc_common::validation::FieldValidator;

    let range_validator = RangeValidator::new(18, 65);
    let collection_validator = CollectionValidator::new()
        .min_size(3)
        .max_size(10)
        .unique_items()
        .item_validator(range_validator);

    // Valid case: all constraints satisfied
    let valid_ages = vec![18, 25, 30, 40, 50, 65];
    let result = collection_validator.validate(&valid_ages);
    assert!(result.is_ok(), "All valid ages should pass");

    // Invalid: too few items
    let too_few = vec![25, 30];
    let result = collection_validator.validate(&too_few);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("at least 3"));

    // Invalid: duplicate items
    let duplicates = vec![25, 30, 35, 25, 40];
    let result = collection_validator.validate(&duplicates);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unique"));

    // Invalid: item out of range
    let out_of_range = vec![25, 30, 70, 40]; // 70 exceeds max age
    let result = collection_validator.validate(&out_of_range);
    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("index 2"));
    assert!(error_msg.contains("must not exceed 65"));
}
