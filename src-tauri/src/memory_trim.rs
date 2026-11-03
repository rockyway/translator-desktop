//! Idle working-set trim for the host, its WebView2 process tree, and the .NET sidecar (Windows).
//!
//! Low trims run only while main is hidden to the tray or minimized; one low trim per epoch
//! plus one settle trim 300 s later. If main stays visible behind another app, one deactivated
//! trim may run after 3600 s. Trims are never periodic.
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
    Hidden,
    Minimized,
    Deactivated,
    PopupHidden,
    Settle,
}
impl Trigger {
    fn delay(self) -> Duration {
        Duration::from_secs(match self {
            Self::Deactivated => 3600,
            Self::Settle => 300,
            _ => 3,
        })
    }
    #[cfg(windows)]
    fn name(self) -> &'static str {
        match self {
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
    minimized: bool,
    task: Option<tauri::async_runtime::JoinHandle<()>>,
}
impl IdleTrimScheduler {
    fn request(&mut self, trigger: Trigger, now: Instant) -> Option<u64> {
        if (trigger == Trigger::Settle && self.epoch != Epoch::Low)
            || (trigger == Trigger::Deactivated && self.epoch != Epoch::Untrimmed)
            || (matches!(trigger, Trigger::Hidden | Trigger::Minimized | Trigger::PopupHidden)
                && !matches!(self.epoch, Epoch::Untrimmed | Epoch::Visible))
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
        // Main in use drops the request instead of re-polling while the user works.
        if self.main_focused || foreground {
            self.pending = None;
            return None;
        }
        // Busy postpones by the trigger's delay; it never drops.
        if busy {
            self.pending = Some((trigger, now + trigger.delay()));
            return None;
        }
        if !low {
            match trigger {
                Trigger::Deactivated => {
                    self.pending = None;
                    self.epoch = Epoch::Visible;
                    return Some(trigger);
                }
                Trigger::PopupHidden => {
                    self.pending = Some((Trigger::Deactivated, now + Trigger::Deactivated.delay()));
                    return None;
                }
                _ => {
                    self.pending = None;
                    return None;
                }
            }
        }
        self.pending = None;
        if trigger == Trigger::Settle {
            self.epoch = Epoch::Settled;
        } else {
            self.epoch = Epoch::Low;
        }
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

    const ALL: [Trigger; 5] = [
        Trigger::Hidden,
        Trigger::Minimized,
        Trigger::Deactivated,
        Trigger::PopupHidden,
        Trigger::Settle,
    ];
    const LOW_TRIGGERS: [Trigger; 3] = [
        Trigger::Hidden,
        Trigger::Minimized,
        Trigger::PopupHidden,
    ];

    fn enter_low_epoch(scheduler: &mut IdleTrimScheduler, now: Instant, trigger: Trigger) {
        let generation = scheduler.request(trigger, now).unwrap();
        assert_eq!(
            scheduler.fire(generation, now + trigger.delay(), false, true, false),
            Some(trigger)
        );
        assert_eq!(scheduler.epoch, Epoch::Low);
    }

    fn enter_visible_epoch(scheduler: &mut IdleTrimScheduler, now: Instant) {
        let generation = scheduler.request(Trigger::Deactivated, now).unwrap();
        assert_eq!(
            scheduler.fire(
                generation,
                now + Trigger::Deactivated.delay(),
                false,
                false,
                false,
            ),
            Some(Trigger::Deactivated)
        );
        assert_eq!(scheduler.epoch, Epoch::Visible);
    }

    fn enter_settled_epoch(scheduler: &mut IdleTrimScheduler, now: Instant, trigger: Trigger) {
        enter_low_epoch(scheduler, now, trigger);
        let settle_at = now + trigger.delay();
        let generation = scheduler.request(Trigger::Settle, settle_at).unwrap();
        assert_eq!(
            scheduler.fire(
                generation,
                settle_at + Trigger::Settle.delay(),
                false,
                true,
                false,
            ),
            Some(Trigger::Settle)
        );
        assert_eq!(scheduler.epoch, Epoch::Settled);
    }

    #[test]
    fn delays_are_exact_and_never_fire_early() {
        for trigger in ALL {
            let mut scheduler = IdleTrimScheduler::default();
            let now = Instant::now();
            let (request_at, low) = match trigger {
                Trigger::Settle => {
                    enter_low_epoch(&mut scheduler, now, Trigger::Hidden);
                    (now + Trigger::Hidden.delay(), true)
                }
                Trigger::Deactivated => (now, false),
                _ => (now, true),
            };
            let delay = trigger.delay();
            assert_eq!(
                delay,
                Duration::from_secs(match trigger {
                    Trigger::Deactivated => 3600,
                    Trigger::Settle => 300,
                    _ => 3,
                })
            );
            let generation = scheduler.request(trigger, request_at).unwrap();
            let deadline = request_at + delay;
            assert_eq!(
                scheduler.fire(
                    generation,
                    deadline - Duration::from_millis(1),
                    false,
                    low,
                    false,
                ),
                None
            );
            assert_eq!(scheduler.fire(generation, deadline, false, low, false), Some(trigger));
            assert_eq!(scheduler.fire(generation, deadline, false, low, false), None);
        }
    }

    #[test]
    fn non_low_hidden_minimized_and_settle_are_dropped_without_consuming_epoch() {
        let now = Instant::now();
        for trigger in [Trigger::Hidden, Trigger::Minimized, Trigger::Settle] {
            let mut scheduler = IdleTrimScheduler::default();
            let generation = if trigger == Trigger::Settle {
                // Settle is normally accepted only in Low; probe the fire-time gate.
                scheduler.generation = 1;
                scheduler.pending = Some((trigger, now + trigger.delay()));
                1
            } else {
                scheduler.request(trigger, now).unwrap()
            };
            assert_eq!(
                scheduler.fire(generation, now + trigger.delay(), false, false, false),
                None
            );
            assert_eq!(scheduler.epoch, Epoch::Untrimmed);
            assert!(scheduler.pending.is_none());

            let hide_at = now + Duration::from_secs(4000);
            let hidden = scheduler.request(Trigger::Hidden, hide_at).unwrap();
            assert_eq!(
                scheduler.fire(
                    hidden,
                    hide_at + Trigger::Hidden.delay(),
                    false,
                    true,
                    false,
                ),
                Some(Trigger::Hidden)
            );
            assert_eq!(scheduler.epoch, Epoch::Low);
        }
    }

    #[test]
    fn popup_hidden_trims_while_main_is_hidden_and_rearms_when_visible() {
        let now = Instant::now();
        let mut hidden = IdleTrimScheduler::default();
        let generation = hidden.request(Trigger::PopupHidden, now).unwrap();
        assert_eq!(
            hidden.fire(
                generation,
                now + Trigger::PopupHidden.delay(),
                false,
                true,
                false,
            ),
            Some(Trigger::PopupHidden)
        );
        assert_eq!(hidden.epoch, Epoch::Low);

        let mut visible = IdleTrimScheduler::default();
        let generation = visible.request(Trigger::PopupHidden, now).unwrap();
        let popup_deadline = now + Trigger::PopupHidden.delay();
        assert_eq!(visible.fire(generation, popup_deadline, false, false, false), None);
        assert_eq!(visible.epoch, Epoch::Untrimmed);
        assert_eq!(
            visible.pending,
            Some((
                Trigger::Deactivated,
                popup_deadline + Trigger::Deactivated.delay()
            ))
        );
        assert_eq!(visible.generation, generation);
        assert_eq!(
            visible.fire(
                generation,
                popup_deadline + Trigger::Deactivated.delay() - Duration::from_millis(1),
                false,
                false,
                false,
            ),
            None
        );
        assert_eq!(
            visible.fire(
                generation,
                popup_deadline + Trigger::Deactivated.delay(),
                false,
                false,
                false,
            ),
            Some(Trigger::Deactivated)
        );
        assert_eq!(visible.epoch, Epoch::Visible);
    }

    #[test]
    fn hidden_low_settle_lifecycle_and_activate_reopens_epoch() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        assert!(scheduler.request(Trigger::Settle, now).is_none());
        enter_settled_epoch(&mut scheduler, now, Trigger::Hidden);
        assert!(scheduler.pending.is_none());
        for trigger in ALL {
            assert!(scheduler.request(trigger, now + Duration::from_secs(400)).is_none());
        }

        scheduler.activate();
        assert_eq!(scheduler.epoch, Epoch::Untrimmed);
        let generation = scheduler
            .request(Trigger::Hidden, now + Duration::from_secs(400))
            .unwrap();
        assert_eq!(
            scheduler.fire(
                generation,
                now + Duration::from_secs(400) + Trigger::Hidden.delay(),
                false,
                true,
                false,
            ),
            Some(Trigger::Hidden)
        );
        assert_eq!(scheduler.epoch, Epoch::Low);
    }

