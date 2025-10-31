#![cfg(target_os = "macos")]

use std::sync::Arc;

use macos_ax::{get_active_app_info, get_recent_apps};

#[derive(Debug, Clone)]
pub struct WindowContext {
    pub app_name: String,
    pub window_title: String,
    pub bundle_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ActivityContext {
    pub active_app: WindowContext,
    pub recent_apps: Vec<WindowContext>,
}

#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("{0}")]
    Message(String),
}

pub trait ActivityProvider {
    fn fetch(&self) -> Result<ActivityContext, ActivityError>;
}

pub struct MacOsActivityProvider {
    debug: bool,
    _enable_enrichment: bool,
    _metrics: Arc<()>,
}

impl MacOsActivityProvider {
    pub fn new(debug: bool) -> Self {
        Self::with_enrichment(debug, true)
    }

    pub fn with_enrichment(debug: bool, enable_enrichment: bool) -> Self {
        Self { debug, _enable_enrichment: enable_enrichment, _metrics: Arc::new(()) }
    }

    pub fn with_config(debug: bool, enable_enrichment: bool, _background: bool) -> Self {
        Self::with_enrichment(debug, enable_enrichment)
    }

    pub fn fetch(&self) -> Result<ActivityContext, ActivityError> {
        if !crate::macos::macos_ax::check_ax_permission(false) {
            return Err(ActivityError::Message("Accessibility permission not granted".to_string()));
        }

        let (app_name, bundle_id, window_title, _pid) = get_active_app_info()
            .ok_or_else(|| ActivityError::Message("Failed to get active app info".into()))?;

        if self.debug {
            log::debug!("Active app: {} ({})", app_name, bundle_id);
        }

        let active_app = WindowContext {
            app_name: app_name.clone(),
            window_title: window_title.unwrap_or_default(),
            bundle_id: if bundle_id.is_empty() { None } else { Some(bundle_id) },
        };

        let recent_apps = get_recent_apps(active_app.bundle_id.as_deref(), 10)
            .into_iter()
            .map(|(name, bundle, title)| WindowContext {
                app_name: name,
                window_title: title.unwrap_or_default(),
                bundle_id: Some(bundle),
            })
            .collect();

        Ok(ActivityContext { active_app, recent_apps })
    }
}

impl ActivityProvider for MacOsActivityProvider {
    fn fetch(&self) -> Result<ActivityContext, ActivityError> {
        self.fetch()
    }
}

mod macos_ax {
    use core_foundation::base::{CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::{CFString, CFStringRef};
    use objc2_app_kit::{NSApplicationActivationPolicy, NSWorkspace};
    use std::sync::OnceLock;

    #[repr(C)]
    struct __AXUIElement(core::ffi::c_void);
    type AXUIElementRef = *const __AXUIElement;

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

    const K_AX_ERROR_SUCCESS: i32 = 0;
    static AX_PERMISSION_CACHE: OnceLock<bool> = OnceLock::new();

    pub fn check_ax_permission(prompt: bool) -> bool {
        if let Some(&cached) = AX_PERMISSION_CACHE.get() {
            return cached;
        }

        unsafe {
            let prompt_key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
            let prompt_value = CFBoolean::from(prompt);
            let options = CFDictionary::from_CFType_pairs(&[(
                prompt_key.as_CFType(),
                prompt_value.as_CFType(),
            )]);

            let is_trusted = AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef().cast());
            let _ = AX_PERMISSION_CACHE.set(is_trusted);
            is_trusted
        }
    }

    pub fn get_active_app_info() -> Option<(String, String, Option<String>, i32)> {
        let workspace = NSWorkspace::sharedWorkspace();
        let active_app = workspace.frontmostApplication()?;
        let bundle_id = active_app.bundleIdentifier()?.to_string();
        let app_name = active_app.localizedName()?.to_string();
        let pid = active_app.processIdentifier();
        let window_title = get_focused_window_title(pid);
        Some((app_name, bundle_id, window_title, pid))
    }

    pub fn get_recent_apps(
        current_bundle_id: Option<&str>,
        limit: usize,
    ) -> Vec<(String, String, Option<String>)> {
        let workspace = NSWorkspace::sharedWorkspace();
        let running_apps = workspace.runningApplications();
        let has_permission = crate::macos::macos_ax::check_ax_permission(false);
        running_apps
            .iter()
            .filter(|app| app.activationPolicy() == NSApplicationActivationPolicy::Regular)
            .filter_map(|app| {
                let bundle = app.bundleIdentifier()?.to_string();
                if current_bundle_id.map(|id| id == bundle).unwrap_or(false) {
                    return None;
                }
                let name = app.localizedName()?.to_string();
                let title = if has_permission {
                    let pid = app.processIdentifier();
                    get_focused_window_title(pid)
                } else {
                    None
                };
                Some((name, bundle, title))
            })
            .take(limit)
            .collect()
    }

    pub fn get_focused_window_title(app_pid: i32) -> Option<String> {
        if !crate::macos::macos_ax::check_ax_permission(false) {
            return None;
        }

        unsafe {
            let app_element = AXUIElementCreateApplication(app_pid);
            if app_element.is_null() {
                return None;
            }

            let focused_window_attr = CFString::from_static_string("AXFocusedWindow");
            let mut focused_window: CFTypeRef = std::ptr::null();
            let result = AXUIElementCopyAttributeValue(
                app_element,
                focused_window_attr.as_concrete_TypeRef(),
                &mut focused_window,
            );
            CFRelease(app_element.cast());

            if result != K_AX_ERROR_SUCCESS || focused_window.is_null() {
                return None;
            }

            let title_attr = CFString::from_static_string("AXTitle");
            let mut title_ref: CFTypeRef = std::ptr::null();
            let title_result = AXUIElementCopyAttributeValue(
                focused_window.cast(),
                title_attr.as_concrete_TypeRef(),
                &mut title_ref,
            );
            CFRelease(focused_window);

            if title_result != K_AX_ERROR_SUCCESS || title_ref.is_null() {
                return None;
            }

            let cf_title = CFString::wrap_under_create_rule(title_ref.cast());
            let rust_title = cf_title.to_string();

            if rust_title.is_empty() {
                None
            } else {
                Some(rust_title)
            }
        }
    }
}

pub fn check_ax_permission(prompt: bool) -> bool {
    crate::macos::macos_ax::check_ax_permission(prompt)
}
