import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  FiMessageSquare,
  FiClock,
  FiSettings,
  FiChevronLeft,
  FiChevronRight,
  FiPower,
  FiAlertTriangle,
  FiX,
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
 * Exit button component - only shown when sidebar is expanded
 */
function ExitButton({ onRequestExit }: { onRequestExit: () => void }) {
  return (
    <button
      type="button"
      onClick={onRequestExit}
      aria-label="Exit application"
      className="
        group relative w-full flex items-center gap-3 px-4 py-3
        text-sm font-medium transition-all duration-200 ease-out
        text-gray-400 hover:text-red-400 hover:bg-red-500/10
        focus:outline-none focus-visible:ring-2 focus-visible:ring-red-500
        focus-visible:ring-offset-2 focus-visible:ring-offset-gray-900
        border-t border-gray-800/50
      "
    >
      <span className="text-gray-500 group-hover:text-red-400 transition-colors duration-200">
        <FiPower className="w-5 h-5 flex-shrink-0" />
      </span>
      <span className="whitespace-nowrap">Exit</span>
    </button>
  );
}

/**
 * Exit confirmation dialog
 */
function ExitConfirmDialog({
  isOpen,
  onConfirm,
  onCancel,
}: {
  isOpen: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  // Handle escape key to close
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onCancel();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onCancel]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      role="dialog"
      aria-modal="true"
      aria-labelledby="exit-dialog-title"
    >
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm"
        onClick={onCancel}
      />

      {/* Dialog */}
      <div className="relative bg-gray-900 border border-gray-700 rounded-lg shadow-2xl p-6 w-full max-w-sm mx-4">
        {/* Close button */}
        <button
          type="button"
          onClick={onCancel}
          className="absolute top-3 right-3 text-gray-500 hover:text-gray-300 transition-colors"
          aria-label="Close dialog"
        >
          <FiX className="w-5 h-5" />
        </button>

        {/* Icon */}
        <div className="flex justify-center mb-4">
          <div className="w-12 h-12 rounded-full bg-red-500/20 flex items-center justify-center">
            <FiAlertTriangle className="w-6 h-6 text-red-400" />
          </div>
        </div>

        {/* Title */}
        <h2
          id="exit-dialog-title"
          className="text-lg font-semibold text-white text-center mb-2"
        >
          Exit Application?
        </h2>

        {/* Message */}
        <p className="text-gray-400 text-sm text-center mb-6">
          Are you sure you want to close Translator Desktop?
        </p>

        {/* Actions */}
        <div className="flex gap-3">
          <button
            type="button"
            onClick={onCancel}
            className="
              flex-1 px-4 py-2 text-sm font-medium
              bg-gray-800 text-gray-300 rounded-lg
              hover:bg-gray-700 hover:text-white
              transition-colors duration-200
              focus:outline-none focus-visible:ring-2 focus-visible:ring-gray-500
            "
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            className="
              flex-1 px-4 py-2 text-sm font-medium
              bg-red-600 text-white rounded-lg
              hover:bg-red-500
              transition-colors duration-200
              focus:outline-none focus-visible:ring-2 focus-visible:ring-red-500
            "
          >
            Exit
          </button>
        </div>
      </div>
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
 * - Accessible keyboard navigation
 */
export function Sidebar({
  activeTab,
  onTabChange,
  isCollapsed,
  onToggleCollapse,
}: SidebarProps) {
  // Track window width for responsive behavior
  const [windowWidth, setWindowWidth] = useState(
    typeof window !== 'undefined' ? window.innerWidth : RESPONSIVE_BREAKPOINT + 1
  );

  // Exit confirmation dialog state
  const [showExitConfirm, setShowExitConfirm] = useState(false);

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

  const handleExitRequest = useCallback(() => {
    setShowExitConfirm(true);
  }, []);

  const handleExitConfirm = useCallback(async () => {
    try {
      await invoke('exit_app');
    } catch (error) {
      console.error('Failed to exit app:', error);
    }
  }, []);

  const handleExitCancel = useCallback(() => {
    setShowExitConfirm(false);
  }, []);

  const sidebarWidth = isCollapsed ? COLLAPSED_WIDTH : EXPANDED_WIDTH;

  return (
    <>
      <aside
        className="
          flex-shrink-0 h-full z-40
          bg-gray-900 dark:bg-gray-950
          border-r border-gray-800/50
          flex flex-col
          transition-all duration-300 ease-in-out
          shadow-xl shadow-black/20
        "
        style={{ width: sidebarWidth }}
        role="navigation"
        aria-label="Main navigation"
      >
        {/* Navigation items */}
        <nav className="flex-1 pt-2" role="menubar">
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

        {/* Exit button - only shown when expanded */}
        {!isCollapsed && <ExitButton onRequestExit={handleExitRequest} />}

        {/* Collapse toggle */}
        <CollapseToggle isCollapsed={isCollapsed} onToggle={onToggleCollapse} />
      </aside>

      {/* Exit confirmation dialog */}
      <ExitConfirmDialog
        isOpen={showExitConfirm}
        onConfirm={handleExitConfirm}
        onCancel={handleExitCancel}
      />
    </>
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
