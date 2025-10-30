// FEATURE-020 Phase 2: WBS Autocomplete Component
// Searchable WBS code picker with FTS5 search

import React, { useState, useEffect, useCallback } from 'react';
import { Check, ChevronsUpDown, Loader2, Star, AlertCircle, AlertTriangle, CheckCircle } from 'lucide-react';
import { cn } from '@/components/ui/utils';
import { Button } from '@/components/ui/button';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { Badge } from '@/components/ui/badge';
import { SapService } from '@/features/settings/services/sapService';
import { WbsUsageService } from '@/features/timer/services/wbsUsageService';
import type { WbsElement } from '@/shared/types/generated';

export type ValidationResponse = {
  status: 'Valid' | 'Warning' | 'Error';
  code: string;
  message: string | null;
};

export type WbsAutocompleteProps = {
  value?: string;
  onChange: (code: string, element?: WbsElement) => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  buttonClassName?: string;
  popoverClassName?: string;
};

/**
 * WBS Autocomplete Component
 * 
 * Features:
 * - Real-time FTS5 full-text search (<50ms)
 * - Displays WBS code + project name + description
 * - Keyboard navigation (arrow keys, Enter, Escape)
 * - Debounced search (200ms) to reduce Tauri IPC calls
 * - Loading state indicator
 * - Empty state handling
 * 
 * @example
 * ```tsx
 * <WbsAutocomplete
 *   value={wbsCode}
 *   onChange={(code, element) => setWbsCode(code)}
 *   placeholder="Search WBS code..."
 * />
 * ```
 */
