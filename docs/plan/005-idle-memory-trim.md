# 005 — Idle memory trim (Windows)

**Date:** 2026-11-03
**Branch:** `feature/idle-memory-trim`
**Guideline:** `~/.claude/docs/guidelines/009-tauri-webview2-idle-memory-trimming.md` (WebView2-specific) and
`008-desktop-app-resident-memory-trimming.md` (general rules). Adapted from Meeting Vault PR #420.

## Problem

Task Manager shows the translator at ~490–700 MB. Like every Tauri app on Windows, ~95 % of that is the
WebView2 process set, not the Rust host.

## Measurement before code (installed release 0.1.4, idle, main hidden in tray)

`scripts/measure-app-memory.ps1 -RootPid <host pid>`:

| Role | Working set MB | Private MB |
|---|---:|---:|
| host (TranslatorDesktop.exe) | 35.8 | 8.9 |
| browser | 129.6 | 38.6 |
| gpu-process | 200.6 | 241.3 |
| renderer × 3 (main, popup, confirmation) | 91.4 / 83.3 / 57.4 | 44.7 / 34.8 / 25.4 |
| network / storage / utility / crashpad | 38.3 / 20.4 / 25.3 / 13.1 | 11.1 / 7.6 / 8.1 / 2.9 |
| **Total (10 processes)** | **695.2** | **423.4** |

External proof (`psapi!EmptyWorkingSet` on every PID of the tree): working set **695 → 1.7 MB**
immediately and **22.6 MB after 60 s idle**; private unchanged (423 MB). The trim works for this app.

The `text-helper` .NET sidecar is a separate process (not a child of WebView2); it is handled by the host's same-epoch trim (see Sidecar below).

## Design

`src-tauri/src/memory_trim.rs`, best effort: the scheduler and its timers run on every OS, but the
native trim and memory target exist only on Windows (no-ops elsewhere).

1. **Trim the whole tree.** Host via `GetCurrentProcess`. WebView2 PIDs come from the main webview
   (`BrowserProcessId` + `ICoreWebView2Environment8::GetProcessInfos`), plus the browser's direct
   `msedgewebview2.exe` children (crashpad). Each one gets `SetProcessWorkingSetSizeEx(-1, -1)`. One
   environment serves all three windows, so every renderer is covered.
2. **Memory target.** `ICoreWebView2_19::SetMemoryUsageTargetLevel(LOW)` on the **main** webview only,
   when a trim runs with main hidden or minimized. It goes back to NORMAL on show, restore or focus. The
   popup stays at NORMAL because its paint latency matters most.
3. **Scheduler**: one trim per quiet epoch (`Untrimmed → Visible → Low`), never periodic.

| Trigger | Delay | Hook |
|---|---|---|
| Startup (main `PageLoadEvent::Finished`) | 8 s | `on_page_load` |
| Main hidden to tray | 3 s | `window_visibility::hide_window("main")` |
| Main minimized | 3 s | `WindowEvent::Resized` + `is_minimized()` |
| Main deactivated while visible | 15 s | `WindowEvent::Focused(false)` |
| Popup / confirmation hidden while main unfocused | 3 s | `window_visibility::hide_window(popup)` |

- **Cancel and new epoch:** main focused, shown or restored; popup or confirmation shown.
- **Admission foreground check:** at fire time, compare the main HWND with the Win32 foreground
  HWND in addition to tracked `Focused` events; HWND lookup failure is treated as not foreground.
- **Main in use drops the request:** main focused (tracked from `Focused` events, never
  `is_focused()`) or found in the foreground HWND check. Leaving main raises its own trigger
  (`Focused(false)`, hide, minimize), so nothing re-polls while the user works. The epoch is not consumed.
