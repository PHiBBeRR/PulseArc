//! Office application document enrichment using AppleScript.
//!
//! Supports extracting the current document name from:
//! - Microsoft Office: Word, Excel, PowerPoint
//! - Apple iWork: Pages, Numbers, Keynote
//!
//! # Example
//! ```rust,no_run
//! use tokio::runtime::Runtime;
//! use pulsearc_infra::platform::macos::enrichers::office::get_office_document;
//!
//! Runtime::new().unwrap().block_on(async {
//!     if let Some(doc) = get_office_document("com.microsoft.Word", "Microsoft Word").await {
//!         println!("Current document: {doc}");
//!     }
//! });
//! ```

use std::time::Duration;

use super::applescript_helpers::{build_document_name_script, execute_applescript_optional};

/// Default timeout for office AppleScript queries (2 seconds).
const OFFICE_SCRIPT_TIMEOUT: Duration = Duration::from_secs(2);

/// Mapping of bundle IDs to office application names for document extraction.
///
/// This list covers Microsoft Office and Apple iWork suite apps.
/// Order matters: checked in sequence for fallback matching.
const OFFICE_MAPPINGS: &[(&str, &str)] = &[
    // Microsoft Office
    ("com.microsoft.Word", "Microsoft Word"),
    ("com.microsoft.Excel", "Microsoft Excel"),
    ("com.microsoft.Powerpoint", "Microsoft PowerPoint"),
    // Apple iWork
    ("com.apple.iWork.Pages", "Pages"),
    ("com.apple.iWork.Numbers", "Numbers"),
    ("com.apple.iWork.Keynote", "Keynote"),
];

/// Get the office application name from a bundle ID.
///
/// # Arguments
/// * `bundle_id` - The application bundle identifier
///
/// # Returns
/// * `Some(&str)` - The office app name if recognized
/// * `None` - If the bundle ID is not a known office application
fn get_office_app_name(bundle_id: &str) -> Option<&'static str> {
    OFFICE_MAPPINGS.iter().find(|(bid, _)| *bid == bundle_id).map(|(_, name)| *name)
}

/// Check if a bundle ID represents a known office application.
///
/// # Arguments
/// * `bundle_id` - The application bundle identifier
///
/// # Returns
/// `true` if the bundle ID is a recognized office app
pub fn is_office_app(bundle_id: &str) -> bool {
    get_office_app_name(bundle_id).is_some()
}

/// Get the current document name from an office application using AppleScript.
///
/// This function attempts to extract the frontmost document's name using
/// app-specific AppleScript commands. It handles differences between Microsoft
/// Office and Apple iWork apps and returns None on any failure.
///
/// # Arguments
/// * `bundle_id` - The application's bundle identifier (e.g.,
///   "com.microsoft.Word")
/// * `app_name` - The application's display name (e.g., "Microsoft Word") -
///   used as fallback
///
/// # Returns
/// * `Some(String)` - The document name if successfully extracted
/// * `None` - If app is not recognized, has no open documents, or script fails
///
/// # Note
/// This function is async to allow calling from async contexts, but internally
/// uses blocking AppleScript execution (should be spawned in blocking task).
#[cfg(target_os = "macos")]
pub async fn get_office_document(bundle_id: &str, app_name: &str) -> Option<String> {
    // First, try to get app name from bundle ID mapping
    let office_app_name = get_office_app_name(bundle_id).unwrap_or(app_name);

    tracing::debug!(
        bundle_id = %bundle_id,
        office_app_name = %office_app_name,
        "Attempting to get document from office app"
    );

    // Build the appropriate AppleScript for this app
    let script = build_document_name_script(office_app_name);

    // Execute the script with timeout (this is a blocking operation)
    tokio::task::spawn_blocking(move || {
        execute_applescript_optional(&script, OFFICE_SCRIPT_TIMEOUT, "office document fetch")
    })
    .await
    .ok()
    .flatten()
}

/// Get the current document name from an office application synchronously.
///
/// This is a blocking variant of `get_office_document` for use in synchronous
/// contexts or within `spawn_blocking` tasks.
///
/// # Arguments
/// * `bundle_id` - The application's bundle identifier
/// * `app_name` - The application's display name (fallback)
///
/// # Returns
/// * `Some(String)` - The document name if successfully extracted
/// * `None` - If app is not recognized or script fails
#[cfg(target_os = "macos")]
pub fn get_office_document_sync(bundle_id: &str, app_name: &str) -> Option<String> {
    let office_app_name = get_office_app_name(bundle_id).unwrap_or(app_name);

    tracing::debug!(
        bundle_id = %bundle_id,
        office_app_name = %office_app_name,
        "Attempting to get document from office app (sync)"
    );

    let script = build_document_name_script(office_app_name);
    execute_applescript_optional(&script, OFFICE_SCRIPT_TIMEOUT, "office document fetch")
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn test_is_office_app_word() {
        assert!(is_office_app("com.microsoft.Word"));
    }

    #[test]
    fn test_is_office_app_excel() {
        assert!(is_office_app("com.microsoft.Excel"));
    }

    #[test]
    fn test_is_office_app_powerpoint() {
        assert!(is_office_app("com.microsoft.Powerpoint"));
    }

    #[test]
    fn test_is_office_app_pages() {
        assert!(is_office_app("com.apple.iWork.Pages"));
    }

    #[test]
    fn test_is_office_app_numbers() {
        assert!(is_office_app("com.apple.iWork.Numbers"));
    }

    #[test]
    fn test_is_office_app_keynote() {
        assert!(is_office_app("com.apple.iWork.Keynote"));
    }

    #[test]
    fn test_is_not_office_app() {
        assert!(!is_office_app("com.apple.Safari"));
        assert!(!is_office_app("com.microsoft.VSCode"));
    }

    #[test]
    fn test_get_office_app_name_word() {
        assert_eq!(get_office_app_name("com.microsoft.Word"), Some("Microsoft Word"));
    }

    #[test]
    fn test_get_office_app_name_pages() {
        assert_eq!(get_office_app_name("com.apple.iWork.Pages"), Some("Pages"));
    }

    #[test]
    fn test_get_office_app_name_unknown() {
        assert_eq!(get_office_app_name("com.unknown.App"), None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_office_document_unknown_app() {
        // Should return None for non-office apps
        let result = get_office_document("com.apple.Terminal", "Terminal").await;
        // May be None (expected) or Some if Terminal somehow responds to AppleScript
        // Just verify it doesn't panic
        drop(result);
    }

    #[test]
    fn test_get_office_document_sync_unknown() {
        let result = get_office_document_sync("com.unknown.App", "Unknown App");
        // Should return None or handle gracefully
        drop(result);
    }

    // Note: Testing actual document extraction requires office apps to be running
    // with open documents, which is not suitable for CI. These are integration
    // tests that should be run manually during development.
    //
    // Manual test procedure:
    // 1. Open Microsoft Word with a document (e.g., "Report.docx")
    // 2. Run: cargo test --package pulsearc-infra get_office_document -- --ignored
    //    --nocapture
    // 3. Verify the document name is extracted correctly
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_get_word_document_manual() {
        let doc = get_office_document("com.microsoft.Word", "Microsoft Word").await;
        println!("Word document: {doc:?}");
        // In manual testing, verify this matches Word's frontmost document
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_get_excel_document_manual() {
        let doc = get_office_document("com.microsoft.Excel", "Microsoft Excel").await;
        println!("Excel document: {doc:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn test_get_pages_document_manual() {
        let doc = get_office_document("com.apple.iWork.Pages", "Pages").await;
        println!("Pages document: {doc:?}");
    }
}
