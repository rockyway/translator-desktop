# UI Redesign: Sidebar Layout + Settings Persistence

## Overview
Redesign the translator desktop app with a collapsible sidebar navigation and persistent settings storage.

## Current State Analysis
- Header takes ~150px height (gradient + tabs)
- Languages reset on component unmount
- Popup has own localStorage for target language
- No shared settings between main window and popup

## Design Direction
**Aesthetic**: Industrial/utilitarian desktop app with refined minimalism
- Dark-first design with subtle gradients
- Collapsible left sidebar (icon-only when collapsed)
- Maximum content area utilization
- Sharp, functional typography

---

## Implementation Plan

### Phase 1: Backend - Settings Persistence
**Location**: `src-tauri/src/`

1. **Create ConfigStore table migration**
   - Table: `config_store`
   - Columns: `key TEXT PRIMARY KEY`, `value JSONB NOT NULL`, `updated_at TIMESTAMP`

2. **Add Tauri commands**
   ```rust
   // commands/settings.rs
   get_setting(key: String) -> Option<JsonValue>
   set_setting(key: String, value: JsonValue) -> Result<()>
   get_all_settings() -> HashMap<String, JsonValue>
   ```

3. **Settings keys**
   - `theme`: "light" | "dark" | "system"
   - `source_language`: language code or "auto"
   - `target_language`: language code
   - `sidebar_collapsed`: boolean

### Phase 2: Frontend - Layout Restructure
**Location**: `src/`

1. **New Layout Architecture**
   ```
   App
   ├── Sidebar (collapsible)
   │   ├── Logo/Brand (collapsed: icon only)
   │   ├── Nav Items (collapsed: icons only)
   │   │   ├── Translate
   │   │   ├── History
   │   │   └── Settings
   │   ├── Connection Status
   │   └── Collapse Toggle
   └── Main Content Area (full remaining space)
       └── Active Panel
   ```

2. **New Components**
   - `components/Sidebar.tsx` - Collapsible navigation
   - `components/NavItem.tsx` - Icon + label button
   - `features/settings/SettingsPanel.tsx` - Settings page

3. **Settings Context**
   - `contexts/SettingsContext.tsx`
   - Loads from Tauri on mount
   - Provides: theme, sourceLanguage, targetLanguage, sidebarCollapsed
   - Auto-saves on change

### Phase 3: Settings Page UI
**Location**: `src/features/settings/`

1. **SettingsPanel.tsx**
   - Theme selector (Light/Dark/System)
   - Default source language dropdown
   - Default target language dropdown
   - Future: hotkey customization, startup options

### Phase 4: Integration
1. **TranslationPanel** - Use settings context for initial values
2. **PopupOverlay** - Use same Tauri commands for language settings
3. **Remove localStorage** usage for languages (use Tauri SQLite)

---

## Component Specifications

### Sidebar Component
```
Width expanded: 200px
Width collapsed: 56px
Trigger: Button at bottom OR responsive breakpoint
Icons: React Icons (Fi* set for consistency)
Active state: Gold accent (amber-500)
```

### Responsive Behavior
```
< 640px: Sidebar collapsed by default, icon-only buttons
≥ 640px: User preference for sidebar state
```

### Color Palette (existing)
- Primary: amber-500/600 (gold)
- Accent: blue-500/600
- Dark bg: gray-900/800
- Light bg: gray-50/white

---

## File Changes Summary

### New Files
- `src-tauri/src/commands/settings.rs`
- `src/components/Sidebar.tsx`
- `src/components/NavItem.tsx`
- `src/features/settings/SettingsPanel.tsx`
- `src/features/settings/index.ts`
- `src/contexts/SettingsContext.tsx`
- `src/hooks/useSettings.ts`

### Modified Files
- `src-tauri/src/lib.rs` - Register settings commands
- `src-tauri/src/commands/mod.rs` - Export settings module
- `src-tauri/src/db.rs` - Add config_store table
- `src/App.tsx` - New sidebar layout
- `src/index.css` - Typography + sidebar transitions
- `src/features/translator/TranslationPanel.tsx` - Use settings context
- `src/features/popup/PopupOverlay.tsx` - Use Tauri settings

---

## Delegation Plan

| Task | Agent Type | Dependencies |
|------|------------|--------------|
| Backend: ConfigStore + commands | Implementer | None |
| Frontend: SettingsContext | Implementer | Backend done |
| Frontend: Sidebar component | Implementer | None |
| Frontend: App.tsx layout | Implementer | Sidebar done |
| Frontend: SettingsPanel | Implementer | SettingsContext done |
| Frontend: TranslationPanel integration | Implementer | SettingsContext done |
| Frontend: PopupOverlay integration | Implementer | Backend done |
| Build verification | QA Agent | All done |

---

## Success Criteria
1. Sidebar collapses/expands smoothly
2. Language settings persist across app restart
3. Popup uses same language settings as main window
4. Theme persists (already works, migrate to Tauri)
5. No header taking vertical space
6. Icon-only buttons when sidebar collapsed
