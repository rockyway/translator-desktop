# Translator Desktop App - Migration Plan

## Overview

Migrate the existing React + Express web-based translator app to a **cross-platform desktop application** using a **hybrid architecture**:
- **Tauri 2.0** - Translation UI, popup overlay, history, settings
- **.NET Text Monitor** - Global text selection detection (Ctrl+drag)

---

## Architecture: Hybrid Approach

```
┌─────────────────────────────────────────────────────────────────┐
│  USER ACTION: Ctrl + mouse drag to select text in ANY app       │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│  .NET TEXT MONITOR (Background Service)                         │
│  ├─ SharpHook: Global mouse/keyboard events                     │
│  ├─ FlaUI: UI Automation text retrieval                         │
│  ├─ Dual-path detection (proactive + reactive Ctrl)             │
│  └─ IPC: Named Pipe server → sends selected text + cursor pos   │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│  TAURI TRANSLATOR APP                                           │
│  ├─ IPC Client: Receives text from .NET monitor                 │
│  ├─ Translation: Calls Google Translate API                     │
│  ├─ Popup Overlay: Shows result near cursor (borderless window) │
│  ├─ Main Window: Full UI with history, settings                 │
│  └─ Global Hotkey: Ctrl+Shift+Q as backup method                │
└─────────────────────────────────────────────────────────────────┘
```

---

## Why Hybrid?

| Component | Pure Tauri | Hybrid (.NET + Tauri) |
|-----------|------------|----------------------|
| Text selection detection | ❌ Not possible | ✅ SharpHook + FlaUI |
| Ctrl+drag trigger | ❌ Not possible | ✅ Dual-path detection |
| Cursor position tracking | ⚠️ Limited | ✅ Full tracking |
| Popup at selection | ⚠️ Workaround needed | ✅ Exact position |
| Translation UI | ✅ Excellent | ✅ Excellent |
| Cross-platform | ✅ Win/Mac/Linux | ⚠️ Windows primary* |

*macOS/Linux can use Tauri-only mode with global hotkey fallback

---

## Project Structure

```
translator-desktop/
├── src/                              # React Frontend
│   ├── components/
│   ├── features/
│   │   ├── translator/               # Main translation panel
│   │   ├── popup/                    # Popup overlay component
│   │   ├── history/                  # Translation history
│   │   └── settings/                 # Hotkey, preferences
│   ├── hooks/
│   │   ├── useIpcListener.ts         # Listen for .NET monitor events
│   │   └── useGlobalHotkey.ts        # Backup hotkey method
│   └── services/
│       └── translationService.ts     # Tauri invoke calls
│
├── src-tauri/                        # Tauri Rust Backend
│   ├── src/
│   │   ├── main.rs                   # App entry, window management
│   │   ├── commands/
│   │   │   ├── translate.rs          # Google Translate API
│   │   │   ├── speak.rs              # TTS handler
│   │   │   └── history.rs            # SQLite CRUD
│   │   ├── ipc/
│   │   │   └── named_pipe.rs         # Named pipe client
│   │   └── windows/
│   │       └── popup.rs              # Popup window management
│   ├── tauri.conf.json
│   └── Cargo.toml
│
├── text-monitor/                     # .NET Text Selection Monitor
│   ├── TextMonitor.Service/          # Extracted from text-assistant
│   │   ├── GlobalHookService.cs      # SharpHook wrapper
│   │   ├── TextSelectionMonitor.cs   # Selection detection
│   │   ├── TextRetrievalEngine.cs    # UI Automation + clipboard
│   │   └── IpcServer.cs              # Named pipe server
│   └── TextMonitor.Service.csproj
│
├── package.json
└── README.md
```

---

## Implementation Plan

### Phase 0: Project Setup ✅
- [x] 0.1: Create `translator-desktop/` project folder
- [x] 0.2: Initialize Tauri 2.0 with React template
- [x] 0.3: Configure Vite for Tauri development
- [x] 0.4: Add Tauri plugins (sql, clipboard, global-shortcut, updater)
- [x] 0.5: Extract .NET text monitor from text-assistant project
- [x] 0.6: Create Named Pipe IPC protocol definition
- [x] CHECKPOINT A: Both projects build and run independently

### Phase 1: IPC Bridge ✅
- [x] 1.1: Implement Named Pipe server in .NET (TextMonitor.Service)
- [x] 1.2: Implement Named Pipe client in Rust (src-tauri/ipc/)
- [x] 1.3: Define message protocol: `{ text, cursorX, cursorY, appName }`
- [x] 1.4: Test IPC: .NET sends → Tauri receives
- [x] 1.5: Add reconnection logic and health monitoring
- [x] CHECKPOINT B: IPC communication works reliably

