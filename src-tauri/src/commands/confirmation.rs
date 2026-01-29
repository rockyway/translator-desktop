//! Confirmation dialog commands for character limit warnings.
//!
//! This module provides a separate confirmation window that appears at cursor position
//! when the user selects text exceeding the character limit threshold.

use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tokio::sync::{oneshot, Mutex};

/// State to hold the pending confirmation data (character count)
/// React fetches this on mount to avoid race condition with events
#[derive(Default)]
pub struct ConfirmationDataState {
    pub char_count: Arc<Mutex<usize>>,
}

/// Global state to hold confirmation response channel
pub struct ConfirmationState {
    pub sender: Arc<Mutex<Option<oneshot::Sender<bool>>>>,
}

impl Default for ConfirmationState {
    fn default() -> Self {
        Self {
            sender: Arc::new(Mutex::new(None)),
        }
    }
}

/// Get the current confirmation data (called by React when window is focused)
#[tauri::command]
pub async fn get_confirmation_data(
    state: tauri::State<'_, ConfirmationDataState>,
) -> Result<usize, String> {
    log::info!("[CONFIRMATION] get_confirmation_data() called");
    let char_count = *state.char_count.lock().await;
    log::info!("[CONFIRMATION] Returning char_count from state: {}", char_count);
    Ok(char_count)
}

