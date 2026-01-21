import { useState, useCallback, useEffect, useRef } from 'react';
import { useForm } from 'react-hook-form';
import { FiVolume2, FiCopy, FiRepeat, FiAlertCircle, FiRefreshCw, FiArrowRight } from 'react-icons/fi';
import { LanguageSelector } from './LanguageSelector';
import { useTranslation } from '../../hooks/useTranslation';
import { useSettings } from '../../hooks/useSettings';
import {
  getSupportedLanguages,
  getTargetLanguages,
  getLanguageByCode,
  playTextToSpeech,
} from '../../services/translationService';

interface TranslationPanelProps {
  className?: string;
  /** Initial text to populate input (e.g., from popup or history) */
  initialText?: string;
  /** Initial translated text (from popup - already translated, no need to re-translate) */
  initialTranslatedText?: string;
  /** Initial source language code */
  initialSourceLang?: string;
  /** Initial target language code */
  initialTargetLang?: string;
}

interface FormInputs {
  inputText: string;
}

/**
 * Detects the browser's preferred language and returns a valid target language code.
 */
function getDefaultTargetLanguage(): string {
  const browserLang = navigator.language.split('-')[0];
  const targetLanguages = getTargetLanguages();
  const isSupported = targetLanguages.some((lang) => lang.code === browserLang);

  if (isSupported && browserLang !== 'en') {
    return browserLang;
  }
  return 'es';
}


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
 * TranslationPanel - Google Translate style UI
 *
 * Layout:
 * - Left Panel: Language selector + Input textarea
 * - Center: Swap button
 * - Right Panel: Language selector + Output textarea
 */
