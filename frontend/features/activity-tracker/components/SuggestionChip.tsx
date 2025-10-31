import { haptic } from '@/shared/utils';
import { Clock, RefreshCw, Sparkles, X } from 'lucide-react';
import type { SuggestionState } from '../types';

interface SuggestionChipProps {
  suggestion: SuggestionState;
  onAccept: () => void;
  onDismiss: () => void;
}

function formatTimestamp(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;
  const minutes = Math.floor(diff / 60000);

  if (minutes < 1) return 'just now';
  if (minutes === 1) return '1m ago';
  return `${minutes}m ago`;
}

export function SuggestionChip({ suggestion, onAccept, onDismiss }: SuggestionChipProps) {
  const isStale = suggestion.metadata?.isStale === true;
  const isUpdated = suggestion.metadata?.isUpdated === true;
  const showTimestamp = isStale;
  const showConfidence =
    !isStale && suggestion.confidence !== undefined && suggestion.confidence > 0;

  return (
    <div className="mt-3">
      <div className="flex items-center gap-1.5 mb-1">
        <span className="text-[10px] font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide">
          Suggestion
        </span>
        {isUpdated && (
          <span className="px-1 py-0.5 text-[9px] font-semibold bg-amber-500/20 text-amber-700 dark:text-amber-400 rounded uppercase tracking-wide animate-in fade-in duration-200">
            Updated
          </span>
        )}
        {showConfidence && (
          <span className="px-1 py-0.5 text-[9px] font-medium bg-green-500/20 text-green-700 dark:text-green-400 rounded">
            {Math.round((suggestion.confidence ?? 0) * 100)}%
          </span>
        )}
      </div>
      <div
        className={`px-2.5 py-1.5 rounded-md flex items-center justify-between gap-2 transition-all duration-300 ${
          isStale
            ? 'bg-gradient-to-br from-gray-50 to-gray-100 dark:from-gray-900/20 dark:to-gray-800/20 border-2 border-dashed border-gray-300 dark:border-gray-600 opacity-70'
            : isUpdated
              ? 'bg-gradient-to-br from-red-50 to-red-100 dark:from-red-950/20 dark:to-red-900/20 border-2 border-amber-400 dark:border-amber-500 shadow-sm animate-in slide-in-from-top-2 duration-300'
              : 'bg-gradient-to-br from-red-50 to-red-100 dark:from-red-950/20 dark:to-red-900/20 border border-red-200 dark:border-red-800/50 animate-in slide-in-from-top-2 duration-300'
        }`}
      >
        <div className="flex items-center gap-1.5 flex-1 min-w-0">
          {isStale ? (
            <Clock className="w-3 h-3 text-gray-500 dark:text-gray-400 flex-shrink-0" />
          ) : isUpdated ? (
            <RefreshCw
              className="w-3 h-3 text-amber-600 dark:text-amber-400 flex-shrink-0 animate-spin"
              style={{ animationDuration: '2s' }}
            />
          ) : (
            <Sparkles className="w-3 h-3 text-red-500 dark:text-red-400 flex-shrink-0" />
          )}
          <div className="flex flex-col flex-1 min-w-0">
            <span
              className={`text-xs truncate ${
                isStale ? 'text-gray-700 dark:text-gray-300' : 'text-red-900 dark:text-red-100'
              }`}
            >
              {suggestion.text}
            </span>
            {showTimestamp && (
              <span className="text-[9px] text-gray-500 dark:text-gray-500">
                {formatTimestamp(suggestion.timestamp)}
              </span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-0.5 flex-shrink-0">
          <button
            onClick={() => {
              haptic.light();
              onAccept();
            }}
            className={`px-1.5 py-0.5 text-[10px] font-medium rounded transition-colors ${
              isStale
                ? 'bg-gray-500 hover:bg-gray-600 text-white'
                : 'bg-red-500 hover:bg-red-600 text-white'
            }`}
          >
            Use
          </button>
          <button
            onClick={() => {
              haptic.light();
              onDismiss();
            }}
            aria-label="Dismiss suggestion"
            className="px-1.5 py-0.5 min-h-[20px] text-[10px] font-medium bg-black/5 dark:bg-white/5 hover:bg-black/10 dark:hover:bg-white/10 text-gray-700 dark:text-gray-300 rounded transition-colors flex items-center justify-center"
          >
            <X className="w-2.5 h-2.5" />
          </button>
        </div>
      </div>
    </div>
  );
}
