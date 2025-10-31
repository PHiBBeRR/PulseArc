//! macOS Event Listener using NSWorkspace notifications
//!
//! Implements event-driven activity detection using NSWorkspace notifications.
//! Fires only when apps actually switch, eliminating polling overhead.
//!
//! # Architecture
//! - Uses objc2-app-kit for NSWorkspace bindings
//! - Uses block2 for Objective-C block callbacks
//! - Registers observer with NSNotificationCenter
//! - Delivers callbacks on NSOperationQueue (off main thread)
//!
//! # Memory Management
//! All Objective-C resources are explicitly owned via `Retained<T>` types.
//! The Drop trait ensures proper cleanup in the correct order:
//! 1. Remove observer (stops callbacks)
//! 2. Drop block (safe now, NC no longer references it)
//! 3. Drop queue (can finish in-flight operations)
//!
//! # Example
//! ```rust,no_run
//! use pulsearc_infra::platform::macos::event_listener::MacOsEventListener;
//!
//! let mut listener = MacOsEventListener::new();
//! listener.start(Box::new(|| {
//!     println!("App switched!");
//! }))?;
//! # Ok::<(), String>(())
//! ```

use std::ptr::NonNull;

#[cfg(target_os = "macos")]
use block2::RcBlock;
#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
#[cfg(target_os = "macos")]
use objc2_app_kit::NSWorkspace;
#[cfg(target_os = "macos")]
use objc2_foundation::{NSNotification, NSNotificationCenter, NSOperationQueue, NSString};

#[cfg(target_os = "macos")]
type ObserverToken = Retained<ProtocolObject<dyn NSObjectProtocol>>;
#[cfg(target_os = "macos")]
type NotificationBlock = RcBlock<dyn Fn(NonNull<NSNotification>)>;

/// Trait for platform-specific OS event listeners
///
/// Implementations of this trait provide OS-level hooks to detect application
/// switches without polling.
pub trait OsEventListener: Send + Sync {
    /// Start listening for app switch events
    ///
    /// The provided callback will be invoked whenever the active application
    /// changes. The callback must be Send + Sync to allow execution on OS
    /// notification threads.
    ///
    /// # Arguments
    /// * `callback` - Closure to invoke when app switch detected
    ///
    /// # Returns
    /// * `Ok(())` - Listener started successfully
    /// * `Err(String)` - Failed to start listener
    fn start(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<(), String>;

    /// Stop listening and cleanup resources
    ///
    /// This method should be idempotent - calling it multiple times should be
    /// safe. It should remove any OS-level observers and free associated
    /// resources.
    ///
    /// # Returns
    /// * `Ok(())` - Cleanup successful
    /// * `Err(String)` - Cleanup failed (logged but typically non-fatal)
    fn stop(&mut self) -> Result<(), String>;

    /// Check if OS events are supported on this platform
    ///
    /// This is a static method that can be called without instantiating the
    /// listener.
    ///
    /// # Returns
    /// * `true` - OS events are supported and can be used
    /// * `false` - OS events not supported, caller should use polling
    fn is_supported() -> bool
    where
        Self: Sized;
}

/// macOS event listener using NSWorkspace notifications
///
/// This implementation uses NSWorkspace.didActivateApplicationNotification to
/// detect app switches without polling, reducing CPU usage from ~5% to <1%.
///
/// # Platform Support
/// Only available on macOS. On other platforms, use [`FallbackEventListener`].
#[cfg(target_os = "macos")]
pub struct MacOsEventListener {
    /// Notification center (from NSWorkspace)
    nc: Option<Retained<NSNotificationCenter>>,
    /// Observer token (needed to remove observer)
    observer_token: Option<ObserverToken>,
    /// Operation queue (callbacks execute here)
    queue: Option<Retained<NSOperationQueue>>,
    /// Block keepalive (CRITICAL: prevents use-after-free)
    block_keepalive: Option<NotificationBlock>,
}

#[cfg(not(target_os = "macos"))]
pub struct MacOsEventListener;

impl MacOsEventListener {
    /// Create a new macOS event listener
    ///
    /// All fields start as None. Call `start()` to register observer.
    #[must_use]
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self { nc: None, observer_token: None, queue: None, block_keepalive: None }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self
        }
    }
}

