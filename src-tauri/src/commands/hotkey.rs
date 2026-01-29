//! Global hotkey handling commands.
//!
//! This module provides commands for global hotkey functionality,
//! including simulating Ctrl+C to copy selected text and triggering
//! the translation popup.

use crate::commands::{get_char_limit_setting, show_translation_confirmation, DbState, PopupTextState};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::POINT,
    UI::WindowsAndMessaging::GetCursorPos,
    UI::Input::KeyboardAndMouse::{GetAsyncKeyState, keybd_event, KEYEVENTF_KEYUP},
};

/// Small delay between key presses for stability (milliseconds).
const KEY_PRESS_DELAY_MS: u64 = 10;

/// Virtual key codes for Windows API
#[cfg(target_os = "windows")]
mod vk_codes {
    pub const VK_CONTROL: i32 = 0x11;
    pub const VK_SHIFT: i32 = 0x10;
    pub const VK_MENU: i32 = 0x12;  // Alt key
    pub const VK_LWIN: i32 = 0x5B;
    pub const VK_RWIN: i32 = 0x5C;
    pub const VK_C: u8 = 0x43;
}

/// Maximum time to wait for modifier keys to release (milliseconds).
#[cfg(target_os = "windows")]
const MAX_MODIFIER_WAIT_MS: u64 = 1000;

/// Interval for checking modifier key state (milliseconds).
#[cfg(target_os = "windows")]
const MODIFIER_CHECK_INTERVAL_MS: u64 = 20;

/// Wait for all modifier keys (Ctrl, Shift, Alt, Win) to be released.
/// This prevents interference when the hotkey includes modifiers.
#[cfg(target_os = "windows")]
async fn wait_for_modifiers_release() {
    use vk_codes::*;

    let mut waited: u64 = 0;

    log::info!("DEBUG: Starting modifier wait (max {}ms)", MAX_MODIFIER_WAIT_MS);

    while waited < MAX_MODIFIER_WAIT_MS {
        let ctrl_held = unsafe { (GetAsyncKeyState(VK_CONTROL) & 0x8000u16 as i16) != 0 };
        let shift_held = unsafe { (GetAsyncKeyState(VK_SHIFT) & 0x8000u16 as i16) != 0 };
        let alt_held = unsafe { (GetAsyncKeyState(VK_MENU) & 0x8000u16 as i16) != 0 };
        let lwin_held = unsafe { (GetAsyncKeyState(VK_LWIN) & 0x8000u16 as i16) != 0 };
        let rwin_held = unsafe { (GetAsyncKeyState(VK_RWIN) & 0x8000u16 as i16) != 0 };

        let any_held = ctrl_held || shift_held || alt_held || lwin_held || rwin_held;

        // Log every 100ms or on first check
        if waited == 0 || waited % 100 == 0 {
            log::info!(
                "DEBUG: Modifier state at {}ms: Ctrl={} Shift={} Alt={} LWin={} RWin={} (any_held={})",
                waited, ctrl_held, shift_held, alt_held, lwin_held, rwin_held, any_held
            );
        }

        if !any_held {
            log::info!("DEBUG: All modifier keys released after {}ms", waited);
            break;
        }

        tokio::time::sleep(Duration::from_millis(MODIFIER_CHECK_INTERVAL_MS)).await;
        waited += MODIFIER_CHECK_INTERVAL_MS;
    }

    if waited >= MAX_MODIFIER_WAIT_MS {
        log::warn!("DEBUG: Modifier key wait TIMED OUT after {}ms", waited);
    }
}

/// Simulate Ctrl+C using Windows keybd_event API (sends ONCE only).
/// This is more reliable than enigo for clipboard operations.
#[cfg(target_os = "windows")]
fn simulate_ctrl_c_once() {
    use vk_codes::*;

    unsafe {
        // Press Ctrl
        keybd_event(VK_CONTROL as u8, 0, Default::default(), 0);
        // Press C
        keybd_event(VK_C, 0, Default::default(), 0);

        // Small delay to ensure key press is processed
        thread::sleep(Duration::from_millis(50));

        // Release C
        keybd_event(VK_C, 0, KEYEVENTF_KEYUP, 0);
        // Release Ctrl
        keybd_event(VK_CONTROL as u8, 0, KEYEVENTF_KEYUP, 0);
    }

    log::info!("Hotkey: Simulated Ctrl+C once via Windows API");
}

