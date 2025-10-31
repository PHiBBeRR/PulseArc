import { Badge } from '@/components/ui/badge';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { CalendarIcon, Check, Clock, Filter, Moon, Plane, SortAsc } from 'lucide-react';
import { useCallback, useState, type ReactNode } from 'react';

export type SortOption = 'most-recent' | 'oldest-first' | 'longest-duration' | 'shortest-duration';
export type ContextFilter = 'travel' | 'after_hours' | 'weekend' | 'calendar';

export interface BuildMyDayFilterState {
  sortBy: SortOption;
  contextFilters: Set<ContextFilter>;
}

export interface BuildMyDayFilterPopoverProps {
  trigger: ReactNode;
  filterState: BuildMyDayFilterState;
  onFilterChange: (state: BuildMyDayFilterState) => void;
  activeFilterCount: number;
}

export function BuildMyDayFilterPopover({
  trigger,
  filterState,
  onFilterChange,
  activeFilterCount,
}: BuildMyDayFilterPopoverProps) {
  const [activeTab, setActiveTab] = useState<'sort' | 'filter'>('sort');

  const handleTabChange = useCallback((value: string) => {
    setActiveTab(value as 'sort' | 'filter');
  }, []);

  const handleSortChange = useCallback(
    (value: SortOption) => {
      onFilterChange({
        ...filterState,
        sortBy: value,
      });
    },
    [filterState, onFilterChange]
  );

  const handleContextFilterToggle = useCallback(
    (context: ContextFilter) => {
      const newContextFilters = new Set(filterState.contextFilters);
      if (newContextFilters.has(context)) {
        newContextFilters.delete(context);
      } else {
        newContextFilters.add(context);
      }
      onFilterChange({
        ...filterState,
        contextFilters: newContextFilters,
      });
    },
    [filterState, onFilterChange]
  );

  return (
    <Popover>
      <PopoverTrigger asChild>{trigger}</PopoverTrigger>
      <PopoverContent
        className="w-56 p-0 bg-neutral-100 dark:bg-neutral-900 border-2 border-neutral-200 dark:border-neutral-700 rounded-3xl shadow-xl"
        align="end"
        sideOffset={8}
      >
        <div className="p-2.5">
          <Tabs value={activeTab} onValueChange={handleTabChange} className="w-full">
            {/* Header with tabs */}
            <div className="flex items-center gap-2 mb-2 pb-2 border-b border-neutral-200 dark:border-neutral-700">
              <TabsList className="w-full bg-transparent p-0 h-auto gap-1">
                <TabsTrigger
                  value="sort"
                  className="flex-1 gap-1 text-xs h-7 data-[state=active]:bg-neutral-200 data-[state=active]:dark:bg-neutral-800"
                >
                  <SortAsc className="size-3.5" />
                  Sort
                </TabsTrigger>
                <TabsTrigger
                  value="filter"
                  className="flex-1 gap-1 text-xs h-7 data-[state=active]:bg-neutral-200 data-[state=active]:dark:bg-neutral-800"
                  onClick={(e) => {
                    // If showing "Clear", clear filters instead of switching tabs
                    if (activeFilterCount > 0) {
                      e.preventDefault();
                      onFilterChange({
                        sortBy: filterState.sortBy,
                        contextFilters: new Set(),
                      });
                    }
                  }}
                >
                  <Filter className="size-3.5" />
                  {activeFilterCount > 0 ? 'Clear' : 'Filter'}
                  {activeFilterCount > 0 && (
                    <Badge
                      variant="secondary"
                      className="ml-1 h-4 min-w-4 px-1.5 text-[10px] bg-blue-500 text-white rounded-full"
                    >
                      {activeFilterCount}
                    </Badge>
                  )}
                </TabsTrigger>
              </TabsList>
            </div>

            <TabsContent value="sort" className="mt-0">
              <div className="space-y-1.5">
                <button
                  onClick={() => handleSortChange('most-recent')}
                  className={`w-full text-left px-2 py-1.5 rounded-lg text-xs font-medium transition-colors border ${
                    filterState.sortBy === 'most-recent'
                      ? 'bg-neutral-200 dark:bg-neutral-800 text-gray-900 dark:text-gray-100 border-neutral-300 dark:border-neutral-700'
                      : 'text-gray-700 dark:text-gray-300 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                  }`}
                >
                  Most Recent
                </button>
                <button
                  onClick={() => handleSortChange('oldest-first')}
                  className={`w-full text-left px-2 py-1.5 rounded-lg text-xs font-medium transition-colors border ${
                    filterState.sortBy === 'oldest-first'
                      ? 'bg-neutral-200 dark:bg-neutral-800 text-gray-900 dark:text-gray-100 border-neutral-300 dark:border-neutral-700'
                      : 'text-gray-700 dark:text-gray-300 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                  }`}
                >
                  Oldest First
                </button>
                <button
                  onClick={() => handleSortChange('longest-duration')}
                  className={`w-full text-left px-2 py-1.5 rounded-lg text-xs font-medium transition-colors border ${
                    filterState.sortBy === 'longest-duration'
                      ? 'bg-neutral-200 dark:bg-neutral-800 text-gray-900 dark:text-gray-100 border-neutral-300 dark:border-neutral-700'
                      : 'text-gray-700 dark:text-gray-300 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                  }`}
                >
                  Longest Duration
                </button>
                <button
                  onClick={() => handleSortChange('shortest-duration')}
                  className={`w-full text-left px-2 py-1.5 rounded-lg text-xs font-medium transition-colors border ${
                    filterState.sortBy === 'shortest-duration'
                      ? 'bg-neutral-200 dark:bg-neutral-800 text-gray-900 dark:text-gray-100 border-neutral-300 dark:border-neutral-700'
                      : 'text-gray-700 dark:text-gray-300 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                  }`}
                >
                  Shortest Duration
                </button>
              </div>
            </TabsContent>

            <TabsContent value="filter" className="mt-0">
              <div className="space-y-2.5">
                {/* CONTEXT Section */}
                <div className="space-y-1.5">
                  <h4 className="text-[10px] font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">
                    Context
                  </h4>

                  {/* Travel */}
                  <button
                    onClick={() => handleContextFilterToggle('travel')}
                    className={`w-full flex items-center gap-2 px-2 py-1.5 rounded-lg transition-all border ${
                      filterState.contextFilters.has('travel')
                        ? 'bg-purple-500/20 dark:bg-purple-500/25 border-purple-500/40 dark:border-purple-500/50'
                        : 'bg-neutral-100/50 dark:bg-neutral-800/30 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                    }`}
                  >
                    <Plane className="size-4 text-gray-700 dark:text-gray-300 flex-shrink-0" />
                    <span className="flex-1 text-left text-xs font-medium text-gray-900 dark:text-gray-100">
                      Travel
                    </span>
                    {filterState.contextFilters.has('travel') && (
                      <Check className="size-3.5 text-gray-900 dark:text-gray-100 flex-shrink-0" />
                    )}
                  </button>

                  {/* After Hours */}
                  <button
                    onClick={() => handleContextFilterToggle('after_hours')}
                    className={`w-full flex items-center gap-2 px-2 py-1.5 rounded-lg transition-all border ${
                      filterState.contextFilters.has('after_hours')
                        ? 'bg-orange-600/25 dark:bg-orange-600/20 border-orange-600/40 dark:border-orange-600/50'
                        : 'bg-neutral-100/50 dark:bg-neutral-800/30 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                    }`}
                  >
                    <Moon className="size-4 text-gray-700 dark:text-gray-300 flex-shrink-0" />
                    <span className="flex-1 text-left text-xs font-medium text-gray-900 dark:text-gray-100">
                      After Hours
                    </span>
                    {filterState.contextFilters.has('after_hours') && (
                      <Check className="size-3.5 text-gray-900 dark:text-gray-100 flex-shrink-0" />
                    )}
                  </button>

                  {/* Weekend */}
                  <button
                    onClick={() => handleContextFilterToggle('weekend')}
                    className={`w-full flex items-center gap-2 px-2 py-1.5 rounded-lg transition-all border ${
                      filterState.contextFilters.has('weekend')
                        ? 'bg-pink-500/20 dark:bg-pink-500/25 border-pink-500/40 dark:border-pink-500/50'
                        : 'bg-neutral-100/50 dark:bg-neutral-800/30 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                    }`}
                  >
                    <CalendarIcon className="size-4 text-gray-700 dark:text-gray-300 flex-shrink-0" />
                    <span className="flex-1 text-left text-xs font-medium text-gray-900 dark:text-gray-100">
                      Weekend
                    </span>
                    {filterState.contextFilters.has('weekend') && (
                      <Check className="size-3.5 text-gray-900 dark:text-gray-100 flex-shrink-0" />
                    )}
                  </button>

                  {/* Calendar Overlap */}
                  <button
                    onClick={() => handleContextFilterToggle('calendar')}
                    className={`w-full flex items-center gap-2 px-2 py-1.5 rounded-lg transition-all border ${
                      filterState.contextFilters.has('calendar')
                        ? 'bg-cyan-500/20 dark:bg-cyan-500/25 border-cyan-500/40 dark:border-cyan-500/50'
                        : 'bg-neutral-100/50 dark:bg-neutral-800/30 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                    }`}
                  >
                    <Clock className="size-4 text-gray-700 dark:text-gray-300 flex-shrink-0" />
                    <span className="flex-1 text-left text-xs font-medium text-gray-900 dark:text-gray-100">
                      Calendar Overlap
                    </span>
                    {filterState.contextFilters.has('calendar') && (
                      <Check className="size-3.5 text-gray-900 dark:text-gray-100 flex-shrink-0" />
                    )}
                  </button>
                </div>
              </div>
            </TabsContent>
          </Tabs>
        </div>
      </PopoverContent>
    </Popover>
  );
}
