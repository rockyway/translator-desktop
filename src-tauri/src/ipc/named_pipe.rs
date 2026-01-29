//! Named Pipe client for receiving text selection events from .NET TextMonitor.

use crate::commands::{get_char_limit_setting, show_translation_confirmation, DbState, PopupTextState};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

/// Global flag to track IPC connection status.
/// This allows querying the connection state without relying on event listeners.
static IPC_CONNECTED: AtomicBool = AtomicBool::new(false);

/// Pipe name matching the .NET server.
const PIPE_NAME: &str = r"\\.\pipe\TranslatorDesktop";

/// Reconnection delay on connection failure.
const RECONNECT_DELAY: Duration = Duration::from_secs(2);

/// IPC message received from the .NET TextMonitor service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub payload: Option<serde_json::Value>,
    pub timestamp: String,
}

/// Payload for text_selected events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSelectedPayload {
    pub text: String,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub source_app: String,
    pub window_title: Option<String>,
}

/// Payload for version events.
#[derive(Debug, Clone, Deserialize)]
pub struct VersionPayload {
    pub version: String,
}

/// Event payload emitted to the Tauri frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSelectedEvent {
    pub text: String,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub source_app: String,
    pub window_title: Option<String>,
    pub timestamp: String,
}

/// Connection status event payload emitted to the Tauri frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatusEvent {
    pub connected: bool,
    pub timestamp: String,
}

/// Event payload for text monitor version info emitted to the Tauri frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextMonitorVersionEvent {
    pub version: String,
    pub timestamp: String,
}

/// Starts the IPC listener that connects to the .NET Named Pipe server.
/// This function spawns a background task that:
/// 1. Connects to the Named Pipe
/// 2. Reads incoming JSON messages
/// 3. Emits Tauri events to the frontend
/// 4. Auto-reconnects on disconnection
pub fn start_ipc_listener(app_handle: AppHandle) {
    thread::spawn(move || {
        ipc_listener_loop(app_handle);
    });
}

/// Main IPC listener loop with auto-reconnection.
fn ipc_listener_loop(app_handle: AppHandle) {
    loop {
        log::info!("IPC: Attempting to connect to pipe: {}", PIPE_NAME);

        match connect_and_listen(&app_handle) {
            Ok(()) => {
                log::info!("IPC: Connection closed normally");
            }
            Err(e) => {
                log::warn!("IPC: Connection error: {}. Reconnecting in {:?}...", e, RECONNECT_DELAY);
            }
        }

        // Wait before reconnecting
        std::thread::sleep(RECONNECT_DELAY);
    }
}

/// Connects to the Named Pipe and listens for messages.
fn connect_and_listen(app_handle: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Open the Named Pipe (read-only client)
    let pipe = std::fs::OpenOptions::new()
        .read(true)
        .open(PIPE_NAME)?;

    log::info!("IPC: Connected to pipe successfully");

    // Emit connected event
    emit_connection_status(app_handle, true);

    let reader = BufReader::new(pipe);

    // Read newline-delimited JSON messages
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                if line.is_empty() {
                    continue;
                }

                log::debug!("IPC: Received raw message: {}", &line[..line.len().min(100)]);

                match serde_json::from_str::<IpcMessage>(&line) {
                    Ok(message) => {
                        handle_message(app_handle, message);
                    }
                    Err(e) => {
                        log::error!("IPC: Failed to parse message: {}. Raw: {}", e, &line[..line.len().min(200)]);
                    }
                }
            }
            Err(e) => {
                log::warn!("IPC: Read error: {}", e);
                // Emit disconnected event on read error
                emit_connection_status(app_handle, false);
                return Err(Box::new(e));
            }
        }
    }

    // Pipe closed normally - emit disconnected event
    emit_connection_status(app_handle, false);

    Ok(())
}

/// Emits a connection status event to the frontend and updates global state.
fn emit_connection_status(app_handle: &AppHandle, connected: bool) {
    // Update global connection state
    IPC_CONNECTED.store(connected, Ordering::SeqCst);

    let event = ConnectionStatusEvent {
        connected,
        timestamp: Utc::now().to_rfc3339(),
    };

    let event_name = if connected { "ipc-connected" } else { "ipc-disconnected" };

    log::info!("IPC: Emitting {} event", event_name);

    if let Err(e) = app_handle.emit(event_name, event) {
        log::error!("IPC: Failed to emit {} event: {}", event_name, e);
    }
}

/// Returns the current IPC connection status.
/// This can be called at any time to check if the named pipe is connected.
pub fn is_ipc_connected() -> bool {
    IPC_CONNECTED.load(Ordering::SeqCst)
}