/// Simulate Ctrl+C and read clipboard with retry mechanism.
///
/// This implementation:
/// 1. Waits for modifier keys to release (prevents Ctrl+Shift+C)
/// 2. Sends Ctrl+C ONCE using Windows API
/// 3. Polls clipboard for changes (max 3 seconds) WITHOUT re-sending Ctrl+C
///
/// # Arguments
/// * `app` - Tauri app handle for clipboard access
/// * `old_clipboard` - Previous clipboard content (to detect when new copy succeeded)
///
/// # Returns
/// * `Ok(String)` - New clipboard content
/// * `Err(String)` - If clipboard is empty after timeout
#[cfg(target_os = "windows")]
async fn simulate_copy_with_retry(
    app: &AppHandle,
    old_clipboard: Option<String>,
) -> Result<String, String> {
    log::info!("DEBUG: Step 1 - Waiting for modifiers to release...");

    // Step 1: Wait for modifier keys to release (max 500ms)
    // This prevents sending Ctrl+Shift+C instead of Ctrl+C
    wait_for_modifiers_release().await;

    log::info!("DEBUG: Step 2 - Sending Ctrl+C once via Windows API...");

    // Step 2: Send Ctrl+C ONCE using Windows API
    simulate_ctrl_c_once();

    log::info!("DEBUG: Step 3 - Polling clipboard for changes (max 3s)...");

    // Step 3: Poll clipboard for changes (max 3s) - NO more Ctrl+C sending!
    let max_duration = Duration::from_secs(3);
    let poll_interval = Duration::from_millis(100);
    let start = std::time::Instant::now();
    let mut poll_count = 0;

    while start.elapsed() < max_duration {
        // Wait before checking (give app time to process copy)
        tokio::time::sleep(poll_interval).await;
        poll_count += 1;

        // Try to read clipboard
        if let Ok(text) = app.clipboard().read_text() {
            if !text.trim().is_empty() {
                // Check if content is different from old (new copy happened)
                let is_new = match &old_clipboard {
                    Some(old) => text != *old,
                    None => true,
                };

                log::info!(
                    "DEBUG: Poll {}: clipboard_len={}, is_new={}, elapsed={:?}",
                    poll_count, text.len(), is_new, start.elapsed()
                );

                if is_new {
                    log::info!(
                        "DEBUG: SUCCESS - Clipboard captured after {} polls ({:?})",
                        poll_count,
                        start.elapsed()
                    );
                    return Ok(text);
                }
            } else {
                log::info!("DEBUG: Poll {}: clipboard is empty or whitespace", poll_count);
            }
        } else {
            log::info!("DEBUG: Poll {}: clipboard read failed", poll_count);
        }
    }

    // Timeout - return whatever is in clipboard as fallback
    log::warn!("DEBUG: TIMEOUT - Clipboard capture timeout after {} polls", poll_count);
    let fallback = app
        .clipboard()
        .read_text()
        .map_err(|e| format!("Clipboard read failed: {}", e))?;

    log::info!("DEBUG: Fallback clipboard content length: {}", fallback.len());

    if fallback.trim().is_empty() {
        Err("Clipboard is empty after timeout".to_string())
    } else {
        Ok(fallback)
    }
}

/// Non-Windows fallback: uses enigo to simulate Ctrl+C with retry.
#[cfg(not(target_os = "windows"))]
async fn simulate_copy_with_retry(
    app: &AppHandle,
    old_clipboard: Option<String>,
) -> Result<String, String> {
    let max_duration = Duration::from_secs(3);
    let poll_interval = Duration::from_millis(100);
    let start = std::time::Instant::now();

    // Simulate Ctrl+C once using enigo
    if let Err(e) = simulate_copy() {
        log::warn!("simulate_copy failed: {}", e);
    }

    // Poll clipboard for changes
    while start.elapsed() < max_duration {
        tokio::time::sleep(poll_interval).await;

        if let Ok(text) = app.clipboard().read_text() {
            if !text.trim().is_empty() {
                let is_new = match &old_clipboard {
                    Some(old) => text != *old,
                    None => true,
                };
                if is_new {
                    return Ok(text);
                }
            }
        }
    }

    // Fallback
    app.clipboard()
        .read_text()
        .map_err(|e| format!("Clipboard read failed: {}", e))
}

/// Gets the current cursor position using Windows API.
///
/// # Returns
/// Tuple of (x, y) coordinates in screen pixels
#[cfg(target_os = "windows")]
fn get_cursor_position() -> (i32, i32) {
    unsafe {
        let mut point = POINT::default();
        if GetCursorPos(&mut point).is_ok() {
            (point.x, point.y)
        } else {
            log::warn!("Failed to get cursor position, defaulting to (0, 0)");
            (0, 0)
        }
    }
}

