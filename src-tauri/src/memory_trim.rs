//! Idle working-set trim for the host, its WebView2 process tree, and the .NET sidecar (Windows).
//!
//! One visible trim, at most one subsequent LOW trim, and one settle trim 5 minutes after the LOW trim per quiet epoch, never periodic.
//! See docs/plan/005-idle-memory-trim.md. Native work stays on the
//! main (webview) thread; detached tasks own `Arc`s, never `State` borrows.
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, Runtime};

/// Windows whose visibility makes the app busy (a trim would fault the pages the
/// user is looking at).
const POPUP_LABELS: [&str; 2] = ["popup", "confirmation"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    Startup,
    Hidden,
    Minimized,
    Deactivated,
    PopupHidden,
    Settle,
}
impl Trigger {
    fn delay(self) -> Duration {
        Duration::from_secs(match self {
            Self::Startup => 8,
            Self::Deactivated => 15,
            Self::Settle => 300,
            _ => 3,
        })
    }
    #[cfg(windows)]
    fn name(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Hidden => "hidden",
            Self::Minimized => "minimized",
            Self::Deactivated => "deactivated",
            Self::PopupHidden => "popup_hidden",
            Self::Settle => "settle",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Epoch {
    #[default]
    Untrimmed,
    Visible,
    Low,
    Settled,
}

#[derive(Default)]
pub struct IdleTrimScheduler {
    generation: u64,
    pending: Option<(Trigger, Instant)>,
    epoch: Epoch,
    // Main launches visible and usually focused, but the initial WebView2 GotFocus can
    // be missed, so admission also checks the foreground HWND at fire time.
    main_focused: bool,
    started: bool,
    minimized: bool,
    task: Option<tauri::async_runtime::JoinHandle<()>>,
}
impl IdleTrimScheduler {
    fn request(&mut self, trigger: Trigger, now: Instant) -> Option<u64> {
        if trigger == Trigger::Startup {
            if self.started {
                return None;
            }
            self.started = true;
        }
        if (trigger == Trigger::Settle && self.epoch != Epoch::Low)
            || (trigger != Trigger::Settle
                && matches!(self.epoch, Epoch::Low | Epoch::Settled))
            || (self.epoch == Epoch::Visible
                && !matches!(
                    trigger,
                    Trigger::Hidden | Trigger::Minimized | Trigger::PopupHidden
                ))
            || (trigger == Trigger::PopupHidden && self.main_focused)
        {
            return None;
        }
        // A longer or equal delay never pushes an existing deadline out.
        if self
            .pending
            .is_some_and(|(_, deadline)| deadline <= now + trigger.delay())
        {
            return None;
        }
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.generation += 1;
        self.pending = Some((trigger, now + trigger.delay()));
        Some(self.generation)
    }
    fn cancel(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.generation += 1;
        self.pending = None;
    }
    /// The user is back in the app: drop any pending trim and open a new epoch.
    fn activate(&mut self) {
        self.cancel();
        self.epoch = Epoch::Untrimmed;
    }
    fn abandon(&mut self, generation: u64) {
        if generation == self.generation {
            self.pending = None;
        }
    }
    fn focused(&mut self, focused: bool) {
        self.main_focused = focused;
        if focused {
            self.activate();
        }
    }
    fn fire(
        &mut self,
        generation: u64,
        now: Instant,
        busy: bool,
        low: bool,
        foreground: bool,
    ) -> Option<Trigger> {
        if generation != self.generation {
            return None;
        }
        let (trigger, deadline) = self.pending?;
        if now < deadline {
            return None;
        }
        // Main in use drops the request instead of re-polling while the user works:
        // leaving main raises its own trigger (Focused(false), hide, minimize). The
        // epoch is not consumed, so that later trigger gets its full delay.
        if self.main_focused || foreground {
            self.pending = None;
            return None;
        }
        // Busy postpones by the trigger's delay; it never drops.
        if busy {
            self.pending = Some((trigger, now + trigger.delay()));
            return None;
        }
        self.pending = None;
        if trigger == Trigger::Settle {
            if !low {
                return None;
            }
            self.epoch = Epoch::Settled;
            return Some(Trigger::Settle);
        }
        if self.epoch == Epoch::Visible && !low {
            return None;
        }
        self.epoch = if low { Epoch::Low } else { Epoch::Visible };
        Some(trigger)
    }
}

impl Drop for IdleTrimScheduler {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

#[derive(Default)]
pub struct MemoryTrim(Arc<Mutex<IdleTrimScheduler>>);

impl Drop for MemoryTrim {
    fn drop(&mut self) {
        // Tasks own scheduler Arcs; break the cycle and invalidate any queued
        // main-thread closure before the state goes away.
        if let Ok(mut scheduler) = self.0.lock() {
            scheduler.cancel();
        }
    }
}

fn scheduler<R: Runtime>(app: &AppHandle<R>) -> Option<Arc<Mutex<IdleTrimScheduler>>> {
    app.try_state::<MemoryTrim>().map(|state| Arc::clone(&state.0))
}

#[cfg(windows)]
fn main_is_foreground<R: Runtime>(window: &tauri::WebviewWindow<R>) -> bool {
    let Ok(hwnd) = window.hwnd() else {
        return false;
    };
    let foreground = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
    foreground.0 as isize == hwnd.0 as isize
}

#[cfg(not(windows))]
fn main_is_foreground<R: Runtime>(_: &tauri::WebviewWindow<R>) -> bool {
    false
}

/// Main window focus from `WindowEvent::Focused`.
pub fn focused<R: Runtime>(app: &AppHandle<R>, focused: bool) {
    let Some(scheduler) = scheduler(app) else {
        return;
    };
    scheduler.lock().unwrap().focused(focused);
    if focused {
        memory_target(app, false);
    }
}

/// Main window shown or restored.
pub fn activate<R: Runtime>(app: &AppHandle<R>) {
    let Some(scheduler) = scheduler(app) else {
        return;
    };
    scheduler.lock().unwrap().activate();
    memory_target(app, false);
}

/// Popup or confirmation shown: the user is using the app again, so a new epoch
/// starts. Main keeps its memory target; it may still be in the tray.
pub fn popup_shown<R: Runtime>(app: &AppHandle<R>) {
    if let Some(scheduler) = scheduler(app) {
        scheduler.lock().unwrap().activate();
    }
}

/// Main focus as tracked from Focused events (false when the state is absent).
#[cfg(test)]
pub fn main_focused<R: Runtime>(app: &AppHandle<R>) -> bool {
    scheduler(app).is_some_and(|s| s.lock().unwrap().main_focused)
}

pub fn request<R: Runtime>(app: &AppHandle<R>, trigger: Trigger) {
    let Some(scheduler) = scheduler(app) else {
        return;
    };
    let Some(generation) = scheduler.lock().unwrap().request(trigger, Instant::now()) else {
        return;
    };
    let owner = Arc::clone(&scheduler);
    let app = app.clone();
    let task = tauri::async_runtime::spawn(async move {
        loop {
            let deadline = {
                let scheduler = scheduler.lock().unwrap();
                if scheduler.generation != generation {
                    return;
                }
                scheduler.pending.map(|(_, deadline)| deadline)
            };
            let Some(deadline) = deadline else {
                return;
            };
            tokio::time::sleep_until(deadline.into()).await;
            let handle = app.clone();
            let admission = Arc::clone(&scheduler);
            let (tx, rx) = tokio::sync::oneshot::channel();
            if app
                .run_on_main_thread(move || {
                    // Native state errors refuse admission.
                    let Some(main) = handle.get_webview_window("main") else {
                        admission.lock().unwrap().abandon(generation);
                        let _ = tx.send(());
                        return;
                    };
                    let (Ok(visible), Ok(minimized)) = (main.is_visible(), main.is_minimized())
                    else {
                        admission.lock().unwrap().abandon(generation);
                        let _ = tx.send(());
                        return;
                    };
                    let foreground = main_is_foreground(&main);
                    let low = !visible || minimized;
                    let busy = POPUP_LABELS.iter().any(|label| {
                        handle
                            .get_webview_window(label)
                            .is_some_and(|w| w.is_visible().unwrap_or(true))
                    });
                    let trigger =
                        admission
                            .lock()
                            .unwrap()
                            .fire(generation, Instant::now(), busy, low, foreground);
                    if let Some(trigger) = trigger {
                        trim(&handle, trigger, low);
                        if low && trigger != Trigger::Settle {
                            request(&handle, Trigger::Settle);
                        }
                    }
                    let _ = tx.send(());
                })
                .is_err()
            {
                scheduler.lock().unwrap().abandon(generation);
                return;
            }
            if rx.await.is_err() {
                // The closure was dropped unrun (event loop gone): don't leave a past deadline.
                scheduler.lock().unwrap().abandon(generation);
                return;
            }
        }
    });
    let mut scheduler = owner.lock().unwrap();
    if scheduler.generation == generation {
        scheduler.task = Some(task);
    } else {
        task.abort();
    }
}

/// Main window `WindowEvent::Resized`; Tauri has no minimize event.
pub fn resized<R: Runtime>(window: &tauri::Window<R>) {
    let Ok(minimized) = window.is_minimized() else {
        return;
    };
    let app = window.app_handle();
    let Some(scheduler) = scheduler(app) else {
        return;
    };
    let changed = {
        let mut scheduler = scheduler.lock().unwrap();
        let changed = scheduler.minimized != minimized;
        scheduler.minimized = minimized;
        changed
    };
    if changed {
        if minimized {
            request(app, Trigger::Minimized);
        } else {
            activate(app);
        }
    }
}

#[cfg(not(windows))]
fn memory_target<R: Runtime>(_: &AppHandle<R>, _: bool) {}
#[cfg(not(windows))]
fn trim<R: Runtime>(_: &AppHandle<R>, _: Trigger, _: bool) {}

/// WebView2 memory target for the main webview only: the popup must paint fast.
#[cfg(windows)]
fn memory_target<R: Runtime>(app: &AppHandle<R>, low: bool) {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use windows_core::Interface;
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.with_webview(move |webview| unsafe {
        let result = (|| {
            let core = webview.controller().CoreWebView2()?;
            core.cast::<ICoreWebView2_19>()?
                .SetMemoryUsageTargetLevel(if low {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                } else {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                })
        })();
        if let Err(error) = result {
            log::warn!("memory_target_failed hresult={:#x}", error.code().0);
        }
    });
}

#[cfg(windows)]
fn trim<R: Runtime>(app: &AppHandle<R>, trigger: Trigger, low: bool) {
    if low {
        memory_target(app, true);
    }
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let sidecar = app
        .try_state::<crate::sidecar::SidecarState>()
        .and_then(|state| state.pid());
    let _ = window.with_webview(move |webview| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::*;
        use windows_core::Interface;
        let start = Instant::now();
        let mut pids = std::collections::BTreeSet::new();
        pids.insert(std::process::id());
        let sidecar_included = sidecar.is_some();
        if let Some(pid) = sidecar {
            pids.insert(pid);
        }
        let mut browser = 0;
        // One environment serves all three windows, so this covers every renderer.
        let result: windows_core::Result<()> = (|| {
            let core = webview.controller().CoreWebView2()?;
            core.BrowserProcessId(&mut browser)?;
            if browser != 0 {
                pids.insert(browser);
            }
            let infos = core
                .cast::<ICoreWebView2_2>()?
                .Environment()?
                .cast::<ICoreWebView2Environment8>()?
                .GetProcessInfos()?;
            let mut count = 0;
            infos.Count(&mut count)?;
            for index in 0..count {
                let result = (|| {
                    let mut pid = 0;
                    infos.GetValueAtIndex(index)?.ProcessId(&mut pid)?;
                    if pid > 0 {
                        pids.insert(pid as u32);
                    }
                    Ok::<_, windows_core::Error>(())
                })();
                if let Err(error) = result {
                    log::warn!("memory_process_info_failed hresult={:#x}", error.code().0);
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            log::warn!("memory_process_discovery_failed hresult={:#x}", error.code().0);
        }
        // GetProcessInfos omits crashpad: add the browser's direct WebView2 children.
        if browser != 0 {
            if let Err(error) = browser_children(browser, &mut pids) {
                log::warn!("memory_browser_children_failed hresult={:#x}", error.code().0);
            }
        }
        let (trimmed, failed) = trim_processes(&pids);
        log::info!(
            "idle_memory_trim trigger={} low={} sidecar={} processes={} trimmed={} failed={} elapsed_ms={}",
            trigger.name(),
            low,
            sidecar_included,
            pids.len(),
            trimmed,
            failed,
            start.elapsed().as_millis()
        );
    });
}

#[cfg(windows)]
fn is_browser_child(browser: u32, parent: u32, image: &[u16]) -> bool {
    let end = image.iter().position(|&c| c == 0).unwrap_or(image.len());
    parent == browser
        && String::from_utf16_lossy(&image[..end]).eq_ignore_ascii_case("msedgewebview2.exe")
}

#[cfg(windows)]
fn browser_children(
    browser: u32,
    pids: &mut std::collections::BTreeSet<u32>,
) -> windows::core::Result<()> {
    use windows::Win32::{
        Foundation::{CloseHandle, ERROR_NO_MORE_FILES},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
    };
    // SAFETY: the snapshot handle is owned here and closed on every path;
    // PROCESSENTRY32W has its required size set.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
        let result = (|| {
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut next = Process32FirstW(snapshot, &mut entry);
            loop {
                match next {
                    Ok(()) => {
                        if is_browser_child(browser, entry.th32ParentProcessID, &entry.szExeFile) {
                            pids.insert(entry.th32ProcessID);
                        }
                    }
                    Err(error)
                        if error.code()
                            == windows::core::HRESULT::from_win32(ERROR_NO_MORE_FILES.0) =>
                    {
                        return Ok(())
                    }
                    Err(error) => return Err(error),
                }
                next = Process32NextW(snapshot, &mut entry);
            }
        })();
        let _ = CloseHandle(snapshot);
        result
    }
}

#[cfg(windows)]
fn trim_processes(pids: &std::collections::BTreeSet<u32>) -> (usize, usize) {
    use windows::Win32::{
        Foundation::CloseHandle,
        System::{
            Memory::{SetProcessWorkingSetSizeEx, SETPROCESSWORKINGSETSIZEEX_FLAGS},
            Threading::{
                GetCurrentProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
                PROCESS_SET_QUOTA,
            },
        },
    };
    let mut trimmed = 0;
    for &pid in pids {
        // SAFETY: handles from OpenProcess are closed right after use;
        // GetCurrentProcess is a pseudo-handle that needs no close.
        let result = unsafe {
            if pid == std::process::id() {
                SetProcessWorkingSetSizeEx(
                    GetCurrentProcess(),
                    usize::MAX,
                    usize::MAX,
                    SETPROCESSWORKINGSETSIZEEX_FLAGS(0),
                )
            } else {
                OpenProcess(
                    PROCESS_SET_QUOTA | PROCESS_QUERY_LIMITED_INFORMATION,
                    false,
                    pid,
                )
                .and_then(|handle| {
                    let result = SetProcessWorkingSetSizeEx(
                        handle,
                        usize::MAX,
                        usize::MAX,
                        SETPROCESSWORKINGSETSIZEEX_FLAGS(0),
                    );
                    let _ = CloseHandle(handle);
                    result
                })
            }
        };
        match result {
            Ok(()) => trimmed += 1,
            Err(error) => log::warn!("memory_working_set_trim_failed hresult={:#x}", error.code().0),
        }
    }
    (trimmed, pids.len() - trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Trigger; 6] = [
        Trigger::Startup,
        Trigger::Hidden,
        Trigger::Minimized,
        Trigger::Deactivated,
        Trigger::PopupHidden,
        Trigger::Settle,
    ];

    fn enter_low_epoch(scheduler: &mut IdleTrimScheduler, now: Instant) {
        let generation = scheduler.request(Trigger::Startup, now).unwrap();
        assert_eq!(
            scheduler.fire(
                generation,
                now + Trigger::Startup.delay(),
                false,
                true,
                false,
            ),
            Some(Trigger::Startup)
        );
        assert_eq!(scheduler.epoch, Epoch::Low);
    }

    #[test]
    fn delays_are_exact_and_never_fire_early() {
        for (trigger, seconds) in [
            (Trigger::Startup, 8),
            (Trigger::Hidden, 3),
            (Trigger::Minimized, 3),
            (Trigger::Deactivated, 15),
            (Trigger::PopupHidden, 3),
            (Trigger::Settle, 300),
        ] {
            let mut s = IdleTrimScheduler::default();
            let now = Instant::now();
            if trigger == Trigger::Settle {
                enter_low_epoch(&mut s, now);
            }
            assert_eq!(trigger.delay(), Duration::from_secs(seconds));
            let request_at = if trigger == Trigger::Settle {
                now + Trigger::Startup.delay()
            } else {
                now
            };
            let generation = s.request(trigger, request_at).unwrap();
            let deadline = request_at + Duration::from_secs(seconds);
            let low = trigger == Trigger::Settle;
            assert_eq!(s.fire(generation, deadline - Duration::from_millis(1), false, low, false), None);
            assert_eq!(s.fire(generation, deadline, false, low, false), Some(trigger));
            assert_eq!(s.fire(generation, deadline, false, low, false), None);
        }

        let mut s = IdleTrimScheduler::default();
        let now = Instant::now();
        let generation = s.request(Trigger::Hidden, now).unwrap();
        assert_eq!(
            s.fire(
                generation,
                now + Trigger::Hidden.delay() + Duration::from_millis(500),
                false,
                false,
                false,
            ),
            Some(Trigger::Hidden)
        );
    }

    #[test]
    fn settle_is_only_accepted_in_low_epoch() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        assert!(scheduler.request(Trigger::Settle, now).is_none());

        let visible = scheduler.request(Trigger::Startup, now).unwrap();
        assert_eq!(
            scheduler.fire(visible, now + Trigger::Startup.delay(), false, false, false),
            Some(Trigger::Startup)
        );
        assert_eq!(scheduler.epoch, Epoch::Visible);
        assert!(scheduler
            .request(Trigger::Settle, now + Trigger::Startup.delay())
            .is_none());

        let low_at = now + Trigger::Startup.delay() + Duration::from_secs(1);
        let low = scheduler.request(Trigger::Hidden, low_at).unwrap();
        assert_eq!(
            scheduler.fire(low, low_at + Trigger::Hidden.delay(), false, true, false),
            Some(Trigger::Hidden)
        );
        assert_eq!(scheduler.epoch, Epoch::Low);
        assert!(scheduler
            .request(Trigger::Settle, low_at + Trigger::Hidden.delay())
            .is_some());
    }

    #[test]
    fn settle_fires_after_exact_delay_and_locks_settled_epoch() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        enter_low_epoch(&mut scheduler, now);
        let requested_at = now + Trigger::Startup.delay();
        let generation = scheduler.request(Trigger::Settle, requested_at).unwrap();
        let deadline = requested_at + Duration::from_secs(300);
        assert_eq!(Trigger::Settle.delay(), Duration::from_secs(300));
        assert_eq!(
            scheduler.fire(
                generation,
                deadline - Duration::from_millis(1),
                false,
                true,
                false,
            ),
            None
        );
        assert_eq!(
            scheduler.fire(generation, deadline, false, true, false),
            Some(Trigger::Settle)
        );
        assert_eq!(scheduler.epoch, Epoch::Settled);
        assert!(scheduler.pending.is_none());
        // Reset startup's one-shot guard to isolate Settled-epoch admission.
        scheduler.started = false;
        for trigger in ALL {
            assert!(scheduler.request(trigger, deadline).is_none(), "{trigger:?}");
        }
    }

    #[test]
    fn settle_dropped_if_main_is_visible() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        enter_low_epoch(&mut scheduler, now);
        let requested_at = now + Trigger::Startup.delay();
        let generation = scheduler.request(Trigger::Settle, requested_at).unwrap();
        assert_eq!(
            scheduler.fire(
                generation,
                requested_at + Trigger::Settle.delay(),
                false,
                false,
                false,
            ),
            None
        );
        assert_eq!(scheduler.epoch, Epoch::Low);
        assert!(scheduler.pending.is_none());
    }

    #[test]
    fn activate_cancels_pending_settle_and_opens_new_epoch() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        enter_low_epoch(&mut scheduler, now);
        let generation = scheduler
            .request(Trigger::Settle, now + Trigger::Startup.delay())
            .unwrap();
        scheduler.activate();
        assert!(scheduler.generation > generation);
        assert!(scheduler.pending.is_none());
        assert_eq!(scheduler.epoch, Epoch::Untrimmed);
        assert_eq!(
            scheduler.fire(
                generation,
                now + Trigger::Startup.delay() + Trigger::Settle.delay(),
                false,
                true,
                false,
            ),
            None
        );
        assert!(scheduler
            .request(Trigger::Hidden, now + Trigger::Startup.delay())
            .is_some());
    }

