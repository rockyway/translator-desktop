import { useState, useCallback } from 'react';
import { FiCheck, FiMoon, FiSun, FiMonitor, FiType } from 'react-icons/fi';
import { useSettings } from '../../hooks/useSettings';
import type { Theme, SelectionModifier, HotkeyModifier, DensityPreset } from '../../contexts/SettingsContext';
import { DENSITY_VALUES } from '../../contexts/SettingsContext';

// ============================================================================
// Types
// ============================================================================

interface ThemeOption {
  value: Theme;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  description: string;
}

interface SelectionModifierOption {
  value: SelectionModifier;
  label: string;
  description: string;
}

interface HotkeyModifierOption {
  value: HotkeyModifier;
  label: string;
  shortcut: string;
  description: string;
}

interface DensityOption {
  value: DensityPreset;
  label: string;
  percentage: number | null; // null for custom
  description: string;
}

// ============================================================================
// Constants
// ============================================================================

const THEME_OPTIONS: ThemeOption[] = [
  {
    value: 'light',
    label: 'Light',
    icon: FiSun,
    description: 'Always use light theme',
  },
  {
    value: 'dark',
    label: 'Dark',
    icon: FiMoon,
    description: 'Always use dark theme',
  },
  {
    value: 'system',
    label: 'System',
    icon: FiMonitor,
    description: 'Follow system preference',
  },
];

const SELECTION_MODIFIER_OPTIONS: SelectionModifierOption[] = [
  {
    value: 'ctrl',
    label: 'Ctrl',
    description: 'Hold Ctrl while selecting text',
  },
  {
    value: 'shift',
    label: 'Shift',
    description: 'Hold Shift while selecting text',
  },
  {
    value: 'alt',
    label: 'Alt',
    description: 'Hold Alt while selecting text',
  },
];

const HOTKEY_MODIFIER_OPTIONS: HotkeyModifierOption[] = [
  {
    value: 'ctrl+shift',
    label: 'Ctrl+Shift',
    shortcut: 'Ctrl+Shift+Q',
    description: 'Default modifier combination',
  },
  {
    value: 'ctrl+alt',
    label: 'Ctrl+Alt',
    shortcut: 'Ctrl+Alt+Q',
    description: 'Alternative using Alt key',
  },
  {
    value: 'alt+shift',
    label: 'Alt+Shift',
    shortcut: 'Alt+Shift+Q',
    description: 'Without Ctrl key',
  },
];

const DENSITY_OPTIONS: DensityOption[] = [
  {
    value: 'default',
    label: 'Default',
    percentage: DENSITY_VALUES.default,
    description: 'Standard text size (100%)',
  },
  {
    value: 'large',
    label: 'Large',
    percentage: DENSITY_VALUES.large,
    description: 'Comfortable reading (110%)',
  },
  {
    value: 'xlarge',
    label: 'X-Large',
    percentage: DENSITY_VALUES.xlarge,
    description: 'Maximum legibility (125%)',
  },
  {
    value: 'custom',
    label: 'Custom',
    percentage: null,
    description: 'Set your own size',
  },
];

const SAVED_INDICATOR_DURATION = 1500;

// ============================================================================
// Sub-Components
// ============================================================================

interface SettingSectionProps {
  title: string;
  children: React.ReactNode;
}

/**
 * A styled section container for grouping related settings
 */
function SettingSection({ title, children }: SettingSectionProps) {
  return (
    <section className="bg-white dark:bg-gray-800/50 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
      <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-5">
        {title}
      </h2>
      <div className="space-y-5">{children}</div>
    </section>
  );
}

interface SettingRowProps {
  label: string;
  description?: string;
  children: React.ReactNode;
}

/**
 * A row within a settings section with label on left and control on right
 */
function SettingRow({ label, description, children }: SettingRowProps) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div className="flex-1 min-w-0">
        <span className="text-sm font-medium text-gray-700 dark:text-gray-200">
          {label}
        </span>
        {description && (
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
            {description}
          </p>
        )}
      </div>
      <div className="flex-shrink-0">{children}</div>
    </div>
  );
}

interface ThemeSelectorProps {
  value: Theme;
  onChange: (theme: Theme) => void;
}

/**
 * Theme selector with styled radio button cards
 */
