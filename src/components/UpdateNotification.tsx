import { useUpdater } from '../hooks/useUpdater';

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

export function UpdateNotification() {
  const {
    available,
    version,
    body,
    downloading,
    progress,
    downloaded,
    total,
    error,
    checking,
    downloadAndInstall,
    dismissUpdate,
  } = useUpdater(true);

  // Don't show anything if no update available and not checking
  if (!available && !checking && !error) return null;

  // Show checking state briefly
  if (checking) {
    return (
      <div className="fixed bottom-4 right-4 z-50 bg-gray-800 text-white px-4 py-3 rounded-lg shadow-lg">
        <div className="flex items-center gap-2">
          <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
          <span className="text-sm">Checking for updates...</span>
        </div>
      </div>
    );
  }

  // Show error state
  if (error && !available) {
    return null; // Silently fail - don't bother user with update errors
  }

  // No update available
  if (!available) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 bg-gradient-to-r from-amber-500 to-amber-600 text-black p-4 rounded-xl shadow-2xl max-w-sm animate-in slide-in-from-bottom-4">
      {/* Header */}
      <div className="flex items-start justify-between gap-2">
        <div className="flex items-center gap-2">
          <svg
            className="w-5 h-5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
            />
          </svg>
          <span className="font-semibold">Update Available</span>
        </div>
        {!downloading && (
          <button
            onClick={dismissUpdate}
            className="text-black/60 hover:text-black transition-colors"
            aria-label="Dismiss"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        )}
      </div>

      {/* Version info */}
      <div className="mt-2 text-sm">
        <span className="font-medium">Version {version}</span> is ready to install.
      </div>

      {/* Release notes (truncated) */}
      {body && (
        <div className="mt-2 text-xs text-black/70 line-clamp-2">
          {body}
        </div>
      )}

      {/* Progress bar when downloading */}
      {downloading && (
        <div className="mt-3">
          <div className="h-2 bg-black/20 rounded-full overflow-hidden">
            <div
              className="h-full bg-black transition-all duration-300 ease-out"
              style={{ width: `${progress}%` }}
            />
          </div>
          <div className="flex justify-between text-xs mt-1 text-black/70">
            <span>Downloading...</span>
            <span>
              {formatBytes(downloaded)} / {formatBytes(total)}
            </span>
          </div>
        </div>
      )}

      {/* Error message */}
      {error && (
        <div className="mt-2 text-xs text-red-800 bg-red-100 px-2 py-1 rounded">
          {error}
        </div>
      )}

      {/* Action button */}
      {!downloading && (
        <button
          onClick={downloadAndInstall}
          className="mt-3 w-full px-4 py-2 bg-black text-amber-500 rounded-lg hover:bg-gray-900 transition-colors text-sm font-medium flex items-center justify-center gap-2"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
            />
          </svg>
          Install & Restart
        </button>
      )}
    </div>
  );
}
