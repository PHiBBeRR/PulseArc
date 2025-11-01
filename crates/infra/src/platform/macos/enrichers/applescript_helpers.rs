//! AppleScript execution utilities with timeout handling.
//!
//! Provides safe wrappers around `osascript` command execution with:
//! - Timeout handling using `wait-timeout` crate
//! - Full error handling with tracing
//! - UTF-8 output sanitization
//!
//! # Example
//! ```rust,no_run
//! use std::time::Duration;
//!
//! use pulsearc_infra::platform::macos::enrichers::applescript_helpers::execute_applescript;
//!
//! fn main() -> pulsearc_domain::Result<()> {
//!     #[cfg(target_os = "macos")]
//!     {
//!         let script = r#"
//!             tell application "Safari"
//!                 if (count of windows) > 0 then
//!                     URL of current tab of front window
//!                 end if
//!             end tell
//!         "#;
//!
//!         let url = execute_applescript(script, Duration::from_secs(2))?;
//!         println!("Active URL: {url}");
//!     }
//!     Ok(())
//! }
//! ```

use std::process::{Command, Stdio};
use std::time::Duration;

use pulsearc_domain::{PulseArcError, Result as DomainResult};
use wait_timeout::ChildExt;

use crate::platform::macos::error_helpers::map_platform_io_error;

/// Execute an AppleScript with a timeout.
///
/// # Arguments
/// * `script` - The AppleScript source code to execute
/// * `timeout` - Maximum execution time before killing the process
///
/// # Returns
/// * `Ok(String)` - Trimmed stdout output if successful
/// * `Err(PulseArcError::Platform)` - If execution fails, times out, or returns
///   non-zero exit
///
/// # Errors
/// - If osascript fails to spawn
/// - If the script times out
/// - If the script returns non-zero exit code
/// - If output is not valid UTF-8
#[cfg(target_os = "macos")]
pub fn execute_applescript(script: &str, timeout: Duration) -> DomainResult<String> {
    tracing::debug!(
        script_preview = %script.chars().take(100).collect::<String>(),
        timeout_secs = timeout.as_secs(),
        "Executing AppleScript with timeout"
    );

    let mut child = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| map_platform_io_error("osascript spawn", e))?;

    // Wait for the command with timeout
    let status_code = match child.wait_timeout(timeout) {
        Ok(Some(status)) => {
            tracing::trace!(exit_code = status.code(), "AppleScript exited");
            status.code()
        }
        Ok(None) => {
            // Timeout occurred - kill the child
            tracing::warn!(
                timeout_secs = timeout.as_secs(),
                "AppleScript execution timed out, killing process"
            );
            let _ = child.kill();
            let _ = child.wait(); // Reap the zombie
            return Err(PulseArcError::Platform(format!(
                "AppleScript execution timed out after {}s",
                timeout.as_secs()
            )));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to wait for osascript");
            return Err(map_platform_io_error("osascript wait", e));
        }
    };

    // Collect output
    let output =
        child.wait_with_output().map_err(|e| map_platform_io_error("osascript output", e))?;

    // Check exit status
    if let Some(code) = status_code {
        if code != 0 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(
                exit_code = code,
                stderr = %stderr.trim(),
                "AppleScript returned non-zero exit code"
            );
            return Err(PulseArcError::Platform(format!(
                "AppleScript failed with exit code {code}: {stderr}"
            )));
        }
    }

    // Parse stdout
    let stdout = String::from_utf8(output.stdout).map_err(|e| {
        tracing::error!(error = %e, "AppleScript output is not valid UTF-8");
        PulseArcError::Platform(format!("AppleScript output is not valid UTF-8: {e}"))
    })?;

    let result = stdout.trim().to_string();
    tracing::debug!(
        output_len = result.len(),
        output_preview = %result.chars().take(100).collect::<String>(),
        "AppleScript executed successfully"
    );

    Ok(result)
}

/// Execute AppleScript and return None if it fails gracefully.
///
/// This is useful for optional enrichment where failures should not halt
/// processing. Logs warnings but does not propagate errors.
///
/// # Arguments
/// * `script` - The AppleScript source code
/// * `timeout` - Maximum execution time
/// * `context` - Description of what the script does (for logging)
///
/// # Returns
/// * `Some(String)` if successful and output is non-empty
/// * `None` if execution fails or output is empty
#[cfg(target_os = "macos")]
pub fn execute_applescript_optional(
    script: &str,
    timeout: Duration,
    context: &str,
) -> Option<String> {
    match execute_applescript(script, timeout) {
        Ok(output) if !output.is_empty() => {
            tracing::trace!(
                context = %context,
                output_len = output.len(),
                "AppleScript succeeded"
            );
            Some(output)
        }
        Ok(_) => {
            tracing::trace!(
                context = %context,
                "AppleScript returned empty output"
            );
            None
        }
        Err(e) => {
            tracing::debug!(
                context = %context,
                error = %e,
                "AppleScript failed (optional)"
            );
            None
        }
    }
}

