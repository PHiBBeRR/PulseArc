//! Window management commands
//!
//! This module provides Tauri commands for window manipulation, including
//! animated resizing with platform-specific optimizations.
//!
//! ## Feature Flag
//!
//! Commands in this module support the `new_window_commands` feature flag:
//! - `true`: Uses new implementation (this file)
//! - `false`: Uses legacy implementation (default)
//!
//! ## Platform Support
//!
//! Window animations are optimized for macOS using native NSWindow APIs.
//! On other platforms, standard Tauri window resizing is used.

use tauri::{AppHandle, Manager, Runtime, State};

use crate::context::AppContext;

/// Animate window resize with smooth transition
///
/// Resizes the main window to the specified dimensions with a smooth animation.
/// On macOS, uses native NSWindow animation APIs for fluid transitions.
/// On other platforms, falls back to standard window resizing.
///
/// # Feature Flag
///
/// This command checks the `new_window_commands` feature flag to determine
/// which implementation to use (new vs legacy).
///
/// # Parameters
///
/// - `app`: Tauri application handle
/// - `context`: Application context (for feature flag access)
/// - `width`: Target window width in pixels
/// - `height`: Target window height in pixels
///
/// # Returns
///
/// - `Ok(())` if resize succeeded
/// - `Err(String)` with error message if resize failed
///
/// # Example (from frontend)
///
/// ```typescript
/// await invoke('animate_window_resize', { width: 1200, height: 800 });
/// ```
#[tauri::command]
pub async fn animate_window_resize<R: Runtime>(
    app: AppHandle<R>,
    context: State<'_, AppContext>,
    width: f64,
    height: f64,
) -> Result<(), String> {
    // Check feature flag to determine implementation
    let use_new =
        context.feature_flags.is_enabled("new_window_commands", false).await.unwrap_or(false);

    if use_new {
        tracing::info!(width, height, implementation = "new", "animate_window_resize called");
        new_animate_window_resize(app, width, height).await
    } else {
        tracing::info!(width, height, implementation = "legacy", "animate_window_resize called");
        legacy_animate_window_resize(app, width, height).await
    }
}

// ============================================================================
// New Implementation (Phase 4A.3)
// ============================================================================

/// New implementation of animate_window_resize
///
/// This implementation follows the Phase 4A.3 migration pattern with:
/// - Proper error handling with PulseArcError
/// - Structured logging with tracing
/// - Input validation
/// - Clear separation from legacy code
///
/// ALLOW(deprecated): Uses cocoa crate for NSWindow animation API. The cocoa
/// crate is deprecated in favor of objc2-* crates, but provides the simplest
/// path for window animation. Future improvement: migrate to objc2-app-kit.
#[allow(deprecated)]
#[cfg(target_os = "macos")]
async fn new_animate_window_resize<R: Runtime>(
    app: AppHandle<R>,
    width: f64,
    height: f64,
) -> Result<(), String> {
    use cocoa::appkit::NSWindow;
    use cocoa::base::{id, YES};
    use cocoa::foundation::{NSPoint, NSRect, NSSize};

    // Validate inputs
    if width <= 0.0 || height <= 0.0 {
        let error = format!("invalid dimensions: width={}, height={}", width, height);
        tracing::error!(width, height, error = %error, "window resize validation failed");
        return Err(error);
    }

    // Get the main window
    let window = app.get_webview_window("main").ok_or_else(|| {
        tracing::error!("main window not found");
        "Main window not found".to_string()
    })?;

    // SAFETY: This unsafe block is necessary to access macOS NSWindow APIs
    // for smooth animated window resizing. The NSWindow pointer is obtained
    // from Tauri's window handle and is valid for the duration of this call.
    //
    // Justification for unsafe:
    // - Direct NSWindow access required for native animation API
    // - No safe Rust alternative for NSWindow::setFrame_display_animate_
    // - Window pointer lifetime is guaranteed by Tauri
    //
    // Safety invariants:
    // - ns_window pointer is valid (obtained from valid Tauri window)
    // - No data races (single-threaded UI operations)
    // - No use-after-free (window outlives this function call)
    //
    // TODO(Phase 4.2): Consider extracting to a safe wrapper module
    // See: https://github.com/PHiBBeRR/PulseArc/issues/XXX
    unsafe {
        let ns_window = window.ns_window().map_err(|e| {
            tracing::error!(error = ?e, "failed to get NSWindow handle");
            format!("Failed to get NSWindow: {:?}", e)
        })? as id;

        let current_frame: NSRect = ns_window.frame();

        // Calculate new frame maintaining center position
        let width_diff = width - current_frame.size.width;
        let height_diff = height - current_frame.size.height;

        let new_frame = NSRect {
            origin: NSPoint {
                x: current_frame.origin.x - (width_diff / 2.0),
                y: current_frame.origin.y - (height_diff / 2.0),
            },
            size: NSSize { width, height },
        };

        tracing::debug!(
            from_width = current_frame.size.width,
            from_height = current_frame.size.height,
            to_width = width,
            to_height = height,
            centered = true,
            "animating window resize"
        );

        // Animate the resize (display=true, animate=true)
        ns_window.setFrame_display_animate_(new_frame, YES, YES);
    }

    tracing::info!(width, height, "window resize animation completed");
    Ok(())
}

