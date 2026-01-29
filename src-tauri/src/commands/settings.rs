//! Settings commands for managing application configuration.
//!
//! This module provides CRUD operations for the config_store table,
//! allowing persistent storage of application settings like theme,
//! language preferences, and UI state.

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tokio::sync::Mutex;

use super::DbState;
use crate::ipc::{send_config, ConfigMessage};

/// State for tracking the current global hotkey shortcut.
pub struct HotkeyState(pub Arc<Mutex<Shortcut>>);

impl Default for HotkeyState {
    fn default() -> Self {
        // Default: Ctrl+Shift+Q
        Self(Arc::new(Mutex::new(Shortcut::new(
            Some(Modifiers::CONTROL | Modifiers::SHIFT),
            Code::KeyQ,
        ))))
    }
}

/// Parses a modifier string to Modifiers enum.
fn parse_modifiers(modifier: &str) -> Result<Modifiers, String> {
    match modifier {
        "ctrl+shift" => Ok(Modifiers::CONTROL | Modifiers::SHIFT),
        "ctrl+alt" => Ok(Modifiers::CONTROL | Modifiers::ALT),
        "alt+shift" => Ok(Modifiers::ALT | Modifiers::SHIFT),
        _ => Err(format!(
            "Invalid modifier: {}. Valid: ctrl+shift, ctrl+alt, alt+shift",
            modifier
        )),
    }
}

/// Parse letter (a-z) to Tauri Code enum.
fn parse_letter_to_code(letter: &str) -> Result<Code, String> {
    match letter.to_lowercase().as_str() {
        "a" => Ok(Code::KeyA),
        "b" => Ok(Code::KeyB),
        "c" => Ok(Code::KeyC),
        "d" => Ok(Code::KeyD),
        "e" => Ok(Code::KeyE),
        "f" => Ok(Code::KeyF),
        "g" => Ok(Code::KeyG),
        "h" => Ok(Code::KeyH),
        "i" => Ok(Code::KeyI),
        "j" => Ok(Code::KeyJ),
        "k" => Ok(Code::KeyK),
        "l" => Ok(Code::KeyL),
        "m" => Ok(Code::KeyM),
        "n" => Ok(Code::KeyN),
        "o" => Ok(Code::KeyO),
        "p" => Ok(Code::KeyP),
        "q" => Ok(Code::KeyQ),
        "r" => Ok(Code::KeyR),
        "s" => Ok(Code::KeyS),
        "t" => Ok(Code::KeyT),
        "u" => Ok(Code::KeyU),
        "v" => Ok(Code::KeyV),
        "w" => Ok(Code::KeyW),
        "x" => Ok(Code::KeyX),
        "y" => Ok(Code::KeyY),
        "z" => Ok(Code::KeyZ),
        _ => Err(format!("Invalid letter: {}. Must be A-Z.", letter)),
    }
}

/// Updates the global hotkey modifier at runtime.
/// Unregisters the old shortcut and registers the new one.
#[tauri::command]
pub async fn update_global_hotkey(
    app: AppHandle,
    hotkey_state: State<'_, HotkeyState>,
    modifier: String,
    letter: String,
) -> Result<(), String> {
    let modifiers = parse_modifiers(&modifier)?;
    let code = parse_letter_to_code(&letter)?;

    let mut current = hotkey_state.0.lock().await;

    // Unregister old shortcut
    if let Err(e) = app.global_shortcut().unregister(*current) {
        log::warn!("Failed to unregister old shortcut: {}", e);
        // Continue anyway - may not have been registered
    }

    // Create and register new shortcut
    let new_shortcut = Shortcut::new(Some(modifiers), code);
    match app.global_shortcut().register(new_shortcut) {
        Ok(_) => {
            // Update state on success
            *current = new_shortcut;
            log::info!(
                "Global hotkey updated to: {}+{}",
                modifier,
                letter.to_uppercase()
            );
            Ok(())
        }
        Err(_e) => {
            // Fallback: re-register old hotkey
            let _ = app.global_shortcut().register(*current);
            Err(format!(
                "Failed to register {}+{}. Key may be in use by another app.",
                modifier,
                letter.to_uppercase()
            ))
        }
    }
}

/// Updates the selection modifier in the .NET Text Monitor.
/// Called when user changes the setting in the UI.
#[tauri::command]
pub async fn update_selection_modifier(modifier: String) -> Result<(), String> {
    let message = ConfigMessage::update_selection_modifier(&modifier);
    send_config(message)
        .await
        .map_err(|e| format!("Failed to update selection modifier: {}", e))?;

    log::info!("Selection modifier updated to: {}", modifier);
    Ok(())
}

/// Represents a single config entry from the database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

/// Error type for settings operations
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Invalid JSON value: {0}")]
    InvalidJson(String),

    #[error("Key cannot be empty")]
    EmptyKey,
}

