import { useState, useCallback, useEffect, useMemo } from 'react';
import { FiCheck, FiMoon, FiSun, FiMonitor, FiType, FiAlertCircle, FiEye, FiEyeOff, FiInfo, FiX, FiPlus } from 'react-icons/fi';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import { useSettings } from '../../hooks/useSettings';
import type { Theme, SelectionModifier, HotkeyModifier, DensityPreset } from '../../contexts/SettingsContext';
import { DENSITY_VALUES } from '../../contexts/SettingsContext';
import { getTargetLanguages } from '../../services/translationService';

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
// Platform detection
// ============================================================================

const IS_MACOS = navigator.platform.startsWith('Mac') || navigator.userAgent.includes('Mac');

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
    label: IS_MACOS ? 'Cmd' : 'Ctrl',
    description: IS_MACOS ? 'Hold Cmd while selecting text' : 'Hold Ctrl while selecting text',
  },
  {
    value: 'shift',
    label: 'Shift',
    description: 'Hold Shift while selecting text',
  },
  {
    value: 'alt',
    label: IS_MACOS ? 'Option' : 'Alt',
    description: IS_MACOS ? 'Hold Option while selecting text' : 'Hold Alt while selecting text',
  },
];

const DEFAULT_LETTER = IS_MACOS ? 'R' : 'Q';

