//! Global hotkey handling commands.
//!
//! This module provides commands for global hotkey functionality,
//! including simulating Ctrl+C to copy selected text and triggering
//! the translation popup.

use crate::commands::{get_char_limit_setting, show_translation_confirmation, DbState};
use crate::popup_handler::show_popup_with_text;
#[cfg(not(target_os = "macos"))]
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::POINT,
    UI::WindowsAndMessaging::GetCursorPos,
    UI::Input::KeyboardAndMouse::{GetAsyncKeyState, keybd_event, KEYEVENTF_KEYUP},
};

/// Small delay between key presses for stability (milliseconds).
#[cfg(not(target_os = "macos"))]
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

/// Non-Windows: simulate Cmd+C and read clipboard.
/// Strategy: simulate copy, wait briefly for clipboard update, then return
/// whatever is in the clipboard. If content changed, great. If not, the
/// existing clipboard text is likely what the user wants to translate.
#[cfg(not(target_os = "windows"))]
async fn simulate_copy_with_retry(
    app: &AppHandle,
    old_clipboard: Option<String>,
) -> Result<String, String> {
    // Brief delay to let hotkey modifier keys release before simulating Cmd+C
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Simulate copy (Cmd+C on macOS, Ctrl+C on Linux)
    if let Err(e) = simulate_copy() {
        log::warn!("simulate_copy failed: {}", e);
    }

    // Wait for clipboard to update, checking a few times
    for i in 0..5 {
        tokio::time::sleep(Duration::from_millis(80)).await;

        if let Ok(text) = app.clipboard().read_text() {
            if !text.trim().is_empty() {
                let is_new = match &old_clipboard {
                    Some(old) => text != *old,
                    None => true,
                };
                if is_new {
                    log::info!("Hotkey: Clipboard changed after {}ms", (i + 1) * 80);
                    return Ok(text);
                }
            }
        }
    }

    // Clipboard didn't change — return existing content (user likely re-selected same text)
    let text = app.clipboard()
        .read_text()
        .map_err(|e| format!("Clipboard read failed: {}", e))?;

    if text.trim().is_empty() {
        Err("No text selected or clipboard is empty".to_string())
    } else {
        log::info!("Hotkey: Using existing clipboard content ({} chars)", text.len());
        Ok(text)
    }
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

/// Gets the current cursor position on macOS using Core Graphics.
#[cfg(target_os = "macos")]
fn get_cursor_position() -> (i32, i32) {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    if let Ok(source) = CGEventSource::new(CGEventSourceStateID::CombinedSessionState) {
        if let Ok(event) = CGEvent::new(source) {
            let point = event.location();
            return (point.x as i32, point.y as i32);
        }
    }
    log::warn!("macOS: Failed to get cursor position, defaulting to (500, 300)");
    (500, 300)
}

/// Fallback for other non-Windows platforms.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn get_cursor_position() -> (i32, i32) {
    (500, 300)
}

/// Simulate copy (Ctrl+C on Windows/Linux, Cmd+C on macOS)
#[tauri::command]
pub fn simulate_copy() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        simulate_copy_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        simulate_copy_enigo()
    }
}

/// macOS: Use CGEvent directly to simulate Cmd+C.
/// This avoids enigo crashes from conflicting modifier key state.
#[cfg(target_os = "macos")]
fn simulate_copy_macos() -> Result<(), String> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    // Virtual keycode for 'C' on macOS
    const KC_C: CGKeyCode = 8;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "Failed to create CGEventSource".to_string())?;

    // Create key-down and key-up events for 'C'
    let key_down = CGEvent::new_keyboard_event(source.clone(), KC_C, true)
        .map_err(|_| "Failed to create key-down event".to_string())?;
    let key_up = CGEvent::new_keyboard_event(source, KC_C, false)
        .map_err(|_| "Failed to create key-up event".to_string())?;

    // Set Cmd flag (⌘) on both events
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);

    // Post events to the HID event system
    key_down.post(core_graphics::event::CGEventTapLocation::HID);
    thread::sleep(Duration::from_millis(50));
    key_up.post(core_graphics::event::CGEventTapLocation::HID);

    log::info!("Hotkey: Simulated Cmd+C via CGEvent");
    Ok(())
}

/// Non-macOS: Use enigo to simulate copy.
#[cfg(not(target_os = "macos"))]
fn simulate_copy_enigo() -> Result<(), String> {
    let settings = Settings::default();
    let mut enigo = Enigo::new(&settings).map_err(|e| format!("Failed to create Enigo: {}", e))?;

    let copy_modifier = Key::Control;

    thread::sleep(Duration::from_millis(KEY_PRESS_DELAY_MS));

    enigo.key(copy_modifier, Direction::Press)
        .map_err(|e| format!("Failed to press copy modifier: {}", e))?;
    thread::sleep(Duration::from_millis(KEY_PRESS_DELAY_MS));

    enigo.key(Key::Unicode('c'), Direction::Press)
        .map_err(|e| format!("Failed to press C: {}", e))?;
    thread::sleep(Duration::from_millis(KEY_PRESS_DELAY_MS));
    enigo.key(Key::Unicode('c'), Direction::Release)
        .map_err(|e| format!("Failed to release C: {}", e))?;
    thread::sleep(Duration::from_millis(KEY_PRESS_DELAY_MS));

    enigo.key(copy_modifier, Direction::Release)
        .map_err(|e| format!("Failed to release copy modifier: {}", e))?;

    log::info!("Hotkey: Simulated copy command");
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

    // Step 4: Show popup at cursor position on the active monitor
    let (cursor_x, cursor_y) = get_cursor_position();
    show_popup_with_text(&app, text.clone(), cursor_x, cursor_y);

    Ok(text)
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

    // Step 4: Show popup at cursor position on the active monitor
    let (cursor_x, cursor_y) = get_cursor_position();
    show_popup_with_text(app, text.clone(), cursor_x, cursor_y);

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
