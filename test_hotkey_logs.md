# Hotkey Test Procedure

## After Build Completes

The app is currently building in background. When you see the app window appear:

## Testing Steps

### Preparation
1. Open Notepad or Chrome
2. Type or paste a paragraph of text (20+ words)
3. Select the text with mouse drag

### Test 1: Working Case (Ctrl+Alt+Q)
1. With text selected, press **Ctrl+Alt+Q**
2. Watch the console/terminal for DEBUG logs
3. Note if popup appears with translation
4. Copy all DEBUG logs from console

### Test 2: Broken Case (Alt+Shift+Q)
1. Select the same text again
2. Press **Alt+Shift+Q**
3. **Carefully watch DEBUG logs**
4. Copy all DEBUG logs from console

### What to Look For in Logs

#### Entry Point
```
DEBUG: ===== HOTKEY TRANSLATION FLOW STARTED =====
```
- If this appears, hotkey was detected
- If this doesn't appear, hotkey registration failed

#### Modifier Wait Phase
```
DEBUG: Starting modifier wait (max 1000ms)
DEBUG: Modifier state at 0ms: Ctrl=false Shift=true Alt=true ...
DEBUG: Modifier state at 100ms: Ctrl=false Shift=false Alt=false ...
DEBUG: All modifier keys released after 120ms
```
- Check which keys show as held (true)
- Check how long until all keys show false
- Check if timeout occurs (1000ms)

#### Copy Phase
```
DEBUG: Step 2 - Sending Ctrl+C once via Windows API...
Hotkey: Simulated Ctrl+C once via Windows API
```
- Confirms Ctrl+C was sent

#### Clipboard Poll Phase
```
DEBUG: Step 3 - Polling clipboard for changes (max 3s)...
DEBUG: Poll 1: clipboard_len=1224, is_new=true, elapsed=100ms
DEBUG: SUCCESS - Clipboard captured after 1 polls (100ms)
```
OR
```
DEBUG: Poll 1: clipboard is empty or whitespace
DEBUG: Poll 2: clipboard is empty or whitespace
...
DEBUG: TIMEOUT - Clipboard capture timeout after 30 polls
```

## Key Questions to Answer

1. **Does Alt+Shift+Q trigger the hotkey?**
   - Look for "HOTKEY TRANSLATION FLOW STARTED"

2. **How long do modifiers take to release?**
   - Ctrl+Alt+Q: ___ms
   - Alt+Shift+Q: ___ms

3. **Which modifiers are stuck (if any)?**
   - At 0ms: Ctrl=___ Shift=___ Alt=___
   - At 100ms: Ctrl=___ Shift=___ Alt=___
   - At timeout (if reached): Ctrl=___ Shift=___ Alt=___

4. **Does Ctrl+C get sent?**
   - Look for "Simulated Ctrl+C once via Windows API"

5. **What happens to clipboard?**
   - Empty throughout?
   - Has old content (is_new=false)?
   - Gets new content (is_new=true)?

## Save Results

Copy all DEBUG logs and paste them into a new file:
`debug_alt_shift_q_results.txt`

Include:
- Which hotkey was pressed
- Full console output for that test
- Whether popup appeared

## Alternative: Run from Terminal to See Logs

If the app console doesn't show logs, stop the background build and run manually:

```bash
cd D:\sources\utils\translator-desktop
npm run tauri dev
```

Then perform the same tests and watch the terminal output directly.