/// Handles an incoming IPC message and emits appropriate Tauri events.
fn handle_message(app_handle: &AppHandle, message: IpcMessage) {
    match message.message_type.as_str() {
        "text_selected" => {
            if let Some(payload_value) = message.payload {
                match serde_json::from_value::<TextSelectedPayload>(payload_value) {
                    Ok(payload) => {
                        let text = payload.text.clone();
                        let cursor_x = payload.cursor_x;
                        let cursor_y = payload.cursor_y;

                        let event = TextSelectedEvent {
                            text: payload.text,
                            cursor_x: payload.cursor_x,
                            cursor_y: payload.cursor_y,
                            source_app: payload.source_app,
                            window_title: payload.window_title,
                            timestamp: message.timestamp,
                        };

                        log::info!(
                            "IPC: Text selected - {} chars from {}",
                            event.text.len(),
                            event.source_app
                        );

                        // Emit event to frontend
                        if let Err(e) = app_handle.emit("text-selected", event) {
                            log::error!("IPC: Failed to emit text-selected event: {}", e);
                        }

                        // Store text in popup state and show popup
                        show_popup_with_text(app_handle, text, cursor_x, cursor_y);
                    }
                    Err(e) => {
                        log::error!("IPC: Failed to parse text_selected payload: {}", e);
                    }
                }
            }
        }
        "version" => {
            if let Some(payload_value) = message.payload {
                match serde_json::from_value::<VersionPayload>(payload_value) {
                    Ok(payload) => {
                        log::info!("IPC: Text Monitor version: {}", payload.version);

                        let event = TextMonitorVersionEvent {
                            version: payload.version,
                            timestamp: Utc::now().to_rfc3339(),
                        };

                        if let Err(e) = app_handle.emit("text-monitor-version", &event) {
                            log::error!("IPC: Failed to emit text-monitor-version event: {}", e);
                        }
                    }
                    Err(e) => {
                        log::error!("IPC: Failed to parse version payload: {}", e);
                    }
                }
            }
        }
        other => {
            log::debug!("IPC: Received unknown message type: {}", other);
        }
    }
}

/// Stores text in popup state and shows the popup window.
/// If popup is already visible, keeps current position.
/// If popup is not visible, positions at cursor with smart screen-edge detection.
/// Supports multi-monitor setups including monitors with negative coordinates (left of main).
fn show_popup_with_text(app_handle: &AppHandle, text: String, cursor_x: i32, cursor_y: i32) {
    // Clone what we need before spawning async task
    let app_handle_clone = app_handle.clone();
    // PopupTextState is Clone (wraps Arc<Mutex<String>>)
    let state: PopupTextState = (*app_handle.state::<PopupTextState>()).clone();

    tauri::async_runtime::spawn(async move {
        // Character limit confirmation check
        if let Some(db_state) = app_handle_clone.try_state::<DbState>() {
            let char_limit = get_char_limit_setting(&db_state).await;
            if char_limit > 0 && text.len() > char_limit {
                log::info!("IPC: Text exceeds {} chars, showing confirmation", char_limit);
                let confirmed = show_translation_confirmation(&app_handle_clone, text.len(), cursor_x, cursor_y).await;
                if !confirmed {
                    log::info!("IPC: User declined translation");
                    return; // Exit without showing popup
                }
                log::info!("IPC: User confirmed translation");
            }
        }

        // Set the popup text
        {
            let mut stored_text = state.0.lock().await;
            *stored_text = text.clone();
            log::info!("IPC: Popup text set: {} chars", text.len());
        }

        // Show the popup window
        if let Some(window) = app_handle_clone.get_webview_window("popup") {
            // Check if popup is currently visible
            let is_visible = window.is_visible().unwrap_or(false);

            if is_visible {
                // Popup already visible - just update text, keep position
                log::info!("IPC: Popup already visible, updating text without repositioning");

                // Just emit event to update text
                if let Err(e) = app_handle_clone.emit("popup-text-updated", text) {
                    log::error!("IPC: Failed to emit popup-text-updated event: {}", e);
                }
            } else {
                // Popup not visible - show at cursor position with smart positioning
                log::info!("IPC: Popup not visible, showing at cursor ({}, {})", cursor_x, cursor_y);

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
                        "IPC: Monitor bounds: ({}, {}) to ({}, {})",
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
                    log::warn!("IPC: No monitor found for cursor position, using direct offset");
                    (cursor_x + offset_x, cursor_y + offset_y)
                };

                if let Err(e) = window.set_position(PhysicalPosition::new(final_x, final_y)) {
                    log::error!("IPC: Failed to set popup position: {}", e);
                    return;
                }

                // Apply Mica effect (Windows 11)
                #[cfg(target_os = "windows")]
                {
                    let _ = window_vibrancy::apply_mica(&window, Some(true));
                }

                if let Err(e) = window.show() {
                    log::error!("IPC: Failed to show popup: {}", e);
                    return;
                }

                if let Err(e) = window.set_focus() {
                    log::error!("IPC: Failed to focus popup: {}", e);
                    return;
                }

                // Emit event to popup window to refresh text
                if let Err(e) = app_handle_clone.emit("popup-text-updated", text) {
                    log::error!("IPC: Failed to emit popup-text-updated event: {}", e);
                }

                log::info!("IPC: Popup shown at ({}, {})", final_x, final_y);
            }
        } else {
            log::warn!("IPC: Popup window not found");
        }
    });
}

/// Finds the monitor that contains the given point (x, y).
/// Supports multi-monitor setups with negative coordinates (monitors left of primary).
fn find_monitor_at_point(app_handle: &AppHandle, x: i32, y: i32) -> Option<tauri::Monitor> {
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
                    "IPC: Found monitor for point ({}, {}): {:?} at ({}, {}) size {}x{}",
                    x, y,
                    monitor.name(),
                    left, top, size.width, size.height
                );
                return Some(monitor);
            }
        }
        log::warn!("IPC: Point ({}, {}) not found on any monitor", x, y);
    } else {
        log::error!("IPC: Failed to enumerate monitors");
    }

    // Fallback to primary monitor if point not found on any monitor
    app_handle.primary_monitor().ok().flatten()
}
