import { useState, useCallback, useEffect, useRef } from 'react';
import {
  FiSearch,
  FiTrash2,
  FiX,
  FiChevronLeft,
  FiChevronRight,
  FiAlertCircle,
  FiClock,
  FiArrowRight,
  FiRefreshCw,
} from 'react-icons/fi';
import { useHistory, HistoryEntry } from '../../hooks/useHistory';
import { getLanguageByCode } from '../../services/translationService';

interface HistoryPanelProps {
  className?: string;
  /** Callback when user wants to use a history entry for new translation */
  onSelectEntry?: (sourceText: string, translatedText: string, sourceLang: string, targetLang: string) => void;
}

/**
 * Format a timestamp to a human-readable relative time or date
 */
function formatTimestamp(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / (1000 * 60));
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffMins < 1) {
    return 'Just now';
  } else if (diffMins < 60) {
    return `${diffMins} min${diffMins > 1 ? 's' : ''} ago`;
  } else if (diffHours < 24) {
    return `${diffHours} hour${diffHours > 1 ? 's' : ''} ago`;
  } else if (diffDays < 7) {
    return `${diffDays} day${diffDays > 1 ? 's' : ''} ago`;
  } else {
    return date.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  }
}

/**
 * Get language display name from code
 */
function getLanguageName(code: string | undefined | null): string {
  if (!code) return 'Unknown';
  if (code === 'auto') return 'Auto';
  const lang = getLanguageByCode(code);
  return lang?.name ?? code.toUpperCase();
}

/**
 * Truncate text to a maximum length with ellipsis
 */
function truncateText(text: string | undefined | null, maxLength: number): string {
  if (!text) return '';
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength - 3) + '...';
}

/**
 * Single history entry card component
 */
