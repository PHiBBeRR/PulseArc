//! Generic pattern extractor for text extraction and transformation
//!
//! This module provides a reusable extraction utility that follows the builder
//! pattern for configurable text processing. It eliminates duplication in
//! pattern extraction logic across the codebase.
//!
//! # Example
//!
//! ```rust
//! use pulsearc_domain::utils::pattern_extractor::PatternExtractor;
//!
//! let extractor = PatternExtractor::builder()
//!     .delimiter(" - ")
//!     .max_length(50)
//!     .filter(|s| s.starts_with('#'))
//!     .build();
//!
//! let result = extractor.extract("#channel - Server Name");
//! assert_eq!(result, Some("#channel".to_string()));
//! ```

use std::sync::Arc;

use crate::utils::title::{extract_by_delimiter, truncate_title};

type FilterFn = dyn Fn(&str) -> bool + Send + Sync;
type TransformFn = dyn Fn(&str) -> String + Send + Sync;

/// Generic pattern extractor with configurable extraction logic
///
/// Supports common extraction patterns including:
/// - Delimiter-based extraction
/// - Filtering by predicate
/// - Length truncation
/// - Custom transformations
#[derive(Clone)]
pub struct PatternExtractor {
    delimiter: Option<String>,
    index: usize,
    max_length: Option<usize>,
    filter_fn: Option<Arc<FilterFn>>,
    transform_fn: Option<Arc<TransformFn>>,
}

impl PatternExtractor {
    /// Creates a new builder for constructing a PatternExtractor
    pub fn builder() -> PatternExtractorBuilder {
        PatternExtractorBuilder::default()
    }

    /// Extract a string from the input using the configured extraction logic
    ///
    /// # Arguments
    ///
    /// * `input` - The input string to extract from
    ///
    /// # Returns
    ///
    /// * `Some(String)` - The extracted and optionally transformed string
    /// * `None` - If extraction failed or filter rejected the result
    pub fn extract(&self, input: &str) -> Option<String> {
        let delimiter = self.delimiter.as_ref()?;
        let candidate = self.extract_part(input, delimiter)?;
        let transformed = self.apply_transform(candidate);
        Some(self.apply_length(&transformed))
    }

    fn extract_part(&self, input: &str, delimiter: &str) -> Option<String> {
        let mut value = extract_by_delimiter(input, delimiter, self.index);

        if let Some(filter) = &self.filter_fn {
            value = value.filter(|item| filter(item));
        }

        value
    }

    fn apply_transform(&self, value: String) -> String {
        match &self.transform_fn {
            Some(transform) => transform(value.as_str()),
            None => value,
        }
    }

    fn apply_length(&self, value: &str) -> String {
        match self.max_length {
            Some(max_len) if value.len() > max_len => value[..max_len].to_string(),
            Some(_) => value.to_string(),
            None => truncate_title(value),
        }
    }
}

/// Builder for constructing a PatternExtractor
#[derive(Default)]
pub struct PatternExtractorBuilder {
    delimiter: Option<String>,
    index: usize,
    max_length: Option<usize>,
    filter_fn: Option<Arc<FilterFn>>,
    transform_fn: Option<Arc<TransformFn>>,
}

impl PatternExtractorBuilder {
    /// Set the delimiter to split on
    ///
    /// # Arguments
    ///
    /// * `delim` - The delimiter string (e.g., " - ", " | ", ":")
    pub fn delimiter(mut self, delim: impl Into<String>) -> Self {
        self.delimiter = Some(delim.into());
        self
    }

    /// Set the index of the part to extract (default: 0)
    ///
    /// # Arguments
    ///
    /// * `idx` - The zero-based index of the part to extract
    pub fn index(mut self, idx: usize) -> Self {
        self.index = idx;
        self
    }

    /// Set the maximum length for truncation
    ///
    /// # Arguments
    ///
    /// * `len` - Maximum length of the extracted string
    pub fn max_length(mut self, len: usize) -> Self {
        self.max_length = Some(len);
        self
    }

