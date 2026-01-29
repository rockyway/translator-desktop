import {
  createContext,
  useCallback,
  useEffect,
  useState,
  useRef,
  ReactNode,
} from 'react';
import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// Types
// ============================================================================

export type Theme = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';
/** Modifier key for text selection trigger (Ctrl+select, Shift+select, etc.) */
export type SelectionModifier = 'ctrl' | 'shift' | 'alt';
/** Modifier key combination for global hotkey (Ctrl+Shift+Q, etc.) */
export type HotkeyModifier = 'ctrl+shift' | 'ctrl+alt' | 'alt+shift';
/** UI density preset affecting font size */
export type DensityPreset = 'default' | 'large' | 'xlarge' | 'custom';

export interface AppSettings {
  theme: Theme;
  sourceLanguage: string; // 'auto' or language code
  targetLanguage: string; // language code
  sidebarCollapsed: boolean;
  selectionModifier: SelectionModifier; // Modifier key for text selection trigger
  hotkeyModifier: HotkeyModifier; // Modifier key combination for global hotkey
  hotkeyLetter: string; // Single letter (a-z) for global hotkey
  minimizeToTray: boolean; // Close button minimizes to system tray instead of exiting
  density: DensityPreset; // UI density preset
  customDensity: number; // Custom density percentage (used when density is 'custom')
  confirmationCharLimit: number; // Character limit for showing confirmation dialog (0 = disabled)
}

export interface SettingsContextValue {
  settings: AppSettings;
  isLoading: boolean;
  resolvedTheme: ResolvedTheme;
  densityPercentage: number;
  hotkeyError: string | null;
  updateSetting: <K extends keyof AppSettings>(
    key: K,
    value: AppSettings[K]
  ) => Promise<void>;
  updateSettings: (updates: Partial<AppSettings>) => Promise<void>;
}

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_SETTINGS: AppSettings = {
  theme: 'system',
  sourceLanguage: 'auto',
  targetLanguage: 'es',
  sidebarCollapsed: false,
  selectionModifier: 'alt',
  hotkeyModifier: 'ctrl+shift',
  hotkeyLetter: 'q',
  minimizeToTray: true, // Default to minimize to tray on close
  density: 'default',
  customDensity: 100, // 100% by default
  confirmationCharLimit: 100, // Default 100 characters
};

/** Maps density presets to font-size percentages */
export const DENSITY_VALUES: Record<Exclude<DensityPreset, 'custom'>, number> = {
  default: 100,
  large: 110,
  xlarge: 125,
};

/** Get the effective density percentage based on settings */
export function getDensityPercentage(settings: AppSettings): number {
  if (settings.density === 'custom') {
    return settings.customDensity;
  }
  return DENSITY_VALUES[settings.density];
}

const DEBOUNCE_DELAY = 300;

// Key mapping from camelCase (frontend) to snake_case (backend)
const KEY_TO_BACKEND: Record<keyof AppSettings, string> = {
  theme: 'theme',
  sourceLanguage: 'source_language',
  targetLanguage: 'target_language',
  sidebarCollapsed: 'sidebar_collapsed',
  selectionModifier: 'selection_modifier',
  hotkeyModifier: 'hotkey_modifier',
  hotkeyLetter: 'hotkey_letter',
  minimizeToTray: 'minimize_to_tray',
  density: 'density',
  customDensity: 'custom_density',
  confirmationCharLimit: 'confirmation_char_limit',
};

const BACKEND_TO_KEY: Record<string, keyof AppSettings> = {
  theme: 'theme',
  source_language: 'sourceLanguage',
  target_language: 'targetLanguage',
  sidebar_collapsed: 'sidebarCollapsed',
  selection_modifier: 'selectionModifier',
  hotkey_modifier: 'hotkeyModifier',
  hotkey_letter: 'hotkeyLetter',
  minimize_to_tray: 'minimizeToTray',
  density: 'density',
  custom_density: 'customDensity',
  confirmation_char_limit: 'confirmationCharLimit',
};

// ============================================================================
// Context
// ============================================================================

export const SettingsContext = createContext<SettingsContextValue | null>(null);

// ============================================================================
// Helper Functions
// ============================================================================

function getSystemTheme(): ResolvedTheme {
  if (typeof window === 'undefined') return 'light';
  return window.matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light';
}

function resolveTheme(theme: Theme): ResolvedTheme {
  if (theme === 'system') {
    return getSystemTheme();
  }
  return theme;
}

function applyThemeToDocument(resolved: ResolvedTheme): void {
  const root = document.documentElement;
  if (resolved === 'dark') {
    root.classList.add('dark');
  } else {
    root.classList.remove('dark');
  }
}

function applyDensityToDocument(percentage: number): void {
  const root = document.documentElement;
  root.style.setProperty('--density-scale', `${percentage / 100}`);
  root.style.fontSize = `${percentage}%`;
}

