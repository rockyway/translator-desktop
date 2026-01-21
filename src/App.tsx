import { useState, useCallback, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { SettingsProvider } from "./contexts/SettingsContext";
import { useSettings } from "./hooks/useSettings";
import { Sidebar, SidebarTab } from "./components/Sidebar";
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
  const { isConnected, textMonitorVersion } = useIpcListener();
  const { settings, updateSetting } = useSettings();

  const [activeTab, setActiveTab] = useState<SidebarTab>('translate');
  const [initialContent, setInitialContent] = useState<InitialContent | null>(null);

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
  const handleSelectHistoryEntry = useCallback((sourceText: string, translatedText: string, sourceLang: string, targetLang: string) => {
    setInitialContent({
      text: sourceText,
      translatedText,
      sourceLang,
      targetLang,
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

      {/* Content wrapper below title bar */}
      <div className="flex flex-1 min-h-0">
        {/* Sidebar Navigation */}
        <Sidebar
          activeTab={activeTab}
          onTabChange={setActiveTab}
          isCollapsed={settings.sidebarCollapsed}
          onToggleCollapse={handleToggleCollapse}
        />

        {/* Main Content Area - Scrollable content contained here */}
        <main
          className="flex-1 overflow-y-auto custom-scrollbar"
        >
          <div className="p-6 min-h-full">
            <div className="max-w-6xl mx-auto">
              {/* Panel Content */}
              <div
                className="bg-white dark:bg-gray-800 rounded-2xl shadow-xl p-6 md:p-8 border border-gray-200 dark:border-gray-700"
                role="tabpanel"
                aria-label={
                  activeTab === 'translate'
                    ? 'Translation panel'
                    : activeTab === 'history'
                      ? 'History panel'
                      : 'Settings panel'
                }
              >
                {activeTab === 'translate' && (
                  <TranslationPanel
                    key={initialContent?.contentKey ?? 'default'}
                    initialText={initialContent?.text}
                    initialTranslatedText={initialContent?.translatedText}
                    initialSourceLang={initialContent?.sourceLang}
                    initialTargetLang={initialContent?.targetLang}
                    initialMetadata={initialContent?.metadata}
                  />
                )}
                {activeTab === 'history' && (
                  <HistoryPanel onSelectEntry={handleSelectHistoryEntry} />
                )}
                {activeTab === 'settings' && (
                  <SettingsPanel />
                )}
              </div>
            </div>
          </div>
        </main>
      </div>

      {/* Status Bar - Fixed at bottom */}
      <StatusBar
        isConnected={isConnected}
        textMonitorVersion={textMonitorVersion}
      />
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
