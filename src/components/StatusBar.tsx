import { memo } from 'react';

// ============================================================================
// Types
// ============================================================================

export interface StatusBarProps {
  isConnected: boolean;
  textMonitorVersion: string | null;
  needsAccessibilityPermission?: boolean;
}

// ============================================================================
// Main Component
// ============================================================================

/**
 * Minimal status bar at the bottom of the app window.
 *
 * Displays text monitor status:
 * - Windows: "Connected" (IPC to .NET sidecar) / "Disconnected"
 * - macOS: "Text Monitor Active" / "Hotkey Only" (if accessibility not granted)
 */
export const StatusBar = memo(function StatusBar({
  isConnected,
  textMonitorVersion,
  needsAccessibilityPermission: _needsAccessibilityPermission = false,
}: StatusBarProps) {
  const IS_MACOS = navigator.platform.toUpperCase().includes("MAC") ||
    navigator.userAgent.toUpperCase().includes("MAC");

  // Determine display state
  // On macOS: show "Text Monitor Active" when connected, "Hotkey Only" otherwise (no "Disconnected")
  // On Windows: show "Connected" / "Disconnected" based on IPC to .NET sidecar
  const isActive = isConnected;
  const statusText = isActive
    ? 'Ready'
    : IS_MACOS
      ? 'Hotkey Only'
      : 'Disconnected';

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
              ${isActive ? 'bg-emerald-400 animate-pulse' : 'bg-amber-400'}
            `}
            style={{ animationDuration: isActive ? '2.5s' : undefined }}
          />
          <span
            className={`
              relative inline-flex rounded-full h-2 w-2
              ${isActive ? 'bg-emerald-500' : 'bg-amber-500'}
            `}
          />
        </span>

        {/* Status text */}
        <span
          className={`
            text-xs font-medium tracking-wide
            ${isActive ? 'text-emerald-400/90' : 'text-amber-400/90'}
          `}
        >
          {statusText}
        </span>

        {/* Separator and version when connected */}
        {isActive && textMonitorVersion && (
          <>
            <span className="text-gray-600 text-xs">&bull;</span>
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
