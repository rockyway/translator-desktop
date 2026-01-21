import { memo } from 'react';

// ============================================================================
// Types
// ============================================================================

export interface StatusBarProps {
  isConnected: boolean;
  textMonitorVersion: string | null;
}

// ============================================================================
// Main Component
// ============================================================================

/**
 * Minimal status bar at the bottom of the app window.
 *
 * Displays connection status to the text monitor service with a sleek,
 * unobtrusive design that complements the app's gold/amber theme.
 */
export const StatusBar = memo(function StatusBar({
  isConnected,
  textMonitorVersion,
}: StatusBarProps) {
  return (
    <footer
      className="
        flex-shrink-0 h-6
        bg-gray-900 dark:bg-gray-950
        border-t border-gray-800/60
        flex items-center justify-between
        px-3
        select-none
      "
      role="contentinfo"
      aria-label="Application status"
    >
      {/* Left side - Connection status */}
      <div className="flex items-center gap-2">
        {/* Status indicator dot */}
        <span className="relative flex h-2 w-2">
          <span
            className={`
              absolute inline-flex h-full w-full rounded-full opacity-60
              ${isConnected ? 'bg-emerald-400 animate-pulse' : 'bg-amber-400'}
            `}
            style={{ animationDuration: isConnected ? '2.5s' : undefined }}
          />
          <span
            className={`
              relative inline-flex rounded-full h-2 w-2
              ${isConnected ? 'bg-emerald-500' : 'bg-amber-500'}
            `}
          />
        </span>

        {/* Status text */}
        <span
          className={`
            text-xs font-medium tracking-wide
            ${isConnected ? 'text-emerald-400/90' : 'text-amber-400/90'}
          `}
        >
          {isConnected ? 'Connected' : 'Disconnected'}
        </span>

        {/* Separator and version when connected */}
        {isConnected && textMonitorVersion && (
          <>
            <span className="text-gray-600 text-xs">•</span>
            <span className="text-xs text-gray-500 font-mono">
              v{textMonitorVersion}
            </span>
          </>
        )}
      </div>

    </footer>
  );
});

export default StatusBar;
