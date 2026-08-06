# Testing Instructions: Confirmation Dialog Debug

## Changes Made

### 1. Enhanced Logging

#### React (ConfirmationWindow.tsx)
- Component mount/unmount tracking
- Focus event lifecycle logging
- Event listener setup tracking
- fetchData() execution tracing
- State change logging
- Render state logging

#### Rust (confirmation.rs)
- Entry point logging with parameters
- State storage and verification
- Window operations tracking
- Event emission logging
- get_confirmation_data() call tracking

### 2. Visual Debug Mode

The loading spinner now shows a **RED BACKGROUND** with debug information:
- White spinner (more visible)
- "LOADING DEBUG" text
- Current `isReady` value
- Current `charCount` value

This makes it immediately obvious if:
- The window is visible but stuck in loading state
- The window dimensions are correct
- The component is actually rendering

## How to Test

### Step 1: Start Dev Mode

```bash
npm run tauri dev
```

Wait for the app to fully start (you should see the main window).

### Step 2: Trigger Long Text Translation

**Option A: Using a text editor**
1. Open Notepad or any text editor
2. Paste this text (300 characters):
   ```
   Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate.
   ```
3. Select all the text (Ctrl+A)
4. Press `Ctrl+Shift+Q`

**Option B: Using the main app**
1. Open the translator app
2. Paste the 300-character text into the source text area
3. Click Translate (if char limit is enforced in UI)

### Step 3: Observe the Confirmation Window

When the confirmation dialog appears, you should see:

**If working correctly:**
- A white dialog with amber header
- "Long Text Detected" title
- Character count displayed
- "Translate" and "Cancel" buttons

**If still broken (showing spinner):**
- **RED BACKGROUND** with debug info
- White spinner
- "LOADING DEBUG" text
- `isReady: false`
- `charCount: 0` (or some value)

### Step 4: Collect Logs

#### A. Browser Console Logs

Open the browser DevTools:
1. Right-click on the **confirmation window** (if you can see it)
2. Select "Inspect" or press F12
3. Go to the Console tab
4. Look for logs starting with `[ConfirmationWindow]`

**Expected logs if working:**
```
[ConfirmationWindow] Component mounted
[ConfirmationWindow] Initial state: isReady=false, charCount=0
[ConfirmationWindow] Window dimensions: 400 x 280
[ConfirmationWindow] Setting up focus listener...
[ConfirmationWindow] Setting up event listener for show-confirmation-window...
[ConfirmationWindow] Focus changed event: { focused: true }
[ConfirmationWindow] Window focused, will fetch data in 50ms...
[ConfirmationWindow] 50ms elapsed, now calling fetchData()
[ConfirmationWindow] fetchData() called
[ConfirmationWindow] Invoking get_confirmation_data...
[ConfirmationWindow] Received count from Rust: 300
[ConfirmationWindow] Count > 0, setting state: charCount=300, isReady=true
[ConfirmationWindow] Rendering confirmation content (isReady=true, charCount=300)
```

**Logs if broken:**
Look for missing logs or errors. Key questions:
- Does `Focus changed event` appear?
- Does `fetchData()` get called?
- What value does `Received count from Rust` show?
- Does `setting state` appear?

#### B. Rust Terminal Logs

In the terminal where you ran `npm run tauri dev`, look for:

**Expected logs if working:**
```
[CONFIRMATION] show_translation_confirmation() called with char_count=300
[CONFIRMATION] ConfirmationDataState found, storing char_count...
[CONFIRMATION] Stored char_count=300 in state
[CONFIRMATION] Verification: state now contains char_count=300
[CONFIRMATION] Getting confirmation window...
[CONFIRMATION] Confirmation window found
[CONFIRMATION] Positioning window at (X, Y)...
[CONFIRMATION] Showing window...
[CONFIRMATION] Setting focus...
[CONFIRMATION] Waiting 150ms for React to process focus event...
[CONFIRMATION] Emitting show-confirmation-window event with charCount=300
[CONFIRMATION] Dialog shown at (X, Y) for 300 chars, waiting for user response...
[CONFIRMATION] get_confirmation_data() called
[CONFIRMATION] Returning char_count from state: 300
```

**Logs if broken:**
Look for:
- Is char_count stored correctly? (Should see 300, not 0)
- Does verification show the correct value?
- Is `get_confirmation_data()` called?
- What value is returned?

### Step 5: Analyze the Issue

Based on the logs and visual state, determine which scenario is happening:

#### Scenario 1: Window Not Visible at All
**Symptoms:**
- No red background visible
- No window appears

**Possible causes:**
- Window creation failed
- Window positioning is off-screen
- Window show() failed

**Check:** Rust logs for errors in showing/positioning

#### Scenario 2: Red Debug Screen Visible
**Symptoms:**
- RED background with "LOADING DEBUG"
- Shows `isReady: false`
- Shows `charCount: 0` or some value

**Possible causes:**
- Focus event not firing (check React logs)
- fetchData() not called (check React logs)
- get_confirmation_data() returns 0 (check Rust logs)
- State update not triggering re-render (check React logs)

**Next steps:** Compare React and Rust logs to find the gap

#### Scenario 3: Works Correctly
**Symptoms:**
- White dialog with proper content
- Character count displayed correctly
- Buttons work

**Next steps:** Consider the fix successful!

## Common Issues and Solutions

### Issue: Focus event never fires

**Symptom:** React logs show no "Focus changed event"

**Solution:**
1. Try increasing delay in Rust before emitting event
2. Use only event-based approach (remove focus dependency)
3. Check if window.set_focus() is actually focusing the window

### Issue: get_confirmation_data() returns 0

**Symptom:** Rust stores 300, but returns 0 when called

**Solution:**
1. Check if state is being cleared somewhere
2. Verify state is not being reset between storage and fetch
3. Add more logging around state access

### Issue: charCount received but isReady stays false

**Symptom:** React logs show "Received count: 300" but no state update

**Solution:**
1. Check for React state update issues
2. Look for errors in console
3. Verify setCharCount and setIsReady are being called

### Issue: Event received but not triggering state update

**Symptom:** "Received show-confirmation-window event" appears but no change

**Solution:**
1. Check if charCount in event payload is > 0
2. Verify event payload structure matches expectations
3. Add logging inside the event handler

## Reporting Results

Please share:

1. **Screenshot** of the window (red debug screen or white dialog)
2. **Full React console logs** (copy all [ConfirmationWindow] lines)
3. **Full Rust terminal logs** (copy all [CONFIRMATION] lines)
4. **Description** of what you observed

Save to:
- `screenshot.png` - Visual state
- `react-console.txt` - Browser console output
- `rust-terminal.txt` - Terminal output

## Reverting Debug Changes

Once debugging is complete, the red background and excessive logging can be removed:

1. Change loading state back to original white background
2. Remove or reduce console.log statements
3. Keep only essential error logging

File locations:
- React: `src/features/confirmation/ConfirmationWindow.tsx`
- Rust: `src-tauri/src/commands/confirmation.rs`
