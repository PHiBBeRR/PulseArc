//! Domain-specific pattern extraction utilities
//!
//! This module provides specialized functions for extracting meaningful context
//! from window titles and app names across different applications and
//! platforms.
//!
//! # Extraction Strategies
//!
//! - **Delimiter-based**: Uses `PatternExtractor` for simple prefix/suffix
//!   removal
//! - **Custom logic**: Complex patterns like Stack Overflow topics require
//!   specialized parsing
//! - **Keyword matching**: Technology detection uses predefined keyword lists
//!
//! # Examples
//!
//! ```
//! use pulsearc_core::utils::patterns::*;
//!
//! // Extract GitHub repository name
//! let repo = extract_github_repo("user/myrepo: Pull Request #123");
//! assert_eq!(repo, Some("user/myrepo".to_string()));
//!
//! // Extract Slack channel
//! let channel = extract_slack_channel("#general | My Workspace");
//! assert_eq!(channel, Some("#general".to_string()));
//! ```

use lazy_static::lazy_static;
use pulsearc_domain::constants::*;
use pulsearc_domain::utils::pattern_extractor::PatternExtractor;
use pulsearc_domain::utils::title::truncate_title;

// Lazy static extractors for common patterns
lazy_static! {
    /// Extractor for Slack channel names (e.g., "#general | Workspace")
    static ref SLACK_CHANNEL_EXTRACTOR: PatternExtractor = PatternExtractor::builder()
        .delimiter(" | ")
        .filter(|s| s.starts_with('#'))
        .build();

    /// Extractor for Discord channel names (e.g., "#general - Server")
    static ref DISCORD_CHANNEL_EXTRACTOR: PatternExtractor = PatternExtractor::builder()
        .delimiter(" - ")
        .filter(|s| s.starts_with('#'))
        .build();

    /// Extractor for Notion page names (e.g., "Page Name | Notion")
    static ref NOTION_PAGE_EXTRACTOR: PatternExtractor = PatternExtractor::builder()
        .delimiter(" | Notion")
        .build();

    /// Extractor for Google Doc names (e.g., "Doc Name - Google Docs")
    static ref GOOGLE_DOC_EXTRACTOR: PatternExtractor = PatternExtractor::builder()
        .delimiter(" - Google ")
        .build();

    /// Extractor for GitHub repo names (e.g., "user/repo: Description")
    static ref GITHUB_REPO_EXTRACTOR: PatternExtractor = PatternExtractor::builder()
        .delimiter(":")
        .filter(|s| s.contains('/'))
        .build();

    /// Extractor for GitHub PR context (e.g., "Pull Request #123")
    static ref GITHUB_PR_EXTRACTOR: PatternExtractor = PatternExtractor::builder()
        .delimiter("Pull Request #")
        .index(1)
        .transform(|s| {
            // Extract just the number (first word)
            if let Some(pr_num) = s.split_whitespace().next() {
                format!("Reviewing PR #{}", pr_num)
            } else {
                "Reviewing pull request".to_string()
            }
        })
        .build();

    /// Extractor for GitHub issue context (e.g., "Issue #456")
    static ref GITHUB_ISSUE_EXTRACTOR: PatternExtractor = PatternExtractor::builder()
        .delimiter("Issue #")
        .index(1)
        .transform(|s| {
            // Extract just the number (first word)
            if let Some(issue_num) = s.split_whitespace().next() {
                format!("Working on issue #{}", issue_num)
            } else {
                "Reviewing GitHub issue".to_string()
            }
        })
        .build();
}

/// Extract GitHub repo name from title
///
/// Looks for "user/repo:" pattern in window titles.
///
/// # Examples
/// ```
/// # use pulsearc_core::utils::patterns::extract_github_repo;
/// assert_eq!(extract_github_repo("user/repo: Pull Request"), Some("user/repo".to_string()));
/// assert_eq!(extract_github_repo("no repo here"), None);
/// ```
pub fn extract_github_repo(title: &str) -> Option<String> {
    // Pattern: "user/repo: Description"
    GITHUB_REPO_EXTRACTOR.extract(title)
}

/// Extract GitHub PR context from title
///
/// Extracts PR number and formats as "Reviewing PR #123".
pub fn extract_github_pr_context(title: &str) -> String {
    // Try to extract PR number from title
    GITHUB_PR_EXTRACTOR.extract(title).unwrap_or_else(|| "Reviewing pull request".to_string())
}

