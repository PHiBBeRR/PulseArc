//! Pure string utility functions for title extraction and manipulation

use crate::constants::{MAX_PROJECT_NAME_LENGTH, MAX_TITLE_LENGTH, TITLE_TRUNCATE_SUFFIX};

/// Extract text by splitting on a delimiter and taking a specific part.
///
/// Splits the input string on the delimiter, trims whitespace, and returns
/// the part at the specified position (0-indexed).
///
/// # Arguments
///
/// * `title` - The input string to split
/// * `delimiter` - The delimiter to split on (e.g., " | ", " - ")
/// * `position` - Zero-based index of the part to extract
///
/// # Returns
///
/// * `Some(String)` - The extracted and trimmed part
/// * `None` - If the position is out of bounds or the result is empty
///
/// # Examples
///
/// ```
/// use pulsearc_domain::utils::title::extract_by_delimiter;
///
/// let title = "Part1 | Part2 | Part3";
/// assert_eq!(extract_by_delimiter(title, " | ", 0), Some("Part1".to_string()));
/// assert_eq!(extract_by_delimiter(title, " | ", 1), Some("Part2".to_string()));
/// assert_eq!(extract_by_delimiter(title, " | ", 99), None);
/// ```
#[must_use]
pub fn extract_by_delimiter(title: &str, delimiter: &str, position: usize) -> Option<String> {
    title
        .split(delimiter)
        .nth(position)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
}

/// Extract text by splitting and filtering with a predicate.
///
/// Similar to `extract_by_delimiter`, but additionally applies a filter
/// function to validate the extracted part.
///
/// # Arguments
///
/// * `title` - The input string to split
/// * `delimiter` - The delimiter to split on
/// * `position` - Zero-based index of the part to extract
/// * `filter` - Predicate function returning true if the part should be kept
///
/// # Returns
///
/// * `Some(String)` - The extracted part if it passes the filter
/// * `None` - If extraction failed or filter rejected the result
///
/// # Examples
///
/// ```
/// use pulsearc_domain::utils::title::extract_with_filter;
///
/// let title = "#channel | Workspace";
/// let result = extract_with_filter(title, " | ", 0, |s| s.starts_with('#'));
/// assert_eq!(result, Some("#channel".to_string()));
///
/// let result = extract_with_filter("channel | Workspace", " | ", 0, |s| s.starts_with('#'));
/// assert_eq!(result, None);
/// ```
pub fn extract_with_filter<F>(
    title: &str,
    delimiter: &str,
    position: usize,
    filter: F,
) -> Option<String>
where
    F: Fn(&str) -> bool,
{
    extract_by_delimiter(title, delimiter, position).filter(|s| filter(s))
}

/// Extract just the filename from editor window titles.
///
/// Handles common editor title formats by removing project names and paths,
/// supporting both em dash (—) and regular dash (-) separators, as well as
/// Unix (/) and Windows (\) path separators.
///
/// # Arguments
///
/// * `title` - The editor window title
///
/// # Returns
///
/// The extracted filename, or the original title if no pattern matches
///
/// # Examples
///
/// ```
/// use pulsearc_domain::utils::title::extract_filename;
///
/// assert_eq!(extract_filename("main.rs — Project"), "main.rs");
/// assert_eq!(extract_filename("main.rs - VSCode"), "main.rs");
/// assert_eq!(extract_filename("/path/to/file.rs"), "file.rs");
/// assert_eq!(extract_filename("C:\\path\\to\\file.rs"), "file.rs");
/// ```
#[must_use]
pub fn extract_filename(title: &str) -> String {
    // Try em dash first, then regular dash
    if title.contains(" — ") {
        return title.split(" — ").next().unwrap_or(title).to_string();
    }
    if title.contains(" - ") {
        return title.split(" - ").next().unwrap_or(title).to_string();
    }
    // Try path separators
    if title.contains('/') {
        return title.split('/').next_back().unwrap_or(title).to_string();
    }
    if title.contains('\\') {
        return title.split('\\').next_back().unwrap_or(title).to_string();
    }
    title.to_string()
}

/// Truncate long titles to a maximum length with ellipsis.
///
/// If the title exceeds `MAX_TITLE_LENGTH`, truncates it and appends
/// `TITLE_TRUNCATE_SUFFIX` (typically "...").
///
/// # Arguments
///
/// * `title` - The title string to potentially truncate
///
/// # Returns
///
/// The original title if within limits, or a truncated version with suffix
///
/// # Examples
///
/// ```
/// use pulsearc_domain::utils::title::truncate_title;
///
/// let short = "Short Title";
/// assert_eq!(truncate_title(short), "Short Title");
///
/// let long = "x".repeat(200);
/// let result = truncate_title(&long);
/// assert!(result.len() <= 100); // Assuming MAX_TITLE_LENGTH is 100
/// ```
#[must_use]
pub fn truncate_title(title: &str) -> String {
    if title.len() > MAX_TITLE_LENGTH {
        format!(
            "{}{}",
            &title[..(MAX_TITLE_LENGTH - TITLE_TRUNCATE_SUFFIX.len())],
            TITLE_TRUNCATE_SUFFIX
        )
    } else {
        title.to_string()
    }
}