function ThemeSelector({ value, onChange }: ThemeSelectorProps) {
  return (
    <div className="flex gap-2">
      {THEME_OPTIONS.map((option) => {
        const Icon = option.icon;
        const isSelected = value === option.value;

        return (
          <button
            key={option.value}
            type="button"
            onClick={() => onChange(option.value)}
            aria-pressed={isSelected}
            aria-label={`${option.label}: ${option.description}`}
            className={`
              flex items-center gap-2 px-4 py-2.5 rounded-lg
              transition-all duration-150 font-medium text-sm
              focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2
              dark:focus:ring-offset-gray-800
              ${
                isSelected
                  ? 'bg-amber-500 text-white shadow-md'
                  : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'
              }
            `}
          >
            <Icon
              className={`w-4 h-4 ${isSelected ? 'text-white' : 'text-gray-500 dark:text-gray-400'}`}
              aria-hidden="true"
            />
            <span>{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}

interface ToggleSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
}

/**
 * Styled toggle switch component
 */
function ToggleSwitch({ checked, onChange, label }: ToggleSwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={`
        relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer
        rounded-full border-2 border-transparent
        transition-colors duration-200 ease-in-out
        focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2
        dark:focus:ring-offset-gray-800
        ${checked ? 'bg-amber-500' : 'bg-gray-300 dark:bg-gray-600'}
      `}
    >
      <span
        aria-hidden="true"
        className={`
          pointer-events-none inline-block h-5 w-5 transform
          rounded-full bg-white shadow ring-0
          transition duration-200 ease-in-out
          ${checked ? 'translate-x-5' : 'translate-x-0'}
        `}
      />
    </button>
  );
}

interface SelectionModifierSelectorProps {
  value: SelectionModifier;
  onChange: (modifier: SelectionModifier) => void;
}

/**
 * Selection modifier selector (Ctrl/Shift/Alt for text selection trigger)
 */
function SelectionModifierSelector({ value, onChange }: SelectionModifierSelectorProps) {
  return (
    <div className="flex gap-2">
      {SELECTION_MODIFIER_OPTIONS.map((option) => {
        const isSelected = value === option.value;

        return (
          <button
            key={option.value}
            type="button"
            onClick={() => onChange(option.value)}
            aria-pressed={isSelected}
            aria-label={`${option.label}: ${option.description}`}
            className={`
              flex items-center px-4 py-2.5 rounded-lg
              transition-all duration-150 font-medium text-sm
              focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2
              dark:focus:ring-offset-gray-800
              ${
                isSelected
                  ? 'bg-amber-500 text-white shadow-md'
                  : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'
              }
            `}
          >
            <span>{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}

interface HotkeyModifierSelectorProps {
  value: HotkeyModifier;
  onChange: (modifier: HotkeyModifier) => void;
}

/**
 * Hotkey modifier selector (Ctrl+Shift/Ctrl+Alt/Alt+Shift for global hotkey)
 */
function HotkeyModifierSelector({ value, onChange }: HotkeyModifierSelectorProps) {
  return (
    <div className="flex gap-2">
      {HOTKEY_MODIFIER_OPTIONS.map((option) => {
        const isSelected = value === option.value;

        return (
          <button
            key={option.value}
            type="button"
            onClick={() => onChange(option.value)}
            aria-pressed={isSelected}
            aria-label={`${option.label}: ${option.shortcut}`}
            className={`
              flex items-center px-4 py-2.5 rounded-lg
              transition-all duration-150 font-medium text-sm
              focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2
              dark:focus:ring-offset-gray-800
              ${
                isSelected
                  ? 'bg-amber-500 text-white shadow-md'
                  : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'
              }
            `}
          >
            <span>{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}

interface DensitySelectorProps {
  value: DensityPreset;
  customValue: number;
  onChange: (preset: DensityPreset) => void;
  onCustomChange: (value: number) => void;
}

/**
 * Density selector with preset options and custom slider
 */
function DensitySelector({
  value,
  customValue,
  onChange,
  onCustomChange,
}: DensitySelectorProps) {
  return (
    <div className="space-y-3">
      {/* Preset buttons */}
      <div className="flex flex-wrap gap-2">
        {DENSITY_OPTIONS.map((option) => {
          const isSelected = value === option.value;

          return (
            <button
              key={option.value}
              type="button"
              onClick={() => onChange(option.value)}
              aria-pressed={isSelected}
              aria-label={option.description}
              className={`
                flex items-center gap-2 px-4 py-2.5 rounded-lg
                transition-all duration-150 font-medium text-sm
                focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2
                dark:focus:ring-offset-gray-800
                ${
                  isSelected
                    ? 'bg-amber-500 text-white shadow-md'
                    : 'bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'
                }
              `}
            >
              {option.value === 'custom' && (
                <FiType className="w-4 h-4" aria-hidden="true" />
              )}
              <span>{option.label}</span>
              {option.percentage && (
                <span className={`text-xs ${isSelected ? 'text-amber-100' : 'text-gray-400 dark:text-gray-500'}`}>
                  {option.percentage}%
                </span>
              )}
            </button>
          );
        })}
      </div>

      {/* Custom value slider - only show when custom is selected */}
      {value === 'custom' && (
        <div className="pt-2 space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-600 dark:text-gray-400">
              Custom size
            </span>
            <span className="text-sm font-semibold text-amber-600 dark:text-amber-400 tabular-nums">
              {customValue}%
            </span>
          </div>
          <div className="relative">
            <input
              type="range"
              min="75"
              max="200"
              step="5"
              value={customValue}
              onChange={(e) => onCustomChange(Number(e.target.value))}
              className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-lg appearance-none cursor-pointer
                [&::-webkit-slider-thumb]:appearance-none
                [&::-webkit-slider-thumb]:w-5
                [&::-webkit-slider-thumb]:h-5
                [&::-webkit-slider-thumb]:rounded-full
                [&::-webkit-slider-thumb]:bg-amber-500
                [&::-webkit-slider-thumb]:shadow-md
                [&::-webkit-slider-thumb]:cursor-pointer
                [&::-webkit-slider-thumb]:transition-transform
                [&::-webkit-slider-thumb]:hover:scale-110
                [&::-moz-range-thumb]:w-5
                [&::-moz-range-thumb]:h-5
                [&::-moz-range-thumb]:rounded-full
                [&::-moz-range-thumb]:bg-amber-500
                [&::-moz-range-thumb]:border-0
                [&::-moz-range-thumb]:shadow-md
                [&::-moz-range-thumb]:cursor-pointer"
              aria-label="Custom density percentage"
            />
            {/* Scale markers */}
            <div className="flex justify-between text-xs text-gray-400 dark:text-gray-500 mt-1 px-0.5">
              <span>75%</span>
              <span>125%</span>
              <span>200%</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

interface SavedIndicatorProps {
  visible: boolean;
}

/**
 * Subtle "Saved" indicator that appears after settings change
 */
function SavedIndicator({ visible }: SavedIndicatorProps) {
  return (
    <div
      className={`
        fixed bottom-6 right-6 flex items-center gap-2
        bg-gray-900 dark:bg-gray-100 text-white dark:text-gray-900
        px-4 py-2 rounded-lg shadow-lg
        transition-all duration-300
        ${visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-2 pointer-events-none'}
      `}
      role="status"
      aria-live="polite"
    >
      <FiCheck className="w-4 h-4 text-green-400 dark:text-green-600" aria-hidden="true" />
      <span className="text-sm font-medium">Saved</span>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

interface SettingsPanelProps {
  className?: string;
}

/**
 * Settings page component with auto-saving configuration options.
 *
 * Sections:
 * - Appearance: Theme selection (Light/Dark/System)
 * - Interface: Sidebar collapse preference
 *
 * Note: Language settings are managed directly from the Translate panel
 * and shared with the popup window automatically.
 *
 * All settings auto-save with a subtle "Saved" indicator.
 */
export function SettingsPanel({ className = '' }: SettingsPanelProps) {
  const { settings, updateSetting, isLoading } = useSettings();
  const [showSaved, setShowSaved] = useState(false);

  /**
   * Show the saved indicator briefly after any setting change
   */
  const flashSavedIndicator = useCallback(() => {
    setShowSaved(true);
    const timer = setTimeout(() => {
      setShowSaved(false);
    }, SAVED_INDICATOR_DURATION);
    return () => clearTimeout(timer);
  }, []);

  // Handlers that update settings and show the saved indicator
  const handleThemeChange = useCallback(
    (theme: Theme) => {
      updateSetting('theme', theme);
      flashSavedIndicator();
    },
    [updateSetting, flashSavedIndicator]
  );

  const handleSidebarCollapsedChange = useCallback(
    (collapsed: boolean) => {
      updateSetting('sidebarCollapsed', collapsed);
      flashSavedIndicator();
    },
    [updateSetting, flashSavedIndicator]
  );

  const handleMinimizeToTrayChange = useCallback(
    (minimizeToTray: boolean) => {
      updateSetting('minimizeToTray', minimizeToTray);
      flashSavedIndicator();
    },
    [updateSetting, flashSavedIndicator]
  );

  const handleSelectionModifierChange = useCallback(
    (modifier: SelectionModifier) => {
      updateSetting('selectionModifier', modifier);
      flashSavedIndicator();
    },
    [updateSetting, flashSavedIndicator]
  );

  const handleHotkeyModifierChange = useCallback(
    (modifier: HotkeyModifier) => {
      updateSetting('hotkeyModifier', modifier);
      flashSavedIndicator();
    },
    [updateSetting, flashSavedIndicator]
  );

  const handleDensityChange = useCallback(
    (preset: DensityPreset) => {
      updateSetting('density', preset);
      flashSavedIndicator();
    },
    [updateSetting, flashSavedIndicator]
  );

  const handleCustomDensityChange = useCallback(
    (value: number) => {
      updateSetting('customDensity', value);
      flashSavedIndicator();
    },
    [updateSetting, flashSavedIndicator]
  );

  // Loading state
  if (isLoading) {
    return (
      <div className={`flex items-center justify-center min-h-[400px] ${className}`}>
        <div className="flex items-center gap-3 text-gray-500 dark:text-gray-400">
          <svg
            className="animate-spin w-5 h-5"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            />
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            />
          </svg>
          <span className="text-sm font-medium">Loading settings...</span>
        </div>
      </div>
    );
  }

  return (
    <div className={`max-w-2xl mx-auto space-y-6 ${className}`}>
      {/* Page Header */}
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Settings
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Configure your translation preferences
        </p>
      </div>

      {/* Appearance Section */}
      <SettingSection title="Appearance">
        <SettingRow
          label="Theme"
          description="Choose how the app looks"
        >
          <ThemeSelector
            value={settings.theme}
            onChange={handleThemeChange}
          />
        </SettingRow>
        <div className="pt-2">
          <div className="flex-1 min-w-0 mb-3">
            <span className="text-sm font-medium text-gray-700 dark:text-gray-200">
              Density
            </span>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
              Adjust text size across the entire app
            </p>
          </div>
          <DensitySelector
            value={settings.density}
            customValue={settings.customDensity}
            onChange={handleDensityChange}
            onCustomChange={handleCustomDensityChange}
          />
        </div>
      </SettingSection>

      {/* Interface Section */}
      <SettingSection title="Interface">
        <SettingRow
          label="Collapse sidebar by default"
          description="Start with the sidebar in icon-only mode"
        >
          <ToggleSwitch
            checked={settings.sidebarCollapsed}
            onChange={handleSidebarCollapsedChange}
            label="Collapse sidebar by default"
          />
        </SettingRow>
        <SettingRow
          label="Minimize to system tray on close"
          description="Closing the window minimizes to tray instead of exiting"
        >
          <ToggleSwitch
            checked={settings.minimizeToTray}
            onChange={handleMinimizeToTrayChange}
            label="Minimize to system tray on close"
          />
        </SettingRow>
      </SettingSection>

      {/* Shortcuts Section */}
      <SettingSection title="Shortcuts">
        <SettingRow
          label="Text selection modifier"
          description="Hold this key while selecting text to trigger translation popup"
        >
          <SelectionModifierSelector
            value={settings.selectionModifier}
            onChange={handleSelectionModifierChange}
          />
        </SettingRow>
        <SettingRow
          label="Global hotkey modifier"
          description={`Press ${settings.hotkeyModifier === 'ctrl+shift' ? 'Ctrl+Shift' : settings.hotkeyModifier === 'ctrl+alt' ? 'Ctrl+Alt' : 'Alt+Shift'}+Q to translate selected text`}
        >
          <HotkeyModifierSelector
            value={settings.hotkeyModifier}
            onChange={handleHotkeyModifierChange}
          />
        </SettingRow>
      </SettingSection>

      {/* Saved Indicator */}
      <SavedIndicator visible={showSaved} />
    </div>
  );
}
