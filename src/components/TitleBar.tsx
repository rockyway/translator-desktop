import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);
  const appWindow = getCurrentWindow();

  // Check if window is maximized on mount and on resize
  useEffect(() => {
    const checkMaximized = async () => {
      try {
        const maximized = await invoke<boolean>("is_window_maximized");
        setIsMaximized(maximized);
      } catch (error) {
        console.error("Failed to check window maximized state:", error);
      }
    };

    checkMaximized();

    // Listen for window resize events
    const unlisten = appWindow.onResized(() => {
      checkMaximized();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [appWindow]);

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

  const handleDragStart = async () => {
    try {
      await invoke("start_drag_window");
    } catch (error) {
      console.error("Failed to start dragging window:", error);
    }
  };

  return (
    <div
      className="fixed top-0 left-0 right-0 h-10 z-50 flex items-center justify-between select-none bg-transparent"
      style={{ WebkitAppRegion: "drag" } as React.CSSProperties}
      onDoubleClick={handleDoubleClick}
    >
      {/* Left side - App Title */}
      <div className="flex items-center h-full px-4">
        <h1 className="text-sm font-semibold text-gray-700 dark:text-gray-200 tracking-wide">
          Translator Desktop
        </h1>
      </div>

      {/* Right side - Window Controls */}
      <div
        className="flex h-full"
        style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
      >
        {/* Minimize Button */}
        <button
          onClick={handleMinimize}
          className="w-12 h-full flex items-center justify-center hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors group"
          aria-label="Minimize"
          title="Minimize"
        >
          <svg
            width="12"
            height="1"
            viewBox="0 0 12 1"
            className="text-gray-700 dark:text-gray-200"
          >
            <rect width="12" height="1" fill="currentColor" />
          </svg>
        </button>

        {/* Maximize/Restore Button */}
        <button
          onClick={handleMaximize}
          className="w-12 h-full flex items-center justify-center hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors group"
          aria-label={isMaximized ? "Restore" : "Maximize"}
          title={isMaximized ? "Restore" : "Maximize"}
        >
          {isMaximized ? (
            // Restore icon (two overlapping squares)
            <svg
              width="12"
              height="12"
              viewBox="0 0 12 12"
              className="text-gray-700 dark:text-gray-200"
            >
              <rect
                x="2"
                y="0"
                width="8"
                height="8"
                fill="none"
                stroke="currentColor"
                strokeWidth="1"
              />
              <rect
                x="0"
                y="2"
                width="8"
                height="8"
                fill="none"
                stroke="currentColor"
                strokeWidth="1"
              />
            </svg>
          ) : (
            // Maximize icon (single square)
            <svg
              width="12"
              height="12"
              viewBox="0 0 12 12"
              className="text-gray-700 dark:text-gray-200"
            >
              <rect
                width="12"
                height="12"
                fill="none"
                stroke="currentColor"
                strokeWidth="1"
              />
            </svg>
          )}
        </button>

        {/* Close Button */}
        <button
          onClick={handleClose}
          className="w-12 h-full flex items-center justify-center hover:bg-red-500 dark:hover:bg-red-600 transition-colors group"
          aria-label="Close"
          title="Close"
        >
          <svg
            width="12"
            height="12"
            viewBox="0 0 12 12"
            className="text-gray-700 dark:text-gray-200 group-hover:text-white"
          >
            <line
              x1="0"
              y1="0"
              x2="12"
              y2="12"
              stroke="currentColor"
              strokeWidth="1"
            />
            <line
              x1="12"
              y1="0"
              x2="0"
              y2="12"
              stroke="currentColor"
              strokeWidth="1"
            />
          </svg>
        </button>
      </div>
    </div>
  );
}
