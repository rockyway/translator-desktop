# Plan: Real-Time Settings Sync for Modifier Keys

## Summary
Implement real-time synchronization of selection modifier and global hotkey modifier settings between React frontend, Tauri backend, and .NET Text Monitor.

## Current State
- **Selection modifier**: Hardcoded Ctrl/Shift in .NET (`TextSelectionMonitor.cs:29-30`)
- **Global hotkey**: Hardcoded `Ctrl+Shift+Q` in Tauri (`lib.rs:41`)
- **IPC**: One-way only (.NET → Tauri), cannot receive configuration
- **Settings UI**: Already exists, stores to SQLite, but doesn't propagate to components

## Implementation

### Phase 1: Change Default Selection Modifier to Alt

**File: `src/contexts/SettingsContext.tsx`** (line 51)
- Change `selectionModifier: 'ctrl'` → `selectionModifier: 'alt'`

---

### Phase 2: Bidirectional IPC - Configuration Channel

#### 2.1 .NET Side - Configuration Receiver

**New file: `text-monitor/TextMonitor.Service/Ipc/ConfigurationReceiver.cs`**
- Named pipe server: `TranslatorDesktopConfig` (separate from existing events pipe)
- Direction: `PipeDirection.In` (receive only)
- Listen for JSON configuration messages
- Event: `ConfigurationReceived`

**New file: `text-monitor/TextMonitor.Service/Models/ConfigurationMessage.cs`**
```csharp
public class ConfigurationMessage {
    public string Type { get; set; }  // "update_selection_modifier"
    public JsonElement? Payload { get; set; }
}
```

**Modify: `text-monitor/TextMonitor.Service/Monitoring/TextSelectionMonitor.cs`**
- Add `VK_MENU = 0x12` constant for Alt key
- Add field `_activeModifierKey` (default: `VK_MENU`)
- Add modifier map: `{ "ctrl": VK_CONTROL, "shift": VK_SHIFT, "alt": VK_MENU }`
- Add method `SetSelectionModifier(string modifier)` - thread-safe update
- Modify `IsModifierKeyPressed()` to use `_activeModifierKey` instead of hardcoded values

**Modify: `text-monitor/TextMonitor.Service/Program.cs`**
- Register `ConfigurationReceiver` in DI
- Wire configuration events to `TextSelectionMonitor.SetSelectionModifier()`

#### 2.2 Rust Side - Configuration Sender

**New file: `src-tauri/src/ipc/config_pipe.rs`**
- Named pipe client connecting to `\\.\pipe\TranslatorDesktopConfig`
- Method `send_config(ConfigMessage)` - connect, write JSON, disconnect
- Retry logic: 3 attempts with exponential backoff

**Modify: `src-tauri/src/ipc/mod.rs`**
- Export `ConfigSender`, `ConfigMessage`

---

### Phase 3: Dynamic Global Hotkey Re-registration (Tauri)

**Modify: `src-tauri/src/commands/settings.rs`**
- Add `HotkeyState(Arc<Mutex<Shortcut>>)` struct
- Add command `update_global_hotkey(modifier: String)`:
  1. Parse modifier string → `Modifiers` enum
  2. Unregister old shortcut via `app.global_shortcut().unregister()`
  3. Register new shortcut via `app.global_shortcut().register()`
  4. Update `HotkeyState`

- Add helper `parse_modifiers(str) -> Modifiers`:
  - `"ctrl+shift"` → `CONTROL | SHIFT`
  - `"ctrl+alt"` → `CONTROL | ALT`
  - `"alt+shift"` → `ALT | SHIFT`

**Modify: `src-tauri/src/lib.rs`**
- Manage `HotkeyState` in app setup
- Load `hotkey_modifier` from settings on startup
- Register initial shortcut based on saved setting (not hardcoded)
- Add `update_global_hotkey` to invoke handler

---

### Phase 4: Startup Configuration Sync

**Modify: `src-tauri/src/lib.rs`** (after sidecar starts, ~line 144)
- Wait 1s for .NET to initialize config receiver
- Load `selection_modifier` from settings
- Send initial configuration to .NET via `ConfigSender`

---

### Phase 5: Real-Time Settings Propagation

**New command: `src-tauri/src/commands/settings.rs`**
```rust
#[tauri::command]
pub async fn update_selection_modifier(modifier: String) -> Result<(), String>
```
- Sends config message to .NET via `ConfigSender`

**Modify: `src/contexts/SettingsContext.tsx`** (`updateSetting` function)
- After saving `selectionModifier` → invoke `update_selection_modifier`
- After saving `hotkeyModifier` → invoke `update_global_hotkey`

**Modify: `src-tauri/src/commands/mod.rs`**
- Export new commands: `update_selection_modifier`, `update_global_hotkey`

**Modify: `src-tauri/src/lib.rs`**
- Add new commands to invoke handler

---

## Files to Modify

| File | Changes |
|------|---------|
| `src/contexts/SettingsContext.tsx` | Change default, add invoke calls |
| `text-monitor/.../TextSelectionMonitor.cs` | Add VK_MENU, configurable modifier |
| `text-monitor/.../Program.cs` | Register ConfigurationReceiver |
| `src-tauri/src/lib.rs` | Dynamic hotkey, startup sync |
| `src-tauri/src/commands/settings.rs` | New commands, HotkeyState |
| `src-tauri/src/commands/mod.rs` | Export new commands |
| `src-tauri/src/ipc/mod.rs` | Export ConfigSender |

## New Files to Create

| File | Purpose |
|------|---------|
| `text-monitor/.../Ipc/ConfigurationReceiver.cs` | Receive config from Tauri |
| `text-monitor/.../Models/ConfigurationMessage.cs` | Config message model |
| `src-tauri/src/ipc/config_pipe.rs` | Send config to .NET |

---

## Verification

1. **Selection modifier change**:
   - Open Settings → Shortcuts → Change "Text selection modifier" to Ctrl
   - In any app, Ctrl+drag to select text → popup appears
   - Change to Shift → Shift+drag works, Ctrl+drag doesn't
   - Change to Alt → Alt+drag works

2. **Global hotkey change**:
   - Open Settings → Shortcuts → Change "Global hotkey modifier" to Ctrl+Alt
   - Press Ctrl+Alt+Q → popup appears with clipboard content
   - Verify old hotkey (Ctrl+Shift+Q) no longer works

3. **Persistence**:
   - Close and reopen app
   - Verify saved modifiers are applied immediately
   - Both .NET and Tauri should use saved values

4. **Build verification**:
   - `npm run build:sidecar` - .NET builds successfully
   - `npm run tauri dev` - App runs without errors