impl Default for MacOsEventListener {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: MacOsEventListener is Send + Sync because:
// 1. All Objective-C objects are accessed only through Retained<T> (thread-safe
//    reference counting)
// 2. NSNotificationCenter is documented as thread-safe for observer operations
// 3. NSOperationQueue handles its own thread safety for callback execution
// 4. The block captures only Send + Sync Rust types (Arc<callback>)
// 5. All mutations happen through &mut self (exclusive access)
#[cfg(target_os = "macos")]
unsafe impl Send for MacOsEventListener {}
#[cfg(target_os = "macos")]
unsafe impl Sync for MacOsEventListener {}

#[cfg(target_os = "macos")]
impl Drop for MacOsEventListener {
    fn drop(&mut self) {
        // CRITICAL ORDER: Remove observer before dropping block/queue
        // 1. Remove observer (stops callbacks from firing)
        if let Some(tok) = self.observer_token.take() {
            if let Some(nc) = &self.nc {
                unsafe {
                    // SAFETY: Transmute ProtocolObject<dyn NSObjectProtocol> to AnyObject
                    // This is safe because:
                    // 1. ProtocolObject<P> is repr(transparent) over AnyObject (as of objc2 v0.5+)
                    // 2. The lifetime of the reference is preserved (&tok -> &AnyObject)
                    // 3. We're only using this for removeObserver, which accepts AnyObject
                    // 4. objc2 guarantees this layout won't change within major version
                    // 5. Both types have the same size and alignment
                    let observer_ref = std::mem::transmute::<
                        &ProtocolObject<dyn NSObjectProtocol>,
                        &objc2::runtime::AnyObject,
                    >(&*tok);
                    nc.removeObserver(observer_ref);
                }
                tracing::debug!(phase = "drop", removed = true, "NSWorkspace observer removed");
            }
        }

        // 2. Drop block (safe now, NC no longer references it)
        self.block_keepalive = None;

        // 3. Drop queue (can finish in-flight operations)
        self.queue = None;

        // 4. Drop notification center
        self.nc = None;
    }
}

impl OsEventListener for MacOsEventListener {
    #[cfg(target_os = "macos")]
    fn start(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<(), String> {
        tracing::debug!(
            has_observer = self.observer_token.is_some(),
            "MacOsEventListener::start() called"
        );

        let start_time = std::time::Instant::now();

        // Prevent double-registration
        if self.observer_token.is_some() {
            tracing::warn!(has_observer = true, "Observer already started - returning error");
            return Err("Observer already started".to_string());
        }

        tracing::debug!(phase = "enter_unsafe", "Accessing NSWorkspace APIs");
        let result: Result<(), String> = unsafe {
            // Get NSWorkspace and notification center
            tracing::trace!(
                phase = "workspace_init",
                step = "shared_workspace",
                "Getting NSWorkspace::sharedWorkspace()"
            );
            let workspace = NSWorkspace::sharedWorkspace();
            tracing::trace!(
                phase = "workspace_init",
                step = "notification_center",
                "Getting workspace.notificationCenter()"
            );
            let nc = workspace.notificationCenter();
            tracing::trace!(
                phase = "workspace_init",
                status = "notification_center_ready",
                "Notification center obtained successfully"
            );

            // Create serial queue for deterministic callback ordering
            tracing::trace!(
                phase = "queue_setup",
                action = "create_queue",
                "Creating NSOperationQueue"
            );
            let queue = NSOperationQueue::new();
            tracing::trace!(
                phase = "queue_setup",
                action = "set_max_concurrent",
                value = 1,
                "Setting max concurrent operations to 1"
            );
            queue.setMaxConcurrentOperationCount(1);
            tracing::trace!(
                phase = "queue_setup",
                status = "configured",
                "Operation queue configured successfully"
            );

            // Create block with Arc'd callback (avoid retain cycles)
            tracing::trace!(
                phase = "callback_setup",
                action = "create_block",
                "Creating callback block"
            );
            let callback_arc = std::sync::Arc::new(callback);
            let blk = RcBlock::new(move |_note: NonNull<NSNotification>| {
                tracing::trace!(phase = "callback", event = "fired");
                // Wrap in panic::catch_unwind for safety
                if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    // Call the actual callback
                    (callback_arc)();
                })) {
                    // Panic occurred - log it
                    let panic_msg = if let Some(s) = e.downcast_ref::<&str>() {
                        (*s).to_string()
                    } else if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Unknown panic".to_string()
                    };

                    tracing::error!(panic = %panic_msg, "Observer callback panicked");
                }
            });

            // Ensure heap allocation (CRITICAL for NSNotificationCenter retention)
            tracing::trace!(
                phase = "callback_setup",
                action = "copy_block",
                "Copying block to heap"
            );
            let blk = blk.copy();

            // Register observer for app activation notifications
            // NSWorkspaceDidActivateApplicationNotification is the notification name
            tracing::trace!(
                phase = "register",
                action = "create_notification",
                "Creating notification name string"
            );
            let notification_name =
                NSString::from_str("NSWorkspaceDidActivateApplicationNotification");
            tracing::trace!(
                phase = "register",
                notification = "NSWorkspaceDidActivateApplicationNotification"
            );

            tracing::trace!(
                phase = "register",
                action = "register_observer",
                "Registering observer with NSNotificationCenter"
            );
            let token = nc.addObserverForName_object_queue_usingBlock(
                Some(&notification_name),
                None,         // Observe all apps
                Some(&queue), // Callback on our serial queue
                &blk,
            );
            tracing::trace!(phase = "register", status = "token_received");

