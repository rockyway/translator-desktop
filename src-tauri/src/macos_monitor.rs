//! macOS native text selection monitor.
//!
//! Uses Core Graphics event taps directly (instead of rdev) to avoid
//! crashes from TSMGetInputSourceProperty being called off the main thread.
//!
//! Requires macOS Accessibility permission (System Settings > Privacy > Accessibility).

use crate::popup_handler::show_popup_with_text;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

/// Minimum drag distance (pixels) to consider it a text selection, not a click.
const MIN_DRAG_DISTANCE: f64 = 5.0;

/// Global flag to control the monitor thread.
static MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);

/// Check if macOS Accessibility permission is granted.
pub fn check_accessibility_permission() -> bool {
    unsafe { accessibility_sys::AXIsProcessTrusted() }
}

/// Request macOS Accessibility permission (opens System Settings dialog).
pub fn request_accessibility_permission() {
    unsafe {
        use core_foundation::base::TCFType;
        use core_foundation::boolean::CFBoolean;
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::string::CFString;

        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key, value.as_CFType())]);
        accessibility_sys::AXIsProcessTrustedWithOptions(
            options.as_concrete_TypeRef() as *const _
        );
    }
    log::info!("macOS: Accessibility permission dialog shown");
}

/// Get selected text from the currently focused application using Accessibility API.
fn get_selected_text() -> Option<String> {
    unsafe {
        use accessibility_sys::*;
        use core_foundation::base::{CFTypeRef, TCFType};
        use core_foundation::string::CFString;
        use core_foundation_sys::base::CFRelease;
        use std::ptr;

        let system = AXUIElementCreateSystemWide();

        // Get the focused application
        let mut focused_app: CFTypeRef = ptr::null();
        let attr = CFString::new("AXFocusedApplication");
        let err = AXUIElementCopyAttributeValue(
            system,
            attr.as_concrete_TypeRef(),
            &mut focused_app,
        );

        if err != kAXErrorSuccess || focused_app.is_null() {
            CFRelease(system as _);
            log::debug!("macOS: No focused application found (err: {})", err);
            return None;
        }

        // Get the focused UI element within that application
        let mut focused_element: CFTypeRef = ptr::null();
        let attr = CFString::new("AXFocusedUIElement");
        let err = AXUIElementCopyAttributeValue(
            focused_app as AXUIElementRef,
            attr.as_concrete_TypeRef(),
            &mut focused_element,
        );

        CFRelease(focused_app);

        if err != kAXErrorSuccess || focused_element.is_null() {
            CFRelease(system as _);
            log::debug!("macOS: No focused element found (err: {})", err);
            return None;
        }

        // Read AXSelectedText from the focused element
        let mut selected_text: CFTypeRef = ptr::null();
        let attr = CFString::new("AXSelectedText");
        let err = AXUIElementCopyAttributeValue(
            focused_element as AXUIElementRef,
            attr.as_concrete_TypeRef(),
            &mut selected_text,
        );

        CFRelease(focused_element);
        CFRelease(system as _);

        if err != kAXErrorSuccess || selected_text.is_null() {
            log::debug!("macOS: No selected text found (err: {})", err);
            return None;
        }

        // Convert CFStringRef to Rust String
        let cf_string = CFString::wrap_under_create_rule(selected_text as _);
        let text = cf_string.to_string();

        if text.trim().is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

/// Selection detection state.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SelectionState {
    Idle,
    Selecting,
}

/// Shared state for the selection state machine.
struct MonitorState {
    state: SelectionState,
    modifier_held: bool,
    mouse_down: bool,
    start_x: f64,
    start_y: f64,
    last_x: f64,
    last_y: f64,
}

impl MonitorState {
    fn new() -> Self {
        Self {
            state: SelectionState::Idle,
            modifier_held: false,
            mouse_down: false,
            start_x: 0.0,
            start_y: 0.0,
            last_x: 0.0,
            last_y: 0.0,
        }
    }
}

// CGEventFlags bit masks
const K_CG_EVENT_FLAG_MASK_ALTERNATE: u64 = 0x00080000; // Option/Alt key

/// Start the macOS native text selection monitor.
/// Spawns a background thread using CGEventTap for global input events.
pub fn start_monitor(app_handle: AppHandle) -> Result<(), String> {
    if MONITOR_RUNNING.load(Ordering::SeqCst) {
        log::info!("macOS monitor: Already running");
        return Ok(());
    }

    MONITOR_RUNNING.store(true, Ordering::SeqCst);

    // Emit connected status so frontend knows monitor is active
    let _ = app_handle.emit("ipc-connected", serde_json::json!({
        "connected": true,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }));

    let monitor_state = Arc::new(Mutex::new(MonitorState::new()));

    thread::spawn(move || {
        let state = monitor_state;
        let app = app_handle;

        log::info!("macOS monitor: Starting CGEventTap listener");

        {
            use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
            use core_graphics::event::{CGEventTap, CGEventTapLocation, CGEventTapPlacement, CGEventTapOptions, CGEventType};

            let state_clone = state.clone();
            let app_clone = app.clone();

            let tap = CGEventTap::new(
                CGEventTapLocation::HID,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                vec![
                    CGEventType::LeftMouseDown,
                    CGEventType::LeftMouseUp,
                    CGEventType::MouseMoved,
                    CGEventType::LeftMouseDragged,
                    CGEventType::FlagsChanged,
                ],
                move |_proxy, event_type, event| {
                    if !MONITOR_RUNNING.load(Ordering::SeqCst) {
                        return None;
                    }

                    let mut ms = match state_clone.lock() {
                        Ok(s) => s,
                        Err(_) => return None,
                    };

                    let flags = event.get_flags().bits();
                    let location = event.location();

                    match event_type {
                        CGEventType::FlagsChanged => {
                            let alt_held = (flags & K_CG_EVENT_FLAG_MASK_ALTERNATE) != 0;
                            let was_held = ms.modifier_held;
                            ms.modifier_held = alt_held;

                            if was_held && !alt_held && ms.state == SelectionState::Selecting {
                                ms.state = SelectionState::Idle;
                                ms.mouse_down = false;
                                log::debug!("macOS monitor: Selection cancelled (modifier released)");
                            }
                        }

                        CGEventType::LeftMouseDown => {
                            ms.mouse_down = true;
                            ms.start_x = location.x;
                            ms.start_y = location.y;
                            ms.last_x = location.x;
                            ms.last_y = location.y;

                            if ms.modifier_held {
                                ms.state = SelectionState::Selecting;
                                log::debug!("macOS monitor: Selection started at ({}, {})", ms.start_x, ms.start_y);
                            }
                        }

                        CGEventType::MouseMoved | CGEventType::LeftMouseDragged => {
                            ms.last_x = location.x;
                            ms.last_y = location.y;
                        }

                        CGEventType::LeftMouseUp => {
                            ms.mouse_down = false;
                            let end_x = location.x;
                            let end_y = location.y;

                            let was_selecting = ms.state == SelectionState::Selecting;
                            ms.state = SelectionState::Idle;

                            let should_check = if was_selecting {
                                let dx = end_x - ms.start_x;
                                let dy = end_y - ms.start_y;
                                let distance = (dx * dx + dy * dy).sqrt();
                                distance >= MIN_DRAG_DISTANCE
                            } else if ms.modifier_held {
                                true
                            } else {
                                false
                            };

                            if should_check {
                                let cursor_x = end_x as i32;
                                let cursor_y = end_y as i32;
                                let app_for_check = app_clone.clone();

                                drop(ms);

                                // Spawn a thread for the blocking accessibility call
                                thread::spawn(move || {
                                    thread::sleep(std::time::Duration::from_millis(50));

                                    if let Some(text) = get_selected_text() {
                                        log::info!("macOS monitor: Selected text ({} chars)", text.len());

                                        let _ = app_for_check.emit("text-selected", serde_json::json!({
                                            "text": text,
                                            "cursorX": cursor_x,
                                            "cursorY": cursor_y,
                                            "sourceApp": "unknown",
                                            "timestamp": chrono::Utc::now().to_rfc3339()
                                        }));

                                        show_popup_with_text(&app_for_check, text, cursor_x, cursor_y);
                                    } else {
                                        log::debug!("macOS monitor: No text selected (accessibility returned empty)");
                                    }
                                });

                                return None;
                            }
                        }

                        _ => {}
                    }

                    None // Don't modify events (listen-only)
                },
            );

            match tap {
                Ok(tap) => {
                    let loop_source = tap.mach_port.create_runloop_source(0)
                        .expect("Failed to create run loop source");
                    let run_loop = CFRunLoop::get_current();
                    run_loop.add_source(&loop_source, unsafe { kCFRunLoopCommonModes });
                    tap.enable();
                    log::info!("macOS monitor: CGEventTap active, entering run loop");
                    CFRunLoop::run_current();
                    log::info!("macOS monitor: Run loop exited");
                }
                Err(()) => {
                    log::error!("macOS monitor: Failed to create CGEventTap. Check Accessibility permission.");
                    MONITOR_RUNNING.store(false, Ordering::SeqCst);
                }
            }
        }
    });

    log::info!("macOS monitor: Started successfully");
    Ok(())
}

/// Check if the macOS monitor is currently running.
pub fn is_monitor_running() -> bool {
    MONITOR_RUNNING.load(Ordering::SeqCst)
}

/// Stop the macOS native monitor.
#[allow(dead_code)]
pub fn stop_monitor() {
    MONITOR_RUNNING.store(false, Ordering::SeqCst);
    log::info!("macOS monitor: Stop requested");
}

/// Open macOS System Settings to Accessibility privacy pane.
pub fn open_accessibility_settings() {
    use std::process::Command;
    let _ = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
    log::info!("macOS: Opened Accessibility settings");
}