    /// Set a filter function to validate the extracted string
    ///
    /// # Arguments
    ///
    /// * `f` - A function that returns true if the string should be kept
    pub fn filter<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.filter_fn = Some(Arc::new(f));
        self
    }

    /// Set a transform function to modify the extracted string
    ///
    /// # Arguments
    ///
    /// * `f` - A function that transforms the extracted string
    pub fn transform<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.transform_fn = Some(Arc::new(f));
        self
    }

    /// Build the PatternExtractor
    pub fn build(self) -> PatternExtractor {
        PatternExtractor {
            delimiter: self.delimiter,
            index: self.index,
            max_length: self.max_length,
            filter_fn: self.filter_fn,
            transform_fn: self.transform_fn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_delimiter_extraction() {
        let extractor = PatternExtractor::builder().delimiter(" - ").build();

        assert_eq!(extractor.extract("Part1 - Part2"), Some("Part1".to_string()));
    }

    #[test]
    fn test_delimiter_with_index() {
        let extractor = PatternExtractor::builder().delimiter(" - ").index(1).build();

        assert_eq!(extractor.extract("Part1 - Part2 - Part3"), Some("Part2".to_string()));
    }

    #[test]
    fn test_delimiter_with_truncation() {
        let extractor = PatternExtractor::builder().delimiter(" - ").max_length(5).build();

        assert_eq!(extractor.extract("LongString - Part2"), Some("LongS".to_string()));
    }

    #[test]
    fn test_delimiter_with_filter() {
        let extractor =
            PatternExtractor::builder().delimiter(" - ").filter(|s| s.starts_with('#')).build();

        assert_eq!(extractor.extract("#channel - Server"), Some("#channel".to_string()));

        assert_eq!(extractor.extract("nochannel - Server"), None);
    }

    #[test]
    fn test_no_delimiter_returns_none() {
        let extractor = PatternExtractor::builder().build();

        assert_eq!(extractor.extract("test string"), None);
    }

    #[test]
    fn test_delimiter_not_found() {
        let extractor = PatternExtractor::builder().delimiter(" | ").build();

        // When delimiter is not found, split returns the whole string at position 0
        // So this will return the whole string, not None
        assert_eq!(extractor.extract("no delimiter here"), Some("no delimiter here".to_string()));
    }

    #[test]
    fn test_filter_rejection() {
        let extractor =
            PatternExtractor::builder().delimiter(" - ").filter(|s| s.len() > 10).build();

        assert_eq!(extractor.extract("short - text"), None);
    }

    #[test]
    fn test_combined_filter_and_truncation() {
        let extractor = PatternExtractor::builder()
            .delimiter(" | ")
            .max_length(10)
            .filter(|s| s.starts_with('#'))
            .build();

        assert_eq!(
            extractor.extract("#very-long-channel-name | Workspace"),
            Some("#very-long".to_string())
        );
    }

    #[test]
    fn test_transform_basic() {
        let extractor = PatternExtractor::builder()
            .delimiter("Pull Request #")
            .index(1)
            .transform(|s| {
                // Extract just the number from the string (first word)
                let num = s.split_whitespace().next().unwrap_or(s);
                format!("Reviewing PR #{num}")
            })
            .build();

        assert_eq!(
            extractor.extract("Fix bug - Pull Request #123 - user/repo"),
            Some("Reviewing PR #123".to_string())
        );
    }

    #[test]
    fn test_transform_with_truncation() {
        let extractor = PatternExtractor::builder()
            .delimiter(" - ")
            .transform(str::to_uppercase)
            .max_length(5)
            .build();

        assert_eq!(extractor.extract("hello - world"), Some("HELLO".to_string()));
    }

    #[test]
    fn test_transform_with_filter() {
        let extractor = PatternExtractor::builder()
            .delimiter(":")
            .filter(|s| s.contains('/'))
            .transform(|s| s.trim().to_owned())
            .build();

        assert_eq!(extractor.extract("user/repo: Pull Request"), Some("user/repo".to_string()));

        // Should return None when filter rejects
        assert_eq!(extractor.extract("noslash: Pull Request"), None);
    }
}
