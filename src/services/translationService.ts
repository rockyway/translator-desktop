import { invoke } from '@tauri-apps/api/core';

/**
 * Result of a translation request
 */
export interface TranslationResult {
  /** The translated text */
  translatedText: string;
  /** The detected source language code (if auto-detect was used) */
  detectedLanguage?: string;
  /** Pronunciation guide for the translated text (if available) */
  pronunciation?: string;
}

/**
 * Options for translation requests
 */
export interface TranslationOptions {
  /** Source language code (e.g., 'en', 'auto' for auto-detect) */
  from?: string;
  /** Target language code (e.g., 'es', 'ja') - required */
  to: string;
}

/**
 * Language option for dropdowns/selectors
 */
export interface LanguageOption {
  /** ISO 639-1 language code */
  code: string;
  /** Human-readable language name */
  name: string;
}

/**
 * Custom error class for translation errors
 */
export class TranslationError extends Error {
  constructor(
    message: string,
    public readonly originalError?: unknown
  ) {
    super(message);
    this.name = 'TranslationError';
  }
}

/**
 * Custom error class for text-to-speech errors
 */
export class TTSError extends Error {
  constructor(
    message: string,
    public readonly originalError?: unknown
  ) {
    super(message);
    this.name = 'TTSError';
  }
}

/**
 * Response from the Tauri translate command
 * Note: Rust uses #[serde(rename_all = "camelCase")] so fields are camelCase
 */
interface TauriTranslateResponse {
  translatedText: string;
  detectedLanguage?: string;
}


/**
 * Translates text using Tauri backend (Google Translate API)
 *
 * @param text - The text to translate
 * @param options - Translation options (source and target languages)
 * @returns Promise resolving to the translation result
 * @throws TranslationError if translation fails
 *
 * @example
 * ```ts
 * const result = await translateText('Hello', { to: 'es' });
 * console.log(result.translatedText); // "Hola"
 * ```
 */
export async function translateText(
  text: string,
  options: TranslationOptions
): Promise<TranslationResult> {
  // Handle empty text - return early without API call
  if (!text || text.trim() === '') {
    return {
      translatedText: '',
      detectedLanguage: undefined,
      pronunciation: undefined,
    };
  }

  try {
    const result = await invoke<TauriTranslateResponse>('translate', {
      text,
      from: options.from ?? 'auto',
      to: options.to,
    });

    return {
      translatedText: result.translatedText,
      detectedLanguage: result.detectedLanguage,
      pronunciation: undefined,
    };
  } catch (error) {
    // Handle Tauri command errors
    if (error instanceof Error) {
      throw new TranslationError(
        `Translation failed: ${error.message}`,
        error
      );
    }

    // Handle string errors from Tauri
    if (typeof error === 'string') {
      throw new TranslationError(`Translation failed: ${error}`, error);
    }

    throw new TranslationError(
      'Translation failed: An unexpected error occurred',
      error
    );
  }
}

/**
 * Plays text using Tauri backend (Google Text-to-Speech API)
 *
 * @param text - The text to convert to speech (max 200 characters)
 * @param languageCode - The language code for the voice (e.g., 'en', 'es', 'ja')
 * @returns Promise that resolves when audio starts playing
 * @throws TTSError if TTS fails or text exceeds 200 characters
 *
 * @example
 * ```ts
 * await playTextToSpeech('Hello world', 'en');
 * ```
 */
export async function playTextToSpeech(
  text: string,
  languageCode: string
): Promise<void> {
  // Handle empty text
  if (!text || text.trim() === '') {
    return;
  }

  // Validate text length (Google TTS API limit)
  if (text.length > 200) {
    throw new TTSError('Text exceeds 200 character limit for text-to-speech');
  }

  try {
    const base64MP3 = await invoke<string>('speak', {
      text,
      languageCode,
    });

    // Convert Base64 to Blob
    const binaryString = atob(base64MP3);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    const blob = new Blob([bytes], { type: 'audio/mpeg' });

    // Create audio element and play
    const url = URL.createObjectURL(blob);
    const audio = new Audio(url);

    // Clean up object URL when audio finishes playing
    audio.addEventListener('ended', () => {
      URL.revokeObjectURL(url);
    });

    // Clean up on error
    audio.addEventListener('error', () => {
      URL.revokeObjectURL(url);
    });

    await audio.play();
  } catch (error) {
    // Re-throw TTSError as-is
    if (error instanceof TTSError) {
      throw error;
    }

    // Handle Tauri command errors
    if (error instanceof Error) {
      throw new TTSError(
        `Text-to-speech failed: ${error.message}`,
        error
      );
    }

    // Handle string errors from Tauri
    if (typeof error === 'string') {
      throw new TTSError(`Text-to-speech failed: ${error}`, error);
    }

    throw new TTSError(
      'Text-to-speech failed: An unexpected error occurred',
      error
    );
  }
}

/**
 * List of commonly used languages for the translation UI
 * Includes Auto Detect option for source language selection
 */
const SUPPORTED_LANGUAGES: LanguageOption[] = [
  { code: 'auto', name: 'Auto Detect' },
  { code: 'en', name: 'English' },
  { code: 'es', name: 'Spanish' },
  { code: 'fr', name: 'French' },
  { code: 'de', name: 'German' },
  { code: 'it', name: 'Italian' },
  { code: 'pt', name: 'Portuguese' },
  { code: 'ru', name: 'Russian' },
  { code: 'ja', name: 'Japanese' },
  { code: 'ko', name: 'Korean' },
  { code: 'zh-CN', name: 'Chinese (Simplified)' },
  { code: 'zh-TW', name: 'Chinese (Traditional)' },
  { code: 'ar', name: 'Arabic' },
  { code: 'hi', name: 'Hindi' },
  { code: 'vi', name: 'Vietnamese' },
  { code: 'th', name: 'Thai' },
  { code: 'id', name: 'Indonesian' },
  { code: 'ms', name: 'Malay' },
  { code: 'nl', name: 'Dutch' },
  { code: 'pl', name: 'Polish' },
  { code: 'tr', name: 'Turkish' },
  { code: 'uk', name: 'Ukrainian' },
  { code: 'el', name: 'Greek' },
  { code: 'he', name: 'Hebrew' },
  { code: 'sv', name: 'Swedish' },
  { code: 'da', name: 'Danish' },
  { code: 'fi', name: 'Finnish' },
  { code: 'no', name: 'Norwegian' },
  { code: 'cs', name: 'Czech' },
  { code: 'ro', name: 'Romanian' },
  { code: 'hu', name: 'Hungarian' },
  { code: 'bg', name: 'Bulgarian' },
];

/**
 * Returns the list of supported languages for the translation UI
 *
 * @returns Array of language options with code and name
 *
 * @example
 * ```ts
 * const languages = getSupportedLanguages();
 * // [{ code: 'auto', name: 'Auto Detect' }, { code: 'en', name: 'English' }, ...]
 * ```
 */
export function getSupportedLanguages(): LanguageOption[] {
  return SUPPORTED_LANGUAGES;
}

/**
 * Returns the list of target languages (excludes 'auto' option)
 *
 * @returns Array of language options available as target languages
 */
export function getTargetLanguages(): LanguageOption[] {
  return SUPPORTED_LANGUAGES.filter((lang) => lang.code !== 'auto');
}

/**
 * Finds a language by its code
 *
 * @param code - The language code to find
 * @returns The language option or undefined if not found
 */
export function getLanguageByCode(code: string): LanguageOption | undefined {
  return SUPPORTED_LANGUAGES.find((lang) => lang.code === code);
}