/// Build an AppleScript to get the current URL from a browser.
///
/// # Arguments
/// * `browser_name` - The application name (e.g., "Safari", "Google Chrome")
///
/// # Returns
/// AppleScript source code that attempts to get the frontmost tab URL
#[cfg(target_os = "macos")]
pub fn build_browser_url_script(browser_name: &str) -> String {
    match browser_name {
        "Safari" | "Safari Technology Preview" => format!(
            r#"
            tell application "{browser_name}"
                if (count of windows) > 0 then
                    URL of current tab of front window
                end if
            end tell
            "#
        ),
        "Google Chrome" | "Chromium" | "Microsoft Edge" | "Brave Browser" | "Arc" => format!(
            r#"
            tell application "{browser_name}"
                if (count of windows) > 0 then
                    URL of active tab of front window
                end if
            end tell
            "#
        ),
        "Firefox" | "Firefox Developer Edition" | "Firefox Nightly" => {
            // Firefox doesn't support AppleScript well, use workaround
            format!(
                r#"
                tell application "System Events"
                    tell process "{browser_name}"
                        if exists (window 1) then
                            tell window 1
                                set theURL to value of attribute "AXDocument" of group 1
                                return theURL
                            end tell
                        end if
                    end tell
                end tell
                "#
            )
        }
        _ => {
            // Generic fallback
            format!(
                r#"
                tell application "{browser_name}"
                    if (count of windows) > 0 then
                        URL of front window
                    end if
                end tell
                "#
            )
        }
    }
}

/// Build an AppleScript to get the current document name from an Office app.
///
/// # Arguments
/// * `app_name` - The application name (e.g., "Microsoft Word", "Pages")
///
/// # Returns
/// AppleScript source code that attempts to get the frontmost document name
#[cfg(target_os = "macos")]
pub fn build_document_name_script(app_name: &str) -> String {
    match app_name {
        "Microsoft Word" | "Microsoft Excel" | "Microsoft PowerPoint" => format!(
            r#"
            tell application "{app_name}"
                if (count of documents) > 0 then
                    name of active document
                end if
            end tell
            "#
        ),
        "Pages" | "Numbers" | "Keynote" => format!(
            r#"
            tell application "{app_name}"
                if (count of documents) > 0 then
                    name of front document
                end if
            end tell
            "#
        ),
        _ => {
            // Generic fallback
            format!(
                r#"
                tell application "{app_name}"
                    if (count of documents) > 0 then
                        name of document 1
                    end if
                end tell
                "#
            )
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn test_build_safari_url_script() {
        let script = build_browser_url_script("Safari");
        assert!(script.contains("Safari"));
        assert!(script.contains("URL of current tab"));
    }

    #[test]
    fn test_build_chrome_url_script() {
        let script = build_browser_url_script("Google Chrome");
        assert!(script.contains("Google Chrome"));
        assert!(script.contains("URL of active tab"));
    }

    #[test]
    fn test_build_firefox_url_script() {
        let script = build_browser_url_script("Firefox");
        assert!(script.contains("Firefox"));
        assert!(script.contains("System Events"));
        assert!(script.contains("AXDocument"));
    }

    #[test]
    fn test_build_word_document_script() {
        let script = build_document_name_script("Microsoft Word");
        assert!(script.contains("Microsoft Word"));
        assert!(script.contains("active document"));
    }

    #[test]
    fn test_build_pages_document_script() {
        let script = build_document_name_script("Pages");
        assert!(script.contains("Pages"));
        assert!(script.contains("front document"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_execute_simple_applescript() {
        let script = r#"return "hello world""#;
        let result = execute_applescript(script, Duration::from_secs(2));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_execute_applescript_timeout() {
        // Script that sleeps longer than timeout
        let script = r#"delay 10"#;
        let result = execute_applescript(script, Duration::from_millis(100));
        assert!(result.is_err());
        if let Err(PulseArcError::Platform(msg)) = result {
            assert!(msg.contains("timed out"));
        } else {
            panic!("Expected Platform error with timeout message");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_execute_applescript_optional_success() {
        let script = r#"return "test output""#;
        let result = execute_applescript_optional(script, Duration::from_secs(2), "test script");
        assert_eq!(result, Some("test output".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_execute_applescript_optional_empty() {
        let script = r#"return """#;
        let result = execute_applescript_optional(script, Duration::from_secs(2), "empty script");
        assert_eq!(result, None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_execute_applescript_optional_failure() {
        let script = r#"error "intentional error""#;
        let result = execute_applescript_optional(script, Duration::from_secs(2), "failing script");
        assert_eq!(result, None);
    }
}
