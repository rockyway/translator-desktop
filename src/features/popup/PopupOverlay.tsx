import { useCallback, useEffect, useRef, useState } from 'react';
import { FiCopy, FiVolume2, FiExternalLink, FiX, FiChevronDown } from 'react-icons/fi';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from '../../hooks/useTranslation';
import {
  playTextToSpeech,
  getLanguageByCode,
  getTargetLanguages,
  LanguageOption,
} from '../../services/translationService';
import { useTheme } from '../../hooks/useTheme';

// Fixed heights for popup elements (in pixels)
const TITLE_BAR_HEIGHT = 32;
const FOOTER_HEIGHT = 44;
const CONTENT_PADDING = 24; // Extra padding for safety
const SECTION_LABEL_HEIGHT = 24; // Language label height
const DIVIDER_HEIGHT = 12; // border + margin
const LINE_HEIGHT = 21; // ~text-sm leading-relaxed
const MIN_LINES = 3; // Minimum lines to show for each section
const MIN_POPUP_HEIGHT = TITLE_BAR_HEIGHT + (SECTION_LABEL_HEIGHT + LINE_HEIGHT * MIN_LINES) * 2 + DIVIDER_HEIGHT + FOOTER_HEIGHT + CONTENT_PADDING; // ~220px
const MAX_POPUP_HEIGHT = 500;

/**
 * Props for the PopupOverlay component
 */
export interface PopupOverlayProps {
  /** The source text to translate */
  text: string;
  /** Callback when the popup should close */
  onClose: () => void;
  /** Callback to open the main translator window with current state */
  onOpenMain: (sourceText: string, translatedText: string, sourceLang: string, targetLang: string) => void;
  /** Default target language code (default: 'vi' for Vietnamese) */
  defaultTargetLanguage?: string;
}

/**
 * Default target language - must match DEFAULT_SETTINGS in SettingsContext
 * This ensures Main UI and Popup have consistent defaults.
 */
const DEFAULT_TARGET_LANGUAGE = 'es';

/**
 * Copy text to clipboard utility.
 */
async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    console.error('Failed to copy text to clipboard');
    return false;
  }
}

/**
 * Simple language selector dropdown for popup
 */
function LanguageDropdown({
  value,
  onChange,
  languages,
}: {
  value: string;
  onChange: (code: string) => void;
  languages: LanguageOption[];
}) {
  return (
    <div className="relative inline-block">
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="appearance-none bg-transparent text-sm font-medium text-blue-600 dark:text-blue-400
          cursor-pointer focus:outline-none pr-5"
      >
        {languages.map((lang) => (
          <option key={lang.code} value={lang.code} className="text-gray-900">
            {lang.name}
          </option>
        ))}
      </select>
      <FiChevronDown className="absolute right-0 top-1/2 -translate-y-1/2 w-4 h-4 pointer-events-none text-blue-600 dark:text-blue-400" />
    </div>
  );
}

/**
 * PopupOverlay - Compact translation popup for quick translations
 *
 * Features:
 * - Auto-translate when text is received
 * - Copy translated text to clipboard
 * - Listen to source and translated text (TTS)
 * - Open in main translator window with content transfer
 * - Dark/light theme support
 * - Draggable window
 * - Language selection persisted across restarts
 */