    #[test]
    fn minimized_follows_low_settle_lifecycle() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        enter_settled_epoch(&mut scheduler, now, Trigger::Minimized);
        assert_eq!(scheduler.epoch, Epoch::Settled);
        assert!(scheduler
            .request(Trigger::Minimized, now + Duration::from_secs(400))
            .is_none());
        scheduler.activate();
        assert_eq!(scheduler.epoch, Epoch::Untrimmed);
        assert!(scheduler
            .request(Trigger::Minimized, now + Duration::from_secs(400))
            .is_some());
    }

    #[test]
    fn deactivated_trim_moves_to_visible_epoch_and_hidden_still_trims_low() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        enter_visible_epoch(&mut scheduler, now);
        let hidden_at = now + Trigger::Deactivated.delay() + Duration::from_secs(1);
        let hidden = scheduler.request(Trigger::Hidden, hidden_at).unwrap();
        assert_eq!(
            scheduler.fire(
                hidden,
                hidden_at + Trigger::Hidden.delay(),
                false,
                true,
                false,
            ),
            Some(Trigger::Hidden)
        );
        assert_eq!(scheduler.epoch, Epoch::Low);

        let settle_at = hidden_at + Trigger::Hidden.delay();
        let settle = scheduler.request(Trigger::Settle, settle_at).unwrap();
        assert_eq!(
            scheduler.fire(
                settle,
                settle_at + Trigger::Settle.delay(),
                false,
                true,
                false,
            ),
            Some(Trigger::Settle)
        );
        assert_eq!(scheduler.epoch, Epoch::Settled);
    }

    #[test]
    fn only_eligible_triggers_are_accepted_in_each_epoch() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        for trigger in [Trigger::Hidden, Trigger::Minimized, Trigger::Deactivated, Trigger::PopupHidden] {
            assert!(scheduler.request(trigger, now).is_some(), "{trigger:?}");
            scheduler.cancel();
        }
        assert!(scheduler.request(Trigger::Settle, now).is_none());

        enter_visible_epoch(&mut scheduler, now);
        for trigger in LOW_TRIGGERS {
            assert!(scheduler
                .request(trigger, now + Duration::from_secs(4000))
                .is_some());
            scheduler.cancel();
        }
        assert!(scheduler
            .request(Trigger::Deactivated, now + Duration::from_secs(4000))
            .is_none());
        assert!(scheduler.request(Trigger::Settle, now).is_none());

        enter_low_epoch(&mut scheduler, now + Duration::from_secs(5000), Trigger::Hidden);
        for trigger in [Trigger::Hidden, Trigger::Minimized, Trigger::Deactivated, Trigger::PopupHidden] {
            assert!(scheduler
                .request(trigger, now + Duration::from_secs(6000))
                .is_none());
        }
        assert!(scheduler
            .request(Trigger::Settle, now + Duration::from_secs(6000))
            .is_some());

        let mut settled = IdleTrimScheduler::default();
        enter_settled_epoch(&mut settled, now, Trigger::Minimized);
        for trigger in ALL {
            assert!(settled
                .request(trigger, now + Duration::from_secs(4000))
                .is_none());
        }
    }

    #[test]
    fn activate_or_focus_cancels_pending_deactivation() {
        let now = Instant::now();
        for use_focus in [false, true] {
            let mut scheduler = IdleTrimScheduler::default();
            let generation = scheduler.request(Trigger::Deactivated, now).unwrap();
            if use_focus {
                scheduler.focused(true);
            } else {
                scheduler.activate();
            }
            assert!(scheduler.generation > generation);
            assert!(scheduler.pending.is_none());
            assert_eq!(scheduler.epoch, Epoch::Untrimmed);
            assert_eq!(
                scheduler.fire(
                    generation,
                    now + Trigger::Deactivated.delay(),
                    false,
                    false,
                    false,
                ),
                None
            );
        }
    }

    #[test]
    fn foreground_drops_deactivation_without_consuming_epoch() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        let generation = scheduler.request(Trigger::Deactivated, now).unwrap();
        let deadline = now + Trigger::Deactivated.delay();
        assert_eq!(
            scheduler.fire(generation, deadline, false, false, true),
            None
        );
        assert_eq!(scheduler.epoch, Epoch::Untrimmed);
        assert!(scheduler.pending.is_none());

        let hidden = scheduler.request(Trigger::Hidden, deadline).unwrap();
        assert_eq!(
            scheduler.fire(hidden, deadline + Trigger::Hidden.delay(), false, true, false),
            Some(Trigger::Hidden)
        );
    }

    #[test]
    fn popup_hidden_is_rejected_while_main_is_focused() {
        let mut scheduler = IdleTrimScheduler::default();
        scheduler.focused(true);
        assert!(scheduler
            .request(Trigger::PopupHidden, Instant::now())
            .is_none());
    }

    #[test]
    fn activate_cancels_pending_settle_and_opens_new_epoch() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        enter_low_epoch(&mut scheduler, now, Trigger::Hidden);
        let settle_at = now + Trigger::Hidden.delay();
        let generation = scheduler.request(Trigger::Settle, settle_at).unwrap();
        scheduler.activate();
        assert!(scheduler.generation > generation);
        assert!(scheduler.pending.is_none());
        assert_eq!(scheduler.epoch, Epoch::Untrimmed);
        assert_eq!(
            scheduler.fire(
                generation,
                settle_at + Trigger::Settle.delay(),
                false,
                true,
                false,
            ),
            None
        );
        assert!(scheduler.request(Trigger::Hidden, settle_at).is_some());
    }

    #[test]
    fn focused_true_cancels_pending_settle_and_opens_new_epoch() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        enter_low_epoch(&mut scheduler, now, Trigger::Hidden);
        let settle_at = now + Trigger::Hidden.delay();
        let generation = scheduler.request(Trigger::Settle, settle_at).unwrap();
        scheduler.focused(true);
        assert!(scheduler.generation > generation);
        assert!(scheduler.pending.is_none());
        assert_eq!(scheduler.epoch, Epoch::Untrimmed);
        assert_eq!(
            scheduler.fire(generation, now + Duration::from_secs(400), false, true, false),
            None
        );
        let hidden = scheduler.request(Trigger::Hidden, settle_at).unwrap();
        assert_eq!(
            scheduler.fire(
                hidden,
                settle_at + Trigger::Hidden.delay(),
                false,
                true,
                false,
            ),
            None
        );
        scheduler.focused(false);
        assert!(scheduler.request(Trigger::Hidden, settle_at).is_some());
    }

    #[test]
    fn settle_drops_when_focused_or_foreground_and_postpones_when_busy() {
        let now = Instant::now();
        for focused in [true, false] {
            let mut scheduler = IdleTrimScheduler::default();
            enter_low_epoch(&mut scheduler, now, Trigger::Hidden);
            let settle_at = now + Trigger::Hidden.delay();
            let generation = scheduler.request(Trigger::Settle, settle_at).unwrap();
            if focused {
                scheduler.main_focused = true;
            }
            assert_eq!(
                scheduler.fire(
                    generation,
                    settle_at + Trigger::Settle.delay(),
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
        enter_low_epoch(&mut scheduler, now, Trigger::Hidden);
        let settle_at = now + Trigger::Hidden.delay();
        let generation = scheduler.request(Trigger::Settle, settle_at).unwrap();
        let deadline = settle_at + Trigger::Settle.delay();
        assert_eq!(scheduler.fire(generation, deadline, true, true, false), None);
        assert_eq!(
            scheduler.pending,
            Some((Trigger::Settle, deadline + Trigger::Settle.delay()))
        );
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
            scheduler.fire(
                generation,
                deadline + Trigger::Settle.delay(),
                false,
                true,
                false,
            ),
            Some(Trigger::Settle)
        );
        assert_eq!(scheduler.epoch, Epoch::Settled);
    }

    #[test]
    fn focus_drops_hidden_trim_without_consuming_epoch() {
        let now = Instant::now();
        let mut scheduler = IdleTrimScheduler::default();
        let generation = scheduler.request(Trigger::Hidden, now).unwrap();
        scheduler.focused(true);
        assert_eq!(
            scheduler.fire(generation, now + Trigger::Hidden.delay(), false, true, false),
            None
        );
        assert_eq!(scheduler.epoch, Epoch::Untrimmed);
        assert!(scheduler.pending.is_none());
        scheduler.focused(false);
        let retry = scheduler.request(Trigger::Hidden, now + Duration::from_secs(4)).unwrap();
        assert_eq!(
            scheduler.fire(retry, now + Duration::from_secs(7), false, true, false),
            Some(Trigger::Hidden)
        );
    }

    #[test]
    fn busy_postpone_uses_trigger_delay() {
        for trigger in [Trigger::Hidden, Trigger::Deactivated] {
            let mut scheduler = IdleTrimScheduler::default();
            let now = Instant::now();
            let generation = scheduler.request(trigger, now).unwrap();
            let deadline = now + trigger.delay();
            let low = trigger != Trigger::Deactivated;
            assert_eq!(scheduler.fire(generation, deadline, true, low, false), None);
            assert_eq!(
                scheduler.pending,
                Some((trigger, deadline + trigger.delay()))
            );
        }
    }

    #[test]
    fn busy_popup_postpones_and_never_drops() {
        let mut scheduler = IdleTrimScheduler::default();
        let now = Instant::now();
        let generation = scheduler.request(Trigger::Hidden, now).unwrap();
        for seconds in [3, 6, 9] {
            assert_eq!(
                scheduler.fire(
                    generation,
                    now + Duration::from_secs(seconds),
                    true,
                    true,
                    false,
                ),
                None
            );
        }
        assert_eq!(
            scheduler.fire(generation, now + Duration::from_secs(11), false, true, false),
            None
        );
        assert_eq!(
            scheduler.fire(generation, now + Duration::from_secs(12), false, true, false),
            Some(Trigger::Hidden)
        );
    }

    #[test]
    fn earlier_deadline_replaces_deactivation_and_rejects_old_generation() {
        let mut scheduler = IdleTrimScheduler::default();
        let now = Instant::now();
        let old = scheduler.request(Trigger::Deactivated, now).unwrap();
        let replacement = scheduler.request(Trigger::Hidden, now).unwrap();
        assert!(replacement > old);
        assert_eq!(
            scheduler.fire(old, now + Trigger::Hidden.delay(), false, true, false),
            None
        );
        assert_eq!(
            scheduler.fire(
                replacement,
                now + Trigger::Hidden.delay(),
                false,
                true,
                false,
            ),
            Some(Trigger::Hidden)
        );
        assert_eq!(
            scheduler.fire(
                old,
                now + Trigger::Deactivated.delay(),
                false,
                false,
                false,
            ),
            None
        );
    }

    #[test]
    fn longer_request_never_pushes_deadline_out() {
        let mut scheduler = IdleTrimScheduler::default();
        let now = Instant::now();
        let generation = scheduler.request(Trigger::Hidden, now).unwrap();
        assert!(scheduler
            .request(Trigger::Deactivated, now + Duration::from_secs(1))
            .is_none());
        assert_eq!(
            scheduler.pending,
            Some((Trigger::Hidden, now + Trigger::Hidden.delay()))
        );
        assert_eq!(
            scheduler.fire(
                generation,
                now + Trigger::Hidden.delay(),
                false,
                true,
                false,
            ),
            Some(Trigger::Hidden)
        );
    }

    #[test]
    fn abandon_clears_only_matching_generation_and_allows_retry() {
        let mut scheduler = IdleTrimScheduler::default();
        let now = Instant::now();
        let generation = scheduler.request(Trigger::Hidden, now).unwrap();
        let pending = scheduler.pending;
        scheduler.abandon(generation + 1);
        assert_eq!(scheduler.pending, pending);
        scheduler.abandon(generation);
        assert!(scheduler.pending.is_none());
        assert_eq!(scheduler.generation, generation);

        let retry_at = now + Duration::from_secs(1);
        let retry = scheduler.request(Trigger::Hidden, retry_at).unwrap();
        assert!(retry > generation);
        assert_eq!(
            scheduler.pending,
            Some((Trigger::Hidden, retry_at + Trigger::Hidden.delay()))
        );
        let retry_pending = scheduler.pending;
        scheduler.abandon(generation);
        assert_eq!(scheduler.pending, retry_pending);
    }

    #[test]
    fn popup_use_reopens_low_epoch_while_main_is_hidden() {
        let mut scheduler = IdleTrimScheduler::default();
        let now = Instant::now();
        for round in 1..=3u64 {
            let shown = now + Duration::from_secs(20 * round);
            scheduler.activate();
            assert_eq!(scheduler.epoch, Epoch::Untrimmed);
            let hidden_at = shown + Duration::from_secs(5);
            let generation = scheduler.request(Trigger::PopupHidden, hidden_at).unwrap();
            assert_eq!(
                scheduler.fire(
                    generation,
                    hidden_at + Trigger::PopupHidden.delay(),
                    false,
                    true,
                    false,
                ),
                Some(Trigger::PopupHidden)
            );
            assert!(scheduler
                .request(Trigger::PopupHidden, hidden_at + Duration::from_secs(4))
                .is_none());
        }
    }

    #[test]
    fn popup_reshown_within_delay_cancels_trim() {
        let mut scheduler = IdleTrimScheduler::default();
        let now = Instant::now();
        let generation = scheduler.request(Trigger::PopupHidden, now).unwrap();
        scheduler.activate();
        assert_eq!(
            scheduler.fire(
                generation,
                now + Trigger::PopupHidden.delay(),
                false,
                true,
                false,
            ),
            None
        );
        assert!(scheduler.pending.is_none());
    }

    #[tokio::test]
    async fn absent_state_request_does_not_panic_or_mutate_another_scheduler() {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let untouched = MemoryTrim::default();
        request(app.handle(), Trigger::Hidden);
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
        assert_eq!(
            scheduler.fire(old, Instant::now() + Duration::from_secs(30), false, true, false),
            None
        );
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
            enter_low_epoch(&mut scheduler, now, Trigger::Hidden);
        }
        popup_shown(app.handle());
        assert_eq!(scheduler.lock().unwrap().epoch, Epoch::Untrimmed);

        for cancel in [popup_shown::<tauri::test::MockRuntime>, activate] {
            request(app.handle(), Trigger::Deactivated);
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
