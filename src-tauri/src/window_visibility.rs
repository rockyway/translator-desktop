//! Window + webview visibility helpers.
//!
//! On Windows, `WebviewWindow::hide()` only hides the parent HWND — it never
//! reaches the WebView2 controller, so `ICoreWebView2Controller::IsVisible`
//! stays `true` and the webview keeps compositing forever. Any running CSS
//! animation in a hidden window therefore keeps the shared WebView2 GPU process
//! busy for the whole lifetime of the app.
//!
//! Pairing every window show/hide with the matching webview show/hide makes
//! WebView2 actually stop rendering while a window sits in the tray.
//!
//! Always route window visibility changes through these helpers: a plain
//! `window.show()` on a window whose webview is hidden renders blank.

use tauri::webview::Webview;
use tauri::{Manager, Runtime, WebviewWindow};

/// Show a window and resume its webview rendering.
///
/// The webview is resumed first so the window never paints a blank frame.
pub fn show_window<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let webview: &Webview<R> = window.as_ref();
    let _ = webview.show();
    window.show()?;

    match window.label() {
        "main" => {
            // A minimized main stays in its epoch; restoring raises Resized, which activates.
            if matches!(window.is_minimized(), Ok(false)) {
                crate::memory_trim::activate(window.app_handle());
            }
        }
        "popup" | "confirmation" => crate::memory_trim::popup_shown(window.app_handle()),
        _ => {}
    }
    Ok(())
}

/// Hide a window and stop its webview rendering.
///
/// The window is hidden first so the user never sees a blank frame.
pub fn hide_window<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    window.hide()?;
    let webview: &Webview<R> = window.as_ref();
    let _ = webview.hide();

    match window.label() {
        "main" => crate::memory_trim::request(
            window.app_handle(),
            crate::memory_trim::Trigger::Hidden,
        ),
        "popup" | "confirmation" => crate::memory_trim::request(
            window.app_handle(),
            crate::memory_trim::Trigger::PopupHidden,
        ),
        _ => {}
    }
    Ok(())
}

/// Stop rendering for a window that is already hidden.
///
/// Used at startup for windows declared with `"visible": false`, whose webviews
/// are nonetheless created visible and would otherwise render indefinitely.
pub fn suspend_hidden_webview<R: Runtime>(window: &WebviewWindow<R>) {
    let webview: &Webview<R> = window.as_ref();
    let _ = webview.hide();
}