/**
 * Parse settings from backend format to frontend format
 */
function parseBackendSettings(
  backendSettings: Record<string, unknown>
): Partial<AppSettings> {
  const parsed: Partial<AppSettings> = {};

  for (const [backendKey, value] of Object.entries(backendSettings)) {
    const frontendKey = BACKEND_TO_KEY[backendKey];
    if (frontendKey && value !== undefined && value !== null) {
      // Type-safe assignment based on key
      if (frontendKey === 'theme') {
        const themeValue = value as string;
        if (
          themeValue === 'light' ||
          themeValue === 'dark' ||
          themeValue === 'system'
        ) {
          parsed.theme = themeValue;
        }
      } else if (frontendKey === 'sidebarCollapsed') {
        parsed.sidebarCollapsed = Boolean(value);
      } else if (frontendKey === 'minimizeToTray') {
        parsed.minimizeToTray = Boolean(value);
      } else if (frontendKey === 'selectionModifier') {
        const modifierValue = value as string;
        if (
          modifierValue === 'ctrl' ||
          modifierValue === 'shift' ||
          modifierValue === 'alt'
        ) {
          parsed.selectionModifier = modifierValue;
        }
      } else if (frontendKey === 'hotkeyModifier') {
        const modifierValue = value as string;
        if (
          modifierValue === 'ctrl+shift' ||
          modifierValue === 'ctrl+alt' ||
          modifierValue === 'alt+shift'
        ) {
          parsed.hotkeyModifier = modifierValue;
        }
      } else if (
        frontendKey === 'sourceLanguage' ||
        frontendKey === 'targetLanguage'
      ) {
        parsed[frontendKey] = String(value);
      } else if (frontendKey === 'density') {
        const densityValue = value as string;
        if (
          densityValue === 'default' ||
          densityValue === 'large' ||
          densityValue === 'xlarge' ||
          densityValue === 'custom'
        ) {
          parsed.density = densityValue;
        }
      } else if (frontendKey === 'customDensity') {
        const numValue = Number(value);
        if (!isNaN(numValue) && numValue >= 75 && numValue <= 200) {
          parsed.customDensity = numValue;
        }
      } else if (frontendKey === 'confirmationCharLimit') {
        const numValue = Number(value);
        if (!isNaN(numValue) && numValue >= 0) {
          parsed.confirmationCharLimit = numValue;
        }
      } else if (frontendKey === 'hotkeyLetter') {
        const letterValue = String(value).toLowerCase();
        if (/^[a-z]$/.test(letterValue)) {
          parsed.hotkeyLetter = letterValue;
        }
      }
    }
  }

  return parsed;
}

// ============================================================================
// Provider Component
// ============================================================================

interface SettingsProviderProps {
  children: ReactNode;
}