function HistoryCard({
  entry,
  onDelete,
  onSelect,
  isDeleting,
}: {
  entry: HistoryEntry;
  onDelete: (id: number) => void;
  onSelect?: (sourceText: string, translatedText: string, sourceLang: string, targetLang: string) => void;
  isDeleting: boolean;
}) {
  const sourceLang = entry.detectedLanguage ?? entry.sourceLanguage ?? 'auto';

  return (
    <div
      className={`group bg-white dark:bg-gray-800 rounded-xl border border-gray-200
        dark:border-gray-700 shadow-sm hover:shadow-md transition-all duration-200
        ${isDeleting ? 'opacity-50 pointer-events-none' : ''}`}
    >
      {/* Header: Language pair and timestamp */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-gray-100 dark:border-gray-700/50">
        <div className="flex items-center gap-2 text-sm">
          <span className="px-2 py-0.5 bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400 rounded font-medium">
            {getLanguageName(sourceLang)}
          </span>
          <FiArrowRight
            className="w-4 h-4 text-gray-400 dark:text-gray-500"
            aria-hidden="true"
          />
          <span className="px-2 py-0.5 bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400 rounded font-medium">
            {getLanguageName(entry.targetLanguage)}
          </span>
        </div>

        <div className="flex items-center gap-3">
          <span className="flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400">
            <FiClock className="w-3 h-3" aria-hidden="true" />
            {formatTimestamp(entry.createdAt)}
          </span>

          <button
            type="button"
            onClick={() => onDelete(entry.id)}
            disabled={isDeleting}
            aria-label="Delete this history entry"
            className="p-1.5 rounded-lg text-gray-400 hover:text-red-500 hover:bg-red-50
              dark:hover:bg-red-900/20 transition-colors opacity-0 group-hover:opacity-100
              focus:opacity-100 focus:outline-none focus:ring-2 focus:ring-red-500"
          >
            <FiTrash2 className="w-4 h-4" aria-hidden="true" />
          </button>
        </div>
      </div>

      {/* Content: Source and translated text */}
      <div
        className={`p-4 ${onSelect ? 'cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700/50' : ''}`}
        onClick={() => onSelect?.(entry.sourceText, entry.translatedText, sourceLang, entry.targetLanguage)}
        role={onSelect ? 'button' : undefined}
        tabIndex={onSelect ? 0 : undefined}
        onKeyDown={(e) => {
          if (onSelect && (e.key === 'Enter' || e.key === ' ')) {
            e.preventDefault();
            onSelect(entry.sourceText, entry.translatedText, sourceLang, entry.targetLanguage);
          }
        }}
      >
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* Source text */}
          <div>
            <p className="text-xs font-medium text-gray-500 dark:text-gray-400 mb-1 uppercase tracking-wide">
              Original
            </p>
            <p className="text-gray-900 dark:text-gray-100 leading-relaxed">
              {truncateText(entry.sourceText, 200)}
            </p>
          </div>

          {/* Translated text */}
          <div>
            <p className="text-xs font-medium text-gray-500 dark:text-gray-400 mb-1 uppercase tracking-wide">
              Translation
            </p>
            <p className="text-gray-900 dark:text-gray-100 leading-relaxed">
              {truncateText(entry.translatedText, 200)}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

/**
 * Empty state component
 */
function EmptyState({ isSearch }: { isSearch: boolean }) {
  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <div className="w-16 h-16 rounded-full bg-gray-100 dark:bg-gray-800 flex items-center justify-center mb-4">
        <FiClock className="w-8 h-8 text-gray-400 dark:text-gray-500" />
      </div>
      <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
        {isSearch ? 'No results found' : 'No translation history'}
      </h3>
      <p className="text-gray-500 dark:text-gray-400 max-w-sm">
        {isSearch
          ? 'Try adjusting your search terms or clear the search to see all entries.'
          : 'Your translation history will appear here once you start translating.'}
      </p>
    </div>
  );
}

/**
 * Confirm dialog for clearing all history
 */
function ClearConfirmDialog({
  isOpen,
  onConfirm,
  onCancel,
  isClearing,
}: {
  isOpen: boolean;
  onConfirm: () => void;
  onCancel: () => void;
  isClearing: boolean;
}) {
  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      role="dialog"
      aria-modal="true"
      aria-labelledby="clear-dialog-title"
    >
      <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl p-6 max-w-md w-full mx-4 border border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-3 mb-4">
          <div className="w-10 h-10 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center">
            <FiAlertCircle className="w-5 h-5 text-red-600 dark:text-red-400" />
          </div>
          <h2
            id="clear-dialog-title"
            className="text-lg font-semibold text-gray-900 dark:text-gray-100"
          >
            Clear all history?
          </h2>
        </div>

        <p className="text-gray-600 dark:text-gray-400 mb-6">
          This will permanently delete all your translation history. This action
          cannot be undone.
        </p>

        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={onCancel}
            disabled={isClearing}
            className="px-4 py-2 rounded-lg font-medium text-gray-700 dark:text-gray-300
              bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600
              transition-colors focus:outline-none focus:ring-2 focus:ring-gray-500"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={isClearing}
            className="px-4 py-2 rounded-lg font-medium text-white bg-red-500
              hover:bg-red-600 transition-colors focus:outline-none focus:ring-2
              focus:ring-red-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800
              disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
          >
            {isClearing ? (
              <>
                <svg
                  className="animate-spin w-4 h-4"
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
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
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                  />
                </svg>
                Clearing...
              </>
            ) : (
              'Clear all'
            )}
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * HistoryPanel - Displays translation history with search, delete, and pagination
 *
 * Features:
 * - Search box to filter history entries
 * - Delete individual entries or clear all
 * - Pagination support for large history
 * - Click on entry to use it for new translation
 */
export function HistoryPanel({ className = '', onSelectEntry }: HistoryPanelProps) {
  const {
    entries,
    total,
    hasMore,
    isLoading,
    error,
    page,
    searchHistory,
    deleteItem,
    clearAll,
    refetch,
    nextPage,
    prevPage,
    resetSearch,
    searchQuery,
  } = useHistory({ pageSize: 10 });

  const [searchInput, setSearchInput] = useState('');
  const [showClearDialog, setShowClearDialog] = useState(false);
  const [deletingId, setDeletingId] = useState<number | null>(null);
  const [isClearing, setIsClearing] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const searchDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Debounced search
  useEffect(() => {
    if (searchDebounceRef.current) {
      clearTimeout(searchDebounceRef.current);
    }

    searchDebounceRef.current = setTimeout(() => {
      searchHistory(searchInput);
    }, 300);

    return () => {
      if (searchDebounceRef.current) {
        clearTimeout(searchDebounceRef.current);
      }
    };
  }, [searchInput, searchHistory]);

  // Handle delete entry
  const handleDelete = useCallback(
    async (id: number) => {
      setDeletingId(id);
      try {
        await deleteItem(id);
      } finally {
        setDeletingId(null);
      }
    },
    [deleteItem]
  );

  // Handle clear all
  const handleClearAll = useCallback(async () => {
    setIsClearing(true);
    try {
      await clearAll();
      setShowClearDialog(false);
      setSearchInput('');
    } finally {
      setIsClearing(false);
    }
  }, [clearAll]);

  // Handle clear search
  const handleClearSearch = useCallback(() => {
    setSearchInput('');
    resetSearch();
    searchInputRef.current?.focus();
  }, [resetSearch]);

  // Calculate pagination info
  const pageSize = 10;
  const startIndex = page * pageSize + 1;
  const endIndex = Math.min((page + 1) * pageSize, total);

  return (
    <div className={`w-full ${className}`}>
      {/* Header: Search and Clear All */}
      <div className="flex flex-col sm:flex-row gap-4 mb-6">
        {/* Search box */}
        <div className="flex-1 relative">
          <label htmlFor="history-search" className="sr-only">
            Search translation history
          </label>
          <FiSearch
            className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400 dark:text-gray-500"
            aria-hidden="true"
          />
          <input
            ref={searchInputRef}
            id="history-search"
            type="text"
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            placeholder="Search translations..."
            className="w-full pl-10 pr-10 py-3 rounded-xl border-2 border-gray-200 dark:border-gray-700
              bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100
              placeholder-gray-400 dark:placeholder-gray-500
              focus:outline-none focus:border-amber-500 dark:focus:border-amber-500
              transition-colors"
          />
          {searchInput && (
            <button
              type="button"
              onClick={handleClearSearch}
              aria-label="Clear search"
              className="absolute right-3 top-1/2 -translate-y-1/2 p-1 rounded-full
                text-gray-400 hover:text-gray-600 dark:hover:text-gray-300
                hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
            >
              <FiX className="w-4 h-4" aria-hidden="true" />
            </button>
          )}
        </div>

        {/* Action buttons */}
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => refetch()}
            disabled={isLoading}
            aria-label="Refresh history"
            className="px-4 py-3 rounded-xl font-medium text-gray-700 dark:text-gray-300
              bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600
              transition-colors focus:outline-none focus:ring-2 focus:ring-amber-500
              disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <FiRefreshCw
              className={`w-5 h-5 ${isLoading ? 'animate-spin' : ''}`}
              aria-hidden="true"
            />
          </button>

          <button
            type="button"
            onClick={() => setShowClearDialog(true)}
            disabled={total === 0 || isLoading}
            className="px-4 py-3 rounded-xl font-medium text-red-600 dark:text-red-400
              bg-red-50 dark:bg-red-900/20 hover:bg-red-100 dark:hover:bg-red-900/30
              transition-colors focus:outline-none focus:ring-2 focus:ring-red-500
              disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
          >
            <FiTrash2 className="w-5 h-5" aria-hidden="true" />
            <span className="hidden sm:inline">Clear all</span>
          </button>
        </div>
      </div>

      {/* Error state */}
      {error && (
        <div className="mb-6 p-4 rounded-xl bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800">
          <div className="flex items-center gap-2 text-red-600 dark:text-red-400">
            <FiAlertCircle className="w-5 h-5" aria-hidden="true" />
            <span className="font-medium">Error loading history</span>
          </div>
          <p className="mt-1 text-sm text-red-500 dark:text-red-400">
            {error.message}
          </p>
          <button
            type="button"
            onClick={() => refetch()}
            className="mt-3 px-4 py-2 rounded-lg font-medium text-white bg-red-500
              hover:bg-red-600 transition-colors focus:outline-none focus:ring-2
              focus:ring-red-500 text-sm"
          >
            Try again
          </button>
        </div>
      )}

      {/* Loading state (initial load) */}
      {isLoading && entries.length === 0 && !error && (
        <div className="flex items-center justify-center py-16">
          <div className="flex items-center gap-3 text-amber-600 dark:text-amber-400">
            <svg
              className="animate-spin w-8 h-8"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
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
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
              />
            </svg>
            <span className="text-lg font-medium">Loading history...</span>
          </div>
        </div>
      )}

      {/* Empty state */}
      {!isLoading && !error && entries.length === 0 && (
        <EmptyState isSearch={!!searchQuery} />
      )}

      {/* History entries list */}
      {entries.length > 0 && (
        <>
          {/* Results count */}
          <div className="mb-4 text-sm text-gray-500 dark:text-gray-400">
            {searchQuery ? (
              <span>
                Found {total} result{total !== 1 ? 's' : ''} for "{searchQuery}"
              </span>
            ) : (
              <span>
                Showing {startIndex}-{endIndex} of {total} translation
                {total !== 1 ? 's' : ''}
              </span>
            )}
          </div>

          {/* Entry cards */}
          <div className="space-y-4">
            {entries.map((entry) => (
              <HistoryCard
                key={entry.id}
                entry={entry}
                onDelete={handleDelete}
                onSelect={onSelectEntry}
                isDeleting={deletingId === entry.id}
              />
            ))}
          </div>

          {/* Pagination */}
          {(page > 0 || hasMore) && (
            <div className="flex items-center justify-between mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
              <button
                type="button"
                onClick={prevPage}
                disabled={page === 0 || isLoading}
                className="flex items-center gap-2 px-4 py-2 rounded-lg font-medium
                  text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700
                  hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors
                  focus:outline-none focus:ring-2 focus:ring-amber-500
                  disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <FiChevronLeft className="w-5 h-5" aria-hidden="true" />
                Previous
              </button>

              <span className="text-sm text-gray-500 dark:text-gray-400">
                Page {page + 1}
              </span>

              <button
                type="button"
                onClick={nextPage}
                disabled={!hasMore || isLoading}
                className="flex items-center gap-2 px-4 py-2 rounded-lg font-medium
                  text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700
                  hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors
                  focus:outline-none focus:ring-2 focus:ring-amber-500
                  disabled:opacity-50 disabled:cursor-not-allowed"
              >
                Next
                <FiChevronRight className="w-5 h-5" aria-hidden="true" />
              </button>
            </div>
          )}
        </>
      )}

      {/* Clear confirmation dialog */}
      <ClearConfirmDialog
        isOpen={showClearDialog}
        onConfirm={handleClearAll}
        onCancel={() => setShowClearDialog(false)}
        isClearing={isClearing}
      />
    </div>
  );
}
