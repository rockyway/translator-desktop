//! Named Pipe client for receiving text selection events from .NET TextMonitor.

use crate::popup_handler::show_popup_with_text;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

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

