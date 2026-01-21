import { useState, useCallback } from 'react';
import {
  FiChevronDown,
  FiBookOpen,
  FiList,
  FiLayers,
  FiLink,
  FiType,
  FiMessageCircle,
} from 'react-icons/fi';
import type { TranslationMetadata } from '../../services/translationService';

interface MetadataSectionProps {
  metadata?: TranslationMetadata;
  isLoading?: boolean;
  className?: string;
}

interface CollapsibleCardProps {
  title: string;
  icon: React.ReactNode;
  defaultExpanded?: boolean;
  accentColor: 'amber' | 'blue' | 'purple' | 'emerald' | 'indigo' | 'slate';
  badge?: string | number;
  children: React.ReactNode;
}

/**
 * Accent color configurations for each section type
 */
const accentColors = {
  amber: {
    gradient: 'from-amber-500/10 to-amber-600/5',
    border: 'border-amber-300/50 dark:border-amber-600/30',
    iconBg: 'bg-amber-100 dark:bg-amber-900/40',
    iconColor: 'text-amber-600 dark:text-amber-400',
    badge: 'bg-amber-100 text-amber-700 dark:bg-amber-900/50 dark:text-amber-300',
    hover: 'hover:bg-amber-50/50 dark:hover:bg-amber-900/20',
    accentBorder: 'border-amber-400 dark:border-amber-500',
    pill: 'bg-amber-50 text-amber-700 border-amber-200 hover:bg-amber-100 dark:bg-amber-900/30 dark:text-amber-300 dark:border-amber-700 dark:hover:bg-amber-900/50',
  },
  blue: {
    gradient: 'from-blue-500/10 to-blue-600/5',
    border: 'border-blue-300/50 dark:border-blue-600/30',
    iconBg: 'bg-blue-100 dark:bg-blue-900/40',
    iconColor: 'text-blue-600 dark:text-blue-400',
    badge: 'bg-blue-100 text-blue-700 dark:bg-blue-900/50 dark:text-blue-300',
    hover: 'hover:bg-blue-50/50 dark:hover:bg-blue-900/20',
    accentBorder: 'border-blue-400 dark:border-blue-500',
    pill: 'bg-blue-50 text-blue-700 border-blue-200 hover:bg-blue-100 dark:bg-blue-900/30 dark:text-blue-300 dark:border-blue-700 dark:hover:bg-blue-900/50',
  },
  purple: {
    gradient: 'from-purple-500/10 to-purple-600/5',
    border: 'border-purple-300/50 dark:border-purple-600/30',
    iconBg: 'bg-purple-100 dark:bg-purple-900/40',
    iconColor: 'text-purple-600 dark:text-purple-400',
    badge: 'bg-purple-100 text-purple-700 dark:bg-purple-900/50 dark:text-purple-300',
    hover: 'hover:bg-purple-50/50 dark:hover:bg-purple-900/20',
    accentBorder: 'border-purple-400 dark:border-purple-500',
    pill: 'bg-purple-50 text-purple-700 border-purple-200 hover:bg-purple-100 dark:bg-purple-900/30 dark:text-purple-300 dark:border-purple-700 dark:hover:bg-purple-900/50',
  },
  emerald: {
    gradient: 'from-emerald-500/10 to-emerald-600/5',
    border: 'border-emerald-300/50 dark:border-emerald-600/30',
    iconBg: 'bg-emerald-100 dark:bg-emerald-900/40',
    iconColor: 'text-emerald-600 dark:text-emerald-400',
    badge: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/50 dark:text-emerald-300',
    hover: 'hover:bg-emerald-50/50 dark:hover:bg-emerald-900/20',
    accentBorder: 'border-emerald-400 dark:border-emerald-500',
    pill: 'bg-emerald-50 text-emerald-700 border-emerald-200 hover:bg-emerald-100 dark:bg-emerald-900/30 dark:text-emerald-300 dark:border-emerald-700 dark:hover:bg-emerald-900/50',
  },
  indigo: {
    gradient: 'from-indigo-500/10 to-indigo-600/5',
    border: 'border-indigo-300/50 dark:border-indigo-600/30',
    iconBg: 'bg-indigo-100 dark:bg-indigo-900/40',
    iconColor: 'text-indigo-600 dark:text-indigo-400',
    badge: 'bg-indigo-100 text-indigo-700 dark:bg-indigo-900/50 dark:text-indigo-300',
    hover: 'hover:bg-indigo-50/50 dark:hover:bg-indigo-900/20',
    accentBorder: 'border-indigo-400 dark:border-indigo-500',
    pill: 'bg-indigo-50 text-indigo-700 border-indigo-200 hover:bg-indigo-100 dark:bg-indigo-900/30 dark:text-indigo-300 dark:border-indigo-700 dark:hover:bg-indigo-900/50',
  },
  slate: {
    gradient: 'from-slate-500/10 to-slate-600/5',
    border: 'border-slate-300/50 dark:border-slate-600/30',
    iconBg: 'bg-slate-100 dark:bg-slate-800/60',
    iconColor: 'text-slate-600 dark:text-slate-400',
    badge: 'bg-slate-100 text-slate-700 dark:bg-slate-800/50 dark:text-slate-300',
    hover: 'hover:bg-slate-50/50 dark:hover:bg-slate-800/30',
    accentBorder: 'border-slate-400 dark:border-slate-500',
    pill: 'bg-slate-50 text-slate-700 border-slate-200 hover:bg-slate-100 dark:bg-slate-800/40 dark:text-slate-300 dark:border-slate-600 dark:hover:bg-slate-800/60',
  },
};