/// Fallback for non-Windows platforms - returns center of primary monitor.
#[cfg(not(target_os = "windows"))]
fn get_cursor_position() -> (i32, i32) {
    (500, 300) // Default center-ish position
}

/// Simulate Ctrl+C to copy selected text to clipboard
/// Releases Alt and Shift keys first to avoid interference
#[tauri::command]
pub fn simulate_copy() -> Result<(), String> {
    let settings = Settings::default();
    let mut enigo = Enigo::new(&settings).map_err(|e| format!("Failed to create Enigo: {}", e))?;

    // Small delay to ensure target app has focus
    thread::sleep(Duration::from_millis(KEY_PRESS_DELAY_MS));

    // Press Ctrl
    enigo.key(Key::Control, Direction::Press)
        .map_err(|e| format!("Failed to press Control: {}", e))?;
    thread::sleep(Duration::from_millis(KEY_PRESS_DELAY_MS));

    // Press and release 'C' key
    enigo.key(Key::Unicode('c'), Direction::Press)
        .map_err(|e| format!("Failed to press C: {}", e))?;
    thread::sleep(Duration::from_millis(KEY_PRESS_DELAY_MS));
    enigo.key(Key::Unicode('c'), Direction::Release)
        .map_err(|e| format!("Failed to release C: {}", e))?;
    thread::sleep(Duration::from_millis(KEY_PRESS_DELAY_MS));

    // Release Ctrl
    enigo.key(Key::Control, Direction::Release)
        .map_err(|e| format!("Failed to release Control: {}", e))?;

    log::info!("Hotkey: Simulated Ctrl+C");
    Ok(())
}

/// Triggers the full hotkey translation flow:
/// 1. Simulates Ctrl+C to copy selected text
/// 2. Waits for clipboard to update
/// 3. Reads clipboard content
/// 4. Sets the popup text state
/// 5. Shows the popup window at screen center or cursor position
///
/// # Arguments
/// * `app` - Tauri app handle for accessing clipboard and popup
///
/// # Returns
/// * `Ok(text)` - The text that was copied and will be translated
/// * `Err(String)` if any step in the flow fails
#[tauri::command]
pub async fn trigger_hotkey_translate(app: AppHandle) -> Result<String, String> {
    // Step 1: Capture old clipboard content BEFORE simulating Ctrl+C
    let old_clipboard = app.clipboard().read_text().ok();

    log::info!(
        "Old clipboard content length: {:?}",
        old_clipboard.as_ref().map(|s| s.len())
    );

    // Step 2: Simulate Ctrl+C with retry mechanism
    // This retries every 50ms until clipboard changes or 3s timeout
    // Handles Alt/Shift modifier keys that may interfere with copy
    let text = simulate_copy_with_retry(&app, old_clipboard).await?;

    if text.trim().is_empty() {
        log::warn!("Hotkey: Clipboard is empty after copy simulation");
        return Err("No text selected or clipboard is empty".to_string());
    }

    log::info!("Hotkey: Read {} chars from clipboard", text.len());

    // Step 3.5: Character limit confirmation check
    if let Some(db_state) = app.try_state::<DbState>() {
        let char_limit = get_char_limit_setting(&db_state).await;
        if char_limit > 0 && text.len() > char_limit {
            log::info!("Hotkey: Text exceeds {} chars, showing confirmation", char_limit);
            // Get cursor position for confirmation dialog placement
            let (cursor_x, cursor_y) = get_cursor_position();
            let confirmed = show_translation_confirmation(&app, text.len(), cursor_x, cursor_y).await;
            if !confirmed {
                log::info!("Hotkey: User declined translation");
                return Err("Translation cancelled by user".to_string());
            }
            log::info!("Hotkey: User confirmed translation");
        }
    }

    // Step 4: Set popup text state
    let state = app.state::<PopupTextState>();
    {
        let mut stored_text = state.0.lock().await;
        *stored_text = text.clone();
    }

    // Step 5: Show popup at screen center
    show_popup_centered(&app, text.clone())?;

    Ok(text)
}

