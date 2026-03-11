import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { SettingsProvider } from "./contexts/SettingsContext";
import { useSettings } from "./hooks/useSettings";
import { Sidebar, SidebarTab } from "./components/Sidebar";
import { UpdateNotification } from "./components/UpdateNotification";
import { TitleBar } from "./components/TitleBar";
import { StatusBar } from "./components/StatusBar";
import { TranslationPanel } from "./features/translator/TranslationPanel";
import { HistoryPanel } from "./features/history";
import { SettingsPanel } from "./features/settings";
import { useIpcListener } from "./hooks/useIpcListener";
import { TranslationMetadata } from "./services/translationService";

// ============================================================================
// Types
// ============================================================================

interface ContentTransfer {
  sourceText: string;
  translatedText: string;
  sourceLang: string;
  targetLang: string;
  metadata?: TranslationMetadata;
}

// Flag to indicate content came from popup (already translated)
interface InitialContent {
  text: string;
  translatedText?: string;
  sourceLang?: string;
  targetLang?: string;
  metadata?: TranslationMetadata;
  /** Unique key to force TranslationPanel remount when content changes */
  contentKey: number;
}

// ============================================================================
// Main App Content
// ============================================================================

function AppContent() {
  // Only use isConnected and textMonitorVersion from IPC - selectedText is handled by popup, not main app
  const { isConnected, textMonitorVersion, needsAccessibilityPermission } = useIpcListener();
  const { settings, updateSetting } = useSettings();

  const [activeTab, setActiveTab] = useState<SidebarTab>('translate');
  const [initialContent, setInitialContent] = useState<InitialContent | null>(null);

  // Ref for history panel scroll container (for preserving scroll position on refetch)
  const historyScrollRef = useRef<HTMLDivElement>(null);

  // Listen for content transfer from popup (already translated)
  useEffect(() => {
    const unlisten = listen<ContentTransfer>('open-main-with-content', (event) => {
      const { sourceText, translatedText, sourceLang, targetLang, metadata } = event.payload;
      setInitialContent({
        text: sourceText,
        translatedText, // Pass the already-translated text
        sourceLang,
        targetLang,
        metadata,
        contentKey: Date.now(), // Force remount
      });
      setActiveTab('translate');
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Handle selecting a history entry to translate
  const handleSelectHistoryEntry = useCallback((sourceText: string, translatedText: string, sourceLang: string, targetLang: string, metadata?: TranslationMetadata) => {
    setInitialContent({
      text: sourceText,
      translatedText,
      sourceLang,
      targetLang,
      metadata,
      contentKey: Date.now(),
    });
    setActiveTab('translate');
  }, []);

  // Handle sidebar collapse toggle
  const handleToggleCollapse = useCallback(() => {
    updateSetting('sidebarCollapsed', !settings.sidebarCollapsed);
  }, [settings.sidebarCollapsed, updateSetting]);

  return (
    <div className="h-screen flex flex-col bg-gray-50 dark:bg-gray-900 transition-colors overflow-hidden">
      {/* Custom Title Bar - Fixed height, non-scrolling */}
      <TitleBar />

      {/* Content wrapper below title bar - no-drag prevents macOS overlay from intercepting clicks */}
      <div className="flex flex-1 min-h-0" style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}>
        {/* Sidebar Navigation */}
        <Sidebar
          activeTab={activeTab}
          onTabChange={setActiveTab}
          isCollapsed={settings.sidebarCollapsed}
          onToggleCollapse={handleToggleCollapse}
        />

        {/* Main Content Area - Each panel has its own scroll container */}
        <main className="flex-1 relative min-h-0">
          {/* Translation Panel */}
          <div
            className={`absolute inset-0 overflow-y-auto custom-scrollbar ${activeTab !== 'translate' ? 'hidden' : ''}`}
            role="tabpanel"
            aria-label="Translation panel"
            aria-hidden={activeTab !== 'translate'}
          >
            <div className="p-6 min-h-full">
              <div className="max-w-6xl mx-auto">
                <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-xl p-6 md:p-8 border border-gray-200 dark:border-gray-700">
                  <TranslationPanel
                    key={initialContent?.contentKey ?? 'default'}
                    initialText={initialContent?.text}
                    initialTranslatedText={initialContent?.translatedText}
                    initialSourceLang={initialContent?.sourceLang}
                    initialTargetLang={initialContent?.targetLang}
                    initialMetadata={initialContent?.metadata}
                  />
                </div>
              </div>
            </div>
          </div>

          {/* History Panel */}
          <div
            ref={historyScrollRef}
            className={`absolute inset-0 overflow-y-auto custom-scrollbar ${activeTab !== 'history' ? 'hidden' : ''}`}
            role="tabpanel"
            aria-label="History panel"
            aria-hidden={activeTab !== 'history'}
          >
            <div className="p-6 min-h-full">
              <div className="max-w-6xl mx-auto">
                <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-xl p-6 md:p-8 border border-gray-200 dark:border-gray-700">
                  <HistoryPanel
                    onSelectEntry={handleSelectHistoryEntry}
                    isVisible={activeTab === 'history'}
                    scrollContainerRef={historyScrollRef}
                  />
                </div>
              </div>
            </div>
          </div>

          {/* Settings Panel */}
          <div
            className={`absolute inset-0 overflow-y-auto custom-scrollbar ${activeTab !== 'settings' ? 'hidden' : ''}`}
            role="tabpanel"
            aria-label="Settings panel"
            aria-hidden={activeTab !== 'settings'}
          >
            <div className="p-6 min-h-full">
              <div className="max-w-6xl mx-auto">
                <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-xl p-6 md:p-8 border border-gray-200 dark:border-gray-700">
                  <SettingsPanel />
                </div>
              </div>
            </div>
          </div>
        </main>
      </div>

      {/* Accessibility Permission Banner (macOS) */}
      {needsAccessibilityPermission && !isConnected && (
        <div className="flex-shrink-0 bg-amber-900/80 border-t border-amber-700/50 px-4 py-2 flex items-center justify-between">
          <span className="text-xs text-amber-200">
            Text selection monitor requires Accessibility permission. Hotkey mode is still available.
          </span>
          <button
            onClick={() => invoke('request_accessibility_permission')}
            className="text-xs font-medium text-amber-100 bg-amber-700 hover:bg-amber-600 px-3 py-1 rounded transition-colors"
          >
            Open Settings
          </button>
        </div>
      )}

      {/* Status Bar - Fixed at bottom */}
      <StatusBar
        isConnected={isConnected}
        textMonitorVersion={textMonitorVersion}
        needsAccessibilityPermission={needsAccessibilityPermission}
      />

      {/* Update Notification - Fixed position overlay */}
      <UpdateNotification />
    </div>
  );
}

// ============================================================================
// App Root
// ============================================================================

function App() {
  return (
    <SettingsProvider>
      <AppContent />
    </SettingsProvider>
  );
}

export default App;
