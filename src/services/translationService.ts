import { invoke } from '@tauri-apps/api/core';

/**
 * Alternative translation for a word
 */
export interface AlternativeTranslation {
  word: string;
}

/**
 * Definition entry with gloss and optional example
 */
export interface DefinitionEntry {
  gloss: string;
  example?: string;
}

/**
 * Definition group with part of speech
 */
export interface Definition {
  partOfSpeech: string;
  entries: DefinitionEntry[];
}

/**
 * Example sentence
 */
export interface TranslationExample {
  text: string;
}

/**
 * Synonym entry
 */
export interface Synonym {
  word: string;
}

/**
 * Related word entry
 */
export interface RelatedWord {
  word: string;
}

/**
 * Translation metadata containing examples, definitions, alternatives, synonyms, related words, and transliteration
 */
export interface TranslationMetadata {
  examples: TranslationExample[];
  definitions: Definition[];
  alternatives: AlternativeTranslation[];
  synonyms: Synonym[];
  relatedWords: RelatedWord[];
  transliteration?: string;
}

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
  /** Translation metadata (examples, definitions, alternatives) */
  metadata?: TranslationMetadata;
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
  metadata?: TranslationMetadata;
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
      metadata: result.metadata,
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
 * @param text - The text to convert to speech (truncated to 200 characters if longer)
 * @param languageCode - The language code for the voice (e.g., 'en', 'es', 'ja')
 * @returns Promise that resolves to the Audio element when audio starts playing
 * @throws TTSError if TTS fails
 *
 * @example
 * ```ts
 * const audio = await playTextToSpeech('Hello world', 'en');
 * // Later you can stop it: audio.pause(); audio.currentTime = 0;
 * ```
 */
export async function playTextToSpeech(
  text: string,
  languageCode: string
): Promise<HTMLAudioElement> {
  // Truncate to 200 chars for Google TTS API limit (handle internally)
  const textToSpeak = text.length > 200 ? text.slice(0, 200) : text;

  try {
    const base64MP3 = await invoke<string>('speak', {
      text: textToSpeak,
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
    return audio;
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
 * Sorted alphabetically by name, with Auto Detect at the top for source selection
 */
const LANGUAGES_SORTED: LanguageOption[] = [
  { code: 'ar', name: 'Arabic' },
  { code: 'bg', name: 'Bulgarian' },
  { code: 'zh-CN', name: 'Chinese (Simplified)' },
  { code: 'zh-TW', name: 'Chinese (Traditional)' },
  { code: 'cs', name: 'Czech' },
  { code: 'da', name: 'Danish' },
  { code: 'nl', name: 'Dutch' },
  { code: 'en', name: 'English' },
  { code: 'fi', name: 'Finnish' },
  { code: 'fr', name: 'French' },
  { code: 'de', name: 'German' },
  { code: 'el', name: 'Greek' },
  { code: 'he', name: 'Hebrew' },
  { code: 'hi', name: 'Hindi' },
  { code: 'hu', name: 'Hungarian' },
  { code: 'id', name: 'Indonesian' },
  { code: 'it', name: 'Italian' },
  { code: 'ja', name: 'Japanese' },
  { code: 'ko', name: 'Korean' },
  { code: 'ms', name: 'Malay' },
  { code: 'no', name: 'Norwegian' },
  { code: 'pl', name: 'Polish' },
  { code: 'pt', name: 'Portuguese' },
  { code: 'ro', name: 'Romanian' },
  { code: 'ru', name: 'Russian' },
  { code: 'es', name: 'Spanish' },
  { code: 'sv', name: 'Swedish' },
  { code: 'th', name: 'Thai' },
  { code: 'tr', name: 'Turkish' },
  { code: 'uk', name: 'Ukrainian' },
  { code: 'vi', name: 'Vietnamese' },
];

/** Auto Detect option for source language */
const AUTO_DETECT: LanguageOption = { code: 'auto', name: 'Auto Detect' };

/** Full list with Auto Detect at top */
const SUPPORTED_LANGUAGES: LanguageOption[] = [AUTO_DETECT, ...LANGUAGES_SORTED];

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
 * @returns Array of language options available as target languages (sorted alphabetically)
 */
export function getTargetLanguages(): LanguageOption[] {
  return LANGUAGES_SORTED;
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
