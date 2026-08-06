# Debug Report: Confirmation Dialog Spinner Issue

**Problem:** Confirmation dialog shows spinner indefinitely instead of displaying content.

## Logging Added

### React Side (ConfirmationWindow.tsx)
Added comprehensive console logging to trace:
1. Component mount/unmount
2. Focus event setup and triggers
3. Event listener setup and triggers
4. fetchData() execution
5. State changes (charCount, isReady)
6. Render states (loading vs content)

### Rust Side (confirmation.rs)
Added comprehensive logging to trace:
1. show_translation_confirmation() entry with parameters
2. State storage and verification
3. Window acquisition
4. Window positioning
5. Window show/focus operations
6. Event emission
7. get_confirmation_data() calls and return values

## Testing Instructions

### Step 1: Start the App
```bash
npm run tauri dev
```

### Step 2: Trigger Long Text Translation
1. Open any text editor (Notepad, VS Code, browser)
2. Select text longer than 200 characters (to trigger confirmation)
3. Press `Ctrl+Shift+Q` (global hotkey)

### Step 3: Check Logs

#### Browser Console (React logs)
Open DevTools in the confirmation window (if possible) or main window and look for:
- `[ConfirmationWindow] Component mounted`
- `[ConfirmationWindow] Setting up focus listener...`
- `[ConfirmationWindow] Focus changed event: { focused: true }`
- `[ConfirmationWindow] 50ms elapsed, now calling fetchData()`
- `[ConfirmationWindow] Received count from Rust: X`
- `[ConfirmationWindow] Setting state: charCount=X, isReady=true`

#### Rust Logs (Terminal)
Look for:
- `[CONFIRMATION] show_translation_confirmation() called with char_count=X`
- `[CONFIRMATION] Stored char_count=X in state`
- `[CONFIRMATION] Verification: state now contains char_count=X`
- `[CONFIRMATION] Showing window...`
- `[CONFIRMATION] Setting focus...`
- `[CONFIRMATION] Emitting show-confirmation-window event with charCount=X`
- `[CONFIRMATION] get_confirmation_data() called`
- `[CONFIRMATION] Returning char_count from state: X`

## Expected Flow

```
[Rust] show_translation_confirmation(char_count=500)
  ↓
[Rust] Store char_count=500 in ConfirmationDataState
  ↓
[Rust] Verify storage: char_count=500
  ↓
[Rust] window.show()
  ↓
[Rust] window.set_focus()
  ↓
[React] onFocusChanged fires: { focused: true }
  ↓
[React] setTimeout 50ms
  ↓
[React] fetchData() calls invoke('get_confirmation_data')
  ↓
[Rust] get_confirmation_data() returns 500
  ↓
[React] Receives count=500
  ↓
[React] setCharCount(500), setIsReady(true)
  ↓
[React] Component re-renders with content
```

## Potential Issues to Check

### Issue 1: State Not Persisting
**Symptom:** Rust logs show storage, but get_confirmation_data returns 0
**Cause:** State might be cleared between storage and fetch
**Check:** Look for verification logs showing char_count=0 when it should be >0

### Issue 2: Focus Event Not Firing
**Symptom:** No "Focus changed event" logs in React
**Cause:** window.show() or window.set_focus() might not trigger focus
**Check:** Look for React focus event logs. If missing, the event isn't firing.

### Issue 3: Race Condition
**Symptom:** fetchData() is called before char_count is stored
**Cause:** Timing issue between Rust storage and React fetch
**Check:** Compare timestamps. If get_confirmation_data logs appear BEFORE storage logs, we have a race.

### Issue 4: Event Not Reaching Window
**Symptom:** Event emission succeeds in Rust but no event received in React
**Cause:** Event target might be wrong or window not ready
**Check:** Look for "Received show-confirmation-window event" in React logs

### Issue 5: State Update Not Triggering Re-render
**Symptom:** React receives count > 0 but stays in loading state
**Cause:** State update might be failing or not triggering re-render
**Check:** Look for "Setting state" log followed by "Rendering loading spinner" instead of "Rendering confirmation content"

## Debugging Steps

1. **Check if ConfirmationDataState is registered:**
   - Look for any errors about missing state in Rust logs
   - Verify state is registered in lib.rs

2. **Check window creation:**
   - Verify confirmation window exists in tauri.conf.json
   - Check if window is created at startup

3. **Test event emission directly:**
   ```rust
   // In confirmation.rs, emit event BEFORE showing window
   app_handle.emit_to("confirmation", "show-confirmation-window", ...)?;
   tokio::time::sleep(Duration::from_millis(500)).await;
   window.show()?;
   ```

4. **Test with longer delays:**
   ```rust
   // Increase delay before emitting event
   tokio::time::sleep(std::time::Duration::from_millis(500)).await;
   ```

5. **Check if window is actually focused:**
   ```rust
   // After set_focus(), check if window is focused
   if let Ok(is_focused) = window.is_focused() {
       log::info!("[CONFIRMATION] Window is_focused: {}", is_focused);
   }
   ```

## Next Steps After Testing

Based on the logs, determine which of the following is happening:

1. **Char count not stored:** Add more error handling in state storage
2. **Focus event not firing:** Use only event-based approach, remove focus dependency
3. **Race condition:** Increase delays or use different synchronization
4. **Event not received:** Check event target, use different event mechanism
5. **State update failing:** Debug React state updates, check for stale closures

## Log Collection

To share logs, please capture:

1. **Full Rust terminal output** from triggering translation to timeout
2. **Full browser console output** from the confirmation window
3. **Screenshots** of the spinner (if visible) and any error messages

Save to:
- `rust_logs.txt` - Terminal output
- `react_logs.txt` - Browser console output
- `screenshot.png` - Visual confirmation of spinner
