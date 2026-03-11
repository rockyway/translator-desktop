//! Shared popup display logic used by both Windows IPC and macOS native monitor.
//!
//! This module contains the core popup positioning and display functions
//! that are platform-independent.

use crate::commands::{get_char_limit_setting, show_translation_confirmation, DbState, PopupTextState};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

/// Stores text in popup state and shows the popup window.
/// If popup is already visible, keeps current position.
/// If popup is not visible, positions at cursor with smart screen-edge detection.
/// Supports multi-monitor setups including monitors with negative coordinates (left of main).
pub fn show_popup_with_text(app_handle: &AppHandle, text: String, cursor_x: i32, cursor_y: i32) {
    // Clone what we need before spawning async task
    let app_handle_clone = app_handle.clone();
    // PopupTextState is Clone (wraps Arc<Mutex<String>>)
    let state: PopupTextState = (*app_handle.state::<PopupTextState>()).clone();

    tauri::async_runtime::spawn(async move {
        // Character limit confirmation check
        if let Some(db_state) = app_handle_clone.try_state::<DbState>() {
            let char_limit = get_char_limit_setting(&db_state).await;
            if char_limit > 0 && text.len() > char_limit {
                log::info!("Popup: Text exceeds {} chars, showing confirmation", char_limit);
                let confirmed = show_translation_confirmation(&app_handle_clone, text.len(), cursor_x, cursor_y).await;
                if !confirmed {
                    log::info!("Popup: User declined translation");
                    return; // Exit without showing popup
                }
                log::info!("Popup: User confirmed translation");
            }
        }

        // Set the popup text
        {
            let mut stored_text = state.0.lock().await;
            *stored_text = text.clone();
            log::info!("Popup: Text set: {} chars", text.len());
        }

        // Show the popup window
        if let Some(window) = app_handle_clone.get_webview_window("popup") {
            // Check if popup is currently visible
            let is_visible = window.is_visible().unwrap_or(false);

            if is_visible {
                // Popup already visible - just update text, keep position
                log::info!("Popup: Already visible, updating text without repositioning");

                // Just emit event to update text
                if let Err(e) = app_handle_clone.emit("popup-text-updated", text) {
                    log::error!("Popup: Failed to emit popup-text-updated event: {}", e);
                }
            } else {
                // Popup not visible - show at cursor position with smart positioning
                log::info!("Popup: Not visible, showing at cursor ({}, {})", cursor_x, cursor_y);

                // Use smart positioning with screen-edge detection
                let popup_size = window.outer_size().unwrap_or_default();
                let scale_factor = window.scale_factor().unwrap_or(1.0);

                let popup_width = popup_size.width as i32;
                let popup_height = popup_size.height as i32;

                // Small offset from cursor position
                let offset_x = (10.0 * scale_factor) as i32;
                let offset_y = (10.0 * scale_factor) as i32;

                // Find the monitor that contains the cursor position
                // This is critical for multi-monitor setups, especially with negative coordinates
                let monitor = find_monitor_at_point(&app_handle_clone, cursor_x, cursor_y);

                // Calculate position with screen-edge detection
                let (final_x, final_y) = if let Some(mon) = monitor {
                    let monitor_pos = mon.position();
                    let monitor_size = mon.size();

                    let monitor_left = monitor_pos.x;
                    let monitor_top = monitor_pos.y;
                    let monitor_right = monitor_pos.x + monitor_size.width as i32;
                    let monitor_bottom = monitor_pos.y + monitor_size.height as i32;

                    log::debug!(
                        "Popup: Monitor bounds: ({}, {}) to ({}, {})",
                        monitor_left, monitor_top, monitor_right, monitor_bottom
                    );

                    let mut pos_x = cursor_x + offset_x;
                    let mut pos_y = cursor_y + offset_y;

                    // Adjust X: if popup would go off right edge, show left of cursor
                    if pos_x + popup_width > monitor_right {
                        pos_x = cursor_x - popup_width - offset_x;
                        if pos_x < monitor_left {
                            pos_x = monitor_left;
                        }
                    }
                    if pos_x < monitor_left {
                        pos_x = monitor_left;
                    }

                    // Adjust Y: if popup would go off bottom edge, show above cursor
                    if pos_y + popup_height > monitor_bottom {
                        pos_y = cursor_y - popup_height - offset_y;
                        if pos_y < monitor_top {
                            pos_y = monitor_top;
                        }
                    }
                    if pos_y < monitor_top {
                        pos_y = monitor_top;
                    }

                    (pos_x, pos_y)
                } else {
                    // No monitor found - use cursor position with offset (fallback)
                    log::warn!("Popup: No monitor found for cursor position, using direct offset");
                    (cursor_x + offset_x, cursor_y + offset_y)
                };

                if let Err(e) = window.set_position(PhysicalPosition::new(final_x, final_y)) {
                    log::error!("Popup: Failed to set popup position: {}", e);
                    return;
                }

                // Apply window effects per platform
                #[cfg(target_os = "windows")]
                {
                    let _ = window_vibrancy::apply_mica(&window, Some(true));
                }

                // macOS: Skip vibrancy — transparent windows cause click-through issues

                if let Err(e) = window.show() {
                    log::error!("Popup: Failed to show popup: {}", e);
                    return;
                }

                if let Err(e) = window.set_focus() {
                    log::error!("Popup: Failed to focus popup: {}", e);
                    return;
                }

                // Emit event to popup window to refresh text
                if let Err(e) = app_handle_clone.emit("popup-text-updated", text) {
                    log::error!("Popup: Failed to emit popup-text-updated event: {}", e);
                }

                log::info!("Popup: Shown at ({}, {})", final_x, final_y);
            }
        } else {
            log::warn!("Popup: Popup window not found");
        }
    });
}

/// Finds the monitor that contains the given point (x, y).
/// Supports multi-monitor setups with negative coordinates (monitors left of primary).
pub fn find_monitor_at_point(app_handle: &AppHandle, x: i32, y: i32) -> Option<tauri::Monitor> {
    if let Ok(monitors) = app_handle.available_monitors() {
        for monitor in monitors {
            let pos = monitor.position();
            let size = monitor.size();

            let left = pos.x;
            let top = pos.y;
            let right = pos.x + size.width as i32;
            let bottom = pos.y + size.height as i32;

            // Check if point is within this monitor's bounds
            if x >= left && x < right && y >= top && y < bottom {
                log::debug!(
                    "Popup: Found monitor for point ({}, {}): {:?} at ({}, {}) size {}x{}",
                    x, y,
                    monitor.name(),
                    left, top, size.width, size.height
                );
                return Some(monitor);
            }
        }
        log::warn!("Popup: Point ({}, {}) not found on any monitor", x, y);
    } else {
        log::error!("Popup: Failed to enumerate monitors");
    }

    // Fallback to primary monitor if point not found on any monitor
    app_handle.primary_monitor().ok().flatten()
}
