import { invoke } from '@tauri-apps/api/core';

/**
 * Text-to-speech provider abstraction.
 *
 * Lets the app swap how word/sentence audio is generated without touching call
 * sites in the UI. To add another provider, implement `SpeechProvider`, export
 * it alongside the existing ones below, and point `activeSpeechProvider` at it.
 */

/**
 * Minimal subset of the HTMLAudioElement interface the UI needs to track playback.
 * Every provider returns one of these regardless of how it actually produces audio.
 */
export interface SpeechHandle {
  addEventListener(type: 'ended' | 'error', listener: () => void): void;
  pause(): void;
  currentTime: number;
}

export interface SpeechProvider {
  /**
   * Speaks the given text in the given language and returns a handle to control
   * playback. Should reject if speech could not be started at all.
   */
  speak(text: string, languageCode: string): Promise<SpeechHandle>;
}

/**
 * Wraps a SpeechSynthesisUtterance so it satisfies the SpeechHandle interface.
 */
class WebSpeechHandle implements SpeechHandle {
  private endedListeners: Array<() => void> = [];
  private errorListeners: Array<() => void> = [];

  constructor(utterance: SpeechSynthesisUtterance) {
    utterance.addEventListener('end', () => {
      this.endedListeners.forEach((listener) => listener());
    });
    utterance.addEventListener('error', () => {
      this.errorListeners.forEach((listener) => listener());
    });
  }

  addEventListener(type: 'ended' | 'error', listener: () => void): void {
    if (type === 'ended') {
      this.endedListeners.push(listener);
    } else {
      this.errorListeners.push(listener);
    }
  }

  pause(): void {
    window.speechSynthesis.cancel();
  }

  // No-op: playback position isn't meaningful for speechSynthesis, but callers
  // set this to reset state alongside pause().
  currentTime = 0;
}

/**
 * Uses the browser's built-in Web Speech API (SpeechSynthesis). Free, offline,
 * no network call - but voice/language coverage depends entirely on what's
 * installed on the OS, which on a typical Windows machine is often just the
 * system's display language.
 */
export const webSpeechProvider: SpeechProvider = {
  speak(text: string, languageCode: string): Promise<SpeechHandle> {
    return new Promise((resolve, reject) => {
      if (!('speechSynthesis' in window)) {
        reject(new Error('Speech synthesis is not supported in this environment'));
        return;
      }

      const utterance = new SpeechSynthesisUtterance(text);
      utterance.lang = languageCode;
      utterance.voice = pickVoiceForLanguage(languageCode);

      const handle = new WebSpeechHandle(utterance);

      window.speechSynthesis.speak(utterance);
      resolve(handle);
    });
  },
};

/**
 * Finds the best installed voice for a (possibly bare, e.g. "vi") language code,
 * preferring an exact BCP-47 match, then a same-language prefix match. Relying on
 * `utterance.lang` alone is unreliable - browsers silently fall back to a default
 * voice when there's no exact tag match, even if a same-language voice exists.
 */
function pickVoiceForLanguage(languageCode: string): SpeechSynthesisVoice | null {
  const voices = window.speechSynthesis.getVoices();
  const lower = languageCode.toLowerCase();
  return (
    voices.find((v) => v.lang.toLowerCase() === lower) ??
    voices.find((v) => v.lang.toLowerCase().startsWith(`${lower}-`)) ??
    voices.find((v) => v.lang.toLowerCase().split('-')[0] === lower.split('-')[0]) ??
    null
  );
}

/**
 * In-memory cache of base64 MP3 audio keyed by "voice:text", so repeating the
 * same word/sentence in the same session replays instantly instead of hitting
 * the TTS server again. Cleared on app reload - not persisted to disk.
 *
 * Bounded two ways so it can't grow unchecked in this long-lived, tray-resident
 * app: a hard cap on entry count (LRU eviction via Map's insertion order - the
 * oldest key is evicted first), plus a sliding TTL so entries that stop being
 * reused free up even while under the cap.
 */
const TTS_CACHE_MAX_ENTRIES = 50;
const TTS_CACHE_TTL_MS = 10 * 60 * 1000; // 10 minutes since last use

interface TtsCacheEntry {
  base64Mp3: string;
  expiresAt: number;
}

const ttsCache = new Map<string, TtsCacheEntry>();

function getCachedTts(key: string): string | undefined {
  const entry = ttsCache.get(key);
  if (!entry) return undefined;

  if (Date.now() > entry.expiresAt) {
    ttsCache.delete(key);
    return undefined;
  }

  // Re-insert to mark as most-recently-used and refresh the sliding TTL.
  ttsCache.delete(key);
  ttsCache.set(key, { base64Mp3: entry.base64Mp3, expiresAt: Date.now() + TTS_CACHE_TTL_MS });
  return entry.base64Mp3;
}

function setCachedTts(key: string, base64Mp3: string): void {
  if (ttsCache.size >= TTS_CACHE_MAX_ENTRIES) {
    const oldestKey = ttsCache.keys().next().value;
    if (oldestKey !== undefined) {
      ttsCache.delete(oldestKey);
    }
  }
  ttsCache.set(key, { base64Mp3, expiresAt: Date.now() + TTS_CACHE_TTL_MS });
}

/**
 * Uses the official Google Cloud Text-to-Speech API (Chirp 3: HD voices) - see the
 * "Text-to-Speech" section in Settings for the API key. Covers far more languages
 * and higher voice quality than locally installed OS voices, at the cost of a paid
 * Google Cloud API call per (uncached) request.
 */
export const googleTtsProvider: SpeechProvider = {
  async speak(text: string, languageCode: string): Promise<SpeechHandle> {
    const cacheKey = `${languageCode.toLowerCase()}:${text}`;

    let base64Mp3 = getCachedTts(cacheKey);
    if (!base64Mp3) {
      base64Mp3 = await invoke<string>('speak_google_tts', { text, languageCode });
      setCachedTts(cacheKey, base64Mp3);
    }

    const binaryString = atob(base64Mp3);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    const blob = new Blob([bytes], { type: 'audio/mpeg' });

    const url = URL.createObjectURL(blob);
    const audio = new Audio(url);
    audio.addEventListener('ended', () => URL.revokeObjectURL(url));
    audio.addEventListener('error', () => URL.revokeObjectURL(url));

    await audio.play();
    return audio;
  },
};

/** The provider currently used for all text-to-speech playback. */
export const activeSpeechProvider: SpeechProvider = googleTtsProvider;
