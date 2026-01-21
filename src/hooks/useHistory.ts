import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { TranslationMetadata } from '../services/translationService';

/**
 * Represents a single translation history entry
 * Note: Rust uses #[serde(rename_all = "camelCase")] so all fields are camelCase
 */
export interface HistoryEntry {
  /** Unique identifier for the history entry */
  id: number;
  /** Original text that was translated */
  sourceText: string;
  /** Translated text result */
  translatedText: string;
  /** Source language code (e.g., 'en', 'auto') */
  sourceLanguage: string;
  /** Target language code (e.g., 'es', 'ja') */
  targetLanguage: string;
  /** Detected language code (if auto-detect was used) */
  detectedLanguage?: string;
  /** Timestamp when the translation was saved (ISO string) */
  createdAt: string;
  /** Translation metadata as JSON string stored in database */
  metadata?: string;
}

/**
 * Response from get_history command
 * Note: Rust uses #[serde(rename_all = "camelCase")] so all fields are camelCase
 */
interface GetHistoryResponse {
  entries: HistoryEntry[];
  total: number;
  limit: number;
  offset: number;
}

/**
 * Options for the useHistory hook
 */
export interface UseHistoryOptions {
  /** Number of entries to fetch per page (default: 20) */
  pageSize?: number;
  /** Whether to fetch history on mount (default: true) */
  fetchOnMount?: boolean;
}

/**
 * Return type for the useHistory hook
 */
export interface UseHistoryReturn {
  /** Array of history entries */
  entries: HistoryEntry[];
  /** Total number of entries in the database */
  total: number;
  /** Whether there are more entries to load */
  hasMore: boolean;
  /** Whether the hook is currently loading data */
  isLoading: boolean;
  /** Error object if an operation failed */
  error: Error | null;
  /** Current page number (0-indexed) */
  page: number;
  /** Fetch history entries with pagination */
  getHistory: (page?: number) => Promise<void>;
  /** Search history entries by query */
  searchHistory: (query: string) => Promise<void>;
  /** Delete a single history entry by ID */
  deleteItem: (id: number) => Promise<void>;
  /** Clear all history entries */
  clearAll: () => Promise<void>;
  /** Refresh the current page */
  refetch: () => Promise<void>;
  /** Go to the next page */
  nextPage: () => Promise<void>;
  /** Go to the previous page */
  prevPage: () => Promise<void>;
  /** Reset search and show all entries */
  resetSearch: () => Promise<void>;
  /** Current search query (empty if not searching) */
  searchQuery: string;
}

/**
 * Custom hook for managing translation history
 *
 * Provides functions to fetch, search, delete, and clear translation history
 * stored in SQLite via Tauri commands.
 *
 * @param options - Configuration options for the hook
 * @returns Object containing history entries, states, and operation functions
 *
 * @example
 * ```tsx
 * const {
 *   entries,
 *   isLoading,
 *   error,
 *   getHistory,
 *   searchHistory,
 *   deleteItem,
 *   clearAll,
 * } = useHistory({ pageSize: 20 });
 *
 * // Delete a single entry
 * await deleteItem(entryId);
 *
 * // Search for specific translations
 * await searchHistory('hello');
 * ```
 */
export function useHistory(options: UseHistoryOptions = {}): UseHistoryReturn {
  const { pageSize = 20, fetchOnMount = true } = options;

  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [page, setPage] = useState(0);
  const [searchQuery, setSearchQuery] = useState('');

  /**
   * Fetch history entries with pagination
   */
  const getHistory = useCallback(
    async (pageNum: number = 0) => {
      setIsLoading(true);
      setError(null);

      try {
        const offset = pageNum * pageSize;
        const response = await invoke<GetHistoryResponse>('get_history', {
          limit: pageSize,
          offset,
        });

        setEntries(response.entries);
        setTotal(response.total);
        // Calculate hasMore from response data
        setHasMore(response.offset + response.entries.length < response.total);
        setPage(pageNum);
        setSearchQuery('');
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : String(err);
        setError(new Error(`Failed to fetch history: ${errorMessage}`));
      } finally {
        setIsLoading(false);
      }
    },
    [pageSize]
  );

  /**
   * Search history entries by query
   */
  const searchHistory = useCallback(
    async (query: string) => {
      if (!query.trim()) {
        await getHistory(0);
        return;
      }

      setIsLoading(true);
      setError(null);

      try {
        const response = await invoke<GetHistoryResponse>('search_history', {
          query: query.trim(),
          limit: pageSize,
          offset: 0,
        });

        setEntries(response.entries);
        setTotal(response.total);
        // Calculate hasMore from response data
        setHasMore(response.offset + response.entries.length < response.total);
        setPage(0);
        setSearchQuery(query.trim());
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : String(err);
        setError(new Error(`Failed to search history: ${errorMessage}`));
      } finally {
        setIsLoading(false);
      }
    },
    [pageSize, getHistory]
  );

  /**
   * Delete a single history entry by ID
   */
  const deleteItem = useCallback(
    async (id: number) => {
      setIsLoading(true);
      setError(null);

      try {
        await invoke('delete_history', { id });

        // Remove the entry from local state
        setEntries((prev) => prev.filter((entry) => entry.id !== id));
        setTotal((prev) => Math.max(0, prev - 1));

        // If we deleted the last item on the page, go back a page
        if (entries.length === 1 && page > 0) {
          await getHistory(page - 1);
        }
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : String(err);
        setError(new Error(`Failed to delete entry: ${errorMessage}`));
      } finally {
        setIsLoading(false);
      }
    },
    [entries.length, page, getHistory]
  );

  /**
   * Clear all history entries
   */
  const clearAll = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      await invoke('clear_history');

      setEntries([]);
      setTotal(0);
      setHasMore(false);
      setPage(0);
      setSearchQuery('');
    } catch (err) {
      const errorMessage =
        err instanceof Error ? err.message : String(err);
      setError(new Error(`Failed to clear history: ${errorMessage}`));
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * Refresh the current page
   */
  const refetch = useCallback(async () => {
    if (searchQuery) {
      await searchHistory(searchQuery);
    } else {
      await getHistory(page);
    }
  }, [searchQuery, page, searchHistory, getHistory]);

  /**
   * Go to the next page
   */
  const nextPage = useCallback(async () => {
    if (hasMore) {
      await getHistory(page + 1);
    }
  }, [hasMore, page, getHistory]);

  /**
   * Go to the previous page
   */
  const prevPage = useCallback(async () => {
    if (page > 0) {
      await getHistory(page - 1);
    }
  }, [page, getHistory]);

  /**
   * Reset search and show all entries
   */
  const resetSearch = useCallback(async () => {
    setSearchQuery('');
    await getHistory(0);
  }, [getHistory]);

  // Fetch history on mount if enabled
  useEffect(() => {
    if (fetchOnMount) {
      getHistory(0);
    }
  }, [fetchOnMount, getHistory]);

  return {
    entries,
    total,
    hasMore,
    isLoading,
    error,
    page,
    getHistory,
    searchHistory,
    deleteItem,
    clearAll,
    refetch,
    nextPage,
    prevPage,
    resetSearch,
    searchQuery,
  };
}