/// Extract project context from IDE window titles.
///
/// Parses common IDE title formats to extract the project name, typically
/// appearing after the filename and a separator (em dash or regular dash).
/// Validates that the extracted project name is within reasonable length
/// limits.
///
/// # Arguments
///
/// * `title` - The IDE window title
///
/// # Returns
///
/// * `Some(String)` - The extracted project name
/// * `None` - If no project pattern found or the name exceeds maximum length
///
/// # Examples
///
/// ```
/// use pulsearc_domain::utils::title::extract_project_context;
///
/// let title = "main.rs - MyProject [~/path]";
/// assert_eq!(extract_project_context(title), Some("MyProject".to_string()));
///
/// let title = "main.rs — VSCode";
/// assert_eq!(extract_project_context(title), Some("VSCode".to_string()));
///
/// let title = "just a file";
/// assert_eq!(extract_project_context(title), None);
/// ```
#[must_use]
pub fn extract_project_context(title: &str) -> Option<String> {
    // Common patterns: "file.rs - project [~/path]" or "file.rs - project"
    // Handle both em dash " — " and regular dash " - "

    // Try em dash first (it's a multi-byte character)
    if let Some(pos) = title.rfind(" — ") {
        let after_dash = &title[pos + " — ".len()..];
        let project = after_dash.split(" [").next().unwrap_or(after_dash);
        if !project.is_empty() && project.len() < MAX_PROJECT_NAME_LENGTH {
            return Some(project.to_string());
        }
    }

    // Try regular dash
    if let Some(pos) = title.rfind(" - ") {
        let after_dash = &title[pos + 3..];
        let project = after_dash.split(" [").next().unwrap_or(after_dash);
        if !project.is_empty() && project.len() < MAX_PROJECT_NAME_LENGTH {
            return Some(project.to_string());
        }
    }

    None
}

/// Clean browser window titles by removing browser-specific suffixes.
///
/// Strips common browser suffixes like " - Google Chrome", " - Mozilla
/// Firefox", " - Safari", and " - Arc", then truncates to the maximum length.
///
/// # Arguments
///
/// * `title` - The browser window title
///
/// # Returns
///
/// A cleaned title with browser suffix removed and truncated if necessary
///
/// # Examples
///
/// ```
/// use pulsearc_domain::utils::title::clean_browser_title;
///
/// assert_eq!(clean_browser_title("GitHub - Google Chrome"), "GitHub");
/// assert_eq!(clean_browser_title("My Site - Safari"), "My Site");
/// assert_eq!(clean_browser_title("Plain Title"), "Plain Title");
/// ```
#[must_use]
pub fn clean_browser_title(title: &str) -> String {
    // Remove common browser suffixes
    let suffixes = [" - Google Chrome", " - Mozilla Firefox", " - Safari", " - Arc"];

    for suffix in &suffixes {
        if let Some(clean) = extract_by_delimiter(title, suffix, 0) {
            return truncate_title(&clean);
        }
    }

    truncate_title(title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_title_short() {
        let short_title = "Short Title";
        assert_eq!(truncate_title(short_title), "Short Title");
    }

    #[test]
    fn test_truncate_title_long() {
        let long_title = "This is a very long title that exceeds the maximum allowed length";
        let result = truncate_title(long_title);
        assert!(result.len() <= MAX_TITLE_LENGTH);
        assert!(result.ends_with(TITLE_TRUNCATE_SUFFIX));
    }

    #[test]
    fn test_truncate_title_exact_length() {
        let exact = "a".repeat(MAX_TITLE_LENGTH);
        let result = truncate_title(&exact);
        assert_eq!(result, exact);
    }

    #[test]
    fn test_extract_by_delimiter_success() {
        let title = "Part1 | Part2 | Part3";
        assert_eq!(extract_by_delimiter(title, " | ", 0), Some("Part1".to_string()));
        assert_eq!(extract_by_delimiter(title, " | ", 1), Some("Part2".to_string()));
        assert_eq!(extract_by_delimiter(title, " | ", 2), Some("Part3".to_string()));
    }

    #[test]
    fn test_extract_by_delimiter_not_found() {
        let title = "No delimiter here";
        assert_eq!(extract_by_delimiter(title, " | ", 1), None);
    }

    #[test]
    fn test_extract_by_delimiter_empty() {
        let title = " | ";
        assert_eq!(extract_by_delimiter(title, " | ", 0), None);
    }

    #[test]
    fn test_extract_with_filter() {
        let title = "#channel | Workspace";
        let result = extract_with_filter(title, " | ", 0, |s| s.starts_with('#'));
        assert_eq!(result, Some("#channel".to_string()));
    }

    #[test]
    fn test_extract_with_filter_fails() {
        let title = "channel | Workspace";
        let result = extract_with_filter(title, " | ", 0, |s| s.starts_with('#'));
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_filename() {
        assert_eq!(extract_filename("main.rs — Project"), "main.rs");
        assert_eq!(extract_filename("main.rs - Project"), "main.rs");
        assert_eq!(extract_filename("/path/to/file.rs"), "file.rs");
        assert_eq!(extract_filename("C:\\path\\to\\file.rs"), "file.rs");
    }

    #[test]
    fn test_extract_project_context() {
        assert_eq!(
            extract_project_context("main.rs - Pulsarc [~/path]"),
            Some("Pulsarc".to_string())
        );
        assert_eq!(extract_project_context("main.rs - my-project"), Some("my-project".to_string()));
        assert_eq!(extract_project_context("just a file"), None);
    }

    #[test]
    fn test_extract_project_context_too_long() {
        let long_project = format!("file.rs - {}", "a".repeat(50));
        assert_eq!(extract_project_context(&long_project), None);
    }

    #[test]
    fn test_clean_browser_title() {
        assert_eq!(clean_browser_title("GitHub - Google Chrome"), "GitHub");
        assert_eq!(
            clean_browser_title("Stack Overflow - Mozilla Firefox"),
            "Stack Overflow - Mozilla Firefox"
        );
        assert_eq!(clean_browser_title("My Site - Arc"), "My Site - Arc");
        assert_eq!(clean_browser_title("Plain Title"), "Plain Title");
    }
}