/// Shows the popup window at the center of the primary monitor.
fn show_popup_centered(app_handle: &AppHandle, text: String) -> Result<(), String> {
    let window = app_handle
        .get_webview_window("popup")
        .ok_or_else(|| "Popup window not found".to_string())?;

    // Get popup window size
    let popup_size = window
        .outer_size()
        .map_err(|e| format!("Failed to get popup size: {}", e))?;

    // Try to get the primary monitor for centering
    let position = if let Ok(Some(monitor)) = window.primary_monitor() {
        let monitor_size = monitor.size();
        let monitor_pos = monitor.position();

        // Calculate center position
        let x = monitor_pos.x + (monitor_size.width as i32 - popup_size.width as i32) / 2;
        let y = monitor_pos.y + (monitor_size.height as i32 - popup_size.height as i32) / 2;

        PhysicalPosition::new(x, y)
    } else {
        // Fallback to fixed position if monitor info unavailable
        PhysicalPosition::new(400, 300)
    };

    // Position and show the window
    window
        .set_position(position)
        .map_err(|e| format!("Failed to set popup position: {}", e))?;

    window
        .show()
        .map_err(|e| format!("Failed to show popup: {}", e))?;

    window
        .set_focus()
        .map_err(|e| format!("Failed to focus popup: {}", e))?;

    // Emit event to popup window to refresh text
    if let Err(e) = app_handle.emit("popup-text-updated", text.clone()) {
        log::error!("Hotkey: Failed to emit popup-text-updated event: {}", e);
    }

    log::info!(
        "Hotkey: Popup shown at ({}, {})",
        position.x,
        position.y
    );

    Ok(())
}

/// Handles the global hotkey event.
/// This is called when the registered hotkey (Ctrl+Shift+Q) is pressed.
///
/// # Arguments
/// * `app_handle` - Tauri app handle
pub fn handle_global_hotkey(app_handle: AppHandle) {
    log::info!("Hotkey: Global hotkey triggered");

    // Spawn async task to handle the translation flow
    tauri::async_runtime::spawn(async move {
        match trigger_hotkey_translate_internal(&app_handle).await {
            Ok(text) => {
                log::info!("Hotkey: Translation triggered for {} chars", text.len());
            }
            Err(e) => {
                log::error!("Hotkey: Failed to trigger translation: {}", e);
            }
        }
    });
}

/// Internal version of trigger_hotkey_translate that takes a reference.
async fn trigger_hotkey_translate_internal(app: &AppHandle) -> Result<String, String> {
    log::info!("DEBUG: ===== HOTKEY TRANSLATION FLOW STARTED =====");

    // Step 1: Capture old clipboard content BEFORE simulating Ctrl+C
    let old_clipboard = app.clipboard().read_text().ok();

    log::info!(
        "DEBUG: Old clipboard content length: {:?}",
        old_clipboard.as_ref().map(|s| s.len())
    );

    // Step 2: Simulate Ctrl+C with retry mechanism
    // This retries every 50ms until clipboard changes or 3s timeout
    // Handles Alt/Shift modifier keys that may interfere with copy
    let text = simulate_copy_with_retry(app, old_clipboard).await?;

    if text.trim().is_empty() {
        log::warn!("Hotkey: Clipboard is empty after copy simulation");
        return Err("No text selected or clipboard is empty".to_string());
    }

    log::info!("Hotkey: Read {} chars from clipboard", text.len());

    // Step 3.5: Character limit confirmation check
    if let Some(db_state) = app.try_state::<DbState>() {
        let char_limit = get_char_limit_setting(&db_state).await;
        if char_limit > 0 && text.len() > char_limit {
            log::info!("Hotkey internal: Text exceeds {} chars ({}), showing confirmation", char_limit, text.len());
            let (cursor_x, cursor_y) = get_cursor_position();
            let confirmed = show_translation_confirmation(app, text.len(), cursor_x, cursor_y).await;
            if !confirmed {
                log::info!("Hotkey internal: User declined translation");
                return Err("Translation cancelled by user".to_string());
            }
            log::info!("Hotkey internal: User confirmed translation");
        }
    } else {
        log::warn!("Hotkey internal: DbState not available, skipping char limit check");
    }

    // Step 4: Set popup text state
    let state = app.state::<PopupTextState>();
    {
        let mut stored_text = state.0.lock().await;
        *stored_text = text.clone();
    }

    // Step 5: Show popup at screen center
    show_popup_centered(app, text.clone())?;

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_press_delay_is_reasonable() {
        // Ensure the delay is not too short (stability)
        // and not too long (user experience)
        assert!(KEY_PRESS_DELAY_MS >= 5);
        assert!(KEY_PRESS_DELAY_MS <= 50);
    }
}
