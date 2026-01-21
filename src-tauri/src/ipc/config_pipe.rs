//! Configuration pipe client for sending settings to .NET Text Monitor.
//!
//! This module provides functionality to send configuration updates
//! (like selection modifier changes) to the .NET sidecar via named pipe.

use serde::Serialize;
use std::io::Write;
use tokio::time::{sleep, Duration};

/// Pipe name for configuration messages to the .NET Text Monitor.
const CONFIG_PIPE_NAME: &str = r"\\.\pipe\TranslatorDesktopConfig";

/// Maximum number of retry attempts for sending configuration.
const MAX_RETRIES: u32 = 5;

/// Initial delay before first retry (doubles each attempt).
const INITIAL_RETRY_DELAY_MS: u64 = 1000;

/// Configuration message sent to .NET Text Monitor.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

impl ConfigMessage {
    /// Creates a new configuration message with the current timestamp.
    pub fn new(message_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            message_type: message_type.into(),
            payload,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Creates an update_selection_modifier message.
    ///
    /// # Arguments
    /// * `modifier` - The modifier key to use for text selection (e.g., "ctrl", "alt", "shift")
    pub fn update_selection_modifier(modifier: &str) -> Self {
        Self::new(
            "update_selection_modifier",
            serde_json::json!({ "modifier": modifier }),
        )
    }
}

/// Error type for configuration pipe operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigPipeError {
    #[error("Failed to connect to config pipe: {0}")]
    ConnectionFailed(String),

    #[error("Failed to send message: {0}")]
    SendFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Sends a configuration message to the .NET Text Monitor.
///
/// Uses retry logic with exponential backoff (3 attempts: 500ms, 1s, 2s delays).
/// Returns Ok(()) if message was sent successfully, or Err if all retries failed.
///
/// # Arguments
/// * `message` - The configuration message to send
///
/// # Example
/// ```ignore
/// let message = ConfigMessage::update_selection_modifier("alt");
/// send_config(message).await?;
/// ```
pub async fn send_config(message: ConfigMessage) -> Result<(), ConfigPipeError> {
    let json = serde_json::to_string(&message)?;
    let message_with_newline = format!("{}\n", json);

    let mut last_error = None;
    let mut delay = INITIAL_RETRY_DELAY_MS;

    for attempt in 1..=MAX_RETRIES {
        match send_to_pipe(&message_with_newline) {
            Ok(()) => {
                log::info!(
                    "Config message sent successfully: {} (attempt {})",
                    message.message_type,
                    attempt
                );
                return Ok(());
            }
            Err(e) => {
                log::warn!(
                    "Failed to send config message (attempt {}/{}): {}",
                    attempt,
                    MAX_RETRIES,
                    e
                );
                last_error = Some(e);

                if attempt < MAX_RETRIES {
                    sleep(Duration::from_millis(delay)).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| ConfigPipeError::ConnectionFailed("Unknown error".into())))
}

/// Internal function to send data to the named pipe.
///
/// Opens the pipe, writes the data, and flushes the buffer.
fn send_to_pipe(data: &str) -> Result<(), ConfigPipeError> {
    use std::fs::OpenOptions;

    #[cfg(target_os = "windows")]
    use std::os::windows::fs::OpenOptionsExt;

    // Open pipe for writing
    #[cfg(target_os = "windows")]
    let mut file = OpenOptions::new()
        .write(true)
        .custom_flags(0) // FILE_ATTRIBUTE_NORMAL
        .open(CONFIG_PIPE_NAME)
        .map_err(|e| ConfigPipeError::ConnectionFailed(e.to_string()))?;

    #[cfg(not(target_os = "windows"))]
    let mut file = OpenOptions::new()
        .write(true)
        .open(CONFIG_PIPE_NAME)
        .map_err(|e| ConfigPipeError::ConnectionFailed(e.to_string()))?;

    // Write message
    file.write_all(data.as_bytes())
        .map_err(|e| ConfigPipeError::SendFailed(e.to_string()))?;

    file.flush()
        .map_err(|e| ConfigPipeError::SendFailed(e.to_string()))?;

    Ok(())
}
