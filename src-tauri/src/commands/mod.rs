//! Tauri commands module.
//!
//! This module contains all IPC commands exposed to the frontend.

mod history;
mod hotkey;
mod popup;
mod settings;
mod speak;
mod translate;
mod window;

pub use history::{
    add_history, clear_history, delete_history, get_history, init_database, search_history,
    DbState,
};
pub use hotkey::{handle_global_hotkey, simulate_copy, trigger_hotkey_translate};
pub use popup::{get_popup_text, hide_popup, is_popup_visible, resize_popup, set_popup_text, show_popup, PopupTextState};
pub use settings::{get_all_settings, get_setting, init_config_store, is_autostart_enabled, set_autostart_enabled, set_setting, update_global_hotkey, update_selection_modifier, HotkeyState};
pub use speak::speak;
pub use translate::{translate, HttpClientState};
pub use window::{
    apply_acrylic_effect, apply_mica_effect, close_window, is_window_maximized, minimize_window,
    start_drag_window, toggle_maximize_window,
};