/// Show confirmation dialog at cursor position and wait for user response.
///
/// # Arguments
/// * `app_handle` - Tauri app handle
/// * `char_count` - Number of characters in the selected text
/// * `cursor_x` - X coordinate of cursor position
/// * `cursor_y` - Y coordinate of cursor position
///
/// # Returns
/// * `true` if user confirmed, `false` if cancelled or timeout
pub async fn show_translation_confirmation(
    app_handle: &AppHandle,
    char_count: usize,
    cursor_x: i32,
    cursor_y: i32,
) -> bool {
    log::info!("[CONFIRMATION] show_translation_confirmation() called with char_count={}", char_count);

    // Store char_count in state BEFORE showing window
    // React fetches this when window gains focus
    if let Some(data_state) = app_handle.try_state::<ConfirmationDataState>() {
        log::info!("[CONFIRMATION] ConfirmationDataState found, storing char_count...");
        *data_state.char_count.lock().await = char_count;
        log::info!("[CONFIRMATION] Stored char_count={} in state", char_count);

        // Verify it was stored correctly
        let verify = *data_state.char_count.lock().await;
        log::info!("[CONFIRMATION] Verification: state now contains char_count={}", verify);
    } else {
        log::error!("[CONFIRMATION] ConfirmationDataState not found!");
        return false;
    }

    // Create oneshot channel for response
    let (tx, rx) = oneshot::channel();

    // Store sender in state
    let state = app_handle.state::<ConfirmationState>();
    *state.sender.lock().await = Some(tx);

    // Get confirmation window
    log::info!("[CONFIRMATION] Getting confirmation window...");
    let window = match app_handle.get_webview_window("confirmation") {
        Some(w) => {
            log::info!("[CONFIRMATION] Confirmation window found");
            w
        }
        None => {
            log::error!("[CONFIRMATION] Confirmation window not found");
            return false;
        }
    };

    // Position window at cursor with smart offset
    let window_width = 400i32;
    let window_height = 220i32;
    let offset = 10i32;

    // Try to position at cursor + offset, adjust if off screen
    let mut pos_x = cursor_x + offset;
    let mut pos_y = cursor_y + offset;

    // Get monitor bounds for smart positioning
    // Use the monitor where the cursor is located
    if let Ok(monitors) = app_handle.available_monitors() {
        let cursor_monitor = monitors.iter().find(|m| {
            let pos = m.position();
            let size = m.size();
            let left = pos.x;
            let top = pos.y;
            let right = pos.x + size.width as i32;
            let bottom = pos.y + size.height as i32;
            cursor_x >= left && cursor_x < right && cursor_y >= top && cursor_y < bottom
        });

        if let Some(monitor) = cursor_monitor {
            let monitor_pos = monitor.position();
            let monitor_size = monitor.size();

            let monitor_left = monitor_pos.x;
            let monitor_top = monitor_pos.y;
            let monitor_right = monitor_pos.x + monitor_size.width as i32;
            let monitor_bottom = monitor_pos.y + monitor_size.height as i32;

            log::debug!(
                "Confirmation: Monitor bounds ({}, {}) to ({}, {})",
                monitor_left,
                monitor_top,
                monitor_right,
                monitor_bottom
            );

            // Adjust if window would go off right edge
            if pos_x + window_width > monitor_right {
                pos_x = cursor_x - window_width - offset;
            }

            // Adjust if window would go off bottom edge
            if pos_y + window_height > monitor_bottom {
                pos_y = cursor_y - window_height - offset;
            }

            // Ensure not off left/top edges
            pos_x = pos_x.max(monitor_left);
            pos_y = pos_y.max(monitor_top);
        }
    }

    // Position and show window
    log::info!("[CONFIRMATION] Positioning window at ({}, {})...", pos_x, pos_y);
    if let Err(e) = window.set_position(PhysicalPosition::new(pos_x, pos_y)) {
        log::error!("[CONFIRMATION] Failed to position confirmation window: {}", e);
    }

    // Apply Mica effect (Windows 11)
    #[cfg(target_os = "windows")]
    {
        log::info!("[CONFIRMATION] Applying Mica effect...");
        let _ = window_vibrancy::apply_mica(&window, Some(true));
    }

    log::info!("[CONFIRMATION] Showing window...");
    if let Err(e) = window.show() {
        log::error!("[CONFIRMATION] Failed to show confirmation window: {}", e);
        return false;
    }

    log::info!("[CONFIRMATION] Setting focus...");
    if let Err(e) = window.set_focus() {
        log::error!("[CONFIRMATION] Failed to focus confirmation window: {}", e);
    }

    // Small delay to allow React's onFocusChanged to trigger and fetch data
    log::info!("[CONFIRMATION] Waiting 150ms for React to process focus event...");
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // Emit event as backup mechanism (in case focus event doesn't trigger)
    log::info!("[CONFIRMATION] Emitting show-confirmation-window event with charCount={}", char_count);
    if let Err(e) = app_handle.emit_to(
        "confirmation",
        "show-confirmation-window",
        serde_json::json!({
            "charCount": char_count
        }),
    ) {
        log::error!("[CONFIRMATION] Failed to emit confirmation event: {}", e);
        // Don't return false - the focus mechanism should still work
    }

    log::info!(
        "[CONFIRMATION] Dialog shown at ({}, {}) for {} chars, waiting for user response...",
        pos_x,
        pos_y,
        char_count
    );

    // Wait for response with timeout
    match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
        Ok(Ok(confirmed)) => {
            // Hide window after response
            let _ = window.hide();
            log::info!("Confirmation: User responded with {}", confirmed);
            confirmed
        }
        Ok(Err(_)) => {
            log::warn!("Confirmation: Channel closed");
            let _ = window.hide();
            false
        }
        Err(_) => {
            log::warn!("Confirmation: Timeout after 30 seconds");
            let _ = window.hide();
            false
        }
    }
}

/// Tauri command called by the confirmation window when user responds.
#[tauri::command]
pub async fn respond_to_confirmation(
    state: tauri::State<'_, ConfirmationState>,
    confirmed: bool,
) -> Result<(), String> {
    let mut sender_opt = state.sender.lock().await;
    if let Some(sender) = sender_opt.take() {
        let _ = sender.send(confirmed);
        log::info!("Confirmation: Response sent: {}", confirmed);
        Ok(())
    } else {
        Err("No pending confirmation".to_string())
    }
}