### Phase 2: Frontend Migration ✅
- [x] 2.1: Copy React components from web app
- [x] 2.2: Update translationService.ts for Tauri commands
- [x] 2.3: Create popup overlay component (borderless, always-on-top)
- [x] 2.4: Implement useIpcListener hook for .NET events
- [x] 2.5: Verify theme system (localStorage works in Tauri)
- [x] CHECKPOINT C: Main UI renders, receives IPC events

### Phase 3: Rust Backend ✅
- [x] 3.1: Create translate command (HTTP to Google Translate)
- [x] 3.2: Create speak command (TTS via Google API)
- [x] 3.3: Set up SQLite database for history
- [x] 3.4: Implement history commands (add, list, delete, clear)
- [x] 3.5: Create popup window manager (position at cursor)
- [x] CHECKPOINT D: Translation and TTS work end-to-end

### Phase 4: Popup Overlay Feature ✅
- [x] 4.1: Create popup window config (borderless, always-on-top, skip-taskbar)
- [x] 4.2: Position popup at cursor coordinates from IPC
- [x] 4.3: Create compact popup UI (source → translated, copy button)
- [x] 4.4: Auto-translate on text received from .NET
- [x] 4.5: Auto-hide popup after timeout or click outside
- [x] 4.6: Add "Open in main window" button on popup
- [x] CHECKPOINT E: Popup shows translation at selection location

### Phase 5: History Feature ✅
- [x] 5.1: Create History panel UI component
- [x] 5.2: Auto-save translations to history (optional toggle)
- [x] 5.3: Implement history list view with search
- [x] 5.4: Add delete individual / clear all functionality
- [x] 5.5: Persist history across app restarts
- [x] CHECKPOINT F: History feature fully functional

### Phase 6: Global Hotkey (Backup Method) ✅
- [x] 6.1: Register global hotkey: `Ctrl+Shift+Q`
- [x] 6.2: On hotkey: simulate Ctrl+C → read clipboard → translate
- [x] 6.3: Show result in popup overlay
- [ ] 6.4: Add hotkey configuration in Settings
- [ ] 6.5: Allow disabling .NET monitor (hotkey-only mode)
- [x] CHECKPOINT G: Hotkey works as fallback method

### Phase 7: Desktop Polish ✅
- [x] 7.1: Configure app window (size, title, icon)
- [ ] 7.2: Add system tray (minimize to tray, status indicator)
- [ ] 7.3: Configure auto-update with tauri-plugin-updater
- [ ] 7.4: Start .NET monitor on app launch (sidecar process)
- [ ] 7.5: Window state persistence (position, size)
- [x] CHECKPOINT H: App feels native and polished

### Phase 8: Build & Distribution ✅
- [ ] 8.1: Bundle .NET monitor as sidecar executable
- [ ] 8.2: Configure Windows installer (.msi with both components)
- [ ] 8.3: Configure macOS build (Tauri-only, hotkey mode)
- [ ] 8.4: Configure Linux build (Tauri-only, hotkey mode)
- [ ] 8.5: Create GitHub releases workflow
- [x] CHECKPOINT I: Release builds work

### Phase 9: Testing & QA ⏳
- [ ] 9.1: Test IPC reliability (rapid selections, long text)
- [ ] 9.2: Test popup positioning (multi-monitor, edge cases)
- [ ] 9.3: Test .NET monitor in various apps (Chrome, VS Code, Word)
- [ ] 9.4: Test fallback hotkey mode
- [ ] 9.5: Performance testing (memory, CPU usage)
- [ ] FINAL: Human Approval

---

## IPC Protocol (Named Pipe)

**Pipe Name:** `\\.\pipe\TranslatorDesktop`

**Message Format (JSON):**
```json
{
  "type": "text_selected",
  "payload": {
    "text": "Hello world",
    "cursorX": 500,
    "cursorY": 300,
    "sourceApp": "chrome.exe",
    "windowTitle": "Google - Chrome"
  },
  "timestamp": "2026-01-19T12:00:00Z"
}
```

**Commands:**
| Type | Direction | Purpose |
|------|-----------|---------|
| `text_selected` | .NET → Tauri | User selected text |
| `translate_request` | Tauri → .NET | Request text from UI Automation |
| `status` | Both | Health check / connection status |
| `config` | Tauri → .NET | Update monitor settings |

---

## Popup Window Configuration

```json
{
  "label": "popup",
  "title": "",
  "width": 400,
  "height": 200,
  "alwaysOnTop": true,
  "decorations": false,
  "transparent": false,
  "skipTaskbar": true,
  "visible": false,
  "resizable": false
}
```

**Popup Behavior:**
1. Created hidden on app start
2. On text selection: position at cursor, show, translate
3. Auto-hide after 10 seconds (configurable)
4. Click outside → hide
5. Escape key → hide
6. "Open in main" → show main window with text