export function TranslationPanel({
  className = '',
  initialText,
  initialTranslatedText,
  initialSourceLang,
  initialTargetLang,
}: TranslationPanelProps) {
  // Settings context for persisting language preferences
  const { settings, updateSetting } = useSettings();

  // Language state - use props if provided, otherwise use settings, finally fallback to defaults
  const [sourceLanguage, setSourceLanguage] = useState(() => {
    if (initialSourceLang) return initialSourceLang;
    return settings.sourceLanguage || 'auto';
  });
  const [targetLanguage, setTargetLanguage] = useState(() => {
    if (initialTargetLang) return initialTargetLang;
    return settings.targetLanguage || getDefaultTargetLanguage();
  });

  // Copy success feedback
  const [copySuccess, setCopySuccess] = useState(false);

  // Pre-translated text from popup (to avoid re-translating)
  const [preTranslatedText, setPreTranslatedText] = useState<string | undefined>(
    initialTranslatedText
  );

  // Form setup with React Hook Form
  const { register, watch, setValue } = useForm<FormInputs>({
    defaultValues: {
      inputText: initialText || '',
    },
  });

  const inputText = watch('inputText');

  // Translation hook
  const {
    translate,
    translatedText: hookTranslatedText,
    detectedLanguage,
    isLoading,
    error,
  } = useTranslation({
    sourceLanguage,
    targetLanguage,
    debounceMs: 500,
  });

  // Effective translated text: prefer pre-translated (from popup) over hook result
  const translatedText = preTranslatedText || hookTranslatedText;

  // Ref for aria-live region
  const translationResultRef = useRef<HTMLDivElement>(null);

  // Manual translation handler
  const handleTranslate = useCallback(() => {
    if (inputText.trim()) {
      // Clear pre-translated text so fresh translation takes over
      setPreTranslatedText(undefined);
      translate(inputText);
    }
  }, [inputText, translate]);

  // Keyboard handler for Ctrl+Enter
  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      handleTranslate();
    }
  }, [handleTranslate]);

  // Track if user has explicitly changed languages (not initial mount)
  const hasUserChangedLanguageRef = useRef(false);

  // Handle source language change - update state and persist to settings
  const handleSourceLanguageChange = useCallback((newLang: string) => {
    hasUserChangedLanguageRef.current = true;
    setPreTranslatedText(undefined); // Clear pre-translated text on language change
    setSourceLanguage(newLang);
    // Persist to settings so it becomes the new default
    updateSetting('sourceLanguage', newLang);
  }, [updateSetting]);

  // Handle target language change - update state and persist to settings
  const handleTargetLanguageChange = useCallback((newLang: string) => {
    hasUserChangedLanguageRef.current = true;
    setPreTranslatedText(undefined); // Clear pre-translated text on language change
    setTargetLanguage(newLang);
    // Persist to settings so it becomes the new default
    updateSetting('targetLanguage', newLang);
  }, [updateSetting]);

  // Re-translate when languages change (only after user explicitly changes them)
  useEffect(() => {
    if (hasUserChangedLanguageRef.current && inputText.trim() && !preTranslatedText) {
      translate(inputText);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sourceLanguage, targetLanguage]);

  // Handle swap languages
  const handleSwap = useCallback(() => {
    const actualSourceLang = sourceLanguage === 'auto' && detectedLanguage
      ? detectedLanguage
      : sourceLanguage;

    if (actualSourceLang === 'auto') return;

    // Update local state
    setSourceLanguage(targetLanguage);
    setTargetLanguage(actualSourceLang);

    // Persist swapped languages to settings
    updateSetting('sourceLanguage', targetLanguage);
    updateSetting('targetLanguage', actualSourceLang);

    if (translatedText) {
      setValue('inputText', translatedText);
    }
  }, [sourceLanguage, targetLanguage, detectedLanguage, translatedText, setValue, updateSetting]);

  // Handle listen (text-to-speech) for input
  const handleListenInput = useCallback(async () => {
    if (!inputText.trim()) return;

    const langCode = sourceLanguage === 'auto' && detectedLanguage
      ? detectedLanguage
      : sourceLanguage === 'auto'
        ? 'en'
        : sourceLanguage;

    // Truncate to 200 chars for Google TTS API limit
    const textToSpeak = inputText.length > 200
      ? inputText.slice(0, 200)
      : inputText;

    try {
      await playTextToSpeech(textToSpeak, langCode);
    } catch (error) {
      console.error('Text-to-speech failed:', error);
    }
  }, [inputText, sourceLanguage, detectedLanguage]);

  // Handle listen (text-to-speech) for output
  const handleListenOutput = useCallback(async () => {
    if (!translatedText.trim()) return;

    // Truncate to 200 chars for Google TTS API limit
    const textToSpeak = translatedText.length > 200
      ? translatedText.slice(0, 200)
      : translatedText;

    try {
      await playTextToSpeech(textToSpeak, targetLanguage);
    } catch (error) {
      console.error('Text-to-speech failed:', error);
    }
  }, [translatedText, targetLanguage]);

  // Handle copy to clipboard
  const handleCopy = useCallback(async () => {
    const success = await copyToClipboard(translatedText);
    if (success) {
      setCopySuccess(true);
      setTimeout(() => setCopySuccess(false), 2000);
    }
  }, [translatedText]);

  // Handle retry on error
  const handleRetry = useCallback(() => {
    if (inputText.trim()) {
      translate(inputText);
    }
  }, [inputText, translate]);

  // Get language options
  const sourceLanguages = getSupportedLanguages();
  const targetLanguages = getTargetLanguages();

  // Get display name for detected language
  const detectedLanguageName = detectedLanguage
    ? getLanguageByCode(detectedLanguage)?.name
    : undefined;

  // Character count
  const charCount = inputText.length;
  const maxChars = 5000;

  // Check if swap is possible
  const canSwap = sourceLanguage !== 'auto' || detectedLanguage;

  return (
    <div className={`w-full ${className}`}>
      {/* Main Layout: Left Panel | Swap | Right Panel */}
      <div className="flex flex-col md:flex-row gap-4">
        {/* LEFT PANEL: Source Language + Input */}
        <div className="flex-1 min-w-0">
          {/* Source Language Selector */}
          <div className="mb-3">
            <LanguageSelector
              id="source-language"
              label="Translate from"
              value={sourceLanguage}
              onChange={handleSourceLanguageChange}
              languages={sourceLanguages}
              sublabel={
                sourceLanguage === 'auto' && detectedLanguageName && inputText.trim()
                  ? `Detected: ${detectedLanguageName}`
                  : undefined
              }
            />
          </div>

          {/* Input Textarea */}
          <div className="bg-gradient-to-br from-amber-50 to-amber-100 dark:from-amber-900/20 dark:to-amber-800/20 rounded-xl border-2 border-amber-300 dark:border-amber-700 shadow-lg overflow-hidden">
            <div className="p-4 flex flex-col">
              <label htmlFor="input-text" className="sr-only">
                Enter text to translate
              </label>
              <textarea
                id="input-text"
                {...register('inputText')}
                onKeyDown={handleKeyDown}
                placeholder="Enter text to translate..."
                aria-label="Text to translate"
                aria-describedby="char-count"
                className="w-full min-h-[180px] resize-none bg-transparent
                  text-gray-900 dark:text-gray-100 placeholder-amber-400 dark:placeholder-amber-500
                  focus:outline-none text-lg leading-relaxed"
                maxLength={maxChars}
              />

              {/* Input Footer */}
              <div className="flex items-center justify-between pt-3 border-t border-amber-200 dark:border-amber-700/50">
                <div className="flex items-center gap-1">
                  <button
                    type="button"
                    onClick={handleTranslate}
                    disabled={!inputText.trim()}
                    aria-label="Translate text"
                    title="Translate"
                    className={`p-2.5 rounded-lg transition-all
                      focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800
                      ${inputText.trim()
                        ? 'text-amber-600 dark:text-amber-400 hover:bg-amber-100 dark:hover:bg-amber-900/30'
                        : 'text-amber-300 dark:text-amber-700 cursor-not-allowed'
                      }`}
                  >
                    <FiArrowRight className="w-5 h-5" aria-hidden="true" />
                  </button>

                  <button
                    type="button"
                    onClick={handleListenInput}
                    disabled={!inputText.trim()}
                    aria-label="Listen to input text"
                    title="Listen"
                    className={`p-2.5 rounded-lg transition-all
                      focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800
                      ${inputText.trim()
                        ? 'text-amber-600 dark:text-amber-400 hover:bg-amber-100 dark:hover:bg-amber-900/30'
                        : 'text-amber-300 dark:text-amber-700 cursor-not-allowed'
                      }`}
                  >
                    <FiVolume2 className="w-5 h-5" aria-hidden="true" />
                  </button>
                </div>

                <span
                  id="char-count"
                  className={`text-sm font-medium ${
                    charCount > maxChars * 0.9
                      ? 'text-red-500 dark:text-red-400'
                      : 'text-amber-600 dark:text-amber-400'
                  }`}
                >
                  {charCount.toLocaleString()} / {maxChars.toLocaleString()}
                </span>
              </div>
            </div>
          </div>
        </div>

        {/* CENTER: Swap Button */}
        <div className="flex md:flex-col justify-center items-center py-2 md:py-0 md:pt-12">
          <button
            type="button"
            onClick={handleSwap}
            disabled={!canSwap}
            aria-label="Swap source and target languages"
            className={`p-4 rounded-full transition-all shadow-lg
              focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800
              ${canSwap
                ? 'bg-gradient-to-r from-amber-500 to-blue-500 text-white hover:from-amber-600 hover:to-blue-600 hover:shadow-xl cursor-pointer'
                : 'bg-gray-200 dark:bg-gray-700 text-gray-400 dark:text-gray-500 cursor-not-allowed'
              }`}
          >
            <FiRepeat className="w-6 h-6" aria-hidden="true" />
          </button>
        </div>

        {/* RIGHT PANEL: Target Language + Output */}
        <div className="flex-1 min-w-0">
          {/* Target Language Selector */}
          <div className="mb-3">
            <LanguageSelector
              id="target-language"
              label="Translate to"
              value={targetLanguage}
              onChange={handleTargetLanguageChange}
              languages={targetLanguages}
            />
          </div>

          {/* Output Area */}
          <div className="bg-gradient-to-br from-blue-50 to-blue-100 dark:from-blue-900/20 dark:to-blue-800/20 rounded-xl border-2 border-blue-300 dark:border-blue-700 shadow-lg overflow-hidden">
            <div className="p-4 flex flex-col min-h-[252px]">
              {/* Translation Result */}
              <div
                ref={translationResultRef}
                aria-live="polite"
                aria-atomic="true"
                className="flex-1 min-h-[180px]"
              >
                {/* Loading State */}
                {isLoading && (
                  <div className="flex items-center gap-3 text-blue-600 dark:text-blue-400">
                    <svg
                      className="animate-spin w-6 h-6"
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
                    <span className="text-lg font-medium">Translating...</span>
                  </div>
                )}

                {/* Error State */}
                {error && !isLoading && (
                  <div className="flex flex-col gap-3">
                    <div className="flex items-center gap-2 text-red-500 dark:text-red-400">
                      <FiAlertCircle className="w-5 h-5" aria-hidden="true" />
                      <span className="font-medium">Translation failed</span>
                    </div>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      {error.message}
                    </p>
                    <button
                      type="button"
                      onClick={handleRetry}
                      className="flex items-center gap-2 px-4 py-2 w-fit
                        bg-blue-500 text-white rounded-lg hover:bg-blue-600
                        focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2
                        dark:focus:ring-offset-gray-800 transition-all shadow-md hover:shadow-lg font-medium"
                    >
                      <FiRefreshCw className="w-4 h-4" aria-hidden="true" />
                      <span>Retry</span>
                    </button>
                  </div>
                )}

                {/* Translated Text */}
                {!isLoading && !error && (
                  <p
                    className={`text-lg leading-relaxed ${
                      translatedText
                        ? 'text-gray-900 dark:text-gray-100'
                        : 'text-blue-400 dark:text-blue-500 italic'
                    }`}
                  >
                    {translatedText || 'Translation will appear here...'}
                  </p>
                )}
              </div>

              {/* Output Footer */}
              <div className="flex items-center justify-end gap-1 pt-3 border-t border-blue-200 dark:border-blue-700/50">
                <button
                  type="button"
                  onClick={handleListenOutput}
                  disabled={!translatedText}
                  aria-label="Listen to translated text"
                  title="Listen"
                  className={`p-2.5 rounded-lg transition-all
                    focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800
                    ${translatedText
                      ? 'text-blue-600 dark:text-blue-400 hover:bg-blue-100 dark:hover:bg-blue-900/30'
                      : 'text-blue-300 dark:text-blue-700 cursor-not-allowed'
                    }`}
                >
                  <FiVolume2 className="w-5 h-5" aria-hidden="true" />
                </button>

                <button
                  type="button"
                  onClick={handleCopy}
                  disabled={!translatedText}
                  aria-label={copySuccess ? 'Copied!' : 'Copy translated text'}
                  title={copySuccess ? 'Copied!' : 'Copy'}
                  className={`p-2.5 rounded-lg transition-all
                    focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800
                    ${translatedText
                      ? copySuccess
                        ? 'bg-green-500 text-white'
                        : 'text-blue-600 dark:text-blue-400 hover:bg-blue-100 dark:hover:bg-blue-900/30'
                      : 'text-blue-300 dark:text-blue-700 cursor-not-allowed'
                    }`}
                >
                  <FiCopy className="w-5 h-5" aria-hidden="true" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
