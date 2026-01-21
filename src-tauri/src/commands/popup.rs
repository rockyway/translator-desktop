//! Popup window management commands.
//!
//! This module provides commands to control the popup overlay window
//! for quick translation functionality.

use std::sync::Arc;
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition};
use tokio::sync::Mutex;

/// State to store the text that the popup should translate.
/// Wrapped in Arc for safe cloning across async boundaries.
#[derive(Clone)]
pub struct PopupTextState(pub Arc<Mutex<String>>);

impl Default for PopupTextState {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(String::new())))
    }
}

/// Check if the popup window is currently visible.
#[tauri::command]
pub async fn is_popup_visible(app: AppHandle) -> Result<bool, String> {
    let window = app
        .get_webview_window("popup")
        .ok_or_else(|| "Popup window not found".to_string())?;

    window
        .is_visible()
        .map_err(|e| format!("Failed to check popup visibility: {}", e))
}

/// Show the popup window at the specified screen coordinates with smart positioning.
///
/// The position is adjusted to ensure the popup stays within screen bounds,
/// accounting for DPI scaling. The popup appears near the cursor but won't
/// extend beyond the screen edges.
///
/// # Arguments
/// * `x` - X coordinate (pixels from left edge of screen, in physical pixels)
/// * `y` - Y coordinate (pixels from top edge of screen, in physical pixels)
/// * `keep_position` - If true, don't reposition the window (for updates to already-visible popup)
#[tauri::command]
pub async fn show_popup(app: AppHandle, x: i32, y: i32, keep_position: Option<bool>) -> Result<(), String> {
    let window = app
        .get_webview_window("popup")
        .ok_or_else(|| "Popup window not found".to_string())?;

    // Check if we should keep the current position
    let should_reposition = !keep_position.unwrap_or(false);

    if should_reposition {
        // Get popup window size
        let popup_size = window
            .outer_size()
            .map_err(|e| format!("Failed to get popup size: {}", e))?;

        // Get the scale factor for DPI awareness
        let scale_factor = window
            .scale_factor()
            .unwrap_or(1.0);

        // Calculate popup dimensions in physical pixels
        let popup_width = popup_size.width as i32;
        let popup_height = popup_size.height as i32;

        // Small offset from cursor position
        let offset_x = (10.0 * scale_factor) as i32;
        let offset_y = (10.0 * scale_factor) as i32;

        // Try to find the monitor at the cursor position
        let (final_x, final_y) = if let Ok(Some(monitor)) = window.current_monitor() {
            let monitor_pos = monitor.position();
            let monitor_size = monitor.size();

            // Calculate monitor bounds in physical pixels
            let monitor_left = monitor_pos.x;
            let monitor_top = monitor_pos.y;
            let monitor_right = monitor_pos.x + monitor_size.width as i32;
            let monitor_bottom = monitor_pos.y + monitor_size.height as i32;

            // Initial position: cursor + offset
            let mut pos_x = x + offset_x;
            let mut pos_y = y + offset_y;

            // Adjust X: if popup would go off right edge, show it to the left of cursor
            if pos_x + popup_width > monitor_right {
                pos_x = x - popup_width - offset_x;
                // If still off-screen (left edge), clamp to left edge
                if pos_x < monitor_left {
                    pos_x = monitor_left;
                }
            }
            // If off left edge, clamp to left edge
            if pos_x < monitor_left {
                pos_x = monitor_left;
            }

            // Adjust Y: if popup would go off bottom edge, show it above cursor
            if pos_y + popup_height > monitor_bottom {
                pos_y = y - popup_height - offset_y;
                // If still off-screen (top edge), clamp to top edge
                if pos_y < monitor_top {
                    pos_y = monitor_top;
                }
            }
            // If off top edge, clamp to top edge
            if pos_y < monitor_top {
                pos_y = monitor_top;
            }

            log::debug!(
                "Smart positioning: cursor=({}, {}), monitor bounds=[{},{},{},{}], final=({}, {}), scale={}",
                x, y, monitor_left, monitor_top, monitor_right, monitor_bottom, pos_x, pos_y, scale_factor
            );

            (pos_x, pos_y)
        } else {
            // Fallback: use cursor position with offset
            log::warn!("Could not determine monitor, using raw coordinates");
            (x + offset_x, y + offset_y)
        };

        // Position the window at the calculated coordinates
        window
            .set_position(PhysicalPosition::new(final_x, final_y))
            .map_err(|e| format!("Failed to set popup position: {}", e))?;

        log::info!("Popup positioned at coordinates ({}, {})", final_x, final_y);
    } else {
        log::info!("Popup keeping current position");
    }

    // Apply Mica effect to popup window (Windows 11)
    #[cfg(target_os = "windows")]
    {
        let _ = window_vibrancy::apply_mica(&window, Some(true));
    }

    // Show and focus the window
    window
        .show()
        .map_err(|e| format!("Failed to show popup: {}", e))?;

    window
        .set_focus()
        .map_err(|e| format!("Failed to focus popup: {}", e))?;

    Ok(())
}

/// Hide the popup window.
#[tauri::command]
pub async fn hide_popup(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("popup")
        .ok_or_else(|| "Popup window not found".to_string())?;

    window
        .hide()
        .map_err(|e| format!("Failed to hide popup: {}", e))?;

    log::info!("Popup hidden");
    Ok(())
}

/// Store text for the popup to translate.
///
/// This sets the text that will be displayed/translated in the popup window.
///
/// # Arguments
/// * `text` - The text to store for translation
#[tauri::command]
pub async fn set_popup_text(
    app: AppHandle,
    text: String,
) -> Result<(), String> {
    let state = app.state::<PopupTextState>();
    let mut stored_text = state.0.lock().await;
    *stored_text = text.clone();

    log::info!("Popup text set: {} chars", text.len());
    Ok(())
}

/// Get the currently stored popup text.
///
/// Returns the text that was previously set via `set_popup_text`.
#[tauri::command]
pub async fn get_popup_text(app: AppHandle) -> Result<String, String> {
    let state = app.state::<PopupTextState>();
    let stored_text = state.0.lock().await;
    Ok(stored_text.clone())
}

/// Resize the popup window to fit content.
///
/// # Arguments
/// * `height` - The desired height in logical pixels
#[tauri::command]
pub async fn resize_popup(app: AppHandle, height: f64) -> Result<(), String> {
    let window = app
        .get_webview_window("popup")
        .ok_or_else(|| "Popup window not found".to_string())?;

    // Get current size to preserve width
    let current_size = window
        .outer_size()
        .map_err(|e| format!("Failed to get popup size: {}", e))?;

    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let current_width = current_size.width as f64 / scale_factor;

    // Clamp height to reasonable bounds (min 150, max 600)
    let clamped_height = height.max(150.0).min(600.0);

    window
        .set_size(LogicalSize::new(current_width, clamped_height))
        .map_err(|e| format!("Failed to resize popup: {}", e))?;

    log::debug!("Popup resized to height: {}", clamped_height);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_popup_text_state_default() {
        let state = PopupTextState::default();
        let text = state.0.lock().await;
        assert!(text.is_empty());
    }
}