/**
 * CollapsibleCard - A refined, animated collapsible card component
 */
function CollapsibleCard({
  title,
  icon,
  defaultExpanded = false,
  accentColor,
  badge,
  children,
}: CollapsibleCardProps) {
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);
  const colors = accentColors[accentColor];

  const toggleExpanded = useCallback(() => {
    setIsExpanded((prev) => !prev);
  }, []);

  return (
    <div
      className={`
        rounded-xl border overflow-hidden
        bg-gradient-to-br ${colors.gradient}
        ${colors.border}
        transition-all duration-300 ease-out
        ${isExpanded ? 'shadow-md' : 'shadow-sm'}
      `}
    >
      {/* Header */}
      <button
        type="button"
        onClick={toggleExpanded}
        aria-expanded={isExpanded}
        className={`
          w-full flex items-center gap-3 px-4 py-3
          transition-colors duration-200
          ${colors.hover}
          focus:outline-none focus:ring-2 focus:ring-inset focus:ring-offset-0
          focus:ring-gray-400/50 dark:focus:ring-gray-500/50
        `}
      >
        {/* Icon */}
        <span
          className={`
            flex items-center justify-center w-8 h-8 rounded-lg
            ${colors.iconBg} ${colors.iconColor}
            transition-transform duration-300
            ${isExpanded ? 'scale-110' : 'scale-100'}
          `}
        >
          {icon}
        </span>

        {/* Title */}
        <span className="flex-1 text-left font-semibold text-gray-800 dark:text-gray-100 tracking-tight">
          {title}
        </span>

        {/* Badge */}
        {badge !== undefined && (
          <span
            className={`
              px-2.5 py-0.5 rounded-full text-xs font-medium
              ${colors.badge}
            `}
          >
            {badge}
          </span>
        )}

        {/* Chevron */}
        <FiChevronDown
          className={`
            w-5 h-5 text-gray-500 dark:text-gray-400
            transition-transform duration-300 ease-out
            ${isExpanded ? 'rotate-180' : 'rotate-0'}
          `}
          aria-hidden="true"
        />
      </button>

      {/* Content */}
      <div
        className={`
          overflow-hidden transition-all duration-300 ease-out
          ${isExpanded ? 'max-h-[500px] opacity-100' : 'max-h-0 opacity-0'}
        `}
      >
        <div className="px-4 pb-4 pt-1">{children}</div>
      </div>
    </div>
  );
}

/**
 * Pill component for word lists (alternatives, synonyms, related words)
 */