const HOTKEY_MODIFIER_OPTIONS: HotkeyModifierOption[] = [
  {
    value: 'ctrl+shift',
    label: 'Ctrl+Shift',
    shortcut: `Ctrl+Shift+${DEFAULT_LETTER}`,
    description: 'Default modifier combination',
  },
  {
    value: 'ctrl+alt',
    label: IS_MACOS ? 'Ctrl+Option' : 'Ctrl+Alt',
    shortcut: IS_MACOS ? `Ctrl+Option+${DEFAULT_LETTER}` : `Ctrl+Alt+${DEFAULT_LETTER}`,
    description: IS_MACOS ? 'Alternative using Option key' : 'Alternative using Alt key',
  },
  {
    value: 'alt+shift',
    label: IS_MACOS ? 'Option+Shift' : 'Alt+Shift',
    shortcut: IS_MACOS ? `Option+Shift+${DEFAULT_LETTER}` : `Alt+Shift+${DEFAULT_LETTER}`,
    description: IS_MACOS ? 'Without Ctrl key' : 'Without Ctrl key',
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

/** config_store key holding the Google Cloud Translation API key */
const GOOGLE_TRANSLATE_API_KEY_SETTING = 'google_translate_api_key';

/** config_store key holding an optional, Text-to-Speech-specific API key override */
const GOOGLE_TTS_API_KEY_SETTING = 'google_tts_api_key';

/** config_store key holding the default Chirp 3 HD voice name (used for any language without its own override) */
const GOOGLE_TTS_DEFAULT_VOICE_SETTING = 'google_tts_default_voice';

/** config_store key holding a JSON object of `{ languageCode: voiceName }` per-language voice overrides */
const GOOGLE_TTS_VOICE_OVERRIDES_SETTING = 'google_tts_voice_overrides';

/** Voice used until the user picks one - matches the Rust command's fallback */
const DEFAULT_CHIRP3_VOICE = 'Kore';

/**
 * All Chirp 3 HD voice personalities (docs.cloud.google.com/text-to-speech/docs/chirp3-hd).
 * Each name is available across every Chirp 3 HD locale, so this list doesn't vary by language.
 */
const CHIRP3_HD_VOICES: { name: string; gender: 'Female' | 'Male' }[] = [
  { name: 'Achernar', gender: 'Female' },
  { name: 'Achird', gender: 'Male' },
  { name: 'Algenib', gender: 'Male' },
  { name: 'Algieba', gender: 'Male' },
  { name: 'Alnilam', gender: 'Male' },
  { name: 'Aoede', gender: 'Female' },
  { name: 'Autonoe', gender: 'Female' },
  { name: 'Callirrhoe', gender: 'Female' },
  { name: 'Charon', gender: 'Male' },
  { name: 'Despina', gender: 'Female' },
  { name: 'Enceladus', gender: 'Male' },
  { name: 'Erinome', gender: 'Female' },
  { name: 'Fenrir', gender: 'Male' },
  { name: 'Gacrux', gender: 'Female' },
  { name: 'Iapetus', gender: 'Male' },
  { name: 'Kore', gender: 'Female' },
  { name: 'Laomedeia', gender: 'Female' },
  { name: 'Leda', gender: 'Female' },
  { name: 'Orus', gender: 'Male' },
  { name: 'Pulcherrima', gender: 'Female' },
  { name: 'Puck', gender: 'Male' },
  { name: 'Rasalgethi', gender: 'Male' },
  { name: 'Sadachbia', gender: 'Male' },
  { name: 'Sadaltager', gender: 'Male' },
  { name: 'Schedar', gender: 'Male' },
  { name: 'Sulafat', gender: 'Female' },
  { name: 'Umbriel', gender: 'Male' },
  { name: 'Vindemiatrix', gender: 'Female' },
  { name: 'Zephyr', gender: 'Female' },
  { name: 'Zubenelgenubi', gender: 'Male' },
];

/** Language codes Chirp 3 HD doesn't cover yet - these always use a fixed Standard-tier voice (see tts.rs). */
const NON_CHIRP3_LANGUAGE_CODES = new Set(['ms', 'zh-TW']);

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
  label: React.ReactNode;
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

/** Google Cloud Console page for creating/restricting API keys */
const GOOGLE_CLOUD_CREDENTIALS_URL = 'https://console.cloud.google.com/apis/credentials';

/** Google Cloud Text-to-Speech pricing page */
const GOOGLE_TTS_PRICING_URL = 'https://cloud.google.com/text-to-speech/pricing';

interface InfoTooltipProps {
  title: string;
  children: React.ReactNode;
  linkLabel: string;
  linkUrl: string;
}

/**
 * Info icon with a hover tooltip (steps + a clickable link that opens in the
 * system browser). Reused for the API key guide and the TTS server guide.
 */
function InfoTooltip({ title, children, linkLabel, linkUrl }: InfoTooltipProps) {
  const handleOpenLink = useCallback(() => {
    open(linkUrl).catch((error) => {
      console.error(`Failed to open link (${linkUrl}):`, error);
    });
  }, [linkUrl]);

  return (
    <span className="relative inline-flex group">
      <FiInfo
        className="w-3.5 h-3.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 cursor-help"
        aria-hidden="true"
      />
      {/*
        The visible card sits inside a slightly larger wrapper whose extra space is
        padding (part of the hoverable box), not margin (a real gap). A margin gap
        breaks hover: the cursor leaves the icon before it ever reaches the card,
        so group-hover turns off and the tooltip vanishes before you can click it.
      */}
      <div
        className="absolute left-1/2 -translate-x-1/2 bottom-full pb-2 w-72 z-10
          opacity-0 invisible group-hover:opacity-100 group-hover:visible
          transition-opacity duration-150"
      >
        <div
          role="tooltip"
          className="p-3 rounded-lg bg-gray-800 text-white text-xs leading-relaxed shadow-lg"
        >
          <p className="font-semibold mb-1">{title}</p>
          {children}
          <button
            type="button"
            onClick={handleOpenLink}
            className="mt-2 text-amber-300 hover:text-amber-200 underline"
          >
            {linkLabel}
          </button>
        </div>
      </div>
    </span>
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
                  ? 'bg-gradient-to-br from-amber-600 to-amber-700 text-white shadow-md'
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
        ${checked ? 'bg-gradient-to-r from-amber-600 to-amber-700' : 'bg-gray-300 dark:bg-gray-600'}
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
                  ? 'bg-gradient-to-br from-amber-600 to-amber-700 text-white shadow-md'
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
                  ? 'bg-gradient-to-br from-amber-600 to-amber-700 text-white shadow-md'
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
                    ? 'bg-gradient-to-br from-amber-600 to-amber-700 text-white shadow-md'
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
                [&::-webkit-slider-thumb]:bg-amber-600
                [&::-webkit-slider-thumb]:shadow-md
                [&::-webkit-slider-thumb]:cursor-pointer
                [&::-webkit-slider-thumb]:transition-transform
                [&::-webkit-slider-thumb]:hover:scale-110
                [&::-moz-range-thumb]:w-5
                [&::-moz-range-thumb]:h-5
                [&::-moz-range-thumb]:rounded-full
                [&::-moz-range-thumb]:bg-amber-600
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

interface VoiceSelectProps {
  value: string;
  onChange: (voice: string) => void;
  id?: string;
  'aria-label'?: string;
}

/** A `<select>` listing every Chirp 3 HD voice as "Name (Gender)". */
function VoiceSelect({ value, onChange, id, ...aria }: VoiceSelectProps) {
  return (
    <select
      id={id}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      aria-label={aria['aria-label']}
      className="px-3 py-2 text-sm rounded-lg
        bg-gray-100 dark:bg-gray-700
        border border-gray-200 dark:border-gray-600
        focus:ring-2 focus:ring-amber-500 focus:outline-none
        text-gray-900 dark:text-gray-100"
    >
      {CHIRP3_HD_VOICES.map((voice) => (
        <option key={voice.name} value={voice.name}>
          {voice.name} ({voice.gender})
        </option>
      ))}
    </select>
  );
}

interface PerLanguageVoiceEditorProps {
  /** All languages Chirp 3 HD supports, i.e. excluding NON_CHIRP3_LANGUAGE_CODES */
  languages: { code: string; name: string }[];
  overrides: Record<string, string>;
  defaultVoice: string;
  onAdd: (languageCode: string, voice: string) => void;
  onRemove: (languageCode: string) => void;
}

/**
 * Lets the user add a handful of "language -> voice" favorites instead of picking a
 * voice for every language. Languages not listed here fall back to the default voice.
 */
function PerLanguageVoiceEditor({
  languages,
  overrides,
  defaultVoice,
  onAdd,
  onRemove,
}: PerLanguageVoiceEditorProps) {
  const [pendingLanguage, setPendingLanguage] = useState('');
  const [pendingVoice, setPendingVoice] = useState(defaultVoice);

  const availableLanguages = languages.filter((lang) => !(lang.code in overrides));

  const handleLanguageChange = (code: string) => {
    setPendingLanguage(code);
    // Suggest the current default voice as a starting point; the user can still change it.
    setPendingVoice(defaultVoice);
  };

  const handleAdd = () => {
    if (!pendingLanguage) return;
    onAdd(pendingLanguage, pendingVoice);
    setPendingLanguage('');
    setPendingVoice(defaultVoice);
  };

  return (
    <div className="space-y-2">
      {Object.entries(overrides).map(([code, voice]) => {
        const language = languages.find((lang) => lang.code === code);
        const voiceInfo = CHIRP3_HD_VOICES.find((v) => v.name === voice);
        return (
          <div
            key={code}
            className="flex items-center justify-between gap-2 px-3 py-2 rounded-lg
              bg-gray-100 dark:bg-gray-700/50"
          >
            <span className="text-sm text-gray-700 dark:text-gray-200">
              {language?.name ?? code}
              <span className="text-gray-400 dark:text-gray-500"> — </span>
              {voice}
              {voiceInfo && (
                <span className="text-gray-400 dark:text-gray-500"> ({voiceInfo.gender})</span>
              )}
            </span>
            <button
              type="button"
              onClick={() => onRemove(code)}
              aria-label={`Remove ${language?.name ?? code} voice`}
              className="text-gray-400 hover:text-red-500 dark:hover:text-red-400"
            >
              <FiX className="w-4 h-4" aria-hidden="true" />
            </button>
          </div>
        );
      })}

      {availableLanguages.length > 0 && (
        <div className="flex items-center gap-2 pt-1">
          <select
            value={pendingLanguage}
            onChange={(e) => handleLanguageChange(e.target.value)}
            aria-label="Language to add a voice for"
            className="flex-1 px-3 py-2 text-sm rounded-lg
              bg-gray-100 dark:bg-gray-700
              border border-gray-200 dark:border-gray-600
              focus:ring-2 focus:ring-amber-500 focus:outline-none
              text-gray-900 dark:text-gray-100"
          >
            <option value="">Add a language...</option>
            {availableLanguages.map((lang) => (
              <option key={lang.code} value={lang.code}>
                {lang.name}
              </option>
            ))}
          </select>

          {pendingLanguage && (
            <VoiceSelect
              value={pendingVoice}
              onChange={setPendingVoice}
              aria-label="Voice for the selected language"
            />
          )}

          <button
            type="button"
            onClick={handleAdd}
            disabled={!pendingLanguage}
            aria-label="Add language voice"
            className="p-2 rounded-lg bg-amber-600 text-white hover:bg-amber-700
              disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <FiPlus className="w-4 h-4" aria-hidden="true" />
          </button>
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
  const { settings, updateSetting, isLoading, hotkeyError } = useSettings();
  const [showSaved, setShowSaved] = useState(false);
  const [autoStartEnabled, setAutoStartEnabled] = useState(false);
  const [autoStartLoading, setAutoStartLoading] = useState(true);
  const [apiKeyInput, setApiKeyInput] = useState('');
  const [apiKeyDirty, setApiKeyDirty] = useState(false);
  const [apiKeySaving, setApiKeySaving] = useState(false);
  const [showApiKey, setShowApiKey] = useState(false);
  const [ttsApiKeyInput, setTtsApiKeyInput] = useState('');
  const [ttsApiKeyDirty, setTtsApiKeyDirty] = useState(false);
  const [ttsApiKeySaving, setTtsApiKeySaving] = useState(false);
  const [showTtsApiKey, setShowTtsApiKey] = useState(false);
  const [defaultVoice, setDefaultVoice] = useState(DEFAULT_CHIRP3_VOICE);
  const [voiceOverrides, setVoiceOverrides] = useState<Record<string, string>>({});

  const chirp3Languages = useMemo(
    () => getTargetLanguages().filter((lang) => !NON_CHIRP3_LANGUAGE_CODES.has(lang.code)),
    []
  );

  // Load auto-start status on mount
  useEffect(() => {
    async function loadAutoStartStatus() {
      try {
        const enabled = await invoke<boolean>('is_autostart_enabled');
        setAutoStartEnabled(enabled);
      } catch (error) {
        console.error('Failed to get auto-start status:', error);
      } finally {
        setAutoStartLoading(false);
      }
    }
    loadAutoStartStatus();
  }, []);

  // Load the saved Google Cloud Translation API key on mount
  useEffect(() => {
    async function loadApiKey() {
      try {
        const key = await invoke<string | null>('get_setting', {
          key: GOOGLE_TRANSLATE_API_KEY_SETTING,
        });
        setApiKeyInput(key ?? '');
      } catch (error) {
        console.error('Failed to load translation API key:', error);
      }
    }
    loadApiKey();
  }, []);

  // Load the saved Text-to-Speech API key override on mount
  useEffect(() => {
    async function loadTtsApiKey() {
      try {
        const key = await invoke<string | null>('get_setting', {
          key: GOOGLE_TTS_API_KEY_SETTING,
        });
        setTtsApiKeyInput(key ?? '');
      } catch (error) {
        console.error('Failed to load TTS API key:', error);
      }
    }
    loadTtsApiKey();
  }, []);

  // Load the saved TTS voice preferences on mount
  useEffect(() => {
    async function loadVoicePreferences() {
      try {
        const [voice, overrides] = await Promise.all([
          invoke<string | null>('get_setting', { key: GOOGLE_TTS_DEFAULT_VOICE_SETTING }),
          invoke<Record<string, string> | null>('get_setting', {
            key: GOOGLE_TTS_VOICE_OVERRIDES_SETTING,
          }),
        ]);
        setDefaultVoice(voice ?? DEFAULT_CHIRP3_VOICE);
        setVoiceOverrides(overrides ?? {});
      } catch (error) {
        console.error('Failed to load TTS voice preferences:', error);
      }
    }
    loadVoicePreferences();
  }, []);

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

  const handleHotkeyLetterChange = useCallback(
    (letter: string) => {
      updateSetting('hotkeyLetter', letter);
      flashSavedIndicator();
    },
    [updateSetting, flashSavedIndicator]
  );

  /**
   * Format hotkey label for display (e.g., "Ctrl+Shift+Q" or "Cmd+Shift+Q" on macOS)
   */
  const formatHotkeyLabel = useCallback(
    (modifier: HotkeyModifier, letter: string): string => {
      const keyMap: Record<string, string> = IS_MACOS
        ? { ctrl: 'Cmd', alt: 'Option', shift: 'Shift' }
        : { ctrl: 'Ctrl', alt: 'Alt', shift: 'Shift' };
      const modParts = modifier
        .split('+')
        .map((m) => keyMap[m.toLowerCase()] ?? (m.charAt(0).toUpperCase() + m.slice(1)))
        .join('+');
      return `${modParts}+${letter.toUpperCase()}`;
    },
    []
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

  const handleAutoStartChange = useCallback(
    async (enabled: boolean) => {
      try {
        await invoke('set_autostart_enabled', { enabled });
        setAutoStartEnabled(enabled);
        flashSavedIndicator();
      } catch (error) {
        console.error('Failed to set auto-start:', error);
      }
    },
    [flashSavedIndicator]
  );

  const handleSaveApiKey = useCallback(async () => {
    setApiKeySaving(true);
    try {
      await invoke('set_setting', {
        key: GOOGLE_TRANSLATE_API_KEY_SETTING,
        value: apiKeyInput.trim(),
      });
      setApiKeyDirty(false);
      flashSavedIndicator();
    } catch (error) {
      console.error('Failed to save translation API key:', error);
    } finally {
      setApiKeySaving(false);
    }
  }, [apiKeyInput, flashSavedIndicator]);

  const handleSaveTtsApiKey = useCallback(async () => {
    setTtsApiKeySaving(true);
    try {
      await invoke('set_setting', {
        key: GOOGLE_TTS_API_KEY_SETTING,
        value: ttsApiKeyInput.trim(),
      });
      setTtsApiKeyDirty(false);
      flashSavedIndicator();
    } catch (error) {
      console.error('Failed to save TTS API key:', error);
    } finally {
      setTtsApiKeySaving(false);
    }
  }, [ttsApiKeyInput, flashSavedIndicator]);

  const handleDefaultVoiceChange = useCallback(
    async (voice: string) => {
      setDefaultVoice(voice);
      try {
        await invoke('set_setting', { key: GOOGLE_TTS_DEFAULT_VOICE_SETTING, value: voice });
        flashSavedIndicator();
      } catch (error) {
        console.error('Failed to save default TTS voice:', error);
      }
    },
    [flashSavedIndicator]
  );

  const saveVoiceOverrides = useCallback(
    async (overrides: Record<string, string>) => {
      setVoiceOverrides(overrides);
      try {
        await invoke('set_setting', { key: GOOGLE_TTS_VOICE_OVERRIDES_SETTING, value: overrides });
        flashSavedIndicator();
      } catch (error) {
        console.error('Failed to save TTS voice overrides:', error);
      }
    },
    [flashSavedIndicator]
  );

  const handleAddVoiceOverride = useCallback(
    (languageCode: string, voice: string) => {
      saveVoiceOverrides({ ...voiceOverrides, [languageCode]: voice });
    },
    [voiceOverrides, saveVoiceOverrides]
  );

  const handleRemoveVoiceOverride = useCallback(
    (languageCode: string) => {
      const { [languageCode]: _removed, ...rest } = voiceOverrides;
      saveVoiceOverrides(rest);
    },
    [voiceOverrides, saveVoiceOverrides]
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
        <SettingRow
          label="Start with system"
          description="Automatically launch the app when you log in"
        >
          {autoStartLoading ? (
            <div className="w-11 h-6 bg-gray-200 dark:bg-gray-700 rounded-full animate-pulse" />
          ) : (
            <ToggleSwitch
              checked={autoStartEnabled}
              onChange={handleAutoStartChange}
              label="Start with system"
            />
          )}
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
          label="Global hotkey"
          description={`Press ${formatHotkeyLabel(settings.hotkeyModifier, settings.hotkeyLetter)} to translate selected text`}
        >
          <div className="flex items-center gap-2">
            <HotkeyModifierSelector
              value={settings.hotkeyModifier}
              onChange={handleHotkeyModifierChange}
            />

            <span className="text-gray-400 dark:text-gray-500 font-medium">+</span>

            <input
              type="text"
              maxLength={1}
              value={settings.hotkeyLetter.toUpperCase()}
              onChange={(e) => {
                const letter = e.target.value.toLowerCase();
                if (/^[a-z]$/.test(letter) || letter === '') {
                  handleHotkeyLetterChange(letter || 'q');
                }
              }}
              placeholder="Q"
              className="w-12 h-10 text-center text-lg font-bold uppercase
                bg-gray-100 dark:bg-gray-700
                border-2 border-gray-300 dark:border-gray-600
                rounded-lg focus:ring-2 focus:ring-amber-500 focus:outline-none
                text-gray-900 dark:text-gray-100"
              aria-label="Hotkey letter"
            />
          </div>
        </SettingRow>
        {hotkeyError && (
          <div className="flex items-start gap-2 p-3 rounded-lg
            bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800">
            <FiAlertCircle className="w-5 h-5 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
            <div className="flex-1">
              <p className="text-sm text-red-700 dark:text-red-300 font-medium">
                Hotkey registration failed
              </p>
              <p className="text-xs text-red-600 dark:text-red-400 mt-1">
                {hotkeyError}
              </p>
              <p className="text-xs text-red-600 dark:text-red-400 mt-1">
                Try a different key combination. Your previous hotkey is still active.
              </p>
            </div>
          </div>
        )}
        <SettingRow
          label="Character limit confirmation"
          description="Ask before translating text longer than this (0 to disable)"
        >
          <input
            type="number"
            min="0"
            max="1000"
            step="10"
            value={settings.confirmationCharLimit}
            onChange={(e) => {
              const value = Math.max(0, Math.min(1000, Number(e.target.value)));
              updateSetting('confirmationCharLimit', value);
              flashSavedIndicator();
            }}
            className="w-24 px-3 py-2 text-sm rounded-lg
              bg-gray-100 dark:bg-gray-700
              border border-gray-200 dark:border-gray-600
              focus:ring-2 focus:ring-amber-500 focus:outline-none"
          />
        </SettingRow>
      </SettingSection>

      {/* Google Cloud API Section */}
      <SettingSection title="Google Cloud API">
        <SettingRow
          label={
            <span className="inline-flex items-center gap-1.5">
              Google Cloud Translation API key
              <InfoTooltip
                title="How to get an API key"
                linkLabel="Open Google Cloud Console"
                linkUrl={GOOGLE_CLOUD_CREDENTIALS_URL}
              >
                <ol className="list-decimal list-inside space-y-0.5">
                  <li>Create or select a project in Google Cloud Console</li>
                  <li>Enable the "Cloud Translation API"</li>
                  <li>Enable the "Cloud Text-to-Speech API" too, for the speaker buttons</li>
                  <li>Go to APIs &amp; Services → Credentials and create an API key</li>
                </ol>
                <p className="mt-2 pt-2 border-t border-white/10">
                  Translation free tier: 500,000 characters/month, then $20 per
                  1M characters.
                </p>
              </InfoTooltip>
            </span>
          }
          description="Required for translation. Also used for text-to-speech unless a separate key is set below."
        >
          <div className="flex items-center gap-2">
            <div className="relative">
              <input
                type={showApiKey ? 'text' : 'password'}
                value={apiKeyInput}
                onChange={(e) => {
                  setApiKeyInput(e.target.value);
                  setApiKeyDirty(true);
                }}
                placeholder="Paste API key"
                autoComplete="off"
                spellCheck={false}
                className="w-56 pl-3 pr-9 py-2 text-sm rounded-lg
                  bg-gray-100 dark:bg-gray-700
                  border border-gray-200 dark:border-gray-600
                  focus:ring-2 focus:ring-amber-500 focus:outline-none
                  text-gray-900 dark:text-gray-100 font-mono"
              />
              <button
                type="button"
                onClick={() => setShowApiKey((prev) => !prev)}
                aria-label={showApiKey ? 'Hide API key' : 'Show API key'}
                className="absolute right-2 top-1/2 -translate-y-1/2
                  text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
              >
                {showApiKey ? (
                  <FiEyeOff className="w-4 h-4" aria-hidden="true" />
                ) : (
                  <FiEye className="w-4 h-4" aria-hidden="true" />
                )}
              </button>
            </div>
            <button
              type="button"
              onClick={handleSaveApiKey}
              disabled={!apiKeyDirty || apiKeySaving}
              className="px-3 py-2 text-sm font-medium rounded-lg
                bg-amber-600 text-white hover:bg-amber-700
                disabled:opacity-50 disabled:cursor-not-allowed
                transition-colors"
            >
              {apiKeySaving ? 'Saving...' : 'Save'}
            </button>
          </div>
        </SettingRow>
        <SettingRow
          label={
            <span className="inline-flex items-center gap-1.5">
              Text-to-Speech API key
              <InfoTooltip
                title="Text-to-speech pricing"
                linkLabel="Open Cloud Text-to-Speech pricing"
                linkUrl={GOOGLE_TTS_PRICING_URL}
              >
                <p>Free tier is per voice type, per month:</p>
                <ul className="list-disc list-inside space-y-0.5 mt-1">
                  <li>Standard / WaveNet: 4M characters, then $4/1M</li>
                  <li>Neural2: 1M characters, then $16/1M</li>
                  <li>Chirp 3: HD: 1M characters, then $30/1M</li>
                </ul>
                <p className="mt-2 pt-2 border-t border-white/10">
                  This app uses Chirp 3: HD voices.
                </p>
              </InfoTooltip>
            </span>
          }
          description="Optional - leave blank to reuse the Translation API key above."
        >
          <div className="flex items-center gap-2">
            <div className="relative">
              <input
                type={showTtsApiKey ? 'text' : 'password'}
                value={ttsApiKeyInput}
                onChange={(e) => {
                  setTtsApiKeyInput(e.target.value);
                  setTtsApiKeyDirty(true);
                }}
                placeholder="Leave blank to share the key above"
                autoComplete="off"
                spellCheck={false}
                className="w-56 pl-3 pr-9 py-2 text-sm rounded-lg
                  bg-gray-100 dark:bg-gray-700
                  border border-gray-200 dark:border-gray-600
                  focus:ring-2 focus:ring-amber-500 focus:outline-none
                  text-gray-900 dark:text-gray-100 font-mono"
              />
              <button
                type="button"
                onClick={() => setShowTtsApiKey((prev) => !prev)}
                aria-label={showTtsApiKey ? 'Hide API key' : 'Show API key'}
                className="absolute right-2 top-1/2 -translate-y-1/2
                  text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
              >
                {showTtsApiKey ? (
                  <FiEyeOff className="w-4 h-4" aria-hidden="true" />
                ) : (
                  <FiEye className="w-4 h-4" aria-hidden="true" />
                )}
              </button>
            </div>
            <button
              type="button"
              onClick={handleSaveTtsApiKey}
              disabled={!ttsApiKeyDirty || ttsApiKeySaving}
              className="px-3 py-2 text-sm font-medium rounded-lg
                bg-amber-600 text-white hover:bg-amber-700
                disabled:opacity-50 disabled:cursor-not-allowed
                transition-colors"
            >
              {ttsApiKeySaving ? 'Saving...' : 'Save'}
            </button>
          </div>
        </SettingRow>
      </SettingSection>

      {/* Text-to-Speech Voice Section */}
      <SettingSection title="Text-to-Speech Voice">
        <SettingRow
          label="Default voice"
          description="Used for any language without its own voice below."
        >
          <VoiceSelect
            value={defaultVoice}
            onChange={handleDefaultVoiceChange}
            aria-label="Default text-to-speech voice"
          />
        </SettingRow>
        <div className="pt-2">
          <div className="flex-1 min-w-0 mb-3">
            <span className="text-sm font-medium text-gray-700 dark:text-gray-200">
              Per-language voices
            </span>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
              Pick a different voice for specific languages - e.g. a male voice for
              English, a female voice for Vietnamese.
            </p>
          </div>
          <PerLanguageVoiceEditor
            languages={chirp3Languages}
            overrides={voiceOverrides}
            defaultVoice={defaultVoice}
            onAdd={handleAddVoiceOverride}
            onRemove={handleRemoveVoiceOverride}
          />
        </div>
      </SettingSection>

      {/* Saved Indicator */}
      <SavedIndicator visible={showSaved} />
    </div>
  );
}
