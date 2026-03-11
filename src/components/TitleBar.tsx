import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const IS_MACOS = navigator.platform.toUpperCase().includes("MAC") ||
  navigator.userAgent.toUpperCase().includes("MAC");

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);
  const appWindowRef = useRef(getCurrentWindow());

  // Check if window is maximized on mount and on resize
  useEffect(() => {
    const appWindow = appWindowRef.current;
    const checkMaximized = async () => {
      try {
        const maximized = await invoke<boolean>("is_window_maximized");
        setIsMaximized(maximized);
      } catch (error) {
        console.error("Failed to check window maximized state:", error);
      }
    };

    checkMaximized();

    const unlisten = appWindow.onResized(() => {
      checkMaximized();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleMinimize = async () => {
    try {
      await invoke("minimize_window");
    } catch (error) {
      console.error("Failed to minimize window:", error);
    }
  };

  const handleMaximize = async () => {
    try {
      await invoke("toggle_maximize_window");
      const maximized = await invoke<boolean>("is_window_maximized");
      setIsMaximized(maximized);
    } catch (error) {
      console.error("Failed to toggle maximize window:", error);
    }
  };

  const handleClose = async () => {
    try {
      await invoke("close_window");
    } catch (error) {
      console.error("Failed to close window:", error);
    }
  };

  const handleDoubleClick = () => {
    handleMaximize();
  };

  return (
    <div
      className="h-10 flex-shrink-0 z-50 flex items-center justify-between select-none bg-gray-50 dark:bg-gray-900"
      data-tauri-drag-region
      {...(!IS_MACOS ? { onDoubleClick: handleDoubleClick } : {})}
    >
      {/* App Icon and Title - centered on macOS, left-aligned on Windows */}
      <div
        data-tauri-drag-region
        className={`flex items-center h-full gap-2.5 pointer-events-none ${IS_MACOS ? 'absolute left-1/2 -translate-x-1/2' : 'px-3'}`}
      >
        <div className="flex items-center justify-center w-6 h-6 rounded-md bg-gradient-to-br from-amber-600 to-amber-700 shadow-sm flex-shrink-0">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="w-3.5 h-3.5 text-white"
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
        <h1 className="text-sm font-semibold text-gray-700 dark:text-gray-200 tracking-wide">
          Translator Desktop
        </h1>
      </div>

      {/* Right side - Window Controls (hidden on macOS, native traffic lights used instead) */}
      {!IS_MACOS && (
        <div className="flex h-full" style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}>
          <button
            onClick={handleMinimize}
            className="w-12 h-full flex items-center justify-center hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors group"
            aria-label="Minimize"
            title="Minimize"
          >
            <svg width="12" height="1" viewBox="0 0 12 1" className="text-gray-700 dark:text-gray-200">
              <rect width="12" height="1" fill="currentColor" />
            </svg>
          </button>
          <button
            onClick={handleMaximize}
            className="w-12 h-full flex items-center justify-center hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors group"
            aria-label={isMaximized ? "Restore" : "Maximize"}
            title={isMaximized ? "Restore" : "Maximize"}
          >
            {isMaximized ? (
              <svg width="12" height="12" viewBox="0 0 12 12" className="text-gray-700 dark:text-gray-200">
                <rect x="2" y="0" width="8" height="8" fill="none" stroke="currentColor" strokeWidth="1" />
                <rect x="0" y="2" width="8" height="8" fill="none" stroke="currentColor" strokeWidth="1" />
              </svg>
            ) : (
              <svg width="12" height="12" viewBox="0 0 12 12" className="text-gray-700 dark:text-gray-200">
                <rect width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1" />
              </svg>
            )}
          </button>
          <button
            onClick={handleClose}
            className="w-12 h-full flex items-center justify-center hover:bg-red-500 dark:hover:bg-red-600 transition-colors group"
            aria-label="Close"
            title="Close"
          >
            <svg width="12" height="12" viewBox="0 0 12 12" className="text-gray-700 dark:text-gray-200 group-hover:text-white">
              <line x1="0" y1="0" x2="12" y2="12" stroke="currentColor" strokeWidth="1" />
              <line x1="12" y1="0" x2="0" y2="12" stroke="currentColor" strokeWidth="1" />
            </svg>
          </button>
        </div>
      )}
    </div>
  );
}
