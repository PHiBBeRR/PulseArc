//! macOS Accessibility API Integration
//!
//! This module provides Rust bindings for macOS Accessibility APIs to fetch
//! focused window titles, active app information, and recent apps from running
//! applications.
//!
//! All functions gracefully degrade when Accessibility permissions are not
//! granted, returning limited app information (name, bundle ID) without window
//! titles.

use std::sync::OnceLock;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
use parking_lot::RwLock;
use pulsearc_domain::{PulseArcError, Result as DomainResult};

/// Details about the currently active application.
#[derive(Debug, Clone)]
pub struct ActiveAppInfo {
    pub app_name: String,
    pub bundle_id: String,
    pub window_title: Option<String>,
    pub pid: i32,
}

/// Details about a recently used application.
#[derive(Debug, Clone)]
pub struct RecentAppInfo {
    pub app_name: String,
    pub bundle_id: String,
    pub window_title: Option<String>,
}

#[cfg(target_os = "macos")]
use core_foundation::base::{CFTypeRef, TCFType};
#[cfg(target_os = "macos")]
use core_foundation::boolean::CFBoolean;
#[cfg(target_os = "macos")]
use core_foundation::dictionary::CFDictionary;
#[cfg(target_os = "macos")]
use core_foundation::string::{CFString, CFStringRef};
#[cfg(target_os = "macos")]
use objc2_app_kit::NSWorkspace;

#[cfg(target_os = "macos")]
use super::error_helpers::ax_permission_error;

// Accessibility API types and functions (macOS only)
#[cfg(target_os = "macos")]
#[repr(C)]
struct __AXUIElement(std::ffi::c_void);
#[cfg(target_os = "macos")]
type AXUIElementRef = *const __AXUIElement;

#[cfg(target_os = "macos")]
extern "C" {
    fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn CFRelease(cf: CFTypeRef);
}

// AX Error codes
#[cfg(target_os = "macos")]
const K_AX_ERROR_SUCCESS: i32 = 0;

#[cfg(target_os = "macos")]
const AX_PERMISSION_CACHE_TTL: Duration = Duration::from_secs(300);

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
struct CachedPermission {
    value: bool,
    checked_at: Instant,
}

// Cache for AX permission state (avoid repeated system calls, but allow
// refresh)
#[cfg(target_os = "macos")]
static AX_PERMISSION_CACHE: OnceLock<RwLock<Option<CachedPermission>>> = OnceLock::new();

#[cfg(target_os = "macos")]
fn permission_cache() -> &'static RwLock<Option<CachedPermission>> {
    AX_PERMISSION_CACHE.get_or_init(|| RwLock::new(None))
}

/// Check if Accessibility permission is granted.
///
/// Uses `AXIsProcessTrustedWithOptions` to query permission state. The result
/// is cached to avoid repeated system calls.
///
/// # Arguments
///
/// * `prompt` - If true, shows system permission prompt to user
///
/// # Returns
///
/// * `Ok(true)` - Accessibility permission is granted
/// * `Ok(false)` - Permission denied or not yet granted
/// * `Err(_)` - Only on non-macOS platforms
///
/// # Platform Support
///
/// * macOS: Full support with AX API
/// * Other: Returns `Err(PulseArcError::Platform)`
///
/// # Examples
///
/// ```rust,ignore
/// let has_permission = check_ax_permission(false)?;
/// if !has_permission {
///     tracing::warn!("Accessibility permission denied - running in app-only mode");
/// }
/// ```
#[cfg(target_os = "macos")]
pub fn check_ax_permission(prompt: bool) -> DomainResult<bool> {
    // Check cache first unless an explicit prompt is requested
    if !prompt {
        let cached = permission_cache().read();
        if let Some(entry) = *cached {
            if entry.checked_at.elapsed() < AX_PERMISSION_CACHE_TTL {
                return Ok(entry.value);
            }
        }
    }

    // SAFETY: AXIsProcessTrustedWithOptions is a C function that:
    // - Accepts a CFDictionary pointer (we create valid CFDictionary)
    // - Returns a boolean (C bool) indicating trust status
    // - Does not retain references to the dictionary after return
    // - Is thread-safe according to Apple documentation
    let is_trusted = unsafe {
        // Create options dictionary for prompt control
        let prompt_key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
        let prompt_value = CFBoolean::from(prompt);

        let options =
            CFDictionary::from_CFType_pairs(&[(prompt_key.as_CFType(), prompt_value.as_CFType())]);

        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef().cast())
    };

    // Cache the result with a TTL so we can refresh after the user changes settings
    {
        let mut cache = permission_cache().write();
        *cache = Some(CachedPermission { value: is_trusted, checked_at: Instant::now() });
    }

    // Log permission status
    if is_trusted {
        tracing::info!("Accessibility permission granted");
    } else {
        tracing::warn!("Accessibility permission denied - running in app-only mode");
    }

    Ok(is_trusted)
}