            // Store all resources (prevents premature deallocation)
            tracing::trace!(
                phase = "store",
                action = "stash_resources",
                "Storing observer resources"
            );
            self.nc = Some(nc);
            self.observer_token = Some(token);
            self.queue = Some(queue);
            self.block_keepalive = Some(blk);
            tracing::trace!(phase = "store", status = "complete");

            Ok(())
        };

        tracing::debug!(
            result = ?result.as_ref().map(|_| "Ok").map_err(|e| e.clone()),
            "Unsafe block completed"
        );

        // Record failure if error occurred
        if let Err(ref e) = result {
            tracing::error!(error = %e, "Observer initialization failed");
            return result;
        }

        // Record registration time on success
        let registration_ms = start_time.elapsed().as_millis() as u64;
        tracing::debug!(duration_ms = registration_ms, "Observer registration time");

        tracing::info!(duration_ms = registration_ms, "NSWorkspace observer registered");

        tracing::debug!(success = result.is_ok(), "MacOsEventListener::start() returning");
        result
    }

    #[cfg(not(target_os = "macos"))]
    fn start(&mut self, _callback: Box<dyn Fn() + Send + Sync>) -> Result<(), String> {
        Err("macOS NSWorkspace integration not available on this platform".to_string())
    }

    fn stop(&mut self) -> Result<(), String> {
        let start_time = std::time::Instant::now();

        // Same teardown as Drop, but callable early
        #[cfg(target_os = "macos")]
        {
            if let Some(tok) = self.observer_token.take() {
                if let Some(nc) = &self.nc {
                    unsafe {
                        // SAFETY: Same as Drop impl - transmute is safe because ProtocolObject
                        // is repr(transparent) over AnyObject in objc2 v0.5+
                        let observer_ref = std::mem::transmute::<
                            &ProtocolObject<dyn NSObjectProtocol>,
                            &objc2::runtime::AnyObject,
                        >(&*tok);
                        nc.removeObserver(observer_ref);
                    }
                    tracing::debug!(phase = "stop", removed = true, "NSWorkspace observer removed");
                }
            }

            self.block_keepalive = None;
            self.queue = None;
            self.nc = None;

            // Record cleanup time
            let cleanup_ms = start_time.elapsed().as_millis() as u64;
            tracing::info!(duration_ms = cleanup_ms, "NSWorkspace observer stopped");
        }

        Ok(())
    }

    fn is_supported() -> bool {
        #[cfg(target_os = "macos")]
        {
            true
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
}

/// Fallback event listener for non-macOS platforms
///
/// This implementation always returns errors, forcing the caller to use
/// polling-based activity detection.
#[cfg(not(target_os = "macos"))]
pub struct FallbackEventListener;

#[cfg(not(target_os = "macos"))]
impl FallbackEventListener {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_os = "macos"))]
impl Default for FallbackEventListener {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_os = "macos"))]
impl OsEventListener for FallbackEventListener {
    fn start(&mut self, _callback: Box<dyn Fn() + Send + Sync>) -> Result<(), String> {
        Err("OS event monitoring is only supported on macOS".to_string())
    }

    fn stop(&mut self) -> Result<(), String> {
        Ok(()) // No-op for fallback
    }

    fn is_supported() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listener_creation() {
        let _listener = MacOsEventListener::new();
    }

    #[test]
    fn test_listener_default() {
        let _listener = MacOsEventListener::default();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_is_supported_macos() {
        assert!(MacOsEventListener::is_supported());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn test_is_supported_non_macos() {
        assert!(!MacOsEventListener::is_supported());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_start_and_stop() {
        let mut listener = MacOsEventListener::new();
        let callback = Box::new(|| {});
        let result = listener.start(callback);
        assert!(result.is_ok(), "Start should succeed on macOS");

        let stop_result = listener.stop();
        assert!(stop_result.is_ok(), "Stop should succeed");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_double_start_fails() {
        let mut listener = MacOsEventListener::new();
        let callback1 = Box::new(|| {});
        let result1 = listener.start(callback1);
        assert!(result1.is_ok());

        let callback2 = Box::new(|| {});
        let result2 = listener.start(callback2);
        assert!(result2.is_err(), "Second start should fail");
        assert!(result2.unwrap_err().contains("Observer already started"));

        let _ = listener.stop();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_stop_idempotent() {
        let mut listener = MacOsEventListener::new();
        let callback = Box::new(|| {});
        listener.start(callback).unwrap();

        // Stop multiple times should all succeed
        assert!(listener.stop().is_ok());
        assert!(listener.stop().is_ok());
        assert!(listener.stop().is_ok());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn test_fallback_listener() {
        let mut listener = FallbackEventListener::new();
        let callback = Box::new(|| {});
        let result = listener.start(callback);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("only supported on macOS"));

        // Stop should be no-op
        assert!(listener.stop().is_ok());
    }
}
