# 006 — Tray memory regrowth after the idle trim

**Date:** 2026-11-03 · **Branch:** `feature/sidecar-memory-trim` · **Follows:** [005](005-idle-memory-trim.md)

## Problem

After main is hidden to the tray, the idle trim takes the app down to ~38 MB in Task Manager. Over the
next ~6 minutes it climbs back to ~70 MB and then stays there. The user reported 178 MB for a copy that
had been in use for a while.

## Findings (release build, hidden in the tray, 10-minute sample)

Task Manager's **Memory** column is the *private working set*. My earlier 120–170 MB figures summed the total
working set, which includes shared DLL pages, so they overstated what the user sees.

| Process | Private working set over 10 min | Behaviour |
|---|---|---|
| text-helper (.NET sidecar) | 7 → **24 MB** | climbs steadily while the mouse moves |
| WebView2 browser + 3 renderers | 3 → 13 / 12 / 9 MB in the first minute | then flat |
| host, gpu, network, storage | < 8 MB each | flat |
| **Total** | **37.7 → 70 MB** | |

- The tree is idle: 16 ms of CPU in 60 s, a quiet host log, and no JS polling in `src/`.
- `dotnet-counters` on the sidecar showed a **gen0 budget of 18 MB** and a GC heap of 19 MB, with almost no
  gen0 GCs while idle.
- `GlobalHookService` uses `TaskPoolGlobalHook`. Every system-wide mouse move allocates a
  `MouseEventData` and a dispatched task (`GlobalHookService.cs:282`), even though the only consumer
  (`TextSelectionMonitor.cs:202`) returns straight away unless the left button is down. The allocations
  land in fresh gen0 pages until the 18 MB budget is used, so the working set climbs up to the budget. A
  working-set trim can't hold it down because the next allocations touch those pages again.
- **Cause:** sidecar gen0 churn from mouse moves. It is the only steady climber; the WebView2 processes
  settle at a small, flat plateau.

## Plan

1. **Stop the per-move allocations at the source** in `GlobalHookService`.
   - Forward `MouseMoved` only while a mouse button is held (tracked from pressed/released), so idle mouse
     movement allocates nothing in our code. The consumer already ignores moves otherwise.
   - → verify: dotnet-counters allocation rate during plain mouse movement drops to ~0.
   - → verify: Ctrl/Alt+drag selection still works.
2. **Cap the sidecar gen0 budget.** Set `DOTNET_GCgen0size` (for example 2 MB) and `DOTNET_gcConcurrent=0`
   on the sidecar spawn in `sidecar.rs`. That bounds whatever SharpHook's task pool still allocates.
   - → verify: counters show the new budget.
   - → verify: the sidecar's private working set stays ≤ ~8 MB over 10 minutes of mouse use.
   - → verify: no extra CPU.
3. **WebView2 plateau (~34 MB across browser + renderers):** measure only, no code change planned. If the
   user wants it lower, the next lever is creating the confirmation window only when needed (one renderer,
   ~9 MB private working set). That is a separate decision.
4. Rebuild the sidecar (`npm run build:sidecar`) and the app (`npx tauri build --no-bundle`), then repeat the
   10-minute tray sample.
   - Success means a Task Manager total ≤ ~50 MB after 10 minutes, with no climb after minute 2.
   - Record the result in 005's results table.

Size: **S** (two small edits plus a live measurement). Per the agent rules this goes to `pi` luna:xhigh, or
I do it inline if it stays this small.

## Results (steps 1–2, release build, hidden in the tray, 10-minute sample)

| Process | Before | After |
|---|---|---|
| text-helper gen0 budget / GC heap | 18 MB / 19 MB | **2 MB / 1.6 MB** |
| text-helper private working set at 10 min | 24 MB, climbing | **4.5 MB**, flat |
| text-helper private bytes | 29 MB | **10.8 MB** |
| WebView2 browser + 3 renderers, private working set | ~34–45 MB | 16 + 18 / 10.5 / 9.7 MB ≈ 54 MB, flat after minute 1 |
| Task Manager total at 10 min | 70 MB | 75 MB |

- The sidecar climb is fixed.
- The WebView2 share is now the whole plateau. It regrows within the first minute after the trim and then
  stays flat. The two runs differ by ~20 MB of WebView2 depending on timing: this run was hidden 12 s after
  launch, before the page had settled.
- `tauri build` did not refresh `target\release\text-helper.exe` (still the Oct 4 build); the new sidecar
  was copied in by hand. The build scripts need a fix (`build-sidecar.ps1` copies only to `target\debug`).

## WebView2 regrowth: settle trim (step 5)

The user's Task Manager figure tracks the **working set**: 162–170 MB, against 72 MB private working set,
because shared DLL pages count once per process. Measured on the same hidden instance after external
`EmptyWorkingSet` passes:

| Trim timing after hide (LOW set at +3 s) | Afterwards (private working set) |
|---|---|
| +3 s (today's hidden trim) | climbs to ~70 MB within 1–3 min, renderers 9–19 MB |
| +30 s | 43 MB, then a second burst ~3 min after the hide → 69 MB |
| +5 min | **36–40 MB flat for 5 min**, renderers ~3 MB; working set **98 MB** (from 170) |
| +14 min | 28 MB flat for 4 min |

Chromium runs its hidden-page cleanup in bursts over the first few minutes. A trim after that holds.

- **Fix:** one `Settle` trim 300 s after each LOW trim, if main is still hidden or minimized and unfocused.
  New epoch state `Settled`; no further trims until the user returns (show, focus, popup).
- The trim is one-shot per epoch, never periodic.
- **Live result** (release build, 57/57 tests): the log shows `trigger=hidden` at +3 s and `trigger=settle` once at
  +300 s, with nothing after. Working set (Task Manager): 163–177 MB before the settle trim, **9 MB** right after it, and
  **~89 MB** 6 minutes later, flat over the last 1.5 minutes. Private working set: 72 → 38 MB.
- About 58 MB of the remaining working set is shared DLL code that live Chromium processes keep loaded.
  No trim can hold it out.

## Risks

- Hook semantics: SharpHook may report moves with a button held as `MouseDragged`, not `MouseMoved`.
  Step 1 must keep whatever the consumer actually receives today.
- `DOTNET_GCgen0size` is an unsupported runtime knob. If counters show it is ignored, drop it and rely on
  step 1.
