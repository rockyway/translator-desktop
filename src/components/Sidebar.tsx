import { useEffect, useState } from 'react';
import {
  FiMessageSquare,
  FiClock,
  FiSettings,
  FiChevronLeft,
  FiChevronRight,
} from 'react-icons/fi';

// ============================================================================
// Types
// ============================================================================

export type SidebarTab = 'translate' | 'history' | 'settings';

export interface SidebarProps {
  activeTab: SidebarTab;
  onTabChange: (tab: SidebarTab) => void;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  isConnected: boolean;
  textMonitorVersion: string | null;
}

interface NavItemProps {
  icon: React.ReactNode;
  label: string;
  isActive: boolean;
  isCollapsed: boolean;
  onClick: () => void;
}

// ============================================================================
// Constants
// ============================================================================

const EXPANDED_WIDTH = 200;
const COLLAPSED_WIDTH = 56;
const RESPONSIVE_BREAKPOINT = 640;

const NAV_ITEMS: Array<{
  tab: SidebarTab;
  icon: React.ReactNode;
  label: string;
}> = [
  {
    tab: 'translate',
    icon: <FiMessageSquare className="w-5 h-5 flex-shrink-0" />,
    label: 'Translate',
  },
  {
    tab: 'history',
    icon: <FiClock className="w-5 h-5 flex-shrink-0" />,
    label: 'History',
  },
  {
    tab: 'settings',
    icon: <FiSettings className="w-5 h-5 flex-shrink-0" />,
    label: 'Settings',
  },
];

// ============================================================================
// Sub-Components
// ============================================================================

/**
 * Navigation item component with icon and optional label
 */
function NavItem({ icon, label, isActive, isCollapsed, onClick }: NavItemProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={label}
      aria-current={isActive ? 'page' : undefined}
      className={`
        group relative w-full flex items-center gap-3 px-4 py-3
        text-sm font-medium transition-all duration-200 ease-out
        focus:outline-none focus-visible:ring-2 focus-visible:ring-amber-500
        focus-visible:ring-offset-2 focus-visible:ring-offset-gray-900
        ${isActive
          ? 'bg-amber-500/15 text-amber-400 border-l-2 border-amber-500'
          : 'text-gray-400 hover:text-white hover:bg-white/5 border-l-2 border-transparent'
        }
        ${isCollapsed ? 'justify-center px-0' : ''}
      `}
    >
      {/* Icon */}
      <span
        className={`
          transition-colors duration-200
          ${isActive ? 'text-amber-400' : 'text-gray-500 group-hover:text-gray-300'}
        `}
      >
        {icon}
      </span>

      {/* Label - fades out when collapsed */}
      <span
        className={`
          transition-all duration-300 ease-in-out whitespace-nowrap overflow-hidden
          ${isCollapsed ? 'w-0 opacity-0' : 'w-auto opacity-100'}
        `}
      >
        {label}
      </span>

      {/* Tooltip on hover when collapsed */}
      {isCollapsed && (
        <span
          className="
            absolute left-full ml-2 px-2 py-1 text-xs font-medium
            bg-gray-800 text-white rounded shadow-lg
            opacity-0 invisible group-hover:opacity-100 group-hover:visible
            transition-all duration-200 z-50 whitespace-nowrap
            pointer-events-none
          "
          role="tooltip"
        >
          {label}
        </span>
      )}
    </button>
  );
}

/**
 * Connection status indicator
 */
function ConnectionStatus({
  isConnected,
  isCollapsed,
  textMonitorVersion,
}: {
  isConnected: boolean;
  isCollapsed: boolean;
  textMonitorVersion: string | null;
}) {
  return (
    <div
      className={`
        flex items-center gap-3 px-4 py-3
        ${isCollapsed ? 'justify-center px-0' : ''}
      `}
      aria-label={isConnected ? 'Connected to text monitor' : 'Disconnected from text monitor'}
    >
      {/* Status dot */}
      <span className="relative flex h-2.5 w-2.5 flex-shrink-0">
        <span
          className={`
            absolute inline-flex h-full w-full rounded-full opacity-75
            ${isConnected ? 'bg-emerald-400 animate-ping' : 'bg-amber-400'}
          `}
          style={{ animationDuration: isConnected ? '2s' : undefined }}
        />
        <span
          className={`
            relative inline-flex rounded-full h-2.5 w-2.5
            ${isConnected ? 'bg-emerald-500' : 'bg-amber-500'}
          `}
        />
      </span>

      {/* Status text and version */}
      <div
        className={`
          flex flex-col transition-all duration-300 ease-in-out
          overflow-hidden
          ${isCollapsed ? 'w-0 opacity-0' : 'w-auto opacity-100'}
        `}
      >
        <span
          className={`
            text-xs font-medium whitespace-nowrap
            ${isConnected ? 'text-emerald-400' : 'text-amber-400'}
          `}
        >
          {isConnected ? 'Connected' : 'Disconnected'}
        </span>
        {isConnected && textMonitorVersion && (
          <span className="text-xs text-gray-400 whitespace-nowrap">
            v{textMonitorVersion}
          </span>
        )}
      </div>
    </div>
  );
}

/**
 * Brand/logo area at top of sidebar
 */