function WordPill({
  word,
  accentColor,
}: {
  word: string;
  accentColor: 'amber' | 'blue' | 'purple' | 'emerald' | 'indigo' | 'slate';
}) {
  const colors = accentColors[accentColor];

  return (
    <span
      className={`
        inline-flex items-center px-3 py-1.5 rounded-full
        text-sm font-medium border
        transition-all duration-200 ease-out
        cursor-default select-text
        ${colors.pill}
      `}
    >
      {word}
    </span>
  );
}

/**
 * ExamplesSection - Displays example sentences in a quote style
 */
function ExamplesSection({ examples }: { examples: { text: string }[] }) {
  if (examples.length === 0) return null;

  return (
    <div className="space-y-3">
      {examples.map((example, index) => (
        <blockquote
          key={index}
          className={`
            relative pl-4 py-2
            border-l-3 border-amber-400 dark:border-amber-500
            bg-amber-50/50 dark:bg-amber-900/20
            rounded-r-lg
          `}
        >
          <p className="text-gray-700 dark:text-gray-200 text-sm leading-relaxed italic">
            "{example.text}"
          </p>
        </blockquote>
      ))}
    </div>
  );
}

/**
 * DefinitionsSection - Displays definitions grouped by part of speech
 */
function DefinitionsSection({
  definitions,
}: {
  definitions: {
    partOfSpeech: string;
    entries: { gloss: string; example?: string }[];
  }[];
}) {
  if (definitions.length === 0) return null;

  return (
    <div className="space-y-4">
      {definitions.map((group, groupIndex) => (
        <div key={groupIndex} className="space-y-2">
          {/* Part of Speech Badge */}
          <span
            className={`
              inline-flex items-center px-2.5 py-1 rounded-md
              text-xs font-semibold uppercase tracking-wider
              bg-blue-100 text-blue-700
              dark:bg-blue-900/50 dark:text-blue-300
            `}
          >
            {group.partOfSpeech}
          </span>

          {/* Definitions List */}
          <ol className="space-y-2 ml-1">
            {group.entries.map((entry, entryIndex) => (
              <li
                key={entryIndex}
                className="flex gap-3 text-sm"
              >
                <span className="flex-shrink-0 w-5 h-5 flex items-center justify-center rounded-full bg-blue-100 dark:bg-blue-900/40 text-blue-600 dark:text-blue-400 text-xs font-semibold">
                  {entryIndex + 1}
                </span>
                <div className="flex-1 space-y-1">
                  <p className="text-gray-800 dark:text-gray-100 leading-relaxed">
                    {entry.gloss}
                  </p>
                  {entry.example && (
                    <p className="text-gray-500 dark:text-gray-400 text-xs italic pl-2 border-l-2 border-blue-200 dark:border-blue-700">
                      "{entry.example}"
                    </p>
                  )}
                </div>
              </li>
            ))}
          </ol>
        </div>
      ))}
    </div>
  );
}

/**
 * WordListSection - Displays a flex-wrapped list of word pills
 */
function WordListSection({
  words,
  accentColor,
}: {
  words: { word: string }[];
  accentColor: 'amber' | 'blue' | 'purple' | 'emerald' | 'indigo' | 'slate';
}) {
  if (words.length === 0) return null;

  return (
    <div className="flex flex-wrap gap-2">
      {words.map((item, index) => (
        <WordPill key={index} word={item.word} accentColor={accentColor} />
      ))}
    </div>
  );
}

/**
 * TransliterationSection - Displays transliteration in monospace
 */
function TransliterationSection({
  transliteration,
}: {
  transliteration: string;
}) {
  return (
    <div
      className={`
        px-4 py-3 rounded-lg
        bg-slate-100 dark:bg-slate-800/60
        border border-slate-200 dark:border-slate-700
      `}
    >
      <p className="font-mono text-sm text-gray-700 dark:text-gray-200 tracking-wide">
        {transliteration}
      </p>
    </div>
  );
}

/**
 * MetadataSection - Displays translation metadata in collapsible cards
 *
 * Shows examples, definitions, alternatives, synonyms, related words,
 * and transliteration in a visually appealing, organized format.
 */
