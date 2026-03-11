import { useEffect, useState, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ThemeProvider } from '../../contexts/ThemeContext';

/**
 * ConfirmationWindow - Standalone confirmation dialog window
 *
 * This component handles confirmation for long text translations.
 *
 * Key insight: The window is pre-created at app startup (visible: false).
 * When show_translation_confirmation() is called, it:
 * 1. Sets char_count in state
 * 2. Shows the window (triggers focus)
 * 3. Emits show-confirmation-window event
 *
 * Therefore, we should NOT fetch on mount (data isn't ready yet).
 * We fetch ONLY when:
 * - Window gains focus (triggered by window.show())
 * - Event is received (backup mechanism)
 */
export function ConfirmationWindow() {
  const [charCount, setCharCount] = useState(0);
  const [isReady, setIsReady] = useState(false);

  // Log component mount
  useEffect(() => {
    console.log('[ConfirmationWindow] Component mounted');
    console.log('[ConfirmationWindow] Initial state: isReady=%s, charCount=%d', isReady, charCount);
    console.log('[ConfirmationWindow] Window dimensions:', window.innerWidth, 'x', window.innerHeight);
    return () => {
      console.log('[ConfirmationWindow] Component unmounted');
    };
  }, []);

  // Fetch confirmation data from Tauri state
  const fetchData = useCallback(async () => {
    console.log('[ConfirmationWindow] fetchData() called');
    try {
      console.log('[ConfirmationWindow] Invoking get_confirmation_data...');
      const count = await invoke<number>('get_confirmation_data');
      console.log('[ConfirmationWindow] Received count from Rust:', count);
      if (count > 0) {
        console.log('[ConfirmationWindow] Count > 0, setting state: charCount=%d, isReady=true', count);
        setCharCount(count);
        setIsReady(true);
      } else {
        console.log('[ConfirmationWindow] Count is 0, staying in loading state');
      }
    } catch (error) {
      console.error('[ConfirmationWindow] Failed to get confirmation data:', error);
    }
  }, []);

  // Listen for window focus to fetch data
  // This is the PRIMARY mechanism: when Rust calls window.show(), this triggers
  useEffect(() => {
    console.log('[ConfirmationWindow] Setting up focus listener...');
    const currentWindow = getCurrentWindow();
    const unlisten = currentWindow.onFocusChanged(({ payload: focused }) => {
      console.log('[ConfirmationWindow] Focus changed event:', { focused });
      if (focused) {
        console.log('[ConfirmationWindow] Window focused, will fetch data in 50ms...');
        // Small delay to ensure Rust has set the char_count
        setTimeout(() => {
          console.log('[ConfirmationWindow] 50ms elapsed, now calling fetchData()');
          fetchData();
        }, 50);
      } else {
        console.log('[ConfirmationWindow] Window lost focus, resetting state');
        // Reset state when window loses focus (for reuse)
        setIsReady(false);
        setCharCount(0);
      }
    });

    return () => {
      console.log('[ConfirmationWindow] Cleaning up focus listener');
      unlisten.then((fn) => fn());
    };
  }, [fetchData]);

  // Also listen for the event (backup mechanism, handles edge cases)
  useEffect(() => {
    console.log('[ConfirmationWindow] Setting up event listener for show-confirmation-window...');
    const unlisten = listen<{ charCount: number }>('show-confirmation-window', (event) => {
      console.log('[ConfirmationWindow] Received show-confirmation-window event:', event.payload);
      if (event.payload.charCount > 0) {
        console.log('[ConfirmationWindow] Event has charCount > 0, setting state:', event.payload.charCount);
        setCharCount(event.payload.charCount);
        setIsReady(true);
      } else {
        console.log('[ConfirmationWindow] Event charCount is 0, ignoring');
      }
    });

    return () => {
      console.log('[ConfirmationWindow] Cleaning up event listener');
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleConfirm = useCallback(async () => {
    try {
      await invoke('respond_to_confirmation', { confirmed: true });
      setIsReady(false);
      const window = getCurrentWindow();
      await window.hide();
    } catch (error) {
      console.error('Failed to confirm:', error);
    }
  }, []);

  const handleCancel = useCallback(async () => {
    try {
      await invoke('respond_to_confirmation', { confirmed: false });
      setIsReady(false);
      const window = getCurrentWindow();
      await window.hide();
    } catch (error) {
      console.error('Failed to cancel:', error);
    }
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isReady) return;
      if (e.key === 'Escape') {
        e.preventDefault();
        handleCancel();
      } else if (e.key === 'Enter') {
        e.preventDefault();
        handleConfirm();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isReady, handleConfirm, handleCancel]);

  // Loading state - subtle spinner while waiting for data
  if (!isReady) {
    console.log('[ConfirmationWindow] Rendering loading spinner (isReady=false, charCount=%d)', charCount);
    return (
      <ThemeProvider>
        <div className="w-full h-full flex items-center justify-center bg-white/95 dark:bg-gray-900/95">
          <div className="animate-spin w-8 h-8 border-4 border-amber-500 border-t-transparent rounded-full" />
        </div>
      </ThemeProvider>
    );
  }

  console.log('[ConfirmationWindow] Rendering confirmation content (isReady=true, charCount=%d)', charCount);

  return (
    <ThemeProvider>
      <div className="w-full h-full flex items-center justify-center p-3 bg-white/95 dark:bg-gray-900/95">
        <div className="w-full max-w-sm bg-white dark:bg-gray-800 rounded-xl shadow-2xl border border-gray-200 dark:border-gray-700 overflow-hidden">
          {/* Header */}
          <div className="px-5 py-4 bg-amber-50 dark:bg-amber-900/20 border-b border-amber-200 dark:border-amber-800">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-full bg-amber-100 dark:bg-amber-900/50 flex items-center justify-center flex-shrink-0">
                <svg className="w-5 h-5 text-amber-600 dark:text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
              </div>
              <div>
                <h2 className="text-base font-semibold text-gray-900 dark:text-white">
                  Long Text Detected
                </h2>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  <span className="font-bold text-amber-600 dark:text-amber-400">{charCount.toLocaleString()}</span> characters selected
                </p>
              </div>
            </div>
          </div>

          {/* Body */}
          <div className="px-5 py-4">
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Translating large amounts of text may take longer. Do you want to continue?
            </p>
          </div>

          {/* Footer */}
          <div className="px-5 py-4 bg-gray-50 dark:bg-gray-900/50 flex justify-end gap-3">
            <button
              onClick={handleCancel}
              className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300
                bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600
                rounded-lg hover:bg-gray-50 dark:hover:bg-gray-600
                focus:outline-none focus:ring-2 focus:ring-amber-500"
            >
              Cancel
            </button>
            <button
              onClick={handleConfirm}
              autoFocus
              className="px-4 py-2 text-sm font-medium text-white
                bg-gradient-to-br from-amber-500 to-amber-600
                rounded-lg hover:from-amber-600 hover:to-amber-700
                focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2
                dark:focus:ring-offset-gray-800"
            >
              Translate
            </button>
          </div>
        </div>
      </div>
    </ThemeProvider>
  );
}
