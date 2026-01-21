//! IPC module for communication with the .NET TextMonitor service.
//!
//! This module provides Named Pipe client functionality to receive
//! text selection events from the TextMonitor.Service, and to send
//! configuration updates to the service.

mod config_pipe;
mod named_pipe;

pub use config_pipe::{send_config, ConfigMessage};
pub use named_pipe::start_ipc_listener;