// Implement serialization for Tauri command error handling
impl Serialize for SettingsError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Initialize the config_store table schema
pub async fn init_config_store(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS config_store (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get a single setting by key
///
/// # Arguments
/// * `db_state` - The database state
/// * `key` - The setting key to retrieve
///
/// # Returns
/// * The setting value as JSON, or None if not found
#[tauri::command]
pub async fn get_setting(
    db_state: State<'_, DbState>,
    key: String,
) -> Result<Option<serde_json::Value>, SettingsError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(SettingsError::EmptyKey);
    }

    let pool = db_state.0.lock().await;

    let result = sqlx::query("SELECT value FROM config_store WHERE key = ?")
        .bind(key)
        .fetch_optional(&*pool)
        .await
        .map_err(|e| SettingsError::DatabaseError(e.to_string()))?;

    match result {
        Some(row) => {
            let value_str: String = row.get("value");
            let parsed: serde_json::Value = serde_json::from_str(&value_str)
                .map_err(|e| SettingsError::InvalidJson(e.to_string()))?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

/// Set a single setting (upsert)
///
/// # Arguments
/// * `db_state` - The database state
/// * `key` - The setting key
/// * `value` - The setting value as JSON
///
/// # Returns
/// * Ok(()) on success
#[tauri::command]
pub async fn set_setting(
    db_state: State<'_, DbState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), SettingsError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(SettingsError::EmptyKey);
    }

    let value_str =
        serde_json::to_string(&value).map_err(|e| SettingsError::InvalidJson(e.to_string()))?;

    let pool = db_state.0.lock().await;

    sqlx::query(
        r#"
        INSERT INTO config_store (key, value, updated_at)
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(key)
    .bind(&value_str)
    .execute(&*pool)
    .await
    .map_err(|e| SettingsError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Get all settings as a HashMap
///
/// # Arguments
/// * `db_state` - The database state
///
/// # Returns
/// * A HashMap of all settings (key -> JSON value)
#[tauri::command]
pub async fn get_all_settings(
    db_state: State<'_, DbState>,
) -> Result<HashMap<String, serde_json::Value>, SettingsError> {
    let pool = db_state.0.lock().await;

    let rows: Vec<ConfigEntry> = sqlx::query_as("SELECT key, value, updated_at FROM config_store")
        .fetch_all(&*pool)
        .await
        .map_err(|e| SettingsError::DatabaseError(e.to_string()))?;

    let mut settings = HashMap::new();
    for entry in rows {
        match serde_json::from_str(&entry.value) {
            Ok(parsed) => {
                settings.insert(entry.key, parsed);
            }
            Err(e) => {
                log::warn!(
                    "Failed to parse setting '{}': {}. Skipping.",
                    entry.key,
                    e
                );
            }
        }
    }

    Ok(settings)
}

/// Check if auto-start is enabled
///
/// # Returns
/// * true if the app is set to start automatically with the system
#[tauri::command]
pub fn is_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    let autostart = app.autolaunch();
    autostart
        .is_enabled()
        .map_err(|e| format!("Failed to check autostart status: {}", e))
}

/// Enable or disable auto-start
///
/// # Arguments
/// * `enabled` - Whether to enable or disable auto-start
#[tauri::command]
pub fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let autostart = app.autolaunch();

    if enabled {
        autostart
            .enable()
            .map_err(|e| format!("Failed to enable autostart: {}", e))?;
        log::info!("Auto-start enabled");
    } else {
        autostart
            .disable()
            .map_err(|e| format!("Failed to disable autostart: {}", e))?;
        log::info!("Auto-start disabled");
    }

    Ok(())
}

/// Get character limit setting from database (0 = disabled)
///
/// # Arguments
/// * `db_state` - The database state
///
/// # Returns
/// * The character limit value (default: 100)
pub async fn get_char_limit_setting(db_state: &DbState) -> usize {
    let pool = db_state.0.lock().await;
    sqlx::query_scalar::<_, String>(
        "SELECT value FROM config_store WHERE key = 'confirmation_char_limit'"
    )
    .fetch_optional(&*pool)
    .await
    .ok()
    .flatten()
    .and_then(|v| serde_json::from_str::<usize>(&v).ok())
    .unwrap_or(100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_key_validation() {
        let key = "   ".trim();
        assert!(key.is_empty());
    }

    #[test]
    fn test_valid_key() {
        let key = "theme".trim();
        assert!(!key.is_empty());
    }

    #[test]
    fn test_json_serialization() {
        let value = serde_json::json!("dark");
        let serialized = serde_json::to_string(&value).unwrap();
        assert_eq!(serialized, "\"dark\"");

        let value = serde_json::json!(true);
        let serialized = serde_json::to_string(&value).unwrap();
        assert_eq!(serialized, "true");

        let value = serde_json::json!({
            "nested": "value"
        });
        let serialized = serde_json::to_string(&value).unwrap();
        assert!(serialized.contains("nested"));
    }

    #[test]
    fn test_json_deserialization() {
        let json_str = "\"light\"";
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed, serde_json::json!("light"));

        let json_str = "true";
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed, serde_json::json!(true));

        let json_str = "\"en\"";
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed, serde_json::json!("en"));
    }

    #[test]
    fn test_settings_error_display() {
        let err = SettingsError::EmptyKey;
        assert_eq!(err.to_string(), "Key cannot be empty");

        let err = SettingsError::DatabaseError("connection failed".to_string());
        assert_eq!(err.to_string(), "Database error: connection failed");

        let err = SettingsError::InvalidJson("parse error".to_string());
        assert_eq!(err.to_string(), "Invalid JSON value: parse error");
    }
}