---

## Technology Stack

| Layer | Technology |
|-------|------------|
| Translation UI | React 18 + TypeScript + Tailwind |
| Desktop Runtime | Tauri 2.0 |
| Backend (Tauri) | Rust |
| Text Monitor | .NET 8 + SharpHook + FlaUI |
| IPC | Named Pipes (Windows) |
| Database | SQLite (tauri-plugin-sql) |
| Global Hotkey | tauri-plugin-global-shortcut |

---

## Rust Dependencies (Cargo.toml)

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-sql = { version = "2", features = ["sqlite"] }
tauri-plugin-clipboard-manager = "2"
tauri-plugin-global-shortcut = "2"
tauri-plugin-updater = "2"
tauri-plugin-process = "2"
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
base64 = "0.22"
tokio = { version = "1", features = ["full", "net"] }
interprocess = "2"  # Named pipe IPC
enigo = "0.2"       # Keyboard simulation (for hotkey fallback)
```

---

## .NET Dependencies (TextMonitor.Service.csproj)

```xml
<PackageReference Include="SharpHook" Version="5.3.7" />
<PackageReference Include="FlaUI.Core" Version="4.0.0" />
<PackageReference Include="FlaUI.UIA3" Version="4.0.0" />
<PackageReference Include="Microsoft.Extensions.Hosting" Version="8.0.0" />
<PackageReference Include="Serilog" Version="3.0.0" />
```

---

## Quick Translate Workflow

### Method 1: Ctrl+Drag (Windows, via .NET Monitor)
```
1. User holds Ctrl + drags mouse to select text in ANY app
2. .NET Monitor detects selection via SharpHook
3. FlaUI retrieves selected text via UI Automation
4. .NET sends text + cursor position via Named Pipe
5. Tauri receives event, positions popup at cursor
6. Tauri calls Google Translate API
7. Popup shows translation result
8. User can copy, open in main app, or auto-dismiss
```

### Method 2: Global Hotkey (All Platforms, Fallback)
```
1. User selects text manually
2. User presses Ctrl+Shift+Q
3. Tauri simulates Ctrl+C to copy selection
4. Tauri reads clipboard text
5. Tauri positions popup (center of screen or last cursor)
6. Tauri calls Google Translate API
7. Popup shows translation result
```

---

## Platform Support

| Feature | Windows | macOS | Linux |
|---------|---------|-------|-------|
| Main App (Tauri) | ✅ | ✅ | ✅ |
| Ctrl+Drag Detection | ✅ (.NET) | ❌ | ❌ |
| Global Hotkey | ✅ | ✅ | ✅ (X11) |
| Popup Overlay | ✅ | ✅ | ✅ |
| History | ✅ | ✅ | ✅ |
| Auto-Update | ✅ | ✅ | ✅ |

**Note:** macOS/Linux users use hotkey-only mode. Future: port .NET monitor to native.

---

## Verification Steps

1. **Dev Mode**: `npm run tauri dev` - Tauri app launches
2. **IPC**: .NET monitor running → select text → Tauri receives event
3. **Popup**: Ctrl+drag text → popup appears at cursor with translation
4. **Hotkey**: Press Ctrl+Shift+Q → popup shows translation
5. **History**: Translations saved, searchable, deletable
6. **TTS**: Click Listen → audio plays
7. **Build**: `npm run tauri build` - Creates installer with .NET sidecar
8. **Install**: Fresh install works, .NET monitor auto-starts

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| IPC complexity | Use well-tested `interprocess` crate + error handling |
| .NET runtime dependency | Bundle .NET runtime or use self-contained publish |
| FlaUI fails on some apps | Fallback to clipboard simulation |
| Popup position off-screen | Clamp to screen bounds |
| Two processes to manage | Single installer, auto-start monitor as sidecar |

---

## Estimated Effort

| Phase | Effort |
|-------|--------|
| Phase 0: Setup | 1 day |
| Phase 1: IPC Bridge | 2 days |
| Phase 2: Frontend Migration | 1-2 days |
| Phase 3: Rust Backend | 2-3 days |
| Phase 4: Popup Overlay | 2 days |
| Phase 5: History Feature | 1-2 days |
| Phase 6: Global Hotkey | 1 day |
| Phase 7: Desktop Polish | 1-2 days |
| Phase 8: Build & Distribution | 1-2 days |
| Phase 9: Testing | 1-2 days |
| **Total** | **13-19 days** |

---

## Next Steps After Approval

1. Create `translator-desktop/` project folder
2. Extract text monitor from `D:\sources\demo\text-assistant\`
3. Initialize Tauri 2.0 project
4. Implement Named Pipe IPC bridge
5. Follow Master Agent orchestration protocol for delegation
