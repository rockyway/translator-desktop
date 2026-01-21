import { useState, useRef, useEffect } from 'react';
import { FiSun, FiMoon, FiMonitor } from 'react-icons/fi';
import { useTheme } from '../hooks/useTheme';
import type { Theme } from '../contexts/ThemeContext';

interface ThemeOption {
  value: Theme;
  label: string;
  icon: React.ReactNode;
}

const themeOptions: ThemeOption[] = [
  { value: 'light', label: 'Light', icon: <FiSun className="w-4 h-4" /> },
  { value: 'dark', label: 'Dark', icon: <FiMoon className="w-4 h-4" /> },
  { value: 'system', label: 'System', icon: <FiMonitor className="w-4 h-4" /> },
];

function getCurrentIcon(theme: Theme) {
  switch (theme) {
    case 'light':
      return <FiSun className="w-5 h-5" />;
    case 'dark':
      return <FiMoon className="w-5 h-5" />;
    case 'system':
      return <FiMonitor className="w-5 h-5" />;
  }
}

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  // Close dropdown when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    }

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, []);

  // Handle keyboard navigation
  function handleKeyDown(event: React.KeyboardEvent) {
    if (event.key === 'Escape') {
      setIsOpen(false);
      buttonRef.current?.focus();
    }
  }

  function handleOptionClick(value: Theme) {
    setTheme(value);
    setIsOpen(false);
    buttonRef.current?.focus();
  }

  function handleOptionKeyDown(event: React.KeyboardEvent, value: Theme) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleOptionClick(value);
    }
  }

  return (
    <div ref={containerRef} className="relative" onKeyDown={handleKeyDown}>
      <button
        ref={buttonRef}
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        aria-label={`Theme: ${theme}. Click to change theme`}
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        className="flex items-center gap-2 p-2 rounded-lg bg-white/20 hover:bg-white/30
                   text-white transition-colors focus:outline-none focus:ring-2
                   focus:ring-white/50 focus:ring-offset-2 focus:ring-offset-amber-500"
      >
        {getCurrentIcon(theme)}
        <span className="hidden sm:inline text-sm font-medium capitalize">{theme}</span>
      </button>

      {isOpen && (
        <div
          role="listbox"
          aria-label="Select theme"
          className="absolute right-0 mt-2 w-40 bg-white dark:bg-gray-800 rounded-lg shadow-lg
                     border border-gray-200 dark:border-gray-700 py-1 z-50"
        >
          {themeOptions.map((option) => (
            <button
              key={option.value}
              type="button"
              role="option"
              aria-selected={theme === option.value}
              onClick={() => handleOptionClick(option.value)}
              onKeyDown={(e) => handleOptionKeyDown(e, option.value)}
              className={`w-full flex items-center gap-3 px-4 py-2 text-left transition-colors
                         focus:outline-none focus:bg-gray-100 dark:focus:bg-gray-700
                         ${theme === option.value
                           ? 'bg-amber-50 dark:bg-amber-900/30 text-amber-600 dark:text-amber-400'
                           : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'
                         }`}
            >
              <span className={theme === option.value ? 'text-amber-500' : 'text-gray-500 dark:text-gray-400'}>
                {option.icon}
              </span>
              <span className="text-sm font-medium">{option.label}</span>
              {theme === option.value && (
                <span className="ml-auto text-amber-500">
                  <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                    <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                  </svg>
                </span>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