/// Extract GitHub issue context from title
///
/// Extracts issue number and formats as "Working on issue #456".
pub fn extract_github_issue_context(title: &str) -> String {
    // Try to extract issue number from title
    GITHUB_ISSUE_EXTRACTOR.extract(title).unwrap_or_else(|| "Reviewing GitHub issue".to_string())
}

/// Extract Stack Overflow topic from title
///
/// Uses complex logic to extract meaningful topics from Stack Overflow titles.
/// This function:
/// 1. Removes "- Stack Overflow" suffix
/// 2. Detects "topic - question" format and extracts just the topic
/// 3. Strips common question prefixes ("How to", "What is", etc.)
/// 4. Extracts "X in Y" patterns from "How to X in Y" questions
///
/// # Note
/// This function uses custom logic (139 lines) with complex conditional
/// prefix stripping and pattern matching. It does not fit the simple
/// delimiter-based PatternExtractor pattern and is kept as-is for
/// maintainability.
pub fn extract_stackoverflow_topic(title: &str) -> Option<String> {
    // Remove "- Stack Overflow" suffix
    let clean = title.split(" - Stack Overflow").next().unwrap_or(title).trim();

    // Check if this is a "topic - question" format and extract just the topic
    // Example: "javascript - How to use promises" -> "javascript"
    if let Some(first_dash) = clean.find(" - ") {
        let potential_topic = &clean[..first_dash].trim();
        let after_dash = &clean[first_dash + 3..].to_lowercase();

        // If what comes after looks like a question, take what's before
        if (after_dash.starts_with("how ")
            || after_dash.starts_with("what ")
            || after_dash.starts_with("why ")
            || after_dash.starts_with("when ")
            || after_dash.starts_with("where "))
            && !potential_topic.is_empty()
            && potential_topic.len() < MAX_STACKOVERFLOW_TOPIC_LENGTH
        {
            return Some(truncate_title(potential_topic));
        }
    }

    let clean = clean.to_string();

    // For "How to X Y in Z" patterns, extract "Y in Z"
    // For example: "How to fix async/await in Rust" -> "async/await in Rust"
    let clean_lower = clean.to_lowercase();
    if let Some(in_pos) = clean_lower.find(" in ") {
        let before_in = &clean[..in_pos];
        let after_in = &clean[in_pos + 4..]; // Skip " in "

        // Common action words to skip (keep stripping until we can't find any more)
        let actions = [
            "how to ",
            "how do i ",
            "how can i ",
            "what is ",
            "why does ",
            "why is ",
            "fix ",
            "use ",
            "implement ",
            "create ",
            "build ",
            "make ",
            "get ",
            "set ",
        ];
        let mut remaining = before_in;
        let mut lower_remaining = remaining.to_lowercase();

        // Keep stripping prefixes until none match
        loop {
            let mut found = false;
            for action in &actions {
                if lower_remaining.starts_with(action) {
                    remaining = &remaining[action.len()..];
                    lower_remaining = remaining.to_lowercase();
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }

        remaining = remaining.trim();
        if !remaining.is_empty() {
            let result = format!("{} in {}", remaining, after_in);
            if result.len() < MAX_STACKOVERFLOW_TOPIC_LENGTH {
                return Some(truncate_title(&result));
            }
        }
    } else {
        // No " in " pattern, just strip common prefixes
        let prefixes = ["How to ", "How do I ", "How can I ", "What is ", "Why does ", "Why is "];
        for prefix in &prefixes {
            if let Some(stripped) = clean.strip_prefix(prefix) {
                if !stripped.is_empty() && stripped.len() < MAX_STACKOVERFLOW_TOPIC_LENGTH {
                    return Some(truncate_title(stripped));
                }
            }
        }
    }

    if !clean.is_empty() && clean.len() < MAX_STACKOVERFLOW_TOPIC_LENGTH {
        Some(truncate_title(&clean))
    } else {
        None
    }
}

/// Extract technology from documentation titles
///
/// Matches common technology keywords in documentation titles.
///
/// # Note
/// This function uses keyword matching from a predefined technology array.
/// It's a different pattern from delimiter-based extraction and is kept as-is.
pub fn extract_tech_from_docs(title: &str) -> Option<String> {
    let lower = title.to_lowercase();

    // Common patterns
    let techs = [
        "react",
        "vue",
        "angular",
        "svelte",
        "next.js",
        "nuxt",
        "rust",
        "python",
        "javascript",
        "typescript",
        "go",
        "java",
        "node.js",
        "deno",
        "bun",
        "tailwind",
        "bootstrap",
        "css",
        "postgres",
        "mysql",
        "mongodb",
        "redis",
        "docker",
        "kubernetes",
        "aws",
        "azure",
        "gcp",
        "tauri",
        "electron",
    ];

    for tech in techs {
        if lower.contains(tech) {
            return Some(tech.to_string());
        }
    }

    None
}

/// Extract Google Doc name from title
///
/// Removes " - Google Docs" or " - Google Sheets" suffix.
pub fn extract_google_doc_name(title: &str) -> String {
    // Pattern: "Doc Name - Google Docs"
    GOOGLE_DOC_EXTRACTOR.extract(title).unwrap_or_else(|| truncate_title(title))
}

/// Extract Notion page name from title
///
/// Removes " | Notion" suffix if present.
pub fn extract_notion_page(title: &str) -> Option<String> {
    // Pattern: "Page Name | Notion"
    NOTION_PAGE_EXTRACTOR.extract(title)
}

/// Extract Jira ticket from title
///
/// Looks for "[PROJ-123]" or "PROJ-123" patterns.
///
/// # Note
/// This function uses custom logic for bracket extraction and word pattern
/// matching with WORD-NUMBER patterns. It's kept as-is due to multiple
/// extraction strategies.
pub fn extract_jira_ticket(title: &str) -> Option<String> {
    // Pattern: "[PROJ-123] Ticket summary" or "PROJ-123"
    if let Some(bracket_start) = title.find('[') {
        if let Some(bracket_end) = title.find(']') {
            let ticket = &title[bracket_start + 1..bracket_end];
            if ticket.contains('-') {
                return Some(ticket.to_string());
            }
        }
    }

    // Alternative: Just look for WORD-NUMBER pattern
    let words: Vec<&str> = title.split_whitespace().collect();
    for word in words {
        if word.contains('-') && word.chars().any(|c| c.is_numeric()) {
            return Some(word.to_string());
        }
    }

    None
}

/// Extract Linear issue from title
///
/// Looks for "TEAM-123:" or "TEAM-123" patterns.
///
/// # Note
/// This function uses word pattern matching with TEAM-123 patterns and custom
/// trimming logic. It's kept as-is due to specialized pattern matching.
pub fn extract_linear_issue(title: &str) -> Option<String> {
    // Pattern: "TEAM-123: Issue title" or similar
    let words: Vec<&str> = title.split_whitespace().collect();
    for word in words {
        let clean = word.trim_end_matches(':');
        if clean.contains('-') && clean.chars().any(|c| c.is_numeric()) {
            return Some(clean.to_string());
        }
    }
    None
}

/// Extract Slack channel from title
///
/// Looks for "#channel | Workspace" pattern.
pub fn extract_slack_channel(title: &str) -> Option<String> {
    // Pattern: "#channel-name | Workspace"
    SLACK_CHANNEL_EXTRACTOR.extract(title)
}

/// Extract Discord channel from title
///
/// Looks for "#channel - Server" pattern.
pub fn extract_discord_channel(title: &str) -> Option<String> {
    // Pattern: "#channel - Server Name"
    DISCORD_CHANNEL_EXTRACTOR.extract(title)
}

/// Extract terminal context from title
///
/// Extracts the current directory from terminal window titles.
///
/// # Note
/// This function uses specialized path parsing logic with custom splitting
/// and directory extraction. It's kept as-is due to complex path handling
/// logic.
pub fn extract_terminal_context(title: &str) -> Option<String> {
    // Common patterns: "user@host: /path" or just "/path"
    if title.contains('@') {
        // Pattern: "user@host: /path"
        if let Some(colon_pos) = title.find(':') {
            let path = title[colon_pos + 1..].trim();
            if path.starts_with('/') || path.starts_with('~') {
                let clean_path = path.split_whitespace().next().unwrap_or(path);
                // Get last directory
                if let Some(last_dir) = clean_path.split('/').next_back() {
                    if !last_dir.is_empty() {
                        return Some(last_dir.to_string());
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_github_repo() {
        assert_eq!(extract_github_repo("user/repo: Pull Request"), Some("user/repo".to_string()));
        assert_eq!(extract_github_repo("no repo here"), None);
    }

    #[test]
    fn test_extract_github_pr_context() {
        let title = "Fix bug - Pull Request #123 - user/repo";
        assert_eq!(extract_github_pr_context(title), "Reviewing PR #123");
    }

    #[test]
    fn test_extract_github_pr_context_no_number() {
        let title = "Pull Request without number";
        assert_eq!(extract_github_pr_context(title), "Reviewing pull request");
    }

    #[test]
    fn test_extract_github_issue_context() {
        let title = "Bug report - Issue #456 - user/repo";
        assert_eq!(extract_github_issue_context(title), "Working on issue #456");
    }

    #[test]
    fn test_extract_github_issue_context_no_number() {
        let title = "Issue without number";
        assert_eq!(extract_github_issue_context(title), "Reviewing GitHub issue");
    }

    #[test]
    fn test_extract_stackoverflow_topic() {
        let title = "How to fix this error - Stack Overflow";
        let result = extract_stackoverflow_topic(title);
        assert_eq!(result, Some("fix this error".to_string()));
    }

    #[test]
    fn test_extract_stackoverflow_topic_too_long() {
        let long = format!("{} - Stack Overflow", "a".repeat(150));
        assert_eq!(extract_stackoverflow_topic(&long), None);
    }

    #[test]
    fn test_extract_tech_from_docs() {
        assert_eq!(extract_tech_from_docs("React Documentation"), Some("react".to_string()));
        assert_eq!(extract_tech_from_docs("Rust by Example"), Some("rust".to_string()));
        assert_eq!(extract_tech_from_docs("Tauri Guides"), Some("tauri".to_string()));
        assert_eq!(extract_tech_from_docs("Random Documentation"), None);
    }

    #[test]
    fn test_extract_google_doc_name() {
        assert_eq!(extract_google_doc_name("My Document - Google Docs"), "My Document");
        assert_eq!(extract_google_doc_name("Budget 2024 - Google Sheets"), "Budget 2024");
    }

    #[test]
    fn test_extract_notion_page() {
        assert_eq!(
            extract_notion_page("Project Planning | Notion"),
            Some("Project Planning".to_string())
        );
        assert_eq!(extract_notion_page("Just Notion"), Some("Just Notion".to_string()));
        assert_eq!(extract_notion_page(" | Notion"), None);
    }

    #[test]
    fn test_extract_jira_ticket() {
        assert_eq!(extract_jira_ticket("[PROJ-123] Fix the bug"), Some("PROJ-123".to_string()));
        assert_eq!(extract_jira_ticket("Working on PROJ-456 today"), Some("PROJ-456".to_string()));
        assert_eq!(extract_jira_ticket("No ticket here"), None);
    }

    #[test]
    fn test_extract_linear_issue() {
        assert_eq!(extract_linear_issue("ENG-123: Implement feature"), Some("ENG-123".to_string()));
        assert_eq!(extract_linear_issue("Working on TEAM-456"), Some("TEAM-456".to_string()));
        assert_eq!(extract_linear_issue("No issue"), None);
    }

    #[test]
    fn test_extract_slack_channel() {
        assert_eq!(extract_slack_channel("#general | My Workspace"), Some("#general".to_string()));
        assert_eq!(extract_slack_channel("general | My Workspace"), None);
    }

    #[test]
    fn test_extract_discord_channel() {
        assert_eq!(extract_discord_channel("#general - My Server"), Some("#general".to_string()));
        assert_eq!(extract_discord_channel("general - My Server"), None);
    }

    #[test]
    fn test_extract_terminal_context() {
        assert_eq!(
            extract_terminal_context("user@host: ~/projects/my-app"),
            Some("my-app".to_string())
        );
        assert_eq!(extract_terminal_context("user@host: /usr/local/bin"), Some("bin".to_string()));
        assert_eq!(extract_terminal_context("simple title"), None);
    }

    #[test]
    fn test_extract_stackoverflow_topic_nested_actions() {
        // Test multiple nested action words (how to + fix + use)
        let title = "How to fix use promises in JavaScript - Stack Overflow";
        let result = extract_stackoverflow_topic(title);
        assert_eq!(result, Some("promises in JavaScript".to_string()));

        // Test "How to use implement X in Y" pattern
        let title2 = "How to use implement async/await in Rust - Stack Overflow";
        let result2 = extract_stackoverflow_topic(title2);
        assert_eq!(result2, Some("async/await in Rust".to_string()));
    }

    #[test]
    fn test_extract_terminal_context_edge_cases() {
        // Test trailing slash (returns None because split('/').next_back() is empty)
        assert_eq!(extract_terminal_context("user@host: ~/projects/"), None);

        // Test root directory (should return None)
        assert_eq!(extract_terminal_context("user@host: /"), None);

        // Test path with spaces (extracts first whitespace-separated token)
        assert_eq!(
            extract_terminal_context("user@host: ~/my-app some other text"),
            Some("my-app".to_string())
        );

        // Test without @ symbol (doesn't match pattern)
        assert_eq!(extract_terminal_context("/usr/local/bin"), None);

        // Test tilde home directory
        assert_eq!(extract_terminal_context("user@host: ~"), Some("~".to_string()));
    }
}
