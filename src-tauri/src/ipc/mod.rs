//! IPC module for communication with the .NET TextMonitor service.
//!
//! This module provides Named Pipe client functionality to receive
//! text selection events from the TextMonitor.Service, and to send
//! configuration updates to the service.
//! Windows-only: Named Pipes are not available on macOS/Linux.

#[cfg(target_os = "windows")]
mod config_pipe;
#[cfg(target_os = "windows")]
mod named_pipe;

#[cfg(target_os = "windows")]
pub use config_pipe::{send_config, ConfigMessage};
#[cfg(target_os = "windows")]
pub use named_pipe::{is_ipc_connected, start_ipc_listener};

// Non-Windows: check macOS monitor running state
#[cfg(not(target_os = "windows"))]
pub fn is_ipc_connected() -> bool {
    #[cfg(target_os = "macos")]
    {
        crate::macos_monitor::is_monitor_running()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn start_ipc_listener(_app_handle: tauri::AppHandle) {
    log::info!("IPC: Named Pipe listener not available on this platform");
}
