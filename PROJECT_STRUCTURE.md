# Translator Desktop - Project Structure

A cross-platform desktop translation application using a hybrid architecture combining **Tauri 2.0** (Rust + React) for the main application with a **.NET Text Monitor** service for global text selection detection on Windows.

**Application Version:** 0.1.0
**Target Frameworks:** Tauri 2.0, .NET 8.0, React 18
**Architecture Style:** Hybrid Desktop Architecture with IPC Bridge

**Last Updated:** January 19, 2026

---

## Project Statistics

| Component | Location | Files | Languages |
|-----------|----------|-------|-----------|
| React Frontend | `src/` | 19 | TypeScript, TSX |
| Tauri Backend | `src-tauri/src/` | 10 | Rust |
| .NET Text Monitor | `text-monitor/TextMonitor.Service/` | 22 | C# |
| Configuration | Root | 8 | JSON, JS, TS |
| Documentation | `docs/` | 1 | Markdown |

### File Breakdown

| Type | Count | Description |
|------|-------|-------------|
| `.tsx` | 9 | React components |
| `.ts` | 10 | TypeScript modules |
| `.rs` | 10 | Rust modules |
| `.cs` | 22 | C# source files |
| Config | 8 | package.json, Cargo.toml, etc. |

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Project Structure](#project-structure)
3. [Component Details](#component-details)
4. [Technology Stack](#technology-stack)
5. [Data Flow](#data-flow)
6. [Key Features](#key-features)
7. [Dependencies](#dependencies)
8. [Configuration](#configuration)
9. [Build & Development](#build--development)
10. [IPC Protocol](#ipc-protocol)

---

## Architecture Overview

### Architecture Pattern

**Hybrid Desktop Architecture** combining:
- **Tauri 2.0** - Main application runtime with Rust backend and React frontend
- **.NET Service** - Windows-specific text selection monitor running as a separate process
- **IPC Bridge** - Named Pipes for inter-process communication

### Design Patterns

| Pattern | Implementation | Location |
|---------|---------------|----------|
| **State Machine** | Text selection state tracking | `SelectionStateMachine.cs` |
| **Observer/Pub-Sub** | Event aggregation for selections | `SelectionEventAggregator.cs` |
| **Dependency Injection** | Service registration | `Program.cs` (.NET) |
| **Repository** | History persistence | `history.rs` |
| **Command Pattern** | Tauri IPC commands | `commands/*.rs` |
| **Custom Hooks** | React state management | `hooks/*.ts` |
| **Feature-Based Structure** | UI organization | `features/*/` |

### Core Principles

- **Separation of Concerns** - Distinct layers for monitoring, IPC, and UI
- **Single Responsibility** - Each module handles one specific task
- **Loose Coupling** - IPC-based communication between processes
- **Interface Segregation** - .NET services implement focused interfaces

---

## Project Structure

```
translator-desktop/
├── src/                          [React Frontend - 19 files]
│   ├── App.tsx                   Main app with tab navigation
│   ├── main.tsx                  Application entry point
│   ├── index.css                 Global Tailwind styles
│   ├── vite-env.d.ts             Vite type definitions
│   ├── components/               [Shared UI Components]
│   │   └── ThemeToggle.tsx       Dark/light mode toggle
│   ├── contexts/                 [React Contexts]
│   │   └── ThemeContext.tsx      Theme provider
│   ├── features/                 [Feature Modules]
│   │   ├── history/              History feature
│   │   │   ├── HistoryPanel.tsx  History list with search
│   │   │   └── index.ts          Feature exports
│   │   ├── popup/                Popup overlay feature
│   │   │   ├── PopupOverlay.tsx  Compact translation popup
│   │   │   ├── PopupWindow.tsx   Popup window container
│   │   │   └── index.ts          Feature exports
│   │   └── translator/           Main translator feature
│   │       ├── TranslationPanel.tsx  Main translation UI
│   │       ├── LanguageSelector.tsx  Language dropdown
│   │       └── index.ts          Feature exports
│   ├── hooks/                    [Custom React Hooks]
│   │   ├── useHistory.ts         History CRUD operations
│   │   ├── useIpcListener.ts     IPC event handling
│   │   ├── useTheme.ts           Theme state management
│   │   └── useTranslation.ts     Translation with debounce
│   ├── services/                 [API Services]
│   │   └── translationService.ts Tauri invoke wrappers
│   └── utils/                    [Utility Functions]
│       └── debounce.ts           Debounce utility
│
├── src-tauri/                    [Tauri Backend]
│   ├── Cargo.toml                Rust dependencies
│   ├── tauri.conf.json           Tauri configuration
│   ├── build.rs                  Build script
│   ├── icons/                    Application icons
│   └── src/                      [Rust Source - 10 files]
│       ├── main.rs               Binary entry point
│       ├── lib.rs                Library entry, plugin setup
│       ├── commands/             [Tauri Commands]
│       │   ├── mod.rs            Module exports
│       │   ├── translate.rs      Google Translate API
│       │   ├── speak.rs          Text-to-speech
│       │   ├── history.rs        SQLite CRUD
│       │   ├── popup.rs          Popup window control
│       │   └── hotkey.rs         Global hotkey handler
│       └── ipc/                  [IPC Client]
│           ├── mod.rs            Module exports
│           └── named_pipe.rs     Named Pipe client
│
├── text-monitor/                 [.NET Text Monitor Service]
│   ├── TextMonitor.Service/      [Main Service - 22 files]
│   │   ├── Program.cs            Entry point, DI setup
│   │   ├── TextMonitor.Service.csproj
│   │   ├── Enums/                [Enumerations]
│   │   │   ├── RetrievalMethod.cs
│   │   │   └── SelectionState.cs
│   │   ├── Events/               [Event System]
│   │   │   └── SelectionEventAggregator.cs
│   │   ├── Interfaces/           [Service Contracts]
│   │   │   ├── IClipboardService.cs
│   │   │   ├── IGlobalHookService.cs
│   │   │   ├── ISelectionEventAggregator.cs
│   │   │   ├── ITextRetrievalService.cs
│   │   │   ├── ITextSelectionMonitor.cs
│   │   │   └── IUIAutomationService.cs
│   │   ├── Ipc/                  [IPC Server]
│   │   │   └── IpcServer.cs      Named Pipe server
│   │   ├── Models/               [Data Models]
│   │   │   ├── ClipboardState.cs
│   │   │   ├── FocusedElementInfo.cs
│   │   │   ├── MouseEventData.cs
│   │   │   ├── SelectionCoordinates.cs
│   │   │   ├── TextRetrievalResult.cs
│   │   │   └── TextSelectionEvent.cs
│   │   ├── Monitoring/           [Selection Detection]
│   │   │   ├── CursorTracker.cs
│   │   │   ├── SelectionStateMachine.cs
│   │   │   └── TextSelectionMonitor.cs
│   │   ├── Services/             [Core Services]
│   │   │   └── GlobalHookService.cs
│   │   └── TextRetrieval/        [Text Extraction]
│   │       ├── ClipboardService.cs
│   │       ├── TextRetrievalEngine.cs
│   │       └── UIAutomationService.cs
│   └── TextMonitor.Test/         [Test Client]
│       └── Program.cs            IPC test tool
│
├── docs/                         [Documentation]
│   └── plan/
│       └── 001-desktop-migration-plan.md
│
├── public/                       [Static Assets]
│   └── vite.svg
│
├── scripts/                      [Build Scripts]
│
├── package.json                  Frontend dependencies
├── tsconfig.json                 TypeScript config
├── vite.config.ts                Vite bundler config
├── tailwind.config.js            Tailwind CSS config
├── postcss.config.js             PostCSS config
├── CLAUDE.md                     Project instructions
└── README.md                     Project readme
```

---

## Component Details

### React Frontend (`src/`)

**Purpose:** User interface for translation, history management, and popup display.

| Directory | Purpose | Key Files |
|-----------|---------|-----------|
| `features/translator/` | Main translation UI | `TranslationPanel.tsx` - Full translation interface |
| `features/history/` | Translation history | `HistoryPanel.tsx` - List with search/delete |
| `features/popup/` | Quick translate popup | `PopupOverlay.tsx` - Compact overlay at cursor |
| `hooks/` | State management | `useTranslation.ts` - Debounced translation |
| `services/` | Tauri communication | `translationService.ts` - Invoke wrappers |
| `contexts/` | App-wide state | `ThemeContext.tsx` - Dark/light mode |

**Notable Implementations:**
- Auto-save to history on successful translation (`useTranslation.ts:101-123`)
- IPC event listener for text selection events (`useIpcListener.ts`)
- TanStack Query for mutation state management

### Tauri Backend (`src-tauri/`)

**Purpose:** Desktop runtime, Rust backend commands, IPC client, database management.

| Module | Purpose | Key Functions |
|--------|---------|---------------|
| `lib.rs` | App entry, plugin setup | Global hotkey registration, DB init |
| `commands/translate.rs` | Translation API | Google Translate via unofficial endpoint |
| `commands/speak.rs` | Text-to-speech | Google TTS, 200 char limit |
| `commands/history.rs` | History CRUD | SQLite with pagination, search |
| `commands/popup.rs` | Popup management | Window positioning and visibility |
| `ipc/named_pipe.rs` | IPC client | Connects to .NET service |

**Tauri Commands:**

| Command | Input | Output |
|---------|-------|--------|
| `translate` | text, from, to | `TranslateResult` |
| `speak` | text, languageCode | Base64 MP3 |
| `add_history` | `AddHistoryInput` | ID |
| `get_history` | limit?, offset? | `HistoryPage` |
| `search_history` | query, limit?, offset? | `HistoryPage` |
| `delete_history` | id | boolean |
| `clear_history` | - | count |
| `show_popup` | x, y | - |
| `hide_popup` | - | - |

### .NET Text Monitor (`text-monitor/`)

**Purpose:** Windows-specific text selection detection using global hooks and UI Automation.

| Layer | Purpose | Key Classes |
|-------|---------|-------------|
| **Monitoring** | Selection detection | `TextSelectionMonitor`, `SelectionStateMachine` |
| **Services** | Global keyboard/mouse hooks | `GlobalHookService` (SharpHook) |
| **TextRetrieval** | Text extraction | `UIAutomationService` (FlaUI), `ClipboardService` |
| **Events** | Event aggregation | `SelectionEventAggregator` |
| **IPC** | Communication with Tauri | `IpcServer` (Named Pipes) |

**State Machine States:**
```
Idle → CtrlPressed → Dragging → DragComplete → Idle
```

---

## Technology Stack

### Frontend

| Technology | Version | Purpose |
|------------|---------|---------|
| React | 18.3.1 | UI framework |
| TypeScript | 5.6.2 | Type safety |
| Vite | 6.0.7 | Build tool |
| Tailwind CSS | 3.4.17 | Styling |
| TanStack Query | 5.62.8 | Server state |
| react-hook-form | 7.54.2 | Form handling |
| react-icons | 5.4.0 | Icon library |

### Desktop Runtime

| Technology | Version | Purpose |
|------------|---------|---------|
| Tauri | 2.x | Desktop framework |
| Rust | 2021 Edition | Backend language |
| sqlx | 0.8 | Database ORM |
| reqwest | 0.12 | HTTP client |
| tokio | 1.x | Async runtime |
| enigo | 0.2 | Input simulation |

### .NET Monitor

| Technology | Version | Purpose |
|------------|---------|---------|
| .NET | 8.0-windows | Runtime |
| SharpHook | 5.3.7 | Global hooks |
| FlaUI.Core | 4.0.0 | UI Automation |
| FlaUI.UIA3 | 4.0.0 | UIA3 provider |
| Serilog | 3.1.1 | Logging |

### Data & Communication

| Technology | Purpose |
|------------|---------|
| SQLite | Translation history storage |
| Named Pipes | IPC between Tauri and .NET |
| JSON | IPC message format |

---

## Data Flow

### Text Selection Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                    User selects text (Ctrl+drag)                     │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│  .NET TextMonitor Service                                            │
│  ┌─────────────┐   ┌──────────────────┐   ┌────────────────────┐   │
│  │ SharpHook   │ → │ SelectionState   │ → │ TextRetrievalEngine│   │
│  │ (Ctrl+drag) │   │   Machine        │   │ (FlaUI/Clipboard)  │   │
│  └─────────────┘   └──────────────────┘   └────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                            Named Pipe IPC
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Tauri Application                                                   │
│  ┌─────────────────┐   ┌───────────────┐   ┌────────────────────┐  │
│  │ named_pipe.rs   │ → │ show_popup    │ → │ React Frontend     │  │
│  │ (IPC Client)    │   │ (Window Mgmt) │   │ (PopupOverlay)     │  │
│  └─────────────────┘   └───────────────┘   └────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Translation & Storage                                               │
│  ┌─────────────────┐   ┌───────────────┐   ┌────────────────────┐  │
│  │ Google Translate│ → │ useTranslation│ → │ SQLite History     │  │
│  │ API (Rust)      │   │ (Auto-save)   │   │ (add_history)      │  │
│  └─────────────────┘   └───────────────┘   └────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### Global Hotkey Flow (Ctrl+Shift+Q)

```
┌───────────────┐   ┌──────────────────┐   ┌────────────────────┐
│ Tauri Plugin  │ → │ simulate_copy    │ → │ Read Clipboard     │
│ (global-      │   │ (Ctrl+C via      │   │ Show popup at      │
│  shortcut)    │   │  enigo)          │   │ cursor position    │
└───────────────┘   └──────────────────┘   └────────────────────┘
```

---

## Key Features

### Core Functionality

| Feature | Implementation |
|---------|---------------|
| **Text Translation** | Google Translate API via Rust backend |
| **Auto-detect Language** | `from: 'auto'` parameter |
| **Text-to-Speech** | Google TTS with 200 char limit |
| **Translation History** | SQLite with full-text search |
| **Global Hotkey** | Ctrl+Shift+Q via Tauri plugin |
| **Text Selection** | Ctrl+drag detection via SharpHook |

### UI Features

| Feature | Component |
|---------|-----------|
| **Main Window** | Tab-based UI (Translate/History) |
| **Popup Overlay** | 400x250px, always-on-top, decorationless |
| **Dark Mode** | System preference + manual toggle |
| **Responsive Design** | Tailwind CSS responsive utilities |

### Advanced Features

| Feature | Description |
|---------|-------------|
| **Auto-save History** | Translations saved on completion |
| **Debounced Input** | 500ms debounce for translation API |
| **IPC Reconnection** | Auto-reconnect on pipe disconnection |
| **Multiple Retrieval Methods** | UI Automation → Clipboard fallback |

---

## Dependencies

### Frontend (package.json)

```json
{
  "dependencies": {
    "@tanstack/react-query": "^5.62.8",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-hook-form": "^7.54.2",
    "react-icons": "^5.4.0"
  },
  "devDependencies": {
    "@tauri-apps/api": "^2.2.0",
    "@tauri-apps/cli": "^2.2.4",
    "tailwindcss": "^3.4.17",
    "typescript": "~5.6.2",
    "vite": "^6.0.7"
  }
}
```

### Rust Backend (Cargo.toml)

```toml
[dependencies]
tauri = "2"
tauri-plugin-sql = { version = "2", features = ["sqlite"] }
tauri-plugin-clipboard-manager = "2"
tauri-plugin-global-shortcut = "2"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full", "net"] }
serde = { version = "1", features = ["derive"] }
enigo = "0.2"
thiserror = "1"
```

### .NET Monitor (TextMonitor.Service.csproj)

```xml
<PackageReference Include="SharpHook" Version="5.3.7" />
<PackageReference Include="FlaUI.Core" Version="4.0.0" />
<PackageReference Include="FlaUI.UIA3" Version="4.0.0" />
<PackageReference Include="Microsoft.Extensions.Hosting" Version="8.0.0" />
<PackageReference Include="Serilog" Version="3.1.1" />
```

---

## Configuration

### Tauri Configuration (`tauri.conf.json`)

| Setting | Value | Purpose |
|---------|-------|---------|
| `productName` | Translator Desktop | App name |
| `identifier` | com.translator.desktop | Bundle ID |
| `windows.main` | 900x700, resizable | Main window |
| `windows.popup` | 400x250, alwaysOnTop | Quick translate |
| `security.csp` | Whitelisted domains | Content Security Policy |

### Environment Configuration

| File | Purpose |
|------|---------|
| `vite.config.ts` | Vite build configuration |
| `tsconfig.json` | TypeScript compiler options |
| `tailwind.config.js` | Tailwind customization |
| `postcss.config.js` | PostCSS plugins |

---

## Build & Development

### Quick Commands

```bash
# Development
npm run tauri dev              # Run Tauri app in dev mode

# .NET Monitor (separate terminal)
cd text-monitor/TextMonitor.Service && dotnet run

# Production Build
npm run tauri build            # Full production build

# Type Checking
npx tsc --noEmit               # TypeScript check
cd src-tauri && cargo test     # Rust tests
```

### Development Workflow

1. Start .NET Text Monitor service
2. Run `npm run tauri dev`
3. Use Ctrl+drag to select text in any app
4. Popup appears at cursor with translation

### Build Output

| Platform | Output |
|----------|--------|
| Windows | `src-tauri/target/release/translator-desktop.exe` |
| Bundle | `.msi`, `.exe` installers |

---

## IPC Protocol

### Pipe Configuration

| Setting | Value |
|---------|-------|
| Pipe Name | `\\.\pipe\TranslatorDesktop` |
| Direction | .NET (Server) → Tauri (Client) |
| Format | Newline-delimited JSON |

### Message Types

**text_selected:**
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

### Event Flow

```
.NET IpcServer → Named Pipe → Rust named_pipe.rs → Tauri Event → React Hook
```

---

## Future Enhancements

### Planned

- [ ] System tray integration (minimize to tray)
- [ ] Bundle .NET monitor as Tauri sidecar
- [ ] Auto-start .NET monitor with app
- [ ] macOS/Linux support (hotkey-only mode)

### Technical Debt

- [ ] Named Pipe ACLs for production security
- [ ] Unit tests for React components
- [ ] Integration tests for IPC

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | Jan 2026 | Initial release with core features |

---

**Document Version:** 1.0
**Generated:** January 19, 2026
