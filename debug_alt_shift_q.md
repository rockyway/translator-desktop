# Debug Report: Alt+Shift+Q Hotkey Investigation

## Problem
- **Ctrl+Alt+Q**: ✅ Works correctly
- **Alt+Shift+Q**: ❌ Doesn't capture text
- TextMonitor (.NET IPC) captures 1224 chars successfully via UI Automation
- Issue is in Rust hotkey handler

## Hypothesis
The `wait_for_modifiers_release()` function may have issues with Alt+Shift combination:
1. Keys might take longer than 500ms to release
2. `GetAsyncKeyState` might not detect Alt key (VK_MENU) properly
3. Target window might lose focus after waiting

## Changes Made

### 1. Enhanced Logging in `wait_for_modifiers_release()`
**File**: `D:\sources\utils\translator-desktop\src-tauri\src\commands\hotkey.rs`

Added detailed logging to track modifier key states:
```rust
// Now logs every 100ms or on first check
log::info!(
    "DEBUG: Modifier state at {}ms: Ctrl={} Shift={} Alt={} LWin={} RWin={} (any_held={})",
    waited, ctrl_held, shift_held, alt_held, lwin_held, rwin_held, any_held
);
```

### 2. Enhanced Logging in `simulate_copy_with_retry()`
Added step-by-step logging:
- Step 1: Waiting for modifiers
- Step 2: Sending Ctrl+C
- Step 3: Polling clipboard
- Each poll shows: clipboard length, is_new status, elapsed time

### 3. Increased Timeout
Changed `MAX_MODIFIER_WAIT_MS` from **500ms** to **1000ms** to give Alt+Shift more time to release.

### 4. Entry Point Logging
Added clear marker when hotkey flow starts:
```rust
log::info!("DEBUG: ===== HOTKEY TRANSLATION FLOW STARTED =====");
```

## Testing Instructions

### Setup
1. Build is running in background (task ID: be78adf)
2. Wait for build to complete and app to launch
3. Open any application (Chrome, Notepad, etc.)
4. Select some text (at least 10-20 words)

### Test Cases

#### Test 1: Ctrl+Alt+Q (Working Baseline)
1. Select text
2. Press **Ctrl+Alt+Q**
3. Check logs for modifier states
4. Verify popup appears

#### Test 2: Alt+Shift+Q (Broken Case)
1. Select text
2. Press **Alt+Shift+Q**
3. **Watch logs carefully** for:
   - How long modifiers take to release (0ms, 100ms, 200ms, etc.)
   - Which keys are still held (Shift? Alt? Both?)
   - If timeout occurs (1000ms)
   - If Ctrl+C is sent
   - Clipboard poll results (empty? old content? new content?)

### What to Look For

#### Scenario A: Modifiers Never Release
```
DEBUG: Modifier state at 0ms: Ctrl=false Shift=true Alt=true ...
DEBUG: Modifier state at 100ms: Ctrl=false Shift=true Alt=true ...
...
DEBUG: Modifier state at 900ms: Ctrl=false Shift=true Alt=true ...
DEBUG: TIMEOUT after 1000ms
```
**Diagnosis**: Keys are stuck in "pressed" state. May need different API or longer timeout.

#### Scenario B: Modifiers Release, But Clipboard Empty
```
DEBUG: All modifier keys released after 120ms
DEBUG: Step 2 - Sending Ctrl+C once...
DEBUG: Step 3 - Polling clipboard...
DEBUG: Poll 1: clipboard is empty or whitespace
DEBUG: Poll 2: clipboard is empty or whitespace
...
DEBUG: TIMEOUT after 30 polls
```
**Diagnosis**: Ctrl+C is sent, but target window doesn't respond. Focus lost or wrong window?

#### Scenario C: Modifiers Release, Clipboard Has Old Content
```
DEBUG: All modifier keys released after 120ms
DEBUG: Step 2 - Sending Ctrl+C once...
DEBUG: Poll 1: clipboard_len=1224, is_new=false
DEBUG: Poll 2: clipboard_len=1224, is_new=false
...
DEBUG: TIMEOUT
```
**Diagnosis**: Ctrl+C sent, clipboard unchanged. Target app didn't process the copy command.

## Next Steps

Based on logs, we'll implement one of these fixes:

### Fix Option 1: Increase Timeout Further
If modifiers take 1000-2000ms to release, increase to 2000ms.

### Fix Option 2: Different Modifier Detection
Use `GetKeyState` instead of `GetAsyncKeyState` or add delay before starting to check.

### Fix Option 3: Focus Management
Explicitly restore focus to target window after modifier wait:
```rust
// After wait_for_modifiers_release()
restore_previous_window_focus();
simulate_ctrl_c_once();
```

### Fix Option 4: Skip Modifier Wait for Alt+Shift
Since TextMonitor works fine with Alt+Shift, maybe we don't need to wait:
```rust
// Only wait if Ctrl was part of the hotkey
if is_ctrl_hotkey {
    wait_for_modifiers_release().await;
}
```

## Log Location
Logs are in the console where `npm run tauri dev` is running.

Look for lines starting with `DEBUG:` to track the flow.

## File Changes
- `src-tauri/src/commands/hotkey.rs`:
  - Line 37: MAX_MODIFIER_WAIT_MS changed from 500 to 1000
  - Lines 46-72: Enhanced `wait_for_modifiers_release()` logging
  - Lines 112-168: Enhanced `simulate_copy_with_retry()` logging
  - Line 402: Added entry point logging