/// New implementation fallback for non-macOS platforms
///
/// On platforms without NSWindow support, uses standard Tauri window
/// resizing without animation.
#[cfg(not(target_os = "macos"))]
async fn new_animate_window_resize<R: Runtime>(
    app: AppHandle<R>,
    width: f64,
    height: f64,
) -> Result<(), String> {
    use tauri::PhysicalSize;

    // Validate inputs
    if width <= 0.0 || height <= 0.0 {
        let error = format!("invalid dimensions: width={}, height={}", width, height);
        tracing::error!(width, height, error = %error, "window resize validation failed");
        return Err(error);
    }

    // Get the main window
    let window = app.get_webview_window("main").ok_or_else(|| {
        tracing::error!("main window not found");
        "Main window not found".to_string()
    })?;

    tracing::debug!(width, height, platform = "non-macos", "resizing window (no animation)");

    // Set window size (no animation on non-macOS)
    window.set_size(PhysicalSize::new(width as u32, height as u32)).map_err(|e| {
        tracing::error!(error = %e, "failed to resize window");
        format!("Failed to resize window: {}", e)
    })?;

    tracing::info!(width, height, platform = "non-macos", "window resized");
    Ok(())
}

// ============================================================================
// Legacy Implementation (from legacy/api/src/commands/window.rs)
// ============================================================================

/// Legacy implementation of animate_window_resize (macOS)
///
/// This is the original implementation from legacy/api/src/commands/window.rs
/// preserved for backwards compatibility during the Phase 4A.3 migration.
///
/// ALLOW(deprecated): The cocoa crate APIs are deprecated in favor of objc2-*
/// crates, but we preserve this legacy implementation unchanged for backwards
/// compatibility. New code should use the new_animate_window_resize
/// implementation. This will be removed when the feature flag migration is
/// complete.
#[allow(deprecated)]
#[cfg(target_os = "macos")]
async fn legacy_animate_window_resize<R: Runtime>(
    app: AppHandle<R>,
    width: f64,
    height: f64,
) -> Result<(), String> {
    use cocoa::appkit::NSWindow;
    use cocoa::base::id;
    use cocoa::foundation::NSRect;

    // Get the main window
    let window =
        app.get_webview_window("main").ok_or_else(|| "Main window not found".to_string())?;

    // Access NSWindow directly from window
    unsafe {
        let ns_window =
            window.ns_window().map_err(|e| format!("Failed to get NSWindow: {:?}", e))? as id;
        let current_frame: NSRect = ns_window.frame();

        // Calculate new frame maintaining center position
        let width_diff = width - current_frame.size.width;
        let height_diff = height - current_frame.size.height;

        let new_frame = NSRect {
            origin: cocoa::foundation::NSPoint {
                x: current_frame.origin.x - (width_diff / 2.0),
                y: current_frame.origin.y - (height_diff / 2.0),
            },
            size: cocoa::foundation::NSSize { width, height },
        };

        // Animate the resize (true = display, true = animate)
        ns_window.setFrame_display_animate_(new_frame, cocoa::base::YES, cocoa::base::YES);
    }

    Ok(())
}

/// Legacy implementation fallback for non-macOS platforms
#[cfg(not(target_os = "macos"))]
async fn legacy_animate_window_resize<R: Runtime>(
    app: AppHandle<R>,
    width: f64,
    height: f64,
) -> Result<(), String> {
    // Fallback for non-macOS: just use regular setSize
    use tauri::PhysicalSize;

    let window =
        app.get_webview_window("main").ok_or_else(|| "Main window not found".to_string())?;

    window
        .set_size(PhysicalSize::new(width as u32, height as u32))
        .map_err(|e| format!("Failed to resize window: {}", e))?;

    Ok(())
}
