import { useState, useRef, useEffect, useCallback } from 'react';
import { FiChevronDown, FiCheck } from 'react-icons/fi';
import type { LanguageOption } from '../../services/translationService';

interface LanguageSelectorProps {
  /** Current language code */
  value: string;
  /** Callback when language selection changes */
  onChange: (code: string) => void;
  /** List of available language options */
  languages: LanguageOption[];
  /** Label for the selector (e.g., "Source Language" or "Target Language") */
  label: string;
  /** Unique ID for accessibility */
  id: string;
  /** Whether the selector is disabled */
  disabled?: boolean;
  /** Optional sublabel to display inline with the label (e.g., "Detected: English") */
  sublabel?: string;
}

/**
 * Language selector dropdown component for translation source/target selection.
 * Provides full keyboard navigation and accessibility support.
 */
export function LanguageSelector({
  value,
  onChange,
  languages,
  label,
  id,
  disabled = false,
  sublabel,
}: LanguageSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(-1);
  const containerRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const listboxRef = useRef<HTMLUListElement>(null);

  // Find the currently selected language
  const selectedLanguage = languages.find((lang) => lang.code === value);
  const selectedIndex = languages.findIndex((lang) => lang.code === value);

  // Close dropdown when clicking outside
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
        setHighlightedIndex(-1);
      }
    }

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, []);

  // Scroll highlighted option into view
  useEffect(() => {
    if (isOpen && highlightedIndex >= 0 && listboxRef.current) {
      const option = listboxRef.current.children[highlightedIndex] as HTMLElement;
      if (option) {
        option.scrollIntoView({ block: 'nearest' });
      }
    }
  }, [highlightedIndex, isOpen]);

  // Reset highlighted index when dropdown opens
  useEffect(() => {
    if (isOpen) {
      setHighlightedIndex(selectedIndex >= 0 ? selectedIndex : 0);
    }
  }, [isOpen, selectedIndex]);

  const closeDropdown = useCallback(() => {
    setIsOpen(false);
    setHighlightedIndex(-1);
    buttonRef.current?.focus();
  }, []);

  const selectOption = useCallback(
    (code: string) => {
      onChange(code);
      closeDropdown();
    },
    [onChange, closeDropdown]
  );

  // Handle keyboard navigation
  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (disabled) return;

      switch (event.key) {
        case 'Escape':
          event.preventDefault();
          closeDropdown();
          break;

        case 'Enter':
        case ' ':
          event.preventDefault();
          if (isOpen && highlightedIndex >= 0) {
            selectOption(languages[highlightedIndex].code);
          } else {
            setIsOpen(true);
          }
          break;

        case 'ArrowDown':
          event.preventDefault();
          if (!isOpen) {
            setIsOpen(true);
          } else {
            setHighlightedIndex((prev) =>
              prev < languages.length - 1 ? prev + 1 : 0
            );
          }
          break;

        case 'ArrowUp':
          event.preventDefault();
          if (!isOpen) {
            setIsOpen(true);
          } else {
            setHighlightedIndex((prev) =>
              prev > 0 ? prev - 1 : languages.length - 1
            );
          }
          break;

        case 'Home':
          event.preventDefault();
          if (isOpen) {
            setHighlightedIndex(0);
          }
          break;

        case 'End':
          event.preventDefault();
          if (isOpen) {
            setHighlightedIndex(languages.length - 1);
          }
          break;

        case 'Tab':
          if (isOpen) {
            closeDropdown();
          }
          break;

        default:
          // Type-ahead: jump to first matching option
          if (event.key.length === 1 && isOpen) {
            const char = event.key.toLowerCase();
            const startIndex = highlightedIndex + 1;
            const matchIndex = languages.findIndex(
              (lang, index) =>
                index >= startIndex && lang.name.toLowerCase().startsWith(char)
            );
            if (matchIndex >= 0) {
              setHighlightedIndex(matchIndex);
            } else {
              // Wrap around and search from beginning
              const wrapIndex = languages.findIndex((lang) =>
                lang.name.toLowerCase().startsWith(char)
              );
              if (wrapIndex >= 0) {
                setHighlightedIndex(wrapIndex);
              }
            }
          }
          break;
      }
    },
    [disabled, isOpen, highlightedIndex, languages, closeDropdown, selectOption]
  );

  const handleButtonClick = () => {
    if (!disabled) {
      setIsOpen(!isOpen);
    }
  };

  const handleOptionClick = (code: string) => {
    selectOption(code);
  };

  const handleOptionMouseEnter = (index: number) => {
    setHighlightedIndex(index);
  };

  const listboxId = `${id}-listbox`;

  return (
    <div ref={containerRef} className="relative" onKeyDown={handleKeyDown}>
      {/* Label */}
      <label
        htmlFor={id}
        className="flex items-center justify-between text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
      >
        <span>{label}</span>
        {sublabel && (
          <span className="text-sm text-amber-600 dark:text-amber-400 font-medium ml-2">
            {sublabel}
          </span>
        )}
      </label>

      {/* Trigger Button */}
      <button
        ref={buttonRef}
        type="button"
        id={id}
        onClick={handleButtonClick}
        aria-label={`${label}: ${selectedLanguage?.name ?? 'Select language'}`}
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        aria-controls={listboxId}
        aria-activedescendant={
          isOpen && highlightedIndex >= 0
            ? `${id}-option-${highlightedIndex}`
            : undefined
        }
        disabled={disabled}
        className={`
          flex items-center justify-between w-full min-h-[44px] px-4 py-2
          rounded-lg border transition-colors
          focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2
          dark:focus:ring-offset-gray-900
          ${
            disabled
              ? 'bg-gray-100 dark:bg-gray-800 border-gray-200 dark:border-gray-700 text-gray-400 dark:text-gray-500 cursor-not-allowed'
              : 'bg-white dark:bg-gray-800 border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-100 hover:border-amber-400 dark:hover:border-amber-500 cursor-pointer'
          }
        `}
      >
        <span className="truncate">
          {selectedLanguage?.name ?? 'Select language'}
        </span>
        <FiChevronDown
          className={`
            w-5 h-5 ml-2 flex-shrink-0 transition-transform
            ${isOpen ? 'transform rotate-180' : ''}
            ${disabled ? 'text-gray-400 dark:text-gray-500' : 'text-gray-500 dark:text-gray-400'}
          `}
          aria-hidden="true"
        />
      </button>

      {/* Dropdown */}
      {isOpen && (
        <ul
          ref={listboxRef}
          role="listbox"
          id={listboxId}
          aria-label={label}
          className="
            absolute z-50 w-full mt-1 max-h-60 overflow-auto
            bg-white dark:bg-gray-800 rounded-lg shadow-lg
            border border-gray-200 dark:border-gray-700 py-1
          "
        >
          {languages.map((language, index) => {
            const isSelected = language.code === value;
            const isHighlighted = index === highlightedIndex;

            return (
              <li
                key={language.code}
                id={`${id}-option-${index}`}
                role="option"
                aria-selected={isSelected}
                onClick={() => handleOptionClick(language.code)}
                onMouseEnter={() => handleOptionMouseEnter(index)}
                className={`
                  flex items-center justify-between px-4 py-2 min-h-[44px]
                  cursor-pointer transition-colors
                  ${
                    isHighlighted
                      ? 'bg-amber-50 dark:bg-amber-900/30'
                      : ''
                  }
                  ${
                    isSelected
                      ? 'text-amber-600 dark:text-amber-400 font-medium'
                      : 'text-gray-700 dark:text-gray-300'
                  }
                  ${
                    !isHighlighted && !isSelected
                      ? 'hover:bg-gray-100 dark:hover:bg-gray-700'
                      : ''
                  }
                `}
              >
                <span className="truncate">{language.name}</span>
                {isSelected && (
                  <FiCheck
                    className="w-5 h-5 ml-2 flex-shrink-0 text-amber-500"
                    aria-hidden="true"
                  />
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