    #[test]
    fn focused_true_cancels_pending_settle_and_opens_new_epoch() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        enter_low_epoch(&mut scheduler, now);
        let generation = scheduler
            .request(Trigger::Settle, now + Trigger::Startup.delay())
            .unwrap();
        scheduler.focused(true);
        assert!(scheduler.generation > generation);
        assert!(scheduler.pending.is_none());
        assert_eq!(scheduler.epoch, Epoch::Untrimmed);
        assert_eq!(scheduler.fire(generation, now + Duration::from_secs(400), false, true, false), None);
        assert!(scheduler
            .request(Trigger::Hidden, now + Trigger::Startup.delay())
            .is_some());
    }

    #[test]
    fn settle_drops_when_focused_or_foreground_and_postpones_when_busy() {
        let now = Instant::now();
        for focused in [true, false] {
            let mut scheduler = IdleTrimScheduler::default();
            enter_low_epoch(&mut scheduler, now);
            let requested_at = now + Trigger::Startup.delay();
            let generation = scheduler.request(Trigger::Settle, requested_at).unwrap();
            if focused {
                scheduler.main_focused = true;
            }
            assert_eq!(
                scheduler.fire(
                    generation,
                    requested_at + Trigger::Settle.delay(),
                    false,
                    true,
                    !focused,
                ),
                None
            );
            assert_eq!(scheduler.epoch, Epoch::Low);
            assert!(scheduler.pending.is_none());
        }

        let mut scheduler = IdleTrimScheduler::default();
        enter_low_epoch(&mut scheduler, now);
        let requested_at = now + Trigger::Startup.delay();
        let generation = scheduler.request(Trigger::Settle, requested_at).unwrap();
        let deadline = requested_at + Trigger::Settle.delay();
        assert_eq!(scheduler.fire(generation, deadline, true, true, false), None);
        assert_eq!(scheduler.pending, Some((Trigger::Settle, deadline + Trigger::Settle.delay())));
        assert_eq!(
            scheduler.fire(
                generation,
                deadline + Trigger::Settle.delay() - Duration::from_millis(1),
                false,
                true,
                false,
            ),
            None
        );
        assert_eq!(
            scheduler.fire(generation, deadline + Trigger::Settle.delay(), false, true, false),
            Some(Trigger::Settle)
        );
        assert_eq!(scheduler.epoch, Epoch::Settled);
    }

    #[test]
    fn activation_alone_invalidates_generation() {
        for trigger in [Trigger::Hidden, Trigger::Startup, Trigger::Deactivated] {
            let mut s = IdleTrimScheduler::default();
            let now = Instant::now();
            let old = s.request(trigger, now).unwrap();
            s.activate();
            assert!(s.generation > old);
            assert!(s.pending.is_none());
            assert_eq!(s.fire(old, now + Duration::from_secs(30), false, true, false), None);
            let new = s.request(Trigger::Deactivated, now).unwrap();
            assert_eq!(
                s.fire(new, now + Duration::from_secs(15), false, false, false),
                Some(Trigger::Deactivated)
            );
        }
    }

    #[test]
    fn shorter_delay_replaces_longer_and_rejects_old_generation() {
        for longer in [Trigger::Deactivated, Trigger::Startup] {
            for shorter in [Trigger::Hidden, Trigger::Minimized, Trigger::PopupHidden] {
                let mut s = IdleTrimScheduler::default();
                let now = Instant::now();
                let old = s.request(longer, now).unwrap();
                let new = s.request(shorter, now).unwrap();
                assert!(new > old);
                assert_eq!(s.fire(old, now + Duration::from_secs(3), false, true, false), None);
                assert_eq!(s.fire(new, now + Duration::from_secs(3), false, true, false), Some(shorter));
                assert_eq!(s.fire(old, now + Duration::from_secs(15), false, true, false), None);
            }
        }
    }

    #[test]
    fn longer_request_never_pushes_deadline_out() {
        let mut s = IdleTrimScheduler::default();
        let now = Instant::now();
        let generation = s.request(Trigger::Hidden, now).unwrap();
        assert!(s.request(Trigger::Deactivated, now + Duration::from_secs(1)).is_none());
        assert!(s.request(Trigger::Hidden, now + Duration::from_secs(1)).is_none());
        assert_eq!(
            s.fire(generation, now + Duration::from_secs(3), false, true, false),
            Some(Trigger::Hidden)
        );
        // A shorter trigger arriving late must not push an earlier deadline out.
        let mut s = IdleTrimScheduler::default();
        let startup = s.request(Trigger::Startup, now).unwrap();
        assert!(s.request(Trigger::Hidden, now + Duration::from_secs(7)).is_none());
        assert_eq!(s.pending, Some((Trigger::Startup, now + Duration::from_secs(8))));
        assert_eq!(
            s.fire(startup, now + Duration::from_secs(8), false, true, false),
            Some(Trigger::Startup)
        );
    }

    #[test]
    fn tracked_focus_gates_admission_and_popup_hide() {
        let mut s = IdleTrimScheduler::default();
        let now = Instant::now();
        assert!(!s.main_focused);
        s.focused(true);
        assert!(s.request(Trigger::PopupHidden, now).is_none());
        let generation = s.request(Trigger::Hidden, now).unwrap();
        // Tracked focus at fire time drops the request without consuming the epoch.
        assert_eq!(s.fire(generation, now + Duration::from_secs(3), false, true, false), None);
        assert!(s.pending.is_none());
        assert_eq!(s.epoch, Epoch::Untrimmed);
        assert_eq!(s.fire(generation, now + Duration::from_secs(6), false, true, false), None);
        s.focused(false);
        let next = s.request(Trigger::Hidden, now + Duration::from_secs(6)).unwrap();
        assert_eq!(
            s.fire(next, now + Duration::from_secs(9), false, true, false),
            Some(Trigger::Hidden)
        );
        s.focused(true);
        assert_eq!(s.epoch, Epoch::Untrimmed);
        assert!(s.pending.is_none());
    }

    #[test]
    fn focused_true_cancels_pending_request() {
        let mut s = IdleTrimScheduler::default();
        let now = Instant::now();
        let generation = s.request(Trigger::Hidden, now).unwrap();
        s.focused(true);
        assert!(s.generation > generation);
        assert!(s.pending.is_none());
        assert_eq!(
            s.fire(generation, now + Duration::from_secs(3), false, false, false),
            None
        );
    }

    #[test]
    fn foreground_startup_drops_without_consuming_epoch() {
        for later in [Trigger::Deactivated, Trigger::Hidden, Trigger::Minimized] {
            let mut s = IdleTrimScheduler::default();
            let now = Instant::now();
            s.focused(true);
            let generation = s.request(Trigger::Startup, now).unwrap();
            // Focus wins even when a popup is busy.
            assert_eq!(s.fire(generation, now + Duration::from_secs(8), true, false, false), None);
            assert!(s.pending.is_none());
            assert_eq!(s.epoch, Epoch::Untrimmed);
            assert!(s.request(Trigger::Startup, now + Duration::from_secs(9)).is_none());
            s.focused(false);
            let start = now + Duration::from_secs(9);
            let next = s.request(later, start).unwrap();
            assert_eq!(s.pending, Some((later, start + later.delay())));
            assert_eq!(
                s.fire(next, start + later.delay() - Duration::from_millis(1), false, true, false),
                None
            );
            assert_eq!(s.fire(next, start + later.delay(), false, true, false), Some(later));
        }
    }

    #[test]
    fn foreground_admission_drops_without_consuming_epoch() {
        let mut s = IdleTrimScheduler::default();
        let now = Instant::now();
        assert!(!s.main_focused);
        let startup = s.request(Trigger::Startup, now).unwrap();
        assert_eq!(
            s.fire(startup, now + Duration::from_secs(8), false, false, true),
            None
        );
        assert_eq!(s.epoch, Epoch::Untrimmed);
        assert!(s.pending.is_none());

        let hidden_at = now + Duration::from_secs(9);
        let hidden = s.request(Trigger::Hidden, hidden_at).unwrap();
        let fire_at = hidden_at + Duration::from_secs(3);
        // Foreground drops (no 3 s re-poll), even when a popup is also busy.
        assert_eq!(s.fire(hidden, fire_at, true, true, true), None);
        assert!(s.pending.is_none());
        assert_eq!(s.epoch, Epoch::Untrimmed);
        assert_eq!(s.fire(hidden, fire_at + Duration::from_secs(3), false, true, false), None);
        let next = s.request(Trigger::Hidden, fire_at).unwrap();
        assert_eq!(
            s.fire(next, fire_at + Duration::from_secs(3), false, true, false),
            Some(Trigger::Hidden)
        );
    }

    #[test]
    fn busy_postpone_uses_trigger_delay() {
        for (trigger, delay) in [
            (Trigger::Deactivated, Duration::from_secs(15)),
            (Trigger::Hidden, Duration::from_secs(3)),
        ] {
            let mut s = IdleTrimScheduler::default();
            let now = Instant::now();
            let generation = s.request(trigger, now).unwrap();
            let fire_at = now + delay;
            assert_eq!(s.fire(generation, fire_at, true, false, false), None);
            assert_eq!(s.pending, Some((trigger, fire_at + delay)));
        }
    }

    #[test]
    fn busy_popup_postpones_and_never_drops() {
        let mut s = IdleTrimScheduler::default();
        let now = Instant::now();
        let generation = s.request(Trigger::Startup, now).unwrap();
        for seconds in [8, 16, 24] {
            assert_eq!(s.fire(generation, now + Duration::from_secs(seconds), true, false, false), None);
        }
        assert_eq!(s.fire(generation, now + Duration::from_secs(31), false, false, false), None);
        assert_eq!(
            s.fire(generation, now + Duration::from_secs(32), false, false, false),
            Some(Trigger::Startup)
        );
    }

    #[test]
    fn visible_then_one_low_trim_per_epoch() {
        for visible in [Trigger::Startup, Trigger::Deactivated] {
            for low in [Trigger::Hidden, Trigger::Minimized, Trigger::PopupHidden] {
                let mut s = IdleTrimScheduler::default();
                let now = Instant::now();
                let first = s.request(visible, now).unwrap();
                let after = now + visible.delay();
                assert_eq!(s.fire(first, after, false, false, false), Some(visible));
                assert_eq!(s.epoch, Epoch::Visible);
                assert!(s.request(Trigger::Deactivated, after).is_none());
                let second = s.request(low, after).unwrap();
                assert_eq!(s.fire(second, after + low.delay(), false, true, false), Some(low));
                assert_eq!(s.epoch, Epoch::Low);
                for trigger in ALL {
                    assert_eq!(
                        s.request(trigger, after + low.delay()).is_some(),
                        trigger == Trigger::Settle
                    );
                }
                s.activate();
                assert_eq!(s.epoch, Epoch::Untrimmed);
                assert!(s.request(Trigger::Hidden, after + low.delay()).is_some());
            }
        }
    }

    #[test]
    fn hidden_startup_locks_low_epoch() {
        let mut s = IdleTrimScheduler::default();
        let now = Instant::now();
        let generation = s.request(Trigger::Startup, now).unwrap();
        assert_eq!(
            s.fire(generation, now + Duration::from_secs(8), false, true, false),
            Some(Trigger::Startup)
        );
        assert_eq!(s.epoch, Epoch::Low);
        for trigger in ALL {
            assert_eq!(
                s.request(trigger, now + Duration::from_secs(9)).is_some(),
                trigger == Trigger::Settle
            );
        }
    }

    #[test]
    fn abandon_clears_only_matching_generation_and_allows_retry() {
        let mut s = IdleTrimScheduler::default();
        let now = Instant::now();
        let generation = s.request(Trigger::Hidden, now).unwrap();
        let pending = s.pending;
        s.abandon(generation + 1);
        assert_eq!(s.pending, pending);
        s.abandon(generation);
        assert!(s.pending.is_none());
        assert_eq!(s.generation, generation);

        let retry_at = now + Duration::from_secs(1);
        let retry = s.request(Trigger::Hidden, retry_at).unwrap();
        assert!(retry > generation);
        assert_eq!(s.pending, Some((Trigger::Hidden, retry_at + Trigger::Hidden.delay())));
        let retry_pending = s.pending;
        s.abandon(generation);
        assert_eq!(s.pending, retry_pending);
    }

    #[test]
    fn popup_hide_does_not_repeat_visible_trim() {
        let mut s = IdleTrimScheduler::default();
        let now = Instant::now();
        let first = s.request(Trigger::Deactivated, now).unwrap();
        assert_eq!(
            s.fire(first, now + Duration::from_secs(15), false, false, false),
            Some(Trigger::Deactivated)
        );
        let second = s.request(Trigger::PopupHidden, now + Duration::from_secs(16)).unwrap();
        assert_eq!(s.fire(second, now + Duration::from_secs(19), false, false, false), None);
        assert_eq!(s.epoch, Epoch::Visible);
    }

    /// Main in the tray: each popup round trip is its own quiet epoch.
    #[test]
    fn popup_use_reopens_low_epoch_while_main_in_tray() {
        let mut s = IdleTrimScheduler::default();
        let now = Instant::now();
        let start = s.request(Trigger::Startup, now).unwrap();
        assert_eq!(s.fire(start, now + Duration::from_secs(8), false, true, false), Some(Trigger::Startup));
        assert_eq!(s.epoch, Epoch::Low);
        for round in 1..=3u64 {
            let shown = now + Duration::from_secs(20 * round);
            s.activate(); // popup_shown
            assert_eq!(s.epoch, Epoch::Untrimmed);
            let hidden = shown + Duration::from_secs(5);
            let generation = s.request(Trigger::PopupHidden, hidden).unwrap();
            assert_eq!(
                s.fire(generation, hidden + Duration::from_secs(3), false, true, false),
                Some(Trigger::PopupHidden)
            );
            assert!(s.request(Trigger::PopupHidden, hidden + Duration::from_secs(4)).is_none());
        }
    }

    #[test]
    fn popup_reshown_within_delay_cancels_trim() {
        let mut s = IdleTrimScheduler::default();
        let now = Instant::now();
        let generation = s.request(Trigger::PopupHidden, now).unwrap();
        s.activate(); // popup_shown again before 3 s
        assert_eq!(s.fire(generation, now + Duration::from_secs(3), false, true, false), None);
        assert!(s.pending.is_none());
    }

    #[tokio::test]
    async fn absent_state_request_does_not_panic_or_mutate_another_scheduler() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let untouched = MemoryTrim::default();
        request(app.handle(), Trigger::Startup);
        let scheduler = untouched.0.lock().unwrap();
        assert_eq!(scheduler.generation, 0);
        assert!(scheduler.pending.is_none());
        assert!(scheduler.task.is_none());
    }

    async fn pending_task() -> (
        tauri::async_runtime::JoinHandle<()>,
        tokio::sync::oneshot::Receiver<()>,
    ) {
        let (started, ready) = tokio::sync::oneshot::channel();
        let (alive, stopped) = tokio::sync::oneshot::channel::<()>();
        let task = tauri::async_runtime::spawn(async move {
            let _alive = alive;
            let _ = started.send(());
            std::future::pending::<()>().await;
        });
        ready.await.unwrap();
        (task, stopped)
    }

    #[tokio::test]
    async fn request_replacement_aborts_previous_task() {
        let mut scheduler = IdleTrimScheduler::default();
        let now = Instant::now();
        let old = scheduler.request(Trigger::Deactivated, now).unwrap();
        let (task, stopped) = pending_task().await;
        scheduler.task = Some(task);
        let replacement = scheduler
            .request(Trigger::Hidden, now + Duration::from_secs(1))
            .unwrap();
        assert!(replacement > old);
        assert!(tokio::time::timeout(Duration::from_secs(2), stopped)
            .await
            .unwrap()
            .is_err());
    }

    #[tokio::test]
    async fn scheduler_drop_aborts_task() {
        let (task, stopped) = pending_task().await;
        let scheduler = IdleTrimScheduler {
            task: Some(task),
            ..Default::default()
        };
        drop(scheduler);
        assert!(tokio::time::timeout(Duration::from_secs(2), stopped)
            .await
            .unwrap()
            .is_err());
    }

    #[tokio::test]
    async fn unmanage_cancels_owned_scheduler_task() {
        let app = tauri::test::mock_builder()
            .manage(MemoryTrim::default())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let scheduler = Arc::clone(&app.state::<MemoryTrim>().0);
        let old = scheduler
            .lock()
            .unwrap()
            .request(Trigger::Hidden, Instant::now())
            .unwrap();
        let (task, stopped) = pending_task().await;
        scheduler.lock().unwrap().task = Some(task);
        // No State borrow is outstanding; the retained scheduler is an owned Arc.
        #[allow(deprecated)]
        drop(app.unmanage::<MemoryTrim>());
        assert!(tokio::time::timeout(Duration::from_secs(2), stopped)
            .await
            .unwrap()
            .is_err());
        let mut scheduler = scheduler.lock().unwrap();
        assert!(scheduler.generation > old);
        assert!(scheduler.pending.is_none());
        assert_eq!(scheduler.fire(old, Instant::now() + Duration::from_secs(30), false, true, false), None);
    }

    #[cfg(windows)]
    #[test]
    fn browser_child_filter() {
        let image = |name: &str| name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        assert!(is_browser_child(42, 42, &image("msedgewebview2.exe")));
        assert!(is_browser_child(42, 42, &image("MSEDGEWEBVIEW2.EXE")));
        assert!(!is_browser_child(42, 99, &image("msedgewebview2.exe")));
        assert!(!is_browser_child(42, 42, &image("other.exe")));
        assert!(!is_browser_child(42, 42, &image("msedgewebview2.exe.extra")));

        let name: Vec<u16> = "msedgewebview2.exe".encode_utf16().collect();
        let mut padded = vec![0xA55A; 260];
        padded[..name.len()].copy_from_slice(&name);
        padded[name.len()] = 0;
        assert!(is_browser_child(42, 42, &padded));
        assert!(is_browser_child(42, 42, &name)); // Full input slice, no terminator.
        assert!(!is_browser_child(42, 42, &vec![b'x' as u16; 260]));
    }

    #[tokio::test]
    async fn public_api_manages_scheduler_lifecycle() {
        let app = tauri::test::mock_builder()
            .manage(MemoryTrim::default())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        request(app.handle(), Trigger::Hidden);
        let scheduler = Arc::clone(&app.state::<MemoryTrim>().0);
        {
            let scheduler = scheduler.lock().unwrap();
            assert!(scheduler.pending.is_some());
            assert!(scheduler.task.is_some());
        }

        focused(app.handle(), true);
        assert!(main_focused(app.handle()));
        {
            let scheduler = scheduler.lock().unwrap();
            assert!(scheduler.pending.is_none());
            assert!(scheduler.task.is_none());
            assert!(scheduler.main_focused);
        }
        focused(app.handle(), false);

        {
            let mut scheduler = scheduler.lock().unwrap();
            let now = Instant::now();
            let generation = scheduler.request(Trigger::Startup, now).unwrap();
            assert_eq!(
                scheduler.fire(generation, now + Trigger::Startup.delay(), false, true, false),
                Some(Trigger::Startup)
            );
            assert_eq!(scheduler.epoch, Epoch::Low);
        }
        popup_shown(app.handle());
        assert_eq!(scheduler.lock().unwrap().epoch, Epoch::Untrimmed);

        // popup_shown and activate cancel a pending request through the public API.
        for cancel in [popup_shown::<tauri::test::MockRuntime>, activate] {
            request(app.handle(), Trigger::PopupHidden);
            let old = {
                let scheduler = scheduler.lock().unwrap();
                assert!(scheduler.pending.is_some() && scheduler.task.is_some());
                scheduler.generation
            };
            cancel(app.handle());
            let scheduler = scheduler.lock().unwrap();
            assert!(scheduler.generation > old);
            assert!(scheduler.pending.is_none() && scheduler.task.is_none());
            assert_eq!(scheduler.epoch, Epoch::Untrimmed);
        }
    }

    #[cfg(windows)]
    #[test]
    fn browser_children_snapshot_is_readable() {
        let mut pids = [std::process::id()].into_iter().collect();
        browser_children(std::process::id(), &mut pids).unwrap();
        assert!(pids.contains(&std::process::id()));
    }

    #[cfg(windows)]
    #[test]
    fn trims_current_process() {
        let (trimmed, failed) = trim_processes(&[std::process::id()].into_iter().collect());
        assert_eq!((trimmed, failed), (1, 0));
    }
}
