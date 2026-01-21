import { useContext } from 'react';
import { SettingsContext, SettingsContextValue } from '../contexts/SettingsContext';

/**
 * Hook to access app-wide settings from SettingsContext.
 *
 * Provides access to:
 * - `settings`: Current app settings (theme, languages, sidebar state)
 * - `isLoading`: Whether settings are being loaded from backend
 * - `resolvedTheme`: The actual theme being applied ('light' | 'dark')
 * - `updateSetting`: Update a single setting
 * - `updateSettings`: Update multiple settings at once
 *
 * @throws Error if used outside of SettingsProvider
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const { settings, updateSetting, isLoading } = useSettings();
 *
 *   if (isLoading) return <Loading />;
 *
 *   return (
 *     <select
 *       value={settings.targetLanguage}
 *       onChange={(e) => updateSetting('targetLanguage', e.target.value)}
 *     >
 *       ...
 *     </select>
 *   );
 * }
 * ```
 */
export function useSettings(): SettingsContextValue {
  const context = useContext(SettingsContext);

  if (context === null) {
    throw new Error('useSettings must be used within a SettingsProvider');
  }

  return context;
}
