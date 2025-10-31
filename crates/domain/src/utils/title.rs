//! Pure string utility functions for title extraction and manipulation

use crate::constants::*;

/// Generic helper to extract text by splitting on a delimiter and taking a
/// specific part
pub fn extract_by_delimiter(title: &str, delimiter: &str, position: usize) -> Option<String> {
    title
        .split(delimiter)
        .nth(position)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Generic helper to extract text by splitting and filtering with a predicate
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

// Helper function to extract just the filename from editor titles
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

// Helper function to truncate long titles
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

// Extract project context from IDE window titles
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

// Clean browser window title
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
