//! Browser URL enrichment using AppleScript.
//!
//! Supports extracting the current URL from:
//! - Safari (including Technology Preview)
//! - Google Chrome
//! - Firefox (Developer Edition, Nightly)
//! - Microsoft Edge
//! - Brave Browser
//! - Arc
//!
//! # Example
//! ```rust,no_run
//! let url = get_browser_url("com.apple.Safari", "Safari").await;
//! if let Some(url) = url {
//!     println!("Current URL: {url}");
//! }
//! ```

use std::time::Duration;

use super::applescript_helpers::{build_browser_url_script, execute_applescript_optional};

/// Default timeout for browser AppleScript queries (2 seconds).
const BROWSER_SCRIPT_TIMEOUT: Duration = Duration::from_secs(2);

/// Mapping of bundle IDs to browser names for URL extraction.
///
/// This list covers the major browsers with AppleScript support.
/// Order matters: checked in sequence for fallback matching.
const BROWSER_MAPPINGS: &[(&str, &str)] = &[
    // Safari
    ("com.apple.Safari", "Safari"),
    ("com.apple.SafariTechnologyPreview", "Safari Technology Preview"),
    // Chrome-based browsers
    ("com.google.Chrome", "Google Chrome"),
    ("org.chromium.Chromium", "Chromium"),
    ("com.microsoft.edgemac", "Microsoft Edge"),
    ("com.brave.Browser", "Brave Browser"),
    ("company.thebrowser.Browser", "Arc"),
    // Firefox-based browsers
    ("org.mozilla.firefox", "Firefox"),
    ("org.mozilla.firefoxdeveloperedition", "Firefox Developer Edition"),
    ("org.mozilla.nightly", "Firefox Nightly"),
];

/// Get the browser name from a bundle ID.
///
/// # Arguments
/// * `bundle_id` - The application bundle identifier
///
/// # Returns
/// * `Some(&str)` - The browser application name if recognized
/// * `None` - If the bundle ID is not a known browser
fn get_browser_name(bundle_id: &str) -> Option<&'static str> {
    BROWSER_MAPPINGS.iter().find(|(bid, _)| *bid == bundle_id).map(|(_, name)| *name)
}

/// Check if a bundle ID represents a known browser.
///
/// # Arguments
/// * `bundle_id` - The application bundle identifier
///
/// # Returns
/// `true` if the bundle ID is a recognized browser
pub fn is_browser(bundle_id: &str) -> bool {
    get_browser_name(bundle_id).is_some()
}

/// Get the current URL from a browser using AppleScript.
///
/// This function attempts to extract the frontmost tab's URL using
/// browser-specific AppleScript commands. It handles different browsers'
/// quirks and returns None on any failure (permission denied, no windows,
/// etc.).
///
/// # Arguments
/// * `bundle_id` - The browser's bundle identifier (e.g., "com.apple.Safari")
/// * `app_name` - The browser's display name (e.g., "Safari") - used as
///   fallback
///
/// # Returns
/// * `Some(String)` - The URL if successfully extracted
/// * `None` - If browser is not recognized, has no windows, or script fails
///
/// # Note
/// This function is async to allow calling from async contexts, but internally
/// uses blocking AppleScript execution (should be spawned in blocking task).
#[cfg(target_os = "macos")]
pub async fn get_browser_url(bundle_id: &str, app_name: &str) -> Option<String> {
    // First, try to get browser name from bundle ID mapping
    let browser_name = get_browser_name(bundle_id).unwrap_or(app_name);

    tracing::debug!(
        bundle_id = %bundle_id,
        browser_name = %browser_name,
        "Attempting to get URL from browser"
    );

    // Build the appropriate AppleScript for this browser
    let script = build_browser_url_script(browser_name);

    // Execute the script with timeout (this is a blocking operation)
    tokio::task::spawn_blocking(move || {
        execute_applescript_optional(&script, BROWSER_SCRIPT_TIMEOUT, "browser URL fetch")
    })
    .await
    .ok()
    .flatten()
}

/// Get the current URL from a browser synchronously.
///
/// This is a blocking variant of `get_browser_url` for use in synchronous
/// contexts or within `spawn_blocking` tasks.
///
/// # Arguments
/// * `bundle_id` - The browser's bundle identifier
/// * `app_name` - The browser's display name (fallback)
///
/// # Returns
/// * `Some(String)` - The URL if successfully extracted
/// * `None` - If browser is not recognized or script fails
#[cfg(target_os = "macos")]
pub fn get_browser_url_sync(bundle_id: &str, app_name: &str) -> Option<String> {
    let browser_name = get_browser_name(bundle_id).unwrap_or(app_name);

    tracing::debug!(
        bundle_id = %bundle_id,
        browser_name = %browser_name,
        "Attempting to get URL from browser (sync)"
    );

    let script = build_browser_url_script(browser_name);
    execute_applescript_optional(&script, BROWSER_SCRIPT_TIMEOUT, "browser URL fetch")
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn test_is_browser_safari() {
        assert!(is_browser("com.apple.Safari"));
    }

    #[test]
    fn test_is_browser_chrome() {
        assert!(is_browser("com.google.Chrome"));
    }

    #[test]
    fn test_is_browser_firefox() {
        assert!(is_browser("org.mozilla.firefox"));
    }

    #[test]
    fn test_is_browser_edge() {
        assert!(is_browser("com.microsoft.edgemac"));
    }

    #[test]
    fn test_is_browser_brave() {
        assert!(is_browser("com.brave.Browser"));
    }

    #[test]
    fn test_is_browser_arc() {
        assert!(is_browser("company.thebrowser.Browser"));
    }

    #[test]
    fn test_is_not_browser() {
        assert!(!is_browser("com.apple.Terminal"));
        assert!(!is_browser("com.microsoft.VSCode"));
    }

    #[test]
    fn test_get_browser_name_safari() {
        assert_eq!(get_browser_name("com.apple.Safari"), Some("Safari"));
    }

    #[test]
    fn test_get_browser_name_chrome() {
        assert_eq!(get_browser_name("com.google.Chrome"), Some("Google Chrome"));
    }

    #[test]
    fn test_get_browser_name_unknown() {
        assert_eq!(get_browser_name("com.unknown.App"), None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_browser_url_unknown_browser() {
        // Should return None for non-browser apps
        let result = get_browser_url("com.apple.Terminal", "Terminal").await;
        // May be None (expected) or Some if Terminal somehow responds to AppleScript
        // Just verify it doesn't panic
        drop(result);
    }

    #[test]
    fn test_get_browser_url_sync_unknown() {
        let result = get_browser_url_sync("com.unknown.App", "Unknown App");
        // Should return None or handle gracefully
        drop(result);
    }

    // Note: Testing actual browser URL extraction requires browsers to be running
    // with open windows, which is not suitable for CI. These are integration tests
    // that should be run manually during development.
    //
    // Manual test procedure:
    // 1. Open Safari with a URL (e.g., https://example.com)
    // 2. Run: cargo test --package pulsearc-infra get_browser_url -- --ignored
    //    --nocapture
    // 3. Verify the URL is extracted correctly
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_get_safari_url_manual() {
        let url = get_browser_url("com.apple.Safari", "Safari").await;
        println!("Safari URL: {url:?}");
        // In manual testing, verify this matches Safari's frontmost tab
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_get_chrome_url_manual() {
        let url = get_browser_url("com.google.Chrome", "Google Chrome").await;
        println!("Chrome URL: {url:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_get_firefox_url_manual() {
        let url = get_browser_url("org.mozilla.firefox", "Firefox").await;
        println!("Firefox URL: {url:?}");
    }
}
