import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, emit } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ThemeProvider } from '../../contexts/ThemeContext';
import { PopupOverlay } from './PopupOverlay';

/**
 * PopupWindow - Tauri-integrated wrapper for PopupOverlay
 *
 * This component:
 * - Fetches initial text from Tauri state via `get_popup_text` command
 * - Listens for `popup-text-updated` events for live updates
 * - Handles close via `hide_popup` Tauri command
 * - Opens main window via Tauri window API with content transfer
 */
export function PopupWindow() {
  const [text, setText] = useState<string>('');
  const [isLoading, setIsLoading] = useState(true);

  // Fetch initial text from Tauri state
  useEffect(() => {
    async function fetchPopupText() {
      try {
        const storedText = await invoke<string>('get_popup_text');
        setText(storedText);
      } catch (error) {
        console.error('Failed to get popup text:', error);
      } finally {
        setIsLoading(false);
      }
    }

    fetchPopupText();
  }, []);

  // Listen for text updates from IPC
  useEffect(() => {
    const unlisten = listen<string>('popup-text-updated', (event) => {
      setText(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Handle close - hide the popup window
  const handleClose = useCallback(async () => {
    try {
      await invoke('hide_popup');
    } catch (error) {
      console.error('Failed to hide popup:', error);
      // Fallback: try to hide via window API
      try {
        const currentWindow = getCurrentWindow();
        await currentWindow.hide();
      } catch (fallbackError) {
        console.error('Fallback hide also failed:', fallbackError);
      }
    }
  }, []);

  // Handle open main window with content transfer
  const handleOpenMain = useCallback(async (
    sourceText: string,
    translatedText: string,
    sourceLang: string,
    targetLang: string
  ) => {
    try {
      // Get the main window and show/focus it
      const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
      const mainWindow = await WebviewWindow.getByLabel('main');

      if (mainWindow) {
        // Emit event to main window with the content to transfer
        await emit('open-main-with-content', {
          sourceText,
          translatedText,
          sourceLang,
          targetLang,
        });

        await mainWindow.show();
        await mainWindow.setFocus();
      } else {
        console.warn('Main window not found');
      }

      await invoke('hide_popup');
    } catch (error) {
      console.error('Failed to open main window:', error);
    }
  }, []);

  // Show loading state briefly
  if (isLoading) {
    return (
      <ThemeProvider>
        <div className="w-full h-full flex items-center justify-center bg-white dark:bg-gray-800">
          <div className="animate-spin w-6 h-6 border-2 border-amber-500 border-t-transparent rounded-full" />
        </div>
      </ThemeProvider>
    );
  }

  return (
    <ThemeProvider>
      <PopupOverlay
        text={text}
        onClose={handleClose}
        onOpenMain={handleOpenMain}
      />
    </ThemeProvider>
  );
}
