# CLAUDE.md - Translator Desktop App

## Project Overview

**Translator Desktop** - Cross-platform desktop translation app using hybrid architecture:
- **Tauri 2.0** - Desktop runtime, React UI, Rust backend
- **.NET Text Monitor** - Global text selection detection (Windows)

| Component | Location | Purpose |
|-----------|----------|---------|
| React Frontend | `src/` | Translation UI, history, popup |
| Tauri Backend | `src-tauri/` | Rust commands, IPC client |
| .NET Monitor | `text-monitor/` | Text selection via SharpHook + FlaUI |

---

## Quick Commands

```bash
# Development (sidecar auto-starts with app)
npm run tauri dev              # Run Tauri app in dev mode

# Build
npm run build                  # Build frontend only
npm run build:sidecar          # Build .NET sidecar only
npm run build:all              # Build sidecar + full Tauri build
npm run tauri build            # Full production build (requires sidecar)

# Testing
cd src-tauri && cargo test     # Rust tests
npx tsc --noEmit               # TypeScript check

# First-time setup (build sidecar before running)
npm run build:sidecar          # Required before first `npm run tauri dev`
```

**Note:** The .NET Text Monitor runs as a bundled sidecar - no separate terminal needed!

---

## Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend | React 18, TypeScript, Vite, Tailwind CSS, TanStack Query |
| Desktop | Tauri 2.0 |
| Backend | Rust (reqwest, sqlx, enigo) |
| Text Monitor | .NET 8, SharpHook, FlaUI |
| IPC | Named Pipes (`\\.\pipe\TranslatorDesktop`) |
| Database | SQLite (via sqlx) |

---

## Architecture

```
User selects text (Ctrl+drag) in ANY app
  → .NET Monitor detects via SharpHook
  → FlaUI retrieves selected text
  → Named Pipe sends to Tauri
  → Popup appears at cursor with translation

Alternative: User presses Ctrl+Shift+Q
  → Tauri simulates Ctrl+C
  → Reads clipboard
  → Shows popup with translation
```

---

## Key Files

### Tauri Backend (`src-tauri/src/`)
- `lib.rs` - App entry, plugin registration, global hotkey
- `sidecar.rs` - .NET text monitor sidecar management
- `commands/translate.rs` - Google Translate API
- `commands/speak.rs` - TTS (200 char limit)
- `commands/history.rs` - SQLite CRUD operations
- `commands/popup.rs` - Popup window management
- `commands/hotkey.rs` - Global hotkey handler
- `ipc/named_pipe.rs` - IPC client for .NET communication

### React Frontend (`src/`)
- `App.tsx` - Main app with tabs (Translate/History)
- `features/translator/TranslationPanel.tsx` - Main translation UI
- `features/popup/PopupOverlay.tsx` - Compact popup component
- `features/history/HistoryPanel.tsx` - History list with search
- `hooks/useIpcListener.ts` - IPC event listener
- `hooks/useTranslation.ts` - Translation hook with auto-save
- `hooks/useHistory.ts` - History operations hook
- `services/translationService.ts` - Tauri invoke wrappers

### .NET Monitor (`text-monitor/`)
- `TextMonitor.Service/` - Main service
- `TextMonitor.Test/` - IPC test tool

---

## Tauri Commands

| Command | Parameters | Returns |
|---------|------------|---------|
| `translate` | text, from, to | { translatedText, detectedLanguage } |
| `speak` | text, languageCode | base64 MP3 string |
| `add_history` | sourceText, translatedText, ... | id |
| `get_history` | limit?, offset? | { entries, total, hasMore } |
| `search_history` | query, limit?, offset? | { entries, total, hasMore } |
| `delete_history` | id | void |
| `clear_history` | - | void |
| `show_popup` | x, y | void |
| `hide_popup` | - | void |
| `get_popup_text` | - | string |
| `set_popup_text` | text | void |
| `start_text_monitor` | - | void |
| `stop_text_monitor` | - | void |
| `is_text_monitor_running` | - | boolean |

---

## Coding Standards

### General
- **Component size:** ~400 lines max
- **Naming:** PascalCase for components, camelCase for functions
- **Styling:** Tailwind only, no custom CSS
- **Theme colors:** Gold (`amber-500/600`), Blue (`blue-500/600`)

### Rust
- Use `thiserror` for error types
- All commands return `Result<T, Error>`
- Serialize errors for Tauri IPC

### TypeScript
- Use Tauri `invoke` from `@tauri-apps/api/core`
- Type all command responses
- Handle errors gracefully

---

## IPC Protocol

**Pipe Name:** `\\.\pipe\TranslatorDesktop`

**Message Format:**
```json
{
  "type": "text_selected",
  "payload": {
    "text": "Hello world",
    "cursorX": 500,
    "cursorY": 300,
    "sourceApp": "chrome.exe",
    "windowTitle": "Google"
  },
  "timestamp": "2026-01-19T12:00:00Z"
}
```

---

## Implementation Status

| Phase | Status | Notes |
|-------|--------|-------|
| 0: Project Setup | ✅ | Tauri + .NET initialized |
| 1: IPC Bridge | ✅ | Named Pipes working |
| 2: Frontend Migration | ✅ | React components migrated |
| 3: Rust Backend | ✅ | translate, speak, history |
| 4: Popup Overlay | ✅ | Shows at cursor |
| 5: History Feature | ✅ | UI with search, auto-save |
| 6: Global Hotkey | ✅ | Ctrl+Shift+Q |
| 7: Desktop Polish | ✅ | CSP, validation |
| 8: Build | ✅ | Release builds work |
| 9: Testing | ⏳ | Manual testing needed |

---

## Known Issues / TODO

- [ ] System tray integration (minimize to tray)
- [ ] Named Pipe ACLs for production security
- [x] Bundle .NET monitor as sidecar
- [x] Auto-start .NET monitor with app
- [ ] macOS/Linux builds (hotkey-only mode)

---

## Commit Messages

Use conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`

Do not include Claude credit/co-author lines.

---

## Agent Role Detection

**Master Agent** if:
- User invoked `/t-as-master`
- TodoWrite contains "MASTER AGENT MODE ACTIVE"

→ Read `master-agent.md`, delegate tasks, never write code directly.

**Subagent** if:
- Spawned by Task tool
- Prompt includes "As [ROLE] AGENT"

→ Execute assigned task only, return results.

---

## Explorer Agent Instructions

**IMPORTANT:** Before exploring this codebase, read `PROJECT_STRUCTURE.md` first.

This file contains:
- Complete directory tree with file purposes
- Architecture patterns and design decisions
- Component relationships and data flow
- Technology stack with versions
- IPC protocol details

**Quick orientation:**
```
src/                 → React frontend (19 files)
src-tauri/src/       → Rust backend (10 files)
text-monitor/        → .NET text monitor (22 files)
```

Reading PROJECT_STRUCTURE.md will significantly speed up exploration tasks.

---

## Resources

| Resource | Location |
|----------|----------|
| **Project Structure** | `PROJECT_STRUCTURE.md` |
| Migration Plan | `docs/plan/001-desktop-migration-plan.md` |
| Original PRD | `../translator-app/docs/requirement/prd.md` |