export function MetadataSection({
  metadata,
  isLoading,
  className = '',
}: MetadataSectionProps) {
  // Don't render if loading or no metadata
  if (isLoading || !metadata) {
    return null;
  }

  // Check if there's any data to display
  const hasExamples = metadata.examples && metadata.examples.length > 0;
  const hasDefinitions =
    metadata.definitions &&
    metadata.definitions.length > 0 &&
    metadata.definitions.some((d) => d.entries.length > 0);
  const hasAlternatives =
    metadata.alternatives && metadata.alternatives.length > 0;
  const hasSynonyms = metadata.synonyms && metadata.synonyms.length > 0;
  const hasRelatedWords =
    metadata.relatedWords && metadata.relatedWords.length > 0;
  const hasTransliteration =
    metadata.transliteration && metadata.transliteration.trim() !== '';

  // If no data at all, don't render
  if (
    !hasExamples &&
    !hasDefinitions &&
    !hasAlternatives &&
    !hasSynonyms &&
    !hasRelatedWords &&
    !hasTransliteration
  ) {
    return null;
  }

  return (
    <div className={`space-y-3 ${className}`}>
      {/* Section Header */}
      <div className="flex items-center gap-2 px-1">
        <div className="h-px flex-1 bg-gradient-to-r from-transparent via-gray-300 dark:via-gray-600 to-transparent" />
        <span className="text-xs font-medium uppercase tracking-widest text-gray-400 dark:text-gray-500">
          Additional Information
        </span>
        <div className="h-px flex-1 bg-gradient-to-r from-transparent via-gray-300 dark:via-gray-600 to-transparent" />
      </div>

      {/* Collapsible Cards */}
      <div className="grid gap-3">
        {/* Examples - Expanded by default */}
        {hasExamples && (
          <CollapsibleCard
            title="Examples"
            icon={<FiMessageCircle className="w-4 h-4" />}
            accentColor="amber"
            defaultExpanded={true}
            badge={metadata.examples.length}
          >
            <ExamplesSection examples={metadata.examples} />
          </CollapsibleCard>
        )}

        {/* Definitions */}
        {hasDefinitions && (
          <CollapsibleCard
            title="Definitions"
            icon={<FiBookOpen className="w-4 h-4" />}
            accentColor="blue"
            defaultExpanded={true}
            badge={metadata.definitions.reduce(
              (acc, d) => acc + d.entries.length,
              0
            )}
          >
            <DefinitionsSection definitions={metadata.definitions} />
          </CollapsibleCard>
        )}

        {/* Alternatives */}
        {hasAlternatives && (
          <CollapsibleCard
            title="Alternatives"
            icon={<FiLayers className="w-4 h-4" />}
            accentColor="purple"
            defaultExpanded={true}
            badge={metadata.alternatives.length}
          >
            <WordListSection
              words={metadata.alternatives}
              accentColor="purple"
            />
          </CollapsibleCard>
        )}

        {/* Synonyms */}
        {hasSynonyms && (
          <CollapsibleCard
            title="Synonyms"
            icon={<FiList className="w-4 h-4" />}
            accentColor="emerald"
            defaultExpanded={true}
            badge={metadata.synonyms.length}
          >
            <WordListSection words={metadata.synonyms} accentColor="emerald" />
          </CollapsibleCard>
        )}

        {/* Related Words */}
        {hasRelatedWords && (
          <CollapsibleCard
            title="Related Words"
            icon={<FiLink className="w-4 h-4" />}
            accentColor="indigo"
            defaultExpanded={true}
            badge={metadata.relatedWords.length}
          >
            <WordListSection
              words={metadata.relatedWords}
              accentColor="indigo"
            />
          </CollapsibleCard>
        )}

        {/* Transliteration */}
        {hasTransliteration && (
          <CollapsibleCard
            title="Transliteration"
            icon={<FiType className="w-4 h-4" />}
            accentColor="slate"
            defaultExpanded={true}
          >
            <TransliterationSection
              transliteration={metadata.transliteration!}
            />
          </CollapsibleCard>
        )}
      </div>
    </div>
  );
}