export function PopupOverlay({
  text,
  onClose,
  onOpenMain,
  defaultTargetLanguage,
}: PopupOverlayProps) {
  const { resolvedTheme } = useTheme();
  const [copySuccess, setCopySuccess] = useState(false);
  const [isPlayingSource, setIsPlayingSource] = useState(false);
  const [isPlayingTarget, setIsPlayingTarget] = useState(false);

  // Refs for measuring content height
  const sourceTextRef = useRef<HTMLParagraphElement>(null);
  const targetTextRef = useRef<HTMLParagraphElement>(null);

  // Target language - load from Tauri settings (set by Main UI)
  const [targetLanguage, setTargetLanguage] = useState(
    defaultTargetLanguage || DEFAULT_TARGET_LANGUAGE
  );

  // Load target language from Tauri settings when text changes (new popup shown)
  // This ensures we always use the latest language setting from Main UI
  useEffect(() => {
    if (defaultTargetLanguage) {
      // If prop is provided, use it directly
      return;
    }

    async function loadTargetLanguage() {
      try {
        const savedLang = await invoke<string | null>('get_setting', {
          key: 'target_language',
        });
        if (savedLang) {
          setTargetLanguage(savedLang);
        }
      } catch (error) {
        console.error('Failed to load target language setting:', error);
        // Keep default on error
      }
    }

    loadTargetLanguage();
  }, [defaultTargetLanguage, text]); // Reload when text changes (new popup)

  // Translation hook with auto-detect source language
  const {
    translate,
    translatedText,
    detectedLanguage,
    isLoading,
    error,
  } = useTranslation({
    sourceLanguage: 'auto',
    targetLanguage,
    debounceMs: 100, // Faster for popup
  });

  // Note: Popup does NOT save language changes back to settings
  // Main UI is the single source of truth for language preferences
  // Changes in popup are temporary for that session only

  // Auto-translate when text changes
  useEffect(() => {
    if (text && text.trim()) {
      translate(text);
    }
  }, [text, translate]);

  // Dynamically resize popup based on content
  useEffect(() => {
    const resizePopup = async () => {
      // Wait for DOM to update
      await new Promise(resolve => setTimeout(resolve, 100));

      const sourceHeight = sourceTextRef.current?.scrollHeight || LINE_HEIGHT * MIN_LINES;
      const targetHeight = targetTextRef.current?.scrollHeight || LINE_HEIGHT * MIN_LINES;

      // Calculate total height needed with generous padding
      // Ensure minimum 3 lines for each section
      const minSourceHeight = Math.max(sourceHeight, LINE_HEIGHT * MIN_LINES);
      const minTargetHeight = Math.max(targetHeight, LINE_HEIGHT * MIN_LINES);

      // Cap source at ~4 lines (scrollable beyond that)
      const sourceSectionHeight = SECTION_LABEL_HEIGHT + Math.min(minSourceHeight, LINE_HEIGHT * 4);
      const targetSectionHeight = SECTION_LABEL_HEIGHT + minTargetHeight;

      const totalHeight =
        TITLE_BAR_HEIGHT +
        sourceSectionHeight +
        DIVIDER_HEIGHT +
        targetSectionHeight +
        FOOTER_HEIGHT +
        CONTENT_PADDING;

      // Clamp to reasonable bounds
      const clampedHeight = Math.max(MIN_POPUP_HEIGHT, Math.min(totalHeight, MAX_POPUP_HEIGHT));

      try {
        await invoke('resize_popup', { height: clampedHeight });
      } catch (error) {
        console.error('Failed to resize popup:', error);
      }
    };

    // Resize when translation completes or text changes
    if (!isLoading && (translatedText || error)) {
      resizePopup();
    }
  }, [text, translatedText, isLoading, error]);

  // Handle keyboard shortcuts (Escape to close)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  // Handle window dragging - using mousedown event
  const handleStartDrag = useCallback(async (e: React.MouseEvent) => {
    // Only drag on left mouse button
    if (e.button !== 0) return;

    try {
      const appWindow = getCurrentWindow();
      await appWindow.startDragging();
    } catch (error) {
      console.error('Failed to start dragging:', error);
    }
  }, []);

  // Handle copy to clipboard
  const handleCopy = useCallback(async () => {
    if (!translatedText) return;
    const success = await copyToClipboard(translatedText);
    if (success) {
      setCopySuccess(true);
      setTimeout(() => setCopySuccess(false), 2000);
    }
  }, [translatedText]);

  // Handle listen source (text-to-speech)
  const handleListenSource = useCallback(async () => {
    if (!text || isPlayingSource) return;

    const langCode = detectedLanguage || 'en';
    // Truncate to 200 chars for Google TTS API limit
    const textToSpeak = text.length > 200 ? text.slice(0, 200) : text;

    try {
      setIsPlayingSource(true);
      await playTextToSpeech(textToSpeak, langCode);
    } catch (error) {
      console.error('Text-to-speech failed:', error);
    } finally {
      setIsPlayingSource(false);
    }
  }, [text, detectedLanguage, isPlayingSource]);

  // Handle listen target (text-to-speech)
  const handleListenTarget = useCallback(async () => {
    if (!translatedText || isPlayingTarget) return;

    // Truncate to 200 chars for Google TTS API limit
    const textToSpeak = translatedText.length > 200
      ? translatedText.slice(0, 200)
      : translatedText;

    try {
      setIsPlayingTarget(true);
      await playTextToSpeech(textToSpeak, targetLanguage);
    } catch (error) {
      console.error('Text-to-speech failed:', error);
    } finally {
      setIsPlayingTarget(false);
    }
  }, [translatedText, targetLanguage, isPlayingTarget]);

  // Handle open in main window with content transfer
  const handleOpenMain = useCallback(() => {
    const sourceLang = detectedLanguage || 'auto';
    onOpenMain(text, translatedText, sourceLang, targetLanguage);
    onClose();
  }, [onOpenMain, onClose, text, translatedText, detectedLanguage, targetLanguage]);

  // Handle language change
  const handleTargetLanguageChange = useCallback((code: string) => {
    setTargetLanguage(code);
  }, []);

  // Get display names for languages
  const detectedLanguageName = detectedLanguage
    ? getLanguageByCode(detectedLanguage)?.name || detectedLanguage
    : 'Detecting...';

  const targetLanguages = getTargetLanguages();

  return (
    <div
      className={`w-full h-full flex flex-col overflow-hidden ${
        resolvedTheme === 'dark' ? 'dark' : ''
      }`}
    >
      {/* Popup Container with Fluent Design - More opaque background */}
      <div className="flex-1 flex flex-col overflow-hidden bg-white/95 dark:bg-gray-900/95 backdrop-blur-xl">
        {/* Fluent Title Bar */}
        <div
          data-tauri-drag-region
          className="flex items-center justify-between h-8 px-3 bg-transparent cursor-move select-none flex-shrink-0"
          onMouseDown={handleStartDrag}
        >
          <span className="text-xs font-medium text-gray-600 dark:text-gray-400" data-tauri-drag-region>
            Translator
          </span>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close popup"
            className="w-8 h-8 flex items-center justify-center hover:bg-red-500 dark:hover:bg-red-600 transition-colors group"
            onMouseDown={(e) => e.stopPropagation()}
          >
            <FiX className="w-3.5 h-3.5 text-gray-600 dark:text-gray-400 group-hover:text-white" aria-hidden="true" />
          </button>
        </div>

        {/* Content - Compact layout */}
        <div className="flex-1 flex flex-col px-3 pb-2 gap-1 overflow-hidden">
          {/* Source Text Section */}
          <div className="flex-shrink-0">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-amber-600 dark:text-amber-400">
                {detectedLanguageName}
              </span>
              <button
                type="button"
                onClick={handleListenSource}
                disabled={!text || isPlayingSource}
                aria-label="Listen to source text"
                title="Listen"
                className={`p-1 rounded transition-colors focus:outline-none
                  ${text && !isPlayingSource
                    ? 'text-amber-500 hover:bg-amber-100 dark:hover:bg-amber-900/30'
                    : 'text-gray-300 dark:text-gray-600 cursor-not-allowed'
                  }`}
              >
                <FiVolume2 className={`w-4 h-4 ${isPlayingSource ? 'animate-pulse' : ''}`} aria-hidden="true" />
              </button>
            </div>
            <div className="max-h-24 overflow-y-auto mt-0.5">
              <p
                ref={sourceTextRef}
                className="text-sm text-gray-700 dark:text-gray-300 leading-relaxed whitespace-pre-wrap"
              >
                {text}
              </p>
            </div>
          </div>

          {/* Minimal Divider */}
          <div className="border-t border-gray-200 dark:border-gray-700/50 my-1 flex-shrink-0" />

          {/* Translated Text Section */}
          <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
            <div className="flex items-center justify-between flex-shrink-0">
              <LanguageDropdown
                value={targetLanguage}
                onChange={handleTargetLanguageChange}
                languages={targetLanguages}
              />
              <button
                type="button"
                onClick={handleListenTarget}
                disabled={!translatedText || isPlayingTarget}
                aria-label="Listen to translated text"
                title="Listen"
                className={`p-1 rounded transition-colors focus:outline-none
                  ${translatedText && !isPlayingTarget
                    ? 'text-blue-500 hover:bg-blue-100 dark:hover:bg-blue-900/30'
                    : 'text-gray-300 dark:text-gray-600 cursor-not-allowed'
                  }`}
              >
                <FiVolume2 className={`w-4 h-4 ${isPlayingTarget ? 'animate-pulse' : ''}`} aria-hidden="true" />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto mt-0.5">
              {/* Loading State */}
              {isLoading && (
                <div className="flex items-center gap-2 text-blue-500 dark:text-blue-400">
                  <svg
                    className="animate-spin w-4 h-4"
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
                  <span className="text-sm">Translating...</span>
                </div>
              )}

              {/* Error State */}
              {error && !isLoading && (
                <p className="text-sm text-red-500 dark:text-red-400">
                  Translation failed.
                </p>
              )}

              {/* Translated Text */}
              {!isLoading && !error && (
                <p
                  ref={targetTextRef}
                  className="text-sm text-gray-900 dark:text-gray-100 leading-relaxed font-medium whitespace-pre-wrap"
                >
                  {translatedText || 'Waiting for text...'}
                </p>
              )}
            </div>
          </div>
        </div>

        {/* Compact Footer Actions - Icon only buttons */}
        <div className="flex items-center justify-end gap-1 px-2 py-1.5 border-t border-gray-200/50 dark:border-gray-700/50 flex-shrink-0">
          {/* Copy Button - Icon only */}
          <button
            type="button"
            onClick={handleCopy}
            disabled={!translatedText}
            aria-label={copySuccess ? 'Copied!' : 'Copy translation'}
            title={copySuccess ? 'Copied!' : 'Copy'}
            className={`relative p-2 rounded-md transition-all focus:outline-none
              ${translatedText
                ? copySuccess
                  ? 'bg-green-500 text-white'
                  : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'
                : 'text-gray-300 dark:text-gray-600 cursor-not-allowed'
              }`}
          >
            <FiCopy className="w-4 h-4" aria-hidden="true" />
          </button>

          {/* Open Main Button - Icon only */}
          <button
            type="button"
            onClick={handleOpenMain}
            aria-label="Open in main window"
            title="Open in main window"
            className="p-2 rounded-md text-gray-600 dark:text-gray-400
              hover:bg-gray-100 dark:hover:bg-gray-700 transition-all focus:outline-none"
          >
            <FiExternalLink className="w-4 h-4" aria-hidden="true" />
          </button>
        </div>
      </div>
    </div>
  );
}
