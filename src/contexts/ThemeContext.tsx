import { createContext, useCallback, useEffect, useState, ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';

export type Theme = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';
export type DensityPreset = 'default' | 'large' | 'xlarge' | 'custom';

/** Maps density presets to font-size percentages */
const DENSITY_VALUES: Record<Exclude<DensityPreset, 'custom'>, number> = {
  default: 100,
  large: 110,
  xlarge: 125,
};

interface ThemeContextValue {
  theme: Theme;
  resolvedTheme: ResolvedTheme;
  densityPercentage: number;
  setTheme: (theme: Theme) => void;
}

export const ThemeContext = createContext<ThemeContextValue | null>(null);

const STORAGE_KEY = 'translator-theme';

function getSystemTheme(): ResolvedTheme {
  if (typeof window === 'undefined') return 'light';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function resolveTheme(theme: Theme): ResolvedTheme {
  if (theme === 'system') {
    return getSystemTheme();
  }
  return theme;
}

function getStoredTheme(): Theme {
  if (typeof window === 'undefined') return 'system';
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === 'light' || stored === 'dark' || stored === 'system') {
    return stored;
  }
  return 'system';
}

interface ThemeProviderProps {
  children: ReactNode;
}

/** Calculate density percentage from settings */
function getDensityPercentage(density: DensityPreset, customDensity: number): number {
  if (density === 'custom') {
    return customDensity;
  }
  return DENSITY_VALUES[density];
}

/** Apply density to document */
function applyDensityToDocument(percentage: number): void {
  const root = document.documentElement;
  root.style.setProperty('--density-scale', `${percentage / 100}`);
  root.style.fontSize = `${percentage}%`;
}

export function ThemeProvider({ children }: ThemeProviderProps) {
  const [theme, setThemeState] = useState<Theme>(() => getStoredTheme());
  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>(() => resolveTheme(theme));
  const [densityPercentage, setDensityPercentage] = useState<number>(100);

  const applyTheme = useCallback((resolved: ResolvedTheme) => {
    const root = document.documentElement;
    if (resolved === 'dark') {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }
  }, []);

  const setTheme = useCallback((newTheme: Theme) => {
    setThemeState(newTheme);
    localStorage.setItem(STORAGE_KEY, newTheme);
    const resolved = resolveTheme(newTheme);
    setResolvedTheme(resolved);
    applyTheme(resolved);
  }, [applyTheme]);

  // Apply theme on mount and when theme changes
  useEffect(() => {
    const resolved = resolveTheme(theme);
    setResolvedTheme(resolved);
    applyTheme(resolved);
  }, [theme, applyTheme]);

  // Load density settings from Tauri on mount
  useEffect(() => {
    async function loadDensitySettings() {
      try {
        const [densitySetting, customDensitySetting] = await Promise.all([
          invoke<string | null>('get_setting', { key: 'density' }),
          invoke<number | null>('get_setting', { key: 'custom_density' }),
        ]);

        const density: DensityPreset = (
          densitySetting === 'default' ||
          densitySetting === 'large' ||
          densitySetting === 'xlarge' ||
          densitySetting === 'custom'
        ) ? densitySetting : 'default';

        const customDensity = typeof customDensitySetting === 'number' ? customDensitySetting : 100;
        const percentage = getDensityPercentage(density, customDensity);

        setDensityPercentage(percentage);
        applyDensityToDocument(percentage);
      } catch (error) {
        console.error('Failed to load density settings:', error);
        // Use default density on error
        applyDensityToDocument(100);
      }
    }

    loadDensitySettings();
  }, []);

  // Listen for system theme changes
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    const handleChange = () => {
      if (theme === 'system') {
        const resolved = getSystemTheme();
        setResolvedTheme(resolved);
        applyTheme(resolved);
      }
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => {
      mediaQuery.removeEventListener('change', handleChange);
    };
  }, [theme, applyTheme]);

  const value: ThemeContextValue = {
    theme,
    resolvedTheme,
    densityPercentage,
    setTheme,
  };

  return (
    <ThemeContext.Provider value={value}>
      {children}
    </ThemeContext.Provider>
  );
}
