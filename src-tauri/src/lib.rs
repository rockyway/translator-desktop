// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

mod commands;
mod ipc;
mod sidecar;

use commands::{
    add_history, apply_acrylic_effect, apply_mica_effect, clear_history, close_window,
    delete_history, get_all_settings, get_confirmation_data, get_history, get_popup_text,
    get_setting, handle_global_hotkey, hide_popup, init_config_store, init_database,
    is_autostart_enabled, is_popup_visible, is_window_maximized, minimize_window, resize_popup,
    respond_to_confirmation, search_history, set_autostart_enabled, set_popup_text, set_setting,
    show_popup, simulate_copy, speak, start_drag_window, toggle_maximize_window, translate,
    trigger_hotkey_translate, update_global_hotkey, update_selection_modifier,
    ConfirmationDataState, ConfirmationState, DbState, HotkeyState, HttpClientState,
    PopupTextState,
};
use ipc::{is_ipc_connected, start_ipc_listener};
use sidecar::{init_job_object, is_text_monitor_running, start_text_monitor, stop_text_monitor, SidecarState};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::fs;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
// Unused imports removed - Code, Modifiers, Shortcut are imported via commands module
use tokio::sync::Mutex;

// Global flag to track if app should truly exit (vs minimize to tray)
static FORCE_EXIT: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Get the current IPC connection status.
/// This allows the frontend to query the connection state on initialization.
#[tauri::command]
fn get_ipc_status() -> bool {
    is_ipc_connected()
}

/// Exit the application completely (bypasses minimize to tray)
#[tauri::command]
async fn exit_app(app: tauri::AppHandle) {
    FORCE_EXIT.store(true, Ordering::SeqCst);

    // Stop the sidecar first
    let sidecar_state = app.state::<SidecarState>();
    if let Err(e) = sidecar_state.stop().await {
        log::error!("Failed to stop sidecar on exit: {}", e);
    }

    app.exit(0);
}

/// Check if minimize_to_tray setting is enabled
async fn get_minimize_to_tray_setting(app: &tauri::AppHandle) -> bool {
    let db_state = app.try_state::<DbState>();
    if let Some(db) = db_state {
        let pool = db.0.lock().await;
        sqlx::query_scalar::<_, String>(
            "SELECT value FROM config_store WHERE key = 'minimize_to_tray'",
        )
        .fetch_optional(&*pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str::<bool>(&v).ok())
        .unwrap_or(true) // Default to true
    } else {
        true // Default to true if no database
    }
}