#[cfg(not(target_os = "macos"))]
pub fn check_ax_permission(_prompt: bool) -> DomainResult<bool> {
    Err(PulseArcError::Platform("Accessibility API is only available on macOS".to_string()))
}

/// Get focused window title from active app using Accessibility API.
///
/// Queries the focused window of the given process using AX APIs:
/// 1. `AXUIElementCreateApplication(pid)`
/// 2. Query `kAXFocusedWindowAttribute`
/// 3. Query `kAXTitleAttribute` from focused window
///
/// # Arguments
///
/// * `app_pid` - Process ID of the active application
///
/// # Returns
///
/// * `Ok(Some(String))` - Window title if Accessibility permission granted and
///   window has title
/// * `Ok(None)` - Permission denied, invalid PID, or no focused window
/// * `Err(_)` - Only on non-macOS platforms
///
/// # Platform Support
///
/// * macOS: Full support with AX API
/// * Other: Returns `Err(PulseArcError::Platform)`
///
/// # Examples
///
/// ```rust,ignore
/// match get_focused_window_title(1234)? {
///     Some(title) => tracing::info!(title = %title, "Got window title"),
///     None => tracing::debug!("No window title available"),
/// }
/// ```
#[cfg(target_os = "macos")]
pub fn get_focused_window_title(app_pid: i32) -> DomainResult<Option<String>> {
    // Check permission first (graceful degradation)
    if !check_ax_permission(false)? {
        let permission_error = ax_permission_error();
        tracing::warn!(
            error = %permission_error,
            pid = app_pid,
            "Accessibility permission not granted"
        );
        return Ok(None);
    }

    // SAFETY: This unsafe block interacts with macOS Accessibility APIs:
    // - AXUIElementCreateApplication: Creates an AX element for the app (must be
    //   released)
    // - AXUIElementCopyAttributeValue: Queries attributes (returns owned CFTypeRef)
    // - CFRelease: Releases CoreFoundation objects
    //
    // Safety invariants:
    // 1. All created/copied CF objects are released before function returns
    // 2. Null pointers are checked before dereferencing
    // 3. CF objects are not accessed after being released
    // 4. CFString is wrapped with proper ownership semantics
    //    (wrap_under_create_rule)
    unsafe {
        // Create AX element for the application
        let app_element = AXUIElementCreateApplication(app_pid);
        if app_element.is_null() {
            tracing::debug!(pid = app_pid, "Failed to create AX element for PID");
            return Ok(None);
        }

        // Query focused window
        let focused_window_attr = CFString::from_static_string("AXFocusedWindow");
        let mut focused_window: CFTypeRef = std::ptr::null();
        let result = AXUIElementCopyAttributeValue(
            app_element,
            focused_window_attr.as_concrete_TypeRef(),
            &mut focused_window,
        );

        // Release app element
        CFRelease(app_element.cast());

        if result != K_AX_ERROR_SUCCESS || focused_window.is_null() {
            tracing::trace!(pid = app_pid, "No focused window for PID");
            return Ok(None);
        }

        // Query window title
        let title_attr = CFString::from_static_string("AXTitle");
        let mut title_ref: CFTypeRef = std::ptr::null();
        let title_result = AXUIElementCopyAttributeValue(
            focused_window.cast(),
            title_attr.as_concrete_TypeRef(),
            &mut title_ref,
        );

        // Release focused window
        CFRelease(focused_window);

        if title_result != K_AX_ERROR_SUCCESS || title_ref.is_null() {
            tracing::trace!(pid = app_pid, "No title for focused window");
            return Ok(None);
        }

        // Convert CFString to Rust String
        // SAFETY: wrap_under_create_rule takes ownership of the CFString and will
        // release it
        let cf_title = CFString::wrap_under_create_rule(title_ref.cast());
        let rust_title = cf_title.to_string();

        if rust_title.is_empty() {
            Ok(None)
        } else {
            Ok(Some(rust_title))
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get_focused_window_title(_app_pid: i32) -> DomainResult<Option<String>> {
    Err(PulseArcError::Platform("Accessibility API is only available on macOS".to_string()))
}

/// Get main window title for a specific application PID (regardless of focus).
///
/// This helper queries the `AXMainWindow` attribute and falls back gracefully
/// when an app does not expose a main window or Accessibility permissions are
/// denied.
#[cfg(target_os = "macos")]
fn get_main_window_title(app_pid: i32) -> DomainResult<Option<String>> {
    if !check_ax_permission(false)? {
        return Ok(None);
    }

    unsafe {
        let app_element = AXUIElementCreateApplication(app_pid);
        if app_element.is_null() {
            tracing::trace!(pid = app_pid, "Failed to create AX element for PID (main window)");
            return Ok(None);
        }

        let main_window_attr = CFString::from_static_string("AXMainWindow");
        let mut main_window: CFTypeRef = std::ptr::null();
        let main_window_status = AXUIElementCopyAttributeValue(
            app_element,
            main_window_attr.as_concrete_TypeRef(),
            &mut main_window,
        );

        if main_window_status != K_AX_ERROR_SUCCESS || main_window.is_null() {
            CFRelease(app_element.cast());
            return Ok(None);
        }

        let title_attr = CFString::from_static_string("AXTitle");
        let mut title_ref: CFTypeRef = std::ptr::null();
        let title_status = AXUIElementCopyAttributeValue(
            main_window.cast(),
            title_attr.as_concrete_TypeRef(),
            &mut title_ref,
        );

        CFRelease(main_window);
        CFRelease(app_element.cast());

        if title_status != K_AX_ERROR_SUCCESS || title_ref.is_null() {
            return Ok(None);
        }

        let cf_title = CFString::wrap_under_create_rule(title_ref.cast());
        let title = cf_title.to_string();

        if title.is_empty() {
            Ok(None)
        } else {
            Ok(Some(title))
        }
    }
}

/// Get active app info from NSWorkspace with optional window title via AX.
///
/// Combines NSWorkspace app info with Accessibility API window title.
/// Falls back to app-only mode if AX permission not granted.
///
/// # Returns
///
/// * `Ok((app_name, bundle_id, window_title_opt, pid))` - Full app context
/// * `Err(PulseArcError::Platform)` - No active app, NSWorkspace unavailable,
///   or non-macOS
///
/// # Behavior
///
/// - Always returns app_name, bundle_id, pid from NSWorkspace
/// - Returns window_title only if Accessibility permission granted
/// - Caches permission state to avoid repeated checks
/// - Never panics on permission denial (graceful degradation)
///
/// # Platform Support
///
/// * macOS: Full support
/// * Other: Returns `Err(PulseArcError::Platform)`
///
/// # Examples
///
/// ```rust,ignore
/// let info = get_active_app_info()?;
/// tracing::info!(app = %info.app_name, bundle_id = %info.bundle_id, pid = info.pid, "Active app");
/// if let Some(title) = info.window_title {
///     tracing::debug!(window_title = %title, "Got window title");
/// }
/// ```
#[cfg(target_os = "macos")]
pub fn get_active_app_info() -> DomainResult<ActiveAppInfo> {
    // Get frontmost application from NSWorkspace
    let workspace = NSWorkspace::sharedWorkspace();
    let frontmost_app = workspace
        .frontmostApplication()
        .ok_or_else(|| PulseArcError::Platform("No frontmost application available".to_string()))?;

    // Extract app name
    let app_name_ns = frontmost_app.localizedName().ok_or_else(|| {
        PulseArcError::Platform("Could not get app name from NSWorkspace".to_string())
    })?;
    let app_name = app_name_ns.to_string();

    // Extract bundle ID
    let bundle_id_ns = frontmost_app.bundleIdentifier().ok_or_else(|| {
        PulseArcError::Platform("Could not get bundle ID from NSWorkspace".to_string())
    })?;
    let bundle_id = bundle_id_ns.to_string();

    // Extract PID
    let pid = frontmost_app.processIdentifier();

    // Try to get window title if AX permission granted
    let has_permission = check_ax_permission(false)?;
    let window_title = if has_permission {
        get_focused_window_title(pid)?
    } else {
        let permission_error = ax_permission_error();
        tracing::warn!(
            error = %permission_error,
            bundle_id = %bundle_id,
            "Accessibility permission not granted - window title unavailable"
        );
        None
    };

    tracing::debug!(
        app_name = %app_name,
        bundle_id = %bundle_id,
        pid = pid,
        has_window_title = window_title.is_some(),
        "Fetched active app info"
    );

    Ok(ActiveAppInfo { app_name, bundle_id, window_title, pid })
}

#[cfg(not(target_os = "macos"))]
pub fn get_active_app_info() -> DomainResult<ActiveAppInfo> {
    Err(PulseArcError::Platform("NSWorkspace API is only available on macOS".to_string()))
}

/// Get list of running applications (excluding background-only apps).
///
/// Uses NSWorkspace to get running apps without AppleScript overhead.
///
/// # Arguments
///
/// * `exclude_bundle_id` - Optional bundle ID to exclude (typically the current
///   app)
/// * `limit` - Maximum number of apps to return
///
/// # Returns
///
/// * `Ok(Vec<(app_name, bundle_id, window_title_opt)>)` - Running apps info
/// * `Err(_)` - Only on non-macOS platforms
///
/// # Behavior
///
/// - Filters out background-only apps (activationPolicy != Regular)
/// - Excludes specified bundle ID if provided
/// - Returns window titles only if AX permission granted
/// - Limited to `limit` apps
///
/// # Platform Support
///
/// * macOS: Full support
/// * Other: Returns `Err(PulseArcError::Platform)`
///
/// # Examples
///
/// ```rust,ignore
/// let recent_apps = get_recent_apps(Some("com.apple.Terminal"), 10)?;
/// for app in recent_apps {
///     tracing::info!(app = %app.app_name, bundle_id = %app.bundle_id, "Recent app");
/// }
/// ```
#[cfg(target_os = "macos")]
pub fn get_recent_apps(
    exclude_bundle_id: Option<&str>,
    limit: usize,
) -> DomainResult<Vec<RecentAppInfo>> {
    use objc2_app_kit::NSApplicationActivationPolicy;

    let workspace = NSWorkspace::sharedWorkspace();
    let running_apps = workspace.runningApplications();

    let has_ax_permission = check_ax_permission(false)?;
    if !has_ax_permission {
        let permission_error = ax_permission_error();
        tracing::debug!(
            error = %permission_error,
            "Accessibility permission denied - returning recent apps without window titles"
        );
    }

    let apps: Vec<RecentAppInfo> = running_apps
        .iter()
        .filter(|app| {
            // Filter: Only regular apps (not background-only)
            app.activationPolicy() == NSApplicationActivationPolicy::Regular
        })
        .filter_map(|app| {
            let bundle_id_ns = app.bundleIdentifier()?;
            let bundle_id = bundle_id_ns.to_string();

            // Filter: Exclude specified bundle ID
            if let Some(exclude) = exclude_bundle_id {
                if bundle_id == exclude {
                    return None;
                }
            }

            let app_name_ns = app.localizedName()?;
            let app_name = app_name_ns.to_string();

            // Get window title if AX permission granted
            let window_title = if has_ax_permission {
                let pid = app.processIdentifier();
                match get_main_window_title(pid) {
                    Ok(title) => {
                        if title.is_some() {
                            title
                        } else {
                            // Fallback to focused window for foreground app if AXMainWindow failed
                            get_focused_window_title(pid).ok().flatten()
                        }
                    }
                    Err(err) => {
                        tracing::debug!(
                            error = %err,
                            pid = pid,
                            "Failed to read main window title via AX"
                        );
                        None
                    }
                }
            } else {
                None
            };

            Some(RecentAppInfo { app_name, bundle_id, window_title })
        })
        .take(limit)
        .collect();

    tracing::debug!(count = apps.len(), limit = limit, "Fetched recent apps");

    Ok(apps)
}

#[cfg(not(target_os = "macos"))]
pub fn get_recent_apps(
    _exclude_bundle_id: Option<&str>,
    _limit: usize,
) -> DomainResult<Vec<RecentAppInfo>> {
    Err(PulseArcError::Platform("NSWorkspace API is only available on macOS".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_ax_permission_compiles() {
        // Verify function signature compiles
        let _result = check_ax_permission(false);
    }

    #[test]
    fn test_get_focused_window_title_compiles() {
        // Verify function signature compiles
        let _result = get_focused_window_title(1234);
    }

    #[test]
    fn test_get_active_app_info_compiles() {
        // Verify function signature compiles
        let _result = get_active_app_info();
    }

    #[test]
    fn test_get_recent_apps_compiles() {
        // Verify function signature compiles
        let _result = get_recent_apps(Some("com.test.App"), 10);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_ax_permission_cache() {
        // First call should query system
        let first = check_ax_permission(false);
        assert!(first.is_ok());

        // Second call should use cache (same result)
        let second = check_ax_permission(false);
        assert!(first.unwrap() == second.unwrap());
    }
}