export function SettingsProvider({ children }: SettingsProviderProps) {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [isLoading, setIsLoading] = useState(true);
  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>(() =>
    resolveTheme(DEFAULT_SETTINGS.theme)
  );
  const [hotkeyError, setHotkeyError] = useState<string | null>(null);

  // Track pending saves for debouncing
  const pendingSavesRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(
    new Map()
  );

  // Track if initial load has completed
  const initialLoadComplete = useRef(false);

  // Load settings from Tauri on mount
  useEffect(() => {
    async function loadSettings() {
      try {
        const backendSettings = await invoke<Record<string, unknown>>(
          'get_all_settings'
        );
        const parsed = parseBackendSettings(backendSettings);
        const mergedSettings = { ...DEFAULT_SETTINGS, ...parsed };

        setSettings(mergedSettings);

        // Apply theme immediately
        const resolved = resolveTheme(mergedSettings.theme);
        setResolvedTheme(resolved);
        applyThemeToDocument(resolved);

        // Apply density immediately
        const densityPercent = getDensityPercentage(mergedSettings);
        applyDensityToDocument(densityPercent);

        // Save any missing defaults to database so popup can read them
        const missingKeys: (keyof AppSettings)[] = [];
        for (const key of Object.keys(DEFAULT_SETTINGS) as (keyof AppSettings)[]) {
          const backendKey = KEY_TO_BACKEND[key];
          if (!(backendKey in backendSettings)) {
            missingKeys.push(key);
          }
        }

        // Save missing defaults without debounce (immediate)
        for (const key of missingKeys) {
          const backendKey = KEY_TO_BACKEND[key];
          const value = DEFAULT_SETTINGS[key];
          try {
            await invoke('set_setting', { key: backendKey, value });
          } catch (saveError) {
            console.error(`Failed to save default for ${backendKey}:`, saveError);
          }
        }

        initialLoadComplete.current = true;
      } catch (error) {
        console.error('Failed to load settings from Tauri:', error);
        // Use defaults on error
        const resolved = resolveTheme(DEFAULT_SETTINGS.theme);
        setResolvedTheme(resolved);
        applyThemeToDocument(resolved);
        applyDensityToDocument(getDensityPercentage(DEFAULT_SETTINGS));
        initialLoadComplete.current = true;
      } finally {
        setIsLoading(false);
      }
    }

    loadSettings();

    // Cleanup pending saves on unmount
    return () => {
      pendingSavesRef.current.forEach((timeoutId) => clearTimeout(timeoutId));
      pendingSavesRef.current.clear();
    };
  }, []);

  // Listen for system theme changes when theme is 'system'
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    const handleChange = () => {
      if (settings.theme === 'system') {
        const resolved = getSystemTheme();
        setResolvedTheme(resolved);
        applyThemeToDocument(resolved);
      }
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => {
      mediaQuery.removeEventListener('change', handleChange);
    };
  }, [settings.theme]);

  /**
   * Save a setting to Tauri backend with debouncing
   */
  const saveSettingToBackend = useCallback(
    (key: keyof AppSettings, value: AppSettings[keyof AppSettings]) => {
      const backendKey = KEY_TO_BACKEND[key];

      // Clear any pending save for this key
      const existingTimeout = pendingSavesRef.current.get(backendKey);
      if (existingTimeout) {
        clearTimeout(existingTimeout);
      }

      // Schedule debounced save
      const timeoutId = setTimeout(async () => {
        try {
          await invoke('set_setting', { key: backendKey, value });
          pendingSavesRef.current.delete(backendKey);
        } catch (error) {
          console.error(`Failed to save setting ${backendKey}:`, error);
        }
      }, DEBOUNCE_DELAY);

      pendingSavesRef.current.set(backendKey, timeoutId);
    },
    []
  );

  /**
   * Update a single setting
   */
  const updateSetting = useCallback(
    async <K extends keyof AppSettings>(
      key: K,
      value: AppSettings[K]
    ): Promise<void> => {
      // Update local state immediately for responsiveness
      setSettings((prev) => ({ ...prev, [key]: value }));

      // Handle theme changes immediately
      if (key === 'theme') {
        const resolved = resolveTheme(value as Theme);
        setResolvedTheme(resolved);
        applyThemeToDocument(resolved);
      }

      // Handle density changes immediately
      if (key === 'density' || key === 'customDensity') {
        // For density changes, we need to get the new settings object to calculate
        const newSettings = { ...settings, [key]: value };
        const newDensity = getDensityPercentage(newSettings as AppSettings);
        applyDensityToDocument(newDensity);
      }

      // Save to backend (debounced)
      saveSettingToBackend(key, value);

      // Propagate modifier changes to respective components (not debounced)
      if (key === 'selectionModifier') {
        try {
          await invoke('update_selection_modifier', { modifier: value });
        } catch (error) {
          console.error('Failed to update selection modifier:', error);
        }
      } else if (key === 'hotkeyModifier' || key === 'hotkeyLetter') {
        try {
          const modifier =
            key === 'hotkeyModifier' ? (value as string) : settings.hotkeyModifier;
          const letter =
            key === 'hotkeyLetter' ? (value as string) : settings.hotkeyLetter;
          await invoke('update_global_hotkey', { modifier, letter });
          setHotkeyError(null); // Clear error on success
        } catch (error) {
          const errorMsg = String(error);
          console.error('Failed to update global hotkey:', errorMsg);
          setHotkeyError(errorMsg);

          // Revert to previous value
          setSettings(prev => ({ ...prev, [key]: prev[key] }));
        }
      }
    },
    [saveSettingToBackend, settings]
  );

  /**
   * Update multiple settings at once
   */
  const updateSettings = useCallback(
    async (updates: Partial<AppSettings>): Promise<void> => {
      // Update local state immediately
      setSettings((prev) => ({ ...prev, ...updates }));

      // Handle theme changes immediately
      if (updates.theme !== undefined) {
        const resolved = resolveTheme(updates.theme);
        setResolvedTheme(resolved);
        applyThemeToDocument(resolved);
      }

      // Handle density changes immediately
      if (updates.density !== undefined || updates.customDensity !== undefined) {
        const newSettings = { ...settings, ...updates };
        const newDensity = getDensityPercentage(newSettings as AppSettings);
        applyDensityToDocument(newDensity);
      }

      // Save each setting to backend (debounced)
      for (const [key, value] of Object.entries(updates)) {
        if (value !== undefined) {
          saveSettingToBackend(
            key as keyof AppSettings,
            value as AppSettings[keyof AppSettings]
          );
        }
      }
    },
    [saveSettingToBackend, settings]
  );

  const contextValue: SettingsContextValue = {
    settings,
    isLoading,
    resolvedTheme,
    densityPercentage: getDensityPercentage(settings),
    hotkeyError,
    updateSetting,
    updateSettings,
  };

  return (
    <SettingsContext.Provider value={contextValue}>
      {children}
    </SettingsContext.Provider>
  );
}
