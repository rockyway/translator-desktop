import { useState, useCallback, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { SettingsProvider } from "./contexts/SettingsContext";
import { useSettings } from "./hooks/useSettings";
import { Sidebar, SidebarTab } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { TranslationPanel } from "./features/translator/TranslationPanel";
import { HistoryPanel } from "./features/history";
import { SettingsPanel } from "./features/settings";
import { useIpcListener } from "./hooks/useIpcListener";

// ============================================================================
// Types
// ============================================================================

interface ContentTransfer {
  sourceText: string;
  translatedText: string;
  sourceLang: string;
  targetLang: string;
}

// Flag to indicate content came from popup (already translated)
interface InitialContent {
  text: string;
  translatedText?: string;
  sourceLang?: string;
  targetLang?: string;
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
      const { sourceText, translatedText, sourceLang, targetLang } = event.payload;
      setInitialContent({
        text: sourceText,
        translatedText, // Pass the already-translated text
        sourceLang,
        targetLang,
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

  // Calculate main content margin based on sidebar state
  const mainMarginClass = settings.sidebarCollapsed ? 'ml-14' : 'ml-[200px]';

  return (
    <div className="h-screen flex bg-gray-50 dark:bg-gray-900 transition-colors">
      {/* Custom Title Bar */}
      <TitleBar />

      {/* Sidebar Navigation */}
      <Sidebar
        activeTab={activeTab}
        onTabChange={setActiveTab}
        isCollapsed={settings.sidebarCollapsed}
        onToggleCollapse={handleToggleCollapse}
        isConnected={isConnected}
        textMonitorVersion={textMonitorVersion}
      />

      {/* Main Content Area */}
      <main
        className={`flex-1 ${mainMarginClass} transition-all duration-300 ease-in-out overflow-y-auto pt-10`}
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
                />
              )}
              {activeTab === 'history' && (
                <HistoryPanel onSelectEntry={handleSelectHistoryEntry} />
              )}
              {activeTab === 'settings' && (
                <SettingsPanel textMonitorVersion={textMonitorVersion} />
              )}
            </div>
          </div>
        </div>
      </main>
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
