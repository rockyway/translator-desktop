//! Global hotkey handling commands.
//!
//! This module provides commands for global hotkey functionality,
//! including simulating Ctrl+C to copy selected text and triggering
//! the translation popup.

use crate::commands::PopupTextState;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Delay after simulating Ctrl+C before reading clipboard (milliseconds).
const COPY_DELAY_MS: u64 = 150;

/// Small delay between key presses for stability (milliseconds).
const KEY_PRESS_DELAY_MS: u64 = 10;

/// Simulates a Ctrl+C keystroke to copy selected text to clipboard.
///
/// Uses the `enigo` crate to send keyboard events at the OS level.
/// Includes small delays between key presses for better compatibility.
///
/// # Returns
/// * `Ok(())` if the copy simulation was successful
/// * `Err(String)` if keyboard simulation failed
#[tauri::command]
pub fn simulate_copy() -> Result<(), String> {
    let settings = Settings::default();
    let mut enigo = Enigo::new(&settings).map_err(|e| format!("Failed to create Enigo: {}", e))?;

    // Small delay to ensure target app has focus
    thread::sleep(Duration::from_millis(KEY_PRESS_DELAY_MS));

    // Press Ctrl
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| format!("Failed to press Ctrl: {}", e))?;

    // Small delay between key presses for stability
    thread::sleep(Duration::from_millis(KEY_PRESS_DELAY_MS));

    // Press and release 'C' key (using layout key for better compatibility)
    enigo
        .key(Key::Unicode('c'), Direction::Press)
        .map_err(|e| format!("Failed to press C: {}", e))?;

    thread::sleep(Duration::from_millis(KEY_PRESS_DELAY_MS));

    enigo
        .key(Key::Unicode('c'), Direction::Release)
        .map_err(|e| format!("Failed to release C: {}", e))?;

    thread::sleep(Duration::from_millis(KEY_PRESS_DELAY_MS));

    // Release Ctrl
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| format!("Failed to release Ctrl: {}", e))?;

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
    // Step 1: Simulate Ctrl+C
    simulate_copy()?;

    // Step 2: Wait for clipboard to update
    thread::sleep(Duration::from_millis(COPY_DELAY_MS));

    // Step 3: Read clipboard content
    let text = app
        .clipboard()
        .read_text()
        .map_err(|e| format!("Failed to read clipboard: {}", e))?;

    if text.is_empty() {
        log::warn!("Hotkey: Clipboard is empty after copy simulation");
        return Err("No text selected or clipboard is empty".to_string());
    }

    log::info!("Hotkey: Read {} chars from clipboard", text.len());

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
    // Step 1: Simulate Ctrl+C
    simulate_copy()?;

    // Step 2: Wait for clipboard to update
    thread::sleep(Duration::from_millis(COPY_DELAY_MS));

    // Step 3: Read clipboard content
    let text = app
        .clipboard()
        .read_text()
        .map_err(|e| format!("Failed to read clipboard: {}", e))?;

    if text.is_empty() {
        log::warn!("Hotkey: Clipboard is empty after copy simulation");
        return Err("No text selected or clipboard is empty".to_string());
    }

    log::info!("Hotkey: Read {} chars from clipboard", text.len());

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
    fn test_copy_delay_is_reasonable() {
        // Ensure the delay is not too short (clipboard needs time)
        // and not too long (user experience)
        assert!(COPY_DELAY_MS >= 50);
        assert!(COPY_DELAY_MS <= 500);
    }
}
