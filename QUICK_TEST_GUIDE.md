# Quick Test Guide - Alt+Shift+Q Debugging

## TL;DR

After the app builds and launches:

### Test 1: Press Ctrl+Alt+Q (should work)
- Select text
- Press **Ctrl+Alt+Q**
- Popup should appear
- Copy console logs → `ctrl_alt_q_logs.txt`

### Test 2: Press Alt+Shift+Q (broken)
- Select text
- Press **Alt+Shift+Q**
- Watch console closely
- Copy console logs → `alt_shift_q_logs.txt`

---

## What to Look For

### Key Questions:
1. Does "HOTKEY TRANSLATION FLOW STARTED" appear? (Yes/No)
2. How long until modifiers release? (___ms)
3. Which keys are stuck at timeout? Ctrl=___ Shift=___ Alt=___
4. Does "Simulated Ctrl+C" appear? (Yes/No)
5. Clipboard result: Empty / Old Content / New Content

### Success Pattern (Ctrl+Alt+Q):
```
DEBUG: ===== HOTKEY TRANSLATION FLOW STARTED =====
DEBUG: Modifier state at 0ms: Ctrl=true Alt=true
DEBUG: All modifier keys released after 100ms
DEBUG: Simulated Ctrl+C once
DEBUG: Poll 1: clipboard_len=1224, is_new=true
DEBUG: SUCCESS
```

### Failure Patterns (Alt+Shift+Q):

**Pattern A - Keys Stuck:**
```
DEBUG: Modifier state at 0ms: Shift=true Alt=true
DEBUG: Modifier state at 100ms: Shift=true Alt=true  ← STUCK
DEBUG: Modifier state at 900ms: Shift=true Alt=true  ← STILL STUCK
DEBUG: TIMED OUT after 1000ms
```

**Pattern B - Focus Lost:**
```
DEBUG: All modifier keys released after 120ms
DEBUG: Simulated Ctrl+C once
DEBUG: Poll 1: clipboard is empty  ← NO TEXT COPIED
DEBUG: Poll 30: clipboard is empty
DEBUG: TIMEOUT
```

**Pattern C - Old Clipboard:**
```
DEBUG: Simulated Ctrl+C once
DEBUG: Poll 1: clipboard_len=150, is_new=false  ← OLD TEXT
DEBUG: Poll 30: clipboard_len=150, is_new=false  ← UNCHANGED
DEBUG: TIMEOUT
```

---

## Fast Debug Checklist

| Check | Ctrl+Alt+Q | Alt+Shift+Q | Notes |
|-------|-----------|------------|-------|
| Hotkey detected? | ☐ | ☐ | Look for "FLOW STARTED" |
| Modifiers release? | ☐ | ☐ | < 1000ms? |
| Ctrl+C sent? | ☐ | ☐ | Look for "Simulated" |
| Clipboard updated? | ☐ | ☐ | is_new=true? |
| Popup appears? | ☐ | ☐ | Visual confirmation |

---

## Share Results

After testing, share these 3 files:
1. `ctrl_alt_q_logs.txt` - Full console output for working case
2. `alt_shift_q_logs.txt` - Full console output for broken case
3. This checklist with checkboxes filled in

---

## Alternative: Run from Terminal

If console logs aren't visible:

```bash
cd D:\sources\utils\translator-desktop
npm run tauri dev
```

Then test and watch terminal output directly.
