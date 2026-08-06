# Idle GPU Usage in Tauri + WebView2 Apps (Windows)

A portable playbook for any **Tauri 2 + WebView2 (Windows)** desktop app whose GPU usage never drops to zero — including while the app is minimised to the system tray with nothing happening.

Written up from a real investigation in Translator Desktop. Verified against `tauri 2.9.5`, `tauri-runtime-wry 2.9.3`, `wry 0.53.5`, `tao 0.34.5`.

---

## Symptom

- `msedgewebview2.exe` (the **GPU process**) sits at a constant few percent GPU — typically jittering between ~1.5% and ~7% in Task Manager.
- It happens with the app **minimised to the tray**, no translation/request/animation visible to the user.
- Usage never settles to zero, for the entire lifetime of the process.

## Is this your bug? 60-second check

Run the [measurement script](#step-2-measure-per-process-cpu-and-gpu) below while your app is hidden in the tray.

| Observation | Meaning |
|---|---|
| One or more **renderer** processes > 0.5% of a core while hidden | Yes — this playbook applies |
| **gpu-process** burning CPU/GPU while all windows are hidden | Yes — this playbook applies |
| All renderers at exactly `0.00%` | Something else; look at Rust threads, timers, or the sidecar |

A truly idle hidden WebView2 renderer costs **exactly 0 ms** of CPU over a 10-second window. Anything above that is real work.

---

## Root cause

Two ingredients have to combine. Neither alone is enough.

### Ingredient 1 — an infinite, compositor-driven CSS animation

Tailwind's `animate-spin`, `animate-pulse`, `animate-ping`, `animate-bounce`, and any custom `@keyframes ... infinite` animate `transform`/`opacity`. Those are **compositor** animations: Chromium requests a new frame every vsync, forever, whether or not anything else changes.

The dangerous case is not an animation the user is looking at. It is an animation that is the **resting state of a window that is usually hidden**:

```tsx
// ANTI-PATTERN: this window's idle state is an infinite animation.
// `isReady` starts false and is reset to false whenever the window loses focus,
// so this spinner runs 24/7 in a window nobody can see.
if (!isReady) {
  return <div className="animate-spin w-8 h-8 border-4 ..." />;
}
```

A status indicator with `animate-pulse` that is active in the app's *normal connected state* is the same trap in the main window.

### Ingredient 2 — hiding a Tauri window does NOT stop WebView2 rendering

This is the part that makes it permanent, and it is not obvious.

```
window.hide()                                   // tauri WebviewWindow
  └─> WindowMessage::Hide                       // tauri-runtime-wry lib.rs:3328
        └─> tao Window::set_visible(false)      // tao windows/window.rs:164
              └─> ShowWindow(hwnd, SW_HIDE)     // parent HWND only
```

Nothing in that chain touches the WebView2 controller. wry *has* the right call, but it lives on a **separate** message that `window.hide()` never sends:

```
webview.set_visible(false)                      // wry webview2/mod.rs:1460
  └─> ShowWindow(...)
  └─> controller.SetIsVisible(false)            // wry webview2/mod.rs:1470  <-- the one that matters
```

reachable only via `WebviewMessage::Hide` (`tauri-runtime-wry lib.rs:3625`).

And wry's subclass on the parent window handles **only `WM_SIZE`** (`wry webview2/mod.rs:1198-1266`) — there is no `WM_SHOWWINDOW` / `WM_WINDOWPOSCHANGED` handler, so hiding the parent is never propagated.

**Net effect:** `ICoreWebView2Controller::IsVisible` stays `true`. The page's `document.visibilityState` stays `"visible"`, so Chromium never applies background throttling, and the compositor keeps producing frames for a window that is not on screen.

> Because `document.visibilityState` never changes, a frontend-only fix such as
> `if (document.hidden) pauseAnimations()` **will not work**. The page does not
> believe it is hidden.

### The amplifier — one shared GPU process

Every WebView2 window in your app (main, popup, tray dialogs…) gets its own **renderer** process but they all share **one gpu-process**. Windows declared in `tauri.conf.json` with `"visible": false` are created at startup and live for the whole session. So a spinner in a never-shown dialog window shows up as GPU usage attributed to your app forever.

---

## Diagnosis

### Step 1 — map the process tree

WebView2 children are not obviously yours; a dev machine can have 70+ `msedgewebview2.exe` processes. Walk down from your app's PID and read `--type=` out of each command line.

```powershell
$root = <YOUR_APP_PID>
$all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name, CommandLine
$tree = @($root); $added = $true
while ($added) {
  $added = $false
  foreach ($p in $all) {
    if ($tree -contains $p.ParentProcessId -and $tree -notcontains $p.ProcessId) { $tree += $p.ProcessId; $added = $true }
  }
}
$all | Where-Object { $tree -contains $_.ProcessId } | ForEach-Object {
  $t = 'n/a'; if ($_.CommandLine -match '--type=([a-zA-Z\-]+)') { $t = $Matches[1] }
  $c = '';    if ($_.CommandLine -match '--renderer-client-id=(\d+)') { $c = "client-id=$($Matches[1])" }
  "{0,-24} pid {1,-7} {2} {3}" -f $_.Name, $_.ProcessId, $t, $c
}
```

**Renderer → window mapping:** `--renderer-client-id` increments in window creation order, which matches the order of the `windows` array in `tauri.conf.json`. So the lowest id is your first-declared window (usually `main`).

### Step 2 — measure per-process CPU and GPU

CPU-time deltas are the reliable signal. `\GPU Engine(*)\Running Time` confirms GPU involvement but its raw units are awkward — use it as zero/non-zero.

```powershell
$pids = @{ <PID>='gpu-process'; <PID>='renderer-main'; <PID>='renderer-popup' }
$t0 = @{}; foreach ($k in $pids.Keys) { $t0[$k] = (Get-Process -Id $k).TotalProcessorTime.TotalMilliseconds }
$g0 = (Get-Counter '\GPU Engine(*)\Running Time').CounterSamples
Start-Sleep -Seconds 10
$g1 = (Get-Counter '\GPU Engine(*)\Running Time').CounterSamples
foreach ($k in $pids.Keys) {
  $d = (Get-Process -Id $k).TotalProcessorTime.TotalMilliseconds - $t0[$k]
  "{0,-18} pid {1,-6} {2,7:N0} ms ({3,5:N2}% of a core)" -f $pids[$k], $k, $d, ($d/10000*100)
}
foreach ($k in $pids.Keys) {
  $tot = 0.0
  foreach ($s in ($g1 | Where-Object { $_.InstanceName -like "pid_$($k)_*" })) {
    $m = $g0 | Where-Object { $_.InstanceName -eq $s.InstanceName }
    if ($m) { $tot += ($s.CookedValue - $m.CookedValue) }
  }
  "{0,-18} pid {1,-6} GPU delta {2,10:N1}" -f $pids[$k], $k, $tot
}
```

### Step 3 — prove the windows really are hidden

Distinguish *hidden* (`window.hide()` → tray) from *minimised*; they are different states and only the former is covered here.

```powershell
Add-Type @"
using System;using System.Text;using System.Runtime.InteropServices;
public class W {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr p);
  public delegate bool EnumProc(IntPtr h, IntPtr p);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
}
"@
$script:res = @()
$cb = [W+EnumProc]{ param($h,$p)
  $wp = 0; [void][W]::GetWindowThreadProcessId($h, [ref]$wp)
  if ($wp -eq <YOUR_APP_PID>) {
    $sb = New-Object System.Text.StringBuilder 256; [void][W]::GetWindowTextW($h, $sb, 256)
    if ($sb.ToString() -ne '') { $script:res += [PSCustomObject]@{ Title=$sb.ToString(); Visible=[W]::IsWindowVisible($h); Minimized=[W]::IsIconic($h) } }
  }; return $true }
[void][W]::EnumWindows($cb, [IntPtr]::Zero)
$res | Format-Table -AutoSize
```

Seeing `Visible=False` on every window while a renderer burns CPU **is** the proof.

### Step 4 — find the animations

```bash
rg "animate-spin|animate-pulse|animate-ping|animate-bounce" src/
rg "@keyframes|animation:.*infinite" src/ --glob '*.css'
```

For each hit ask: *can this be on-screen while the window is hidden, or while the app is simply idle?* Pay special attention to loading/idle branches of windows that are pre-created hidden.

---

## The fix

Two layers. Layer 1 removes the current cost; Layer 2 makes it structurally impossible to reintroduce.

### Layer 1 — never leave an infinite animation as a resting state

```tsx
// Idle/loading state - deliberately static.
// This is the window's resting state; anything animated here runs for the
// entire lifetime of the app in a window nobody can see.
if (!isReady) {
  return <div className="w-full h-full bg-white/95 dark:bg-gray-900/95" />;
}
```

Rules of thumb:

- A spinner is fine **while a user is watching a real operation**. It is not fine as a default/idle render.
- Prefer a static dot/badge over `animate-pulse` for "connected"/"ready" indicators — those states are permanent by definition.
- Leave a comment saying *why* it is static, or someone will "restore" the spinner later.

### Layer 2 — pair window visibility with webview visibility

`Webview::hide()` / `Webview::show()` exist in Tauri 2 (`tauri/src/webview/mod.rs:1523`, `#[cfg(desktop)]`, **not** behind the `unstable` feature) and reach `SetIsVisible`. Get at them through `WebviewWindow: AsRef<Webview<R>>` (`tauri/src/webview/webview_window.rs:1387`).

Create one small module and route **all** visibility changes through it:

```rust
//! Window + webview visibility helpers.
//!
//! On Windows, `WebviewWindow::hide()` only hides the parent HWND — it never
//! reaches the WebView2 controller, so `IsVisible` stays `true` and the webview
//! keeps compositing forever. Pairing the two makes it actually stop.
//!
//! Always route visibility changes through these helpers: a plain `window.show()`
//! on a window whose webview is hidden renders blank.

use tauri::webview::Webview;
use tauri::{Runtime, WebviewWindow};

/// Show a window and resume its webview rendering.
/// Webview first, so the window never paints a blank frame.
pub fn show_window<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let webview: &Webview<R> = window.as_ref();
    let _ = webview.show();
    window.show()
}

/// Hide a window and stop its webview rendering.
/// Window first, so the user never sees a blank frame.
pub fn hide_window<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    window.hide()?;
    let webview: &Webview<R> = window.as_ref();
    let _ = webview.hide();
    Ok(())
}

/// Stop rendering for a window that is *already* hidden — e.g. windows declared
/// `"visible": false`, whose webviews are still created visible.
pub fn suspend_hidden_webview<R: Runtime>(window: &WebviewWindow<R>) {
    let webview: &Webview<R> = window.as_ref();
    let _ = webview.hide();
}
```

**Ordering matters.** Show: webview → window. Hide: window → webview. Reversing either produces a visible blank frame.

Then find and convert **every** call site:

```bash
rg "\.show\(\)|\.hide\(\)" src-tauri/src/      # Rust
rg "\.hide\(\)|\.show\(\)|getCurrentWindow"  src/   # frontend
```

Typical sites: tray menu "Show", tray icon click, close-to-tray handler, popup show/hide commands, dialog show/hide.

Suspend the pre-created hidden windows once at startup:

```rust
// Windows declared `"visible": false` still get *visible* webviews, which would
// render for the whole session. Suspend until something actually shows them.
for label in ["popup", "confirmation"] {
    if let Some(window) = app.get_webview_window(label) {
        window_visibility::suspend_hidden_webview(&window);
    }
}
```

### Layer 2 pitfall — frontend `show()` calls will blank your window

This is the one thing that will bite you. A frontend call like:

```ts
const main = await WebviewWindow.getByLabel('main');
await main.show();      // shows the WINDOW only -> blank if the webview is suspended
```

The JS `Window` API has no webview-visibility equivalent. Add a Rust command and call that instead:

```rust
#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), WindowError> {
    if let Some(window) = app.get_webview_window("main") {
        crate::window_visibility::show_window(&window)?;
        window.set_focus()?;
    }
    Ok(())
}
```

```ts
await invoke('show_main_window');
```

Frontend `hide()` calls are lower risk (worst case the webview keeps rendering, as before), but prefer routing them through Rust too.

**Regression checklist after Layer 2** — every one of these must render, not blank:

- [ ] App start → main window
- [ ] Close to tray → restore via tray icon
- [ ] Close to tray → restore via tray menu "Show"
- [ ] Every secondary window (popup/dialog) on first show *and* on a later re-show
- [ ] Any frontend-initiated window show

---

## Results

Measured with the same script and a 10 s window, release build, all windows hidden in the tray:

| Process | Before | After |
|---|---|---|
| gpu-process | **33.6% of a core** | **0.00%** |
| renderer — main | 5.00% | 0.00% |
| renderer — dialog | 1.09% | 0.00% |
| renderer — popup | 0.00% | 0.00% |
| GPU engine time | continuously accumulating | 0.0 |

---

## Gotchas that cost real time

### `cargo build --release` is not a Tauri build

It skips `beforeBuildCommand` and the frontend-asset codegen, so the binary keeps pointing at `build.devUrl`. With no dev server running, **every window silently renders an `ERR_CONNECTION_REFUSED` page** — which has no animations, so idle GPU measures as a beautiful, completely meaningless `0.00%`.

Always build with the real pipeline. To skip installer bundling while iterating:

```bash
npm run tauri build -- --no-bundle     # or: bun run tauri build --no-bundle
```

Sanity-check before trusting any measurement: screenshot the window, and confirm the expected renderer count (one per window).

### Screenshotting an occluded/background window

`SetForegroundWindow` is refused by Windows' foreground lock from a background script, so `CopyFromScreen` silently captures whatever is on top — often your terminal. Use `PrintWindow` with `PW_RENDERFULLCONTENT` (`2`), which captures the window's own content regardless of z-order:

```powershell
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc(); [void][P]::PrintWindow($hwnd, $hdc, 2); $g.ReleaseHdc($hdc)
```

Cheap blank-detection without eyeballing every capture — sample pixels and count distinct colours; `1` means blank:

```powershell
$d=@{}; for($y=0;$y -lt $bmp.Height;$y+=15){ for($x=0;$x -lt $bmp.Width;$x+=15){ $d[$bmp.GetPixel($x,$y).ToArgb()]=1 } }
"distinct colors: $($d.Count)"
```

### Redirected stdio changes startup timing

Launching via `Start-Process -RedirectStandardOutput/-RedirectStandardError` perturbed startup enough to turn a latent race into a deterministic crash that did not occur on a normal launch. If a bug only reproduces under your instrumentation, verify it without redirection before chasing it.

---

## Related: `state() called before manage()`

Found while fixing the above, and very likely to hit any Tauri 2 app with pre-created windows.

```
state() called before manage() for <YourState>
```

Windows declared in `tauri.conf.json` **start loading before the `setup` hook runs**, so their frontends can invoke commands before `setup` registers state with `app.manage(...)`. Anything that delays `setup` — an updater plugin, autostart, DB init — widens the window until the crash becomes deterministic. The frontend calling a command on mount (`invoke('get_popup_text')` in a `useEffect`) is enough to lose the race.

**Fix:** register state on the builder, not in `setup`. Builder-level state exists before any window.

```rust
tauri::Builder::default()
    .plugin(/* ... */)
    // Register on the builder — config windows load before `setup` runs.
    .manage(PopupTextState::default())
    .manage(HotkeyState::default())
    .manage(ConfirmationState::default())
    .setup(|app| {
        // only genuinely app-handle-dependent or async init here
        Ok(())
    })
```

State that genuinely needs async construction (a DB pool) cannot move — for those, have commands use `try_state::<T>()` and degrade gracefully rather than panicking.

---

## Platform notes

Everything above is **Windows/WebView2**. The helper module is safe to call cross-platform (`Webview::hide/show` is `#[cfg(desktop)]`; on macOS wry maps it to `setHidden`), but the *problem* was only measured on Windows. Re-measure before assuming macOS/Linux behave the same — WKWebView and WebKitGTK have their own occlusion handling.

---

## Checklist for a new project on this stack

- [ ] No infinite CSS animation is reachable as an idle/resting state
- [ ] Secondary windows declared `"visible": false` are webview-suspended at startup
- [ ] All Rust show/hide routed through `window_visibility`
- [ ] No frontend `WebviewWindow.show()` on a window that can be webview-suspended
- [ ] Shared state registered via `Builder::manage()`, not inside `setup`
- [ ] Idle-in-tray measurement shows `0.00%` on every renderer and the gpu-process
