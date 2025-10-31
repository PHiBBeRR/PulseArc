//! Test data builders with fluent API
//!
//! Provides builder patterns for constructing test data.

// Allow missing panics docs for test builders - methods are designed to be infallible
// in test contexts and any panics indicate test setup errors
#![allow(clippy::missing_panics_doc)]

use std::collections::HashMap;

/// Generic test builder for creating complex test objects
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::builders::TestBuilder;
///
/// let data =
///     TestBuilder::new().with("name", "Alice".to_string()).with("age", "30".to_string()).build();
/// ```
#[derive(Debug, Clone)]
pub struct TestBuilder<T> {
    data: HashMap<String, T>,
}

impl<T> TestBuilder<T> {
    /// Create a new test builder
    pub fn new() -> Self {
        Self { data: HashMap::new() }
    }

    /// Add a key-value pair to the builder
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: T) -> Self {
        self.data.insert(key.into(), value);
        self
    }

    /// Build the final HashMap
    pub fn build(self) -> HashMap<String, T> {
        self.data
    }

    /// Get a reference to the current data
    pub fn data(&self) -> &HashMap<String, T> {
        &self.data
    }
}

impl<T> Default for TestBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// String builder for constructing test strings
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::builders::StringBuilder;
///
/// let s = StringBuilder::new().append("Hello").append(" ").append("World").build();
/// assert_eq!(s, "Hello World");
/// ```
#[derive(Debug, Clone)]
pub struct StringBuilder {
    parts: Vec<String>,
}

impl StringBuilder {
    /// Create a new string builder
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }

    /// Append a string
    #[must_use]
    pub fn append(mut self, s: impl Into<String>) -> Self {
        self.parts.push(s.into());
        self
    }

    /// Append multiple strings
    #[must_use]
    pub fn append_all(mut self, strings: Vec<String>) -> Self {
        self.parts.extend(strings);
        self
    }

    /// Append with a separator
    #[must_use]
    pub fn append_with_sep(mut self, s: impl Into<String>, sep: &str) -> Self {
        if !self.parts.is_empty() {
            self.parts.push(sep.to_string());
        }
        self.parts.push(s.into());
        self
    }

    /// Build the final string
    pub fn build(self) -> String {
        self.parts.join("")
    }

    /// Build with a custom separator
    pub fn build_with_sep(self, sep: &str) -> String {
        self.parts.join(sep)
    }
}

impl Default for StringBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating test user objects
#[derive(Debug, Clone)]
pub struct UserBuilder {
    id: Option<String>,
    name: Option<String>,
    email: Option<String>,
    age: Option<u32>,
    active: bool,
}

impl UserBuilder {
    /// Create a new user builder with defaults
    pub fn new() -> Self {
        Self { id: None, name: None, email: None, age: None, active: true }
    }

    /// Set the user ID
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the user name
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the user email
    #[must_use]
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set the user age
    #[must_use]
    pub fn age(mut self, age: u32) -> Self {
        self.age = Some(age);
        self
    }

    /// Set whether the user is active
    #[must_use]
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Build a HashMap representation
    pub fn build(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(id) = self.id {
            map.insert("id".to_string(), id);
        }
        if let Some(name) = self.name {
            map.insert("name".to_string(), name);
        }
        if let Some(email) = self.email {
            map.insert("email".to_string(), email);
        }
        if let Some(age) = self.age {
            map.insert("age".to_string(), age.to_string());
        }
        map.insert("active".to_string(), self.active.to_string());
        map
    }
}

impl Default for UserBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for testing::builders.
    use super::*;

    /// Validates `TestBuilder::new` behavior for the test builder scenario.
    ///
    /// Assertions:
    /// - Confirms `data.get("key1")` equals `Some(&"value1")`.
    /// - Confirms `data.get("key2")` equals `Some(&"value2")`.
    #[test]
    fn test_test_builder() {
        let data = TestBuilder::new().with("key1", "value1").with("key2", "value2").build();

        assert_eq!(data.get("key1"), Some(&"value1"));
        assert_eq!(data.get("key2"), Some(&"value2"));
    }

    /// Validates `StringBuilder::new` behavior for the string builder scenario.
    ///
    /// Assertions:
    /// - Confirms `s` equals `"Hello World"`.
    #[test]
    fn test_string_builder() {
        let s = StringBuilder::new().append("Hello").append(" ").append("World").build();

        assert_eq!(s, "Hello World");
    }

    /// Validates `StringBuilder::new` behavior for the string builder with sep
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `s` equals `"Hello`.
    #[test]
    fn test_string_builder_with_sep() {
        let s = StringBuilder::new()
            .append_with_sep("Hello", ", ")
            .append_with_sep("World", ", ")
            .build();

        assert_eq!(s, "Hello, World");
    }

    /// Validates `UserBuilder::new` behavior for the user builder scenario.
    ///
    /// Assertions:
    /// - Confirms `user.get("id")` equals `Some(&"123".to_string())`.
    /// - Confirms `user.get("name")` equals `Some(&"Alice".to_string())`.
    /// - Confirms `user.get("email")` equals
    ///   `Some(&"alice@example.com".to_string())`.
    /// - Confirms `user.get("age")` equals `Some(&"30".to_string())`.
    #[test]
    fn test_user_builder() {
        let user =
            UserBuilder::new().id("123").name("Alice").email("alice@example.com").age(30).build();

        assert_eq!(user.get("id"), Some(&"123".to_string()));
        assert_eq!(user.get("name"), Some(&"Alice".to_string()));
        assert_eq!(user.get("email"), Some(&"alice@example.com".to_string()));
        assert_eq!(user.get("age"), Some(&"30".to_string()));
    }
}