function BrandArea({ isCollapsed }: { isCollapsed: boolean }) {
  return (
    <div
      className={`
        flex items-center gap-3 px-4 py-5 border-b border-gray-800/50
        ${isCollapsed ? 'justify-center px-0' : ''}
      `}
    >
      {/* Logo icon - always visible */}
      <div
        className="
          flex items-center justify-center w-8 h-8 rounded-lg
          bg-gradient-to-br from-amber-400 to-amber-600
          shadow-lg shadow-amber-500/20 flex-shrink-0
        "
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="w-4 h-4 text-white"
          aria-hidden="true"
        >
          <path d="M5 8l6 6" />
          <path d="M4 14l6-6 2-3" />
          <path d="M2 5h12" />
          <path d="M7 2h1" />
          <path d="M22 22l-5-10-5 10" />
          <path d="M14 18h6" />
        </svg>
      </div>

      {/* Brand text - fades when collapsed */}
      <span
        className={`
          text-base font-semibold text-white tracking-tight
          transition-all duration-300 ease-in-out whitespace-nowrap overflow-hidden
          ${isCollapsed ? 'w-0 opacity-0' : 'w-auto opacity-100'}
        `}
      >
        Translator
      </span>
    </div>
  );
}

/**
 * Collapse toggle button
 */
function CollapseToggle({
  isCollapsed,
  onToggle,
}: {
  isCollapsed: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onToggle}
      aria-label={isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
      className="
        group flex items-center justify-center w-full py-3
        text-gray-500 hover:text-white hover:bg-white/5
        transition-all duration-200 ease-out
        focus:outline-none focus-visible:ring-2 focus-visible:ring-amber-500
        focus-visible:ring-inset border-t border-gray-800/50
      "
    >
      <span
        className="
          transition-transform duration-300 ease-in-out
          group-hover:scale-110
        "
      >
        {isCollapsed ? (
          <FiChevronRight className="w-5 h-5" />
        ) : (
          <FiChevronLeft className="w-5 h-5" />
        )}
      </span>
    </button>
  );
}

// ============================================================================
// Main Component
// ============================================================================

/**
 * Collapsible sidebar navigation component
 *
 * Features:
 * - Smooth expand/collapse transitions
 * - Icon-only mode when collapsed
 * - Responsive behavior (auto-collapse on small screens)
 * - Connection status indicator
 * - Accessible keyboard navigation
 */
export function Sidebar({
  activeTab,
  onTabChange,
  isCollapsed,
  onToggleCollapse,
  isConnected,
  textMonitorVersion,
}: SidebarProps) {
  // Track window width for responsive behavior
  const [windowWidth, setWindowWidth] = useState(
    typeof window !== 'undefined' ? window.innerWidth : RESPONSIVE_BREAKPOINT + 1
  );

  // Listen for window resize
  useEffect(() => {
    function handleResize() {
      setWindowWidth(window.innerWidth);
    }

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Auto-collapse on small screens
  useEffect(() => {
    if (windowWidth < RESPONSIVE_BREAKPOINT && !isCollapsed) {
      onToggleCollapse();
    }
  }, [windowWidth, isCollapsed, onToggleCollapse]);

  const sidebarWidth = isCollapsed ? COLLAPSED_WIDTH : EXPANDED_WIDTH;

  return (
    <aside
      className="
        fixed left-0 top-10 z-40
        bg-gray-900 dark:bg-gray-950
        border-r border-gray-800/50
        flex flex-col
        transition-all duration-300 ease-in-out
        shadow-xl shadow-black/20
      "
      style={{ width: sidebarWidth, height: 'calc(100vh - 2.5rem)' }}
      role="navigation"
      aria-label="Main navigation"
    >
      {/* Brand area */}
      <BrandArea isCollapsed={isCollapsed} />

      {/* Navigation items */}
      <nav className="flex-1 py-4" role="menubar">
        <ul className="space-y-1" role="menu">
          {NAV_ITEMS.map((item) => (
            <li key={item.tab} role="none">
              <NavItem
                icon={item.icon}
                label={item.label}
                isActive={activeTab === item.tab}
                isCollapsed={isCollapsed}
                onClick={() => onTabChange(item.tab)}
              />
            </li>
          ))}
        </ul>
      </nav>

      {/* Spacer */}
      <div className="flex-grow" />

      {/* Connection status */}
      <ConnectionStatus isConnected={isConnected} isCollapsed={isCollapsed} textMonitorVersion={textMonitorVersion} />

      {/* Collapse toggle */}
      <CollapseToggle isCollapsed={isCollapsed} onToggle={onToggleCollapse} />
    </aside>
  );
}

// ============================================================================
// Utility Hook for Sidebar State
// ============================================================================

/**
 * Custom hook to manage sidebar collapsed state with localStorage persistence
 */
export function useSidebarState(defaultCollapsed = false) {
  const [isCollapsed, setIsCollapsed] = useState(() => {
    // Check localStorage for persisted state
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem('sidebar-collapsed');
      if (stored !== null) {
        return stored === 'true';
      }
      // Default to collapsed on small screens
      if (window.innerWidth < RESPONSIVE_BREAKPOINT) {
        return true;
      }
    }
    return defaultCollapsed;
  });

  const toggleCollapse = () => {
    setIsCollapsed((prev) => {
      const newValue = !prev;
      localStorage.setItem('sidebar-collapsed', String(newValue));
      return newValue;
    });
  };

  return { isCollapsed, toggleCollapse };
}

export default Sidebar;