/// Show a native error dialog for critical errors
fn show_error_dialog(title: &str, message: &str) {
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(message)
        .set_level(rfd::MessageLevel::Error)
        .show();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger (respects RUST_LOG env variable: RUST_LOG=debug)
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug) // Set default level to Debug
        .init();

    // Set up panic hook to log crashes to file (critical for release builds with no console)
    std::panic::set_hook(Box::new(|panic_info| {
        let msg = format!(
            "Application panic: {}\n\nBacktrace:\n{:?}",
            panic_info,
            std::backtrace::Backtrace::capture()
        );

        // Try to write to a crash log file in user's data directory
        if let Some(data_dir) = dirs::data_dir() {
            let crash_log = data_dir.join("translator-desktop-crash.log");
            let _ = std::fs::write(&crash_log, &msg);
        }

        // Also try stderr (useful for dev builds)
        eprintln!("{}", msg);
    }));

    log::info!("=== Translator Desktop Starting ===");

    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    // Only handle key press events (not release)
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        // Check if this matches the current hotkey (dynamic)
                        if let Some(hotkey_state) = app.try_state::<HotkeyState>() {
                            if let Ok(current) = hotkey_state.0.try_lock() {
                                if *shortcut == *current {
                                    handle_global_hotkey(app.clone());
                                }
                            }
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]), // Start minimized when auto-launched
        ))
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize job object to ensure child processes terminate with parent
            init_job_object();

            // Initialize popup text state
            app.manage(PopupTextState::default());

            // Initialize sidecar state for text monitor
            app.manage(SidecarState::new());

            // Initialize hotkey state with default
            app.manage(HotkeyState::default());

            // Initialize confirmation state for character limit dialog
            app.manage(ConfirmationState::default());

            // Initialize confirmation data state for React to fetch on mount
            app.manage(ConfirmationDataState::default());

            // Initialize shared HTTP client for translation requests
            // Creating once avoids TLS/connection pool setup on first request
            app.manage(HttpClientState::default());

            // Apply Mica effect and set window icon (Windows 11)
            #[cfg(target_os = "windows")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window_vibrancy::apply_mica(&window, Some(true));

                    // Set window icon for taskbar using embedded PNG
                    let icon_bytes = include_bytes!("../icons/128x128.png");
                    if let Ok(img) = image::load_from_memory(icon_bytes) {
                        let rgba = img.to_rgba8();
                        let (width, height) = rgba.dimensions();
                        let icon = tauri::image::Image::new_owned(
                            rgba.into_raw(),
                            width,
                            height,
                        );
                        let _ = window.set_icon(icon);
                    }
                }
            }

            // Create system tray icon with context menu
            let icon_bytes = include_bytes!("../icons/32x32.png");
            let tray_icon = if let Ok(img) = image::load_from_memory(icon_bytes) {
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();
                tauri::image::Image::new_owned(rgba.into_raw(), width, height)
            } else {
                // Fallback to empty icon if load fails
                tauri::image::Image::new_owned(vec![0; 32 * 32 * 4], 32, 32)
            };

            // Create tray menu
            let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let exit_item = MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &exit_item])?;

            // Build the tray icon
            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&tray_menu)
                .tooltip("Translator Desktop")
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "exit" => {
                            FORCE_EXIT.store(true, Ordering::SeqCst);
                            let app_clone = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let sidecar_state = app_clone.state::<SidecarState>();
                                let _ = sidecar_state.stop().await;
                                app_clone.exit(0);
                            });
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Set up Ctrl+C handler for graceful shutdown
            let app_handle_ctrlc = app.handle().clone();
            ctrlc::set_handler(move || {
                println!("\n[App] Received Ctrl+C, shutting down gracefully...");

                // Stop the sidecar
                let sidecar_state = app_handle_ctrlc.state::<SidecarState>();
                tauri::async_runtime::block_on(async {
                    let _ = sidecar_state.stop().await;
                });

                println!("[App] Cleanup complete, exiting.");
                std::process::exit(0);
            }).expect("Error setting Ctrl+C handler");

            let app_handle = app.handle().clone();
            let sidecar_app_handle = app.handle().clone();

            // Initialize SQLite database for history
            tauri::async_runtime::block_on(async move {
                log::info!("Initializing database...");

                // Get app data directory with fallback
                let app_data_dir = match app_handle.path().app_data_dir() {
                    Ok(dir) => dir,
                    Err(e) => {
                        log::error!(
                            "Failed to get app data directory: {}. Using temp directory.",
                            e
                        );
                        std::env::temp_dir().join("translator-desktop")
                    }
                };

                // Ensure directory exists
                if let Err(e) = fs::create_dir_all(&app_data_dir) {
                    log::error!("Failed to create app data directory: {}", e);
                }

                log::info!("App data directory: {:?}", app_data_dir);

                // Set up database connection
                let db_path = app_data_dir.join("translator.db");
                let db_url = format!("sqlite:{}", db_path.to_string_lossy());
                log::info!("Database path: {}", db_url);

                // Parse database options with fallback chain
                let options = match SqliteConnectOptions::from_str(&db_url) {
                    Ok(opts) => opts.create_if_missing(true),
                    Err(e) => {
                        log::error!("Failed to parse database URL '{}': {}", db_url, e);
                        // Try a fallback path in temp directory
                        let fallback_path = std::env::temp_dir()
                            .join("translator-desktop")
                            .join("translator.db");
                        let fallback_url = format!("sqlite:{}", fallback_path.display());
                        log::warn!("Trying fallback database path: {}", fallback_url);

                        // Ensure fallback directory exists
                        if let Some(parent) = fallback_path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }

                        match SqliteConnectOptions::from_str(&fallback_url) {
                            Ok(opts) => opts.create_if_missing(true),
                            Err(e2) => {
                                log::error!("Fallback path also failed: {}", e2);
                                log::warn!("Using in-memory database as last resort");
                                SqliteConnectOptions::from_str("sqlite::memory:")
                                    .expect("In-memory SQLite should always work")
                                    .create_if_missing(true)
                            }
                        }
                    }
                };

                match SqlitePool::connect_with(options).await {
                    Ok(pool) => {
                        // Initialize database schema
                        if let Err(e) = init_database(&pool).await {
                            log::error!("Failed to initialize database schema: {}", e);
                        }

                        // Initialize config_store table for settings
                        if let Err(e) = init_config_store(&pool).await {
                            log::error!("Failed to initialize config_store schema: {}", e);
                        }

                        // Register the database pool as managed state
                        app_handle.manage(DbState(Mutex::new(pool)));
                        log::info!("Database initialized successfully");
                    }
                    Err(e) => {
                        log::error!("Failed to connect to database: {}", e);
                        show_error_dialog(
                            "Database Error",
                            &format!(
                                "Failed to initialize database: {}\n\n\
                                The application may not work correctly.\n\
                                Please check your disk space and permissions.",
                                e
                            ),
                        );
                    }
                }
            });

            // Start IPC listener to receive text selection events from .NET TextMonitor
            start_ipc_listener(app.handle().clone());

            // Register global hotkey based on saved setting (or default to Ctrl+Shift+Q)
            let app_for_hotkey = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // Wait for database to be ready
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                // Load both hotkey_modifier and hotkey_letter from database
                let (modifier, letter) = {
                    let db_state = app_for_hotkey.try_state::<DbState>();
                    if let Some(db) = db_state {
                        let pool = db.0.lock().await;

                        let modifier = sqlx::query_scalar::<_, String>(
                            "SELECT value FROM config_store WHERE key = 'hotkey_modifier'",
                        )
                        .fetch_optional(&*pool)
                        .await
                        .ok()
                        .flatten()
                        .and_then(|v| serde_json::from_str::<String>(&v).ok())
                        .unwrap_or_else(|| "ctrl+shift".to_string());

                        let letter = sqlx::query_scalar::<_, String>(
                            "SELECT value FROM config_store WHERE key = 'hotkey_letter'",
                        )
                        .fetch_optional(&*pool)
                        .await
                        .ok()
                        .flatten()
                        .and_then(|v| serde_json::from_str::<String>(&v).ok())
                        .unwrap_or_else(|| "q".to_string());

                        drop(pool);
                        (modifier, letter)
                    } else {
                        ("ctrl+shift".to_string(), "q".to_string())
                    }
                };

                // Register initial hotkey using the update_global_hotkey function
                if let Err(e) = update_global_hotkey(
                    app_for_hotkey.clone(),
                    app_for_hotkey.state::<HotkeyState>(),
                    modifier.clone(),
                    letter.clone(),
                )
                .await
                {
                    log::error!("Failed to register initial hotkey: {}", e);
                } else {
                    log::info!(
                        "Initial global shortcut {}+{} registered successfully",
                        modifier,
                        letter.to_uppercase()
                    );
                }
            });

            // Start the text monitor sidecar after a brief delay to ensure app is ready
            let config_app_handle = sidecar_app_handle.clone();
            tauri::async_runtime::spawn(async move {
                // Small delay to ensure all services are initialized
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                log::info!("Attempting to start text monitor sidecar...");
                let sidecar_state = sidecar_app_handle.state::<SidecarState>();
                match sidecar_state.start(&sidecar_app_handle).await {
                    Ok(()) => {
                        log::info!("Sidecar started successfully");
                        println!("[App] Text monitor sidecar started");
                    }
                    Err(e) => {
                        log::error!("Failed to start text monitor sidecar: {}", e);
                        eprintln!("[App Error] Failed to start text monitor sidecar: {}", e);
                        // Don't crash - app can work without sidecar (hotkey-only mode)
                    }
                }
            });

            // Send initial configuration to .NET after sidecar starts
            tauri::async_runtime::spawn(async move {
                // Wait for .NET config receiver to be ready
                tokio::time::sleep(std::time::Duration::from_millis(3000)).await;

                // Get saved selection_modifier from database
                let modifier = {
                    let db_state = config_app_handle.try_state::<DbState>();
                    if let Some(db) = db_state {
                        let pool = db.0.lock().await;
                        sqlx::query_scalar::<_, String>(
                            "SELECT value FROM config_store WHERE key = 'selection_modifier'",
                        )
                        .fetch_optional(&*pool)
                        .await
                        .ok()
                        .flatten()
                        .and_then(|v| serde_json::from_str::<String>(&v).ok())
                        .unwrap_or_else(|| "alt".to_string()) // Default to alt
                    } else {
                        "alt".to_string()
                    }
                };

                // Send to .NET
                use crate::ipc::{send_config, ConfigMessage};
                log::info!("Sending initial selection modifier to .NET: {}", modifier);
                let message = ConfigMessage::update_selection_modifier(&modifier);
                match send_config(message).await {
                    Ok(()) => {
                        log::info!("Initial selection modifier '{}' sent to .NET", modifier);
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to send initial config to .NET (may not be ready yet): {}",
                            e
                        );
                    }
                }
            });

            log::info!("=== Initialization Complete ===");

            Ok(())
        })
        .on_window_event(|window, event| {
            // Handle close request for main window
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    // Check if force exit is requested (from Exit button or tray menu)
                    if FORCE_EXIT.load(Ordering::SeqCst) {
                        // Allow the close, sidecar cleanup happens in exit_app or tray handler
                        return;
                    }

                    // Check minimize_to_tray setting
                    let app_handle = window.app_handle().clone();
                    let window_clone = window.clone();

                    // Prevent the default close behavior
                    api.prevent_close();

                    tauri::async_runtime::spawn(async move {
                        let minimize_to_tray = get_minimize_to_tray_setting(&app_handle).await;

                        if minimize_to_tray {
                            // Hide window to tray instead of closing
                            let _ = window_clone.hide();
                        } else {
                            // Actually close the app
                            FORCE_EXIT.store(true, Ordering::SeqCst);
                            let sidecar_state = app_handle.state::<SidecarState>();
                            let _ = sidecar_state.stop().await;
                            app_handle.exit(0);
                        }
                    });
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            translate,
            speak,
            add_history,
            get_history,
            delete_history,
            clear_history,
            search_history,
            show_popup,
            hide_popup,
            resize_popup,
            is_popup_visible,
            set_popup_text,
            get_popup_text,
            simulate_copy,
            trigger_hotkey_translate,
            start_text_monitor,
            stop_text_monitor,
            is_text_monitor_running,
            get_ipc_status,
            get_setting,
            set_setting,
            get_all_settings,
            update_global_hotkey,
            update_selection_modifier,
            apply_mica_effect,
            apply_acrylic_effect,
            minimize_window,
            toggle_maximize_window,
            close_window,
            is_window_maximized,
            start_drag_window,
            exit_app,
            is_autostart_enabled,
            set_autostart_enabled,
            respond_to_confirmation,
            get_confirmation_data
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // Handle app exit to ensure sidecar is stopped
            if let tauri::RunEvent::Exit = event {
                println!("[App] Application exiting, stopping sidecar...");
                let sidecar_state = app_handle.state::<SidecarState>();
                // Use block_on since we're in the exit handler
                tauri::async_runtime::block_on(async {
                    if let Err(e) = sidecar_state.stop().await {
                        eprintln!("[App Error] Failed to stop sidecar on exit: {}", e);
                    } else {
                        println!("[App] Sidecar stopped successfully");
                    }
                });
            }
        });
}
