import { useCallback, useEffect, useRef, useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import {
  translateText,
  getDictionaryMetadata,
  DICTIONARY_LOOKUP_MAX_WORDS,
  TranslationResult,
  TranslationError,
  TranslationMetadata,
} from '../services/translationService';

/**
 * Options for the useTranslation hook
 */
export interface UseTranslationOptions {
  /** Source language code (e.g., 'en', 'auto' for auto-detect) */
  sourceLanguage: string;
  /** Target language code (e.g., 'es', 'ja') */
  targetLanguage: string;
  /** Debounce delay in milliseconds (default: 500ms) */
  debounceMs?: number;
}

/**
 * Return type for the useTranslation hook
 */
export interface UseTranslationReturn {
  /** Function to trigger translation */
  translate: (text: string) => void;
  /** The translated text result */
  translatedText: string;
  /** The detected source language (if auto-detect was used) */
  detectedLanguage?: string;
  /** Translation metadata (examples, definitions, alternatives) */
  metadata?: TranslationMetadata;
  /** Whether a translation is in progress */
  isLoading: boolean;
  /** Error object if translation failed */
  error: Error | null;
  /** Function to reset the translation state */
  reset: () => void;
}

/**
 * Custom hook for text translation with debouncing and React Query integration
 *
 * Features:
 * - Debounced translation requests to avoid excessive API calls
 * - Loading and error state management
 * - Automatic cleanup on unmount
 * - Reset functionality
 *
 * @param options - Configuration options for the hook
 * @returns Object containing translate function, states, and reset
 *
 * @example
 * ```tsx
 * const { translate, translatedText, isLoading, error, reset } = useTranslation({
 *   sourceLanguage: 'auto',
 *   targetLanguage: 'es',
 *   debounceMs: 500,
 * });
 *
 * // Trigger translation
 * translate('Hello world');
 *
 * // Display result
 * if (isLoading) return <Spinner />;
 * if (error) return <Error message={error.message} />;
 * return <div>{translatedText}</div>;
 * ```
 */
export function useTranslation(
  options: UseTranslationOptions
): UseTranslationReturn {
  const { sourceLanguage, targetLanguage, debounceMs = 500 } = options;

  // Store the latest text to translate
  const [pendingText, setPendingText] = useState<string>('');

  // Supplementary dictionary metadata (definitions/examples/synonyms), fetched separately
  // from the translation itself so a slow lookup never delays the translated text.
  const [metadata, setMetadata] = useState<TranslationMetadata | undefined>(undefined);

  // Track the text that was last successfully translated (to match with mutation result)
  const lastTranslatedTextRef = useRef<string>('');

  // Debounce timeout ref
  const debounceTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Track if component is mounted
  const isMountedRef = useRef(true);

  // Guards against a stale metadata fetch overwriting state after a newer translation started
  const metadataRequestIdRef = useRef(0);

  // Fetches supplementary dictionary metadata (definitions/examples/synonyms/antonyms) for
  // short text and saves the translation to history. Deliberately NOT awaited by onSuccess
  // below: TanStack Query's mutation only dispatches its "success" state (which flips
  // isPending to false and populates `data`) *after* onSuccess's returned promise settles.
  // Awaiting this here previously meant the whole popup sat on "Translating..." for as long
  // as the dictionary lookup + history write took, even though the translation itself had
  // already come back.
  const fetchMetadataAndSaveHistory = useCallback(
    async (sourceText: string, data: TranslationResult) => {
      const wordCount = sourceText.split(/\s+/).length;
      const requestId = ++metadataRequestIdRef.current;

      let fetchedMetadata: TranslationMetadata | undefined;
      if (wordCount <= DICTIONARY_LOOKUP_MAX_WORDS) {
        const lookupLanguage =
          sourceLanguage !== 'auto' ? sourceLanguage : data.detectedLanguage ?? 'en';
        fetchedMetadata = await getDictionaryMetadata(sourceText, lookupLanguage);

        if (isMountedRef.current && requestId === metadataRequestIdRef.current) {
          setMetadata(fetchedMetadata);
        }
      }

      try {
        await invoke('add_history', {
          input: {
            sourceText: sourceText,
            translatedText: data.translatedText,
            sourceLanguage: sourceLanguage,
            targetLanguage: targetLanguage,
            detectedLanguage: data.detectedLanguage,
            metadata: fetchedMetadata ? JSON.stringify(fetchedMetadata) : undefined,
          }
        });
      } catch (error) {
        // Silently log error - don't break the translation flow
        console.error('Failed to save translation to history:', error);
      }
    },
    [sourceLanguage, targetLanguage]
  );

  // Translation mutation using React Query
  const mutation = useMutation<
    TranslationResult,
    TranslationError | Error,
    string
  >({
    mutationFn: async (text: string) => {
      return translateText(text, {
        from: sourceLanguage,
        to: targetLanguage,
      });
    },
    onSuccess: (data, sourceText) => {
      // Save to history using the sourceText from the mutation call (captured at mutation time)
      // This prevents the race condition where pendingText changes before translation completes
      lastTranslatedTextRef.current = sourceText;
      setMetadata(undefined);

      const trimmedText = sourceText.trim();
      if (!trimmedText) return;

      // Fire-and-forget - see the comment on fetchMetadataAndSaveHistory above.
      void fetchMetadataAndSaveHistory(trimmedText, data);
    },
  });

  // Extract stable references from mutation
  const { reset: resetMutation, mutate } = mutation;

  // Cleanup on unmount
  useEffect(() => {
    isMountedRef.current = true;

    return () => {
      isMountedRef.current = false;
      if (debounceTimeoutRef.current !== null) {
        clearTimeout(debounceTimeoutRef.current);
      }
    };
  }, []);

  // Handle debounced translation when pendingText changes
  useEffect(() => {
    // Clear any existing timeout
    if (debounceTimeoutRef.current !== null) {
      clearTimeout(debounceTimeoutRef.current);
      debounceTimeoutRef.current = null;
    }

    // Don't translate empty strings
    if (!pendingText || pendingText.trim() === '') {
      return;
    }

    // Set up debounced translation
    debounceTimeoutRef.current = setTimeout(() => {
      if (isMountedRef.current) {
        mutate(pendingText);
      }
    }, debounceMs);

    return () => {
      if (debounceTimeoutRef.current !== null) {
        clearTimeout(debounceTimeoutRef.current);
      }
    };
  }, [pendingText, debounceMs, sourceLanguage, targetLanguage, mutate]);

  // Translate function exposed to consumers
  const translate = useCallback((text: string) => {
    setPendingText(text);
  }, []);

  // Reset function to clear all state
  const reset = useCallback(() => {
    setPendingText('');
    setMetadata(undefined);
    metadataRequestIdRef.current++;
    resetMutation();
    if (debounceTimeoutRef.current !== null) {
      clearTimeout(debounceTimeoutRef.current);
      debounceTimeoutRef.current = null;
    }
  }, [resetMutation]);

  return {
    translate,
    translatedText: mutation.data?.translatedText ?? '',
    detectedLanguage: mutation.data?.detectedLanguage,
    metadata,
    isLoading: mutation.isPending,
    error: mutation.error ?? null,
    reset,
  };
}
