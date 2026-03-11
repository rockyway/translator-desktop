use tauri::Window;

#[derive(Debug, thiserror::Error)]
pub enum WindowError {
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    #[error("Failed to apply window effect: {0}")]
    EffectError(String),
    #[error("Tauri error: {0}")]
    TauriError(#[from] tauri::Error),
}

impl serde::Serialize for WindowError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Apply Mica effect (Windows 11) or vibrancy (macOS)
#[tauri::command]
pub fn apply_mica_effect(_window: Window) -> Result<(), WindowError> {
    #[cfg(target_os = "windows")]
    {
        window_vibrancy::apply_mica(&_window, Some(true))
            .map_err(|e| WindowError::EffectError(e.to_string()))?;
    }

    Ok(())
}

/// Apply Acrylic effect (Windows 10/11 fallback)
#[tauri::command]
pub fn apply_acrylic_effect(_window: Window) -> Result<(), WindowError> {
    #[cfg(target_os = "windows")]
    {
        window_vibrancy::apply_acrylic(&_window, Some((18, 18, 18, 125)))
            .map_err(|e| WindowError::EffectError(e.to_string()))?;
    }

    Ok(())
}

/// Minimize the window
#[tauri::command]
pub fn minimize_window(window: Window) -> Result<(), WindowError> {
    window.minimize()?;
    Ok(())
}

/// Toggle maximize/restore window
#[tauri::command]
pub fn toggle_maximize_window(window: Window) -> Result<(), WindowError> {
    if window.is_maximized()? {
        window.unmaximize()?;
    } else {
        window.maximize()?;
    }
    Ok(())
}

/// Close the window
#[tauri::command]
pub fn close_window(window: Window) -> Result<(), WindowError> {
    window.close()?;
    Ok(())
}

/// Check if window is maximized
#[tauri::command]
pub fn is_window_maximized(window: Window) -> Result<bool, WindowError> {
    Ok(window.is_maximized()?)
}

/// Start drag operation for window movement
#[tauri::command]
pub fn start_drag_window(window: Window) -> Result<(), WindowError> {
    window.start_dragging()?;
    Ok(())
}
