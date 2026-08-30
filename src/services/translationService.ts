import { invoke } from '@tauri-apps/api/core';
import { activeSpeechProvider, SpeechHandle } from './speechProvider';

export type { SpeechHandle };

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
 * Antonym entry
 */
export interface Antonym {
  word: string;
}

/**
 * Related word entry
 */
export interface RelatedWord {
  word: string;
}

/**
 * Translation metadata containing examples, definitions, alternatives, synonyms, antonyms, related words, and transliteration
 */
export interface TranslationMetadata {
  examples: TranslationExample[];
  definitions: Definition[];
  alternatives: AlternativeTranslation[];
  synonyms: Synonym[];
  antonyms: Antonym[];
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
}

/** Maximum number of words in the source text to still attempt a dictionary lookup */
export const DICTIONARY_LOOKUP_MAX_WORDS = 3;

/**
 * Looks up dictionary metadata (definitions, examples, synonyms, pronunciation) for a
 * short word/phrase via the Tauri backend (freedictionaryapi.com). Never throws - a
 * lookup failure just means no metadata, since it's a supplementary feature.
 *
 * Callers should fetch this separately from {@link translateText} rather than await
 * it inline - freedictionaryapi.com can be slow (especially the first request of a
 * session, before its connection is warm) and must not block the translated text
 * from being displayed.
 */
export async function getDictionaryMetadata(
  word: string,
  language: string
): Promise<TranslationMetadata | undefined> {
  try {
    const result = await invoke<TranslationMetadata | null>(
      'get_dictionary_metadata',
      { word, language }
    );
    return result ?? undefined;
  } catch (error) {
    console.error('Failed to fetch dictionary metadata:', error);
    return undefined;
  }
}

/**
 * Translates text using Tauri backend (Google Cloud Translation API)
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

    // Dictionary metadata (definitions/examples/synonyms) is fetched separately by the
    // caller via getDictionaryMetadata() - it must not block the translated text result.
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
 * Plays text as speech via the active {@link SpeechProvider} (see `speechProvider.ts`).
 *
 * @param text - The text to convert to speech
 * @param languageCode - The language code for the voice (e.g., 'en', 'es', 'ja')
 * @returns Promise that resolves to a handle for controlling playback
 * @throws TTSError if TTS fails
 *
 * @example
 * ```ts
 * const handle = await playTextToSpeech('Hello world', 'en');
 * // Later you can stop it: handle.pause(); handle.currentTime = 0;
 * ```
 */
export async function playTextToSpeech(
  text: string,
  languageCode: string
): Promise<SpeechHandle> {
  try {
    return await activeSpeechProvider.speak(text, languageCode);
  } catch (error) {
    if (error instanceof Error) {
      throw new TTSError(`Text-to-speech failed: ${error.message}`, error);
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