export function WbsAutocomplete({
  value = '',
  onChange,
  placeholder = 'Select WBS code...',
  disabled = false,
  className,
  buttonClassName,
  popoverClassName,
}: WbsAutocompleteProps) {
  const [open, setOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [results, setResults] = useState<WbsElement[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [selectedElement, setSelectedElement] = useState<WbsElement | undefined>();
  const [recentElements, setRecentElements] = useState<WbsElement[]>([]);
  const [favoriteElements, setFavoriteElements] = useState<WbsElement[]>([]);
  const [favoriteCodes, setFavoriteCodes] = useState<string[]>([]);
  const [validationStatus, setValidationStatus] = useState<ValidationResponse | null>(null);
  const [isValidating, setIsValidating] = useState(false);

  // Load favorites and recent on mount
  useEffect(() => {
    const loadFavoritesAndRecent = () => {
      setRecentElements(WbsUsageService.getRecentElements());
      setFavoriteCodes(WbsUsageService.getFavorites());
    };

    loadFavoritesAndRecent();

    // Reload when popover opens
    if (open) {
      loadFavoritesAndRecent();
    }
  }, [open]);

  // Fetch favorite elements from cache
  useEffect(() => {
    const fetchFavorites = async () => {
      if (favoriteCodes.length === 0) {
        setFavoriteElements([]);
        return;
      }

      try {
        // Search for each favorite code to get full element data
        const favoritePromises = favoriteCodes.map(code => SapService.searchWbs(code));
        const favoriteResults = await Promise.all(favoritePromises);
        const favorites = favoriteResults.flat().filter((elem, index, self) =>
          self.findIndex(e => e.wbs_code === elem.wbs_code) === index
        );
        setFavoriteElements(favorites);
      } catch (error) {
        console.error('Failed to fetch favorite WBS elements:', error);
        setFavoriteElements([]);
      }
    };

    void fetchFavorites();
  }, [favoriteCodes]);

  // Validate WBS code when value changes
  useEffect(() => {
    if (!value || value.trim().length === 0) {
      setValidationStatus(null);
      return;
    }

    setIsValidating(true);
    void (async () => {
      try {
        const validation = await SapService.validateWbs(value);
        setValidationStatus(validation);
      } catch (error) {
        console.error('WBS validation failed:', error);
        setValidationStatus({
          status: 'Error',
          code: value,
          message: 'Validation failed - please try again',
        });
      } finally {
        setIsValidating(false);
      }
    })();
  }, [value]);

  // Debounced search handler
  useEffect(() => {
    if (!searchQuery || searchQuery.trim().length === 0) {
      setResults([]);
      setIsSearching(false);
      return;
    }

    setIsSearching(true);
    const timer = setTimeout(() => {
      void (async () => {
        try {
          const wbsResults = await SapService.searchWbs(searchQuery);
          setResults(wbsResults);
        } catch (error) {
          console.error('WBS search failed:', error);
          setResults([]);
        } finally {
          setIsSearching(false);
        }
      })();
    }, 200); // 200ms debounce

    return () => clearTimeout(timer);
  }, [searchQuery]);

  const handleSelect = useCallback(
    (element: WbsElement) => {
      setSelectedElement(element);
      WbsUsageService.addRecentWbs(element.wbs_code, element);
      onChange(element.wbs_code, element);
      setOpen(false);
    },
    [onChange]
  );

  const handleToggleFavorite = useCallback(
    (event: React.MouseEvent, code: string) => {
      event.stopPropagation();
      const newIsFavorite = WbsUsageService.toggleFavorite(code);
      setFavoriteCodes(WbsUsageService.getFavorites());
      return newIsFavorite;
    },
    []
  );

  const handleClear = useCallback(() => {
    setSelectedElement(undefined);
    onChange('');
    setSearchQuery('');
    setResults([]);
  }, [onChange]);

  // FEATURE-029: Format display text with enriched metadata
  const formatWbsDisplay = (element: WbsElement): string => {
    const projectName = element.project_name || 'Unknown Project';
    const clientName = element.target_company_name || 'Internal';
    const wbsCode = element.wbs_code;

    // Special handling for G&A codes (no opportunity data)
    if (!element.target_company_name) {
      return `General & Administrative - Internal - ${wbsCode}`;
    }

    // Standard format with opportunity data
    return `${projectName} - ${clientName} - ${wbsCode}`;
  };

  const displayText = selectedElement
    ? formatWbsDisplay(selectedElement)
    : value || placeholder;

  return (
    <div className={cn('relative', className)}>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            role="combobox"
            aria-expanded={open}
            className={cn('w-full justify-between', buttonClassName)}
            disabled={disabled}
          >
            <span className={cn('truncate', !value && 'text-muted-foreground')}>
              {displayText}
            </span>
            <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className={cn('w-[400px] p-0', popoverClassName)} align="start">
          <Command shouldFilter={false}>
            <CommandInput
              placeholder="Search WBS code, project, or description..."
              value={searchQuery}
              onValueChange={setSearchQuery}
            />
            <CommandList>
              {isSearching && (
                <div className="flex items-center justify-center p-4">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  <span className="ml-2 text-sm text-muted-foreground">
                    Searching...
                  </span>
                </div>
              )}

              {/* Show favorites and recent when no search query */}
              {!isSearching && !searchQuery && (
                <>
                  {favoriteElements.length > 0 && (
                    <CommandGroup heading="Favorites">
                      {favoriteElements.map((element) => (
                        <CommandItem
                          key={`fav-${element.wbs_code}`}
                          value={element.wbs_code}
                          onSelect={() => void handleSelect(element)}
                          className="flex items-start gap-2"
                        >
                          <Check
                            className={cn(
                              'mt-1 h-4 w-4 shrink-0',
                              value === element.wbs_code ? 'opacity-100' : 'opacity-0'
                            )}
                          />
                          <div className="flex flex-1 flex-col">
                            <span className="text-sm">
                              {formatWbsDisplay(element)}
                            </span>
                          </div>
                          <button
                            type="button"
                            onClick={(e) => void handleToggleFavorite(e, element.wbs_code)}
                            className="mt-1 shrink-0"
                            aria-label="Remove from favorites"
                          >
                            <Star className="h-4 w-4 fill-yellow-400 text-yellow-400" />
                          </button>
                        </CommandItem>
                      ))}
                    </CommandGroup>
                  )}

                  {recentElements.length > 0 && (
                    <CommandGroup heading="Recent">
                      {recentElements.map((element) => (
                        <CommandItem
                          key={`recent-${element.wbs_code}`}
                          value={element.wbs_code}
                          onSelect={() => void handleSelect(element)}
                          className="flex items-start gap-2"
                        >
                          <Check
                            className={cn(
                              'mt-1 h-4 w-4 shrink-0',
                              value === element.wbs_code ? 'opacity-100' : 'opacity-0'
                            )}
                          />
                          <div className="flex flex-1 flex-col">
                            <span className="text-sm">
                              {formatWbsDisplay(element)}
                            </span>
                          </div>
                          <button
                            type="button"
                            onClick={(e) => void handleToggleFavorite(e, element.wbs_code)}
                            className="mt-1 shrink-0"
                            aria-label={
                              favoriteCodes.includes(element.wbs_code)
                                ? 'Remove from favorites'
                                : 'Add to favorites'
                            }
                          >
                            <Star
                              className={cn(
                                'h-4 w-4',
                                favoriteCodes.includes(element.wbs_code)
                                  ? 'fill-yellow-400 text-yellow-400'
                                  : 'text-muted-foreground'
                              )}
                            />
                          </button>
                        </CommandItem>
                      ))}
                    </CommandGroup>
                  )}

                  {favoriteElements.length === 0 && recentElements.length === 0 && (
                    <CommandEmpty>No recent or favorite WBS codes</CommandEmpty>
                  )}
                </>
              )}

              {/* Show search results when query present */}
              {!isSearching && searchQuery && results.length === 0 && (
                <CommandEmpty>No WBS codes found.</CommandEmpty>
              )}

              {!isSearching && searchQuery && results.length > 0 && (
                <CommandGroup heading="Search Results">
                  {results
                    .filter(element => {
                      // Filter to show favorites/recent matching search query
                      const query = searchQuery.toLowerCase();
                      const matchesQuery =
                        element.wbs_code.toLowerCase().includes(query) ||
                        (element.project_name?.toLowerCase().includes(query) ?? false) ||
                        (element.description?.toLowerCase().includes(query) ?? false);
                      return matchesQuery;
                    })
                    .map((element) => (
                      <CommandItem
                        key={element.wbs_code}
                        value={element.wbs_code}
                        onSelect={() => void handleSelect(element)}
                        className="flex items-start gap-2"
                      >
                        <Check
                          className={cn(
                            'mt-1 h-4 w-4 shrink-0',
                            value === element.wbs_code ? 'opacity-100' : 'opacity-0'
                          )}
                        />
                        <div className="flex flex-1 flex-col">
                          <span className="font-mono text-sm font-semibold">
                            {element.wbs_code}
                          </span>
                          {element.project_name && (
                            <span className="text-sm text-muted-foreground">
                              {element.project_name}
                            </span>
                          )}
                          {element.description && (
                            <span className="text-xs text-muted-foreground">
                              {element.description}
                            </span>
                          )}
                        </div>
                        <button
                          type="button"
                          onClick={(e) => void handleToggleFavorite(e, element.wbs_code)}
                          className="mt-1 shrink-0"
                          aria-label={
                            favoriteCodes.includes(element.wbs_code)
                              ? 'Remove from favorites'
                              : 'Add to favorites'
                          }
                        >
                          <Star
                            className={cn(
                              'h-4 w-4',
                              favoriteCodes.includes(element.wbs_code)
                                ? 'fill-yellow-400 text-yellow-400'
                                : 'text-muted-foreground'
                            )}
                          />
                        </button>
                      </CommandItem>
                    ))}
                </CommandGroup>
              )}
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>

      {value && !disabled && (
        <button
          type="button"
          onClick={handleClear}
          className="absolute right-10 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
          aria-label="Clear selection"
        >
          ×
        </button>
      )}

      {/* Validation feedback UI */}
      {isValidating && value && (
        <div className="mt-2 flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="h-3 w-3 animate-spin" />
          <span>Validating...</span>
        </div>
      )}

      {!isValidating && validationStatus?.status === 'Error' && (
        <div className="mt-2 flex items-center gap-2">
          <Badge variant="destructive" className="flex items-center gap-1">
            <AlertCircle className="h-3 w-3" />
            <span>{validationStatus.message || 'Invalid WBS code'}</span>
          </Badge>
        </div>
      )}

      {!isValidating && validationStatus?.status === 'Warning' && (
        <div className="mt-2 flex flex-col gap-2">
          <Badge variant="outline" className="flex items-center gap-1 border-yellow-500 text-yellow-700 dark:text-yellow-400">
            <AlertTriangle className="h-3 w-3" />
            <span>{validationStatus.message || 'Warning'}</span>
          </Badge>
          {validationStatus.message?.includes('not in cache') && (
            <Button
              size="sm"
              variant="outline"
              onClick={() => void SapService.triggerSyncNow()}
              className="w-fit"
            >
              Sync Now
            </Button>
          )}
        </div>
      )}

      {!isValidating && validationStatus?.status === 'Valid' && value && (
        <div className="mt-2 flex items-center gap-2 text-sm text-green-600 dark:text-green-400">
          <CheckCircle className="h-3 w-3" />
          <span>Valid WBS code</span>
        </div>
      )}
    </div>
  );
}