- **Busy check (postpones by the trigger's delay, never drops):** the popup or confirmation is visible.
- **Translator-specific rule:** showing the popup opens a new epoch. Users drive this app through the
  popup while main sits in the tray. Without the rule, the Low epoch from the startup trim would never
  reopen, and popup use would never be trimmed again. Each popup round trip therefore gets one trim,
  3 s after it closes, and is cancelled if the popup reopens within 3 s (008 §4).

Pitfalls from 009 §4 that this design addresses:
- focus comes from the tracked `Focused` events, not `is_focused()`;
- detached tasks own `Arc`s, never hold a `State` borrow;
- the scheduler cancels its task on drop;
- tests pin exact and late deadlines, generation replacement and abandonment, tracked and foreground
  focus admission, trigger-specific busy postponement, low-epoch trigger rejection, popup epoch
  reopening, managed and absent public API state, task abortion on replacement/drop/unmanage, and the
  browser-child image-name boundary cases on Windows.

## Verification

1. `cargo test memory_trim` covers:
   - exact delays, no early fire, and a late fire after the deadline;
   - shorter-delay replacement, stale-generation rejection, and preservation of an existing deadline;
   - `activate()` and `focused(true)` invalidate pending work; `abandon()` clears only its generation;
   - tracked and foreground focus admission drop the request without consuming the epoch;
   - busy postponement by the trigger's own delay;
   - Visible → Low escalation and rejection of all trigger kinds in Low;
   - popup epoch reopening and cancellation through the public API;
   - managed/absent public API state and task abortion on replacement, drop, and unmanage;
   - on Windows, browser-child image-name handling, snapshot discovery, and current-process trim.
2. `cargo build`; full `cargo test`.
3. Live check (009 §7): release build (`tauri build --no-bundle`), driven over CDP and `ShowWindow`
   no-activate calls. The app has no hidden-launch path (`--minimized` from autostart is never parsed), so
   the launch itself shows and focuses main once. Results (round-1 code; the later fixes only change
   admission and bail-out paths):

| State | Working set MB | Private MB | Log |
|---|---:|---:|---|
| Start-up, ~5 s | 520.0 | 310.4 | — |
| After first trim (main visible, unfocused) | **14.3** | 307.6 | `trigger=deactivated low=false processes=9 trimmed=9 elapsed_ms=45` |
| Visible after a translation and tab switches | 217.9 | 302.1 | — |
| 6 s after minimize | **32.1** | 297.0 | `trigger=minimized low=true processes=9 trimmed=9 elapsed_ms=36` |
| Restore → minimize again | — | — | no trim while restored; `trigger=minimized low=true elapsed_ms=21` |

   **Regrowth while hidden in the tray** (sidecar branch, 2026-11-03): after the `hidden` trim the tree
   regrows to a plateau of ~120 MB working set within ~3 minutes (136 MB at 50 s, 170 MB at 2 min), with
   private unchanged at ~280 MB. A second, external `EmptyWorkingSet` after LOW had settled regrew the same
   way (34 → 117 MB in 3 min), so the regrowth does not come from the trim order. The tree is idle (16 ms CPU in 60 s, all of it
   in the browser process), there is no JS polling in `src/`, and the host log is quiet, so this is
   Chromium/V8 and .NET re-touching their live heaps, not a loop to fix. Each renderer settles near its
   private size (~20–25 MB). A further trim would only reset the plateau, which 009 forbids as periodic.

   The popup round trip was not driven live: showing the popup activates it and would take the user's
   focus. The scheduler tests cover it; it still needs one manual check.

## Sidecar (text-helper)

The sidecar measured 49 MB working set and 14.8 MB private. `EmptyWorkingSet` reduced working set
49 → 0 MB, then it regrew to a ~13 MB plateau within 2 minutes; private stayed unchanged. The host
trims the sidecar in the same epoch trim, using the PID tracked at spawn and cleared on stop/terminate.
.NET GC tuning was not pursued: private memory is only ~15 MB, while measured GC configuration impact
is about 2% (guideline 008 §2).

## Out of scope

GPU-process browser switches (009 §6). Meeting Vault measured them and rejected both:
`--in-process-gpu` saves nothing, and `--disable-gpu*` adds about 15 MB private and about 6× idle CPU.
`MemoryUsageTargetLevel LOW` is the only lever that moves GPU private memory.
