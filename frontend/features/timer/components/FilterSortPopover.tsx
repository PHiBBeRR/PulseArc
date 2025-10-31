import { Badge } from '@/shared/components/ui/badge';
import { Popover, PopoverContent, PopoverTrigger } from '@/shared/components/ui/popover';
import { Separator } from '@/shared/components/ui/separator';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/shared/components/ui/tabs';
import { Brain, Calendar, Check, Filter, SortAsc, User, Users } from 'lucide-react';
import { useCallback, useState, type ReactNode } from 'react';

export type SortOption = 'most-recent' | 'oldest-first' | 'longest-duration' | 'shortest-duration';
export type SourceFilter = 'calendar' | 'ai';
export type CategoryFilter = 'personal' | 'general' | 'project' | 'ai';

export interface FilterSortState {
  sortBy: SortOption;
  sourceFilters: Set<SourceFilter>;
  categoryFilters: Set<CategoryFilter>;
}

export interface FilterSortPopoverProps {
  trigger: ReactNode;
  filterSortState: FilterSortState;
  onFilterSortChange: (state: FilterSortState) => void;
  activeFilterCount: number;
}

export function FilterSortPopover({
  trigger,
  filterSortState,
  onFilterSortChange,
  activeFilterCount,
}: FilterSortPopoverProps) {
  const [activeTab, setActiveTab] = useState<'sort' | 'filter'>('sort');

  const handleTabChange = useCallback((value: string) => {
    setActiveTab(value as 'sort' | 'filter');
  }, []);

  const handleSortChange = useCallback(
    (value: SortOption) => {
      onFilterSortChange({
        ...filterSortState,
        sortBy: value,
      });
    },
    [filterSortState, onFilterSortChange]
  );

  const handleSourceFilterToggle = useCallback(
    (source: SourceFilter) => {
      const newSourceFilters = new Set(filterSortState.sourceFilters);
      if (newSourceFilters.has(source)) {
        newSourceFilters.delete(source);
      } else {
        newSourceFilters.add(source);
      }
      onFilterSortChange({
        ...filterSortState,
        sourceFilters: newSourceFilters,
      });
    },
    [filterSortState, onFilterSortChange]
  );

  const handleCategoryFilterToggle = useCallback(
    (category: CategoryFilter) => {
      const newCategoryFilters = new Set(filterSortState.categoryFilters);
      if (newCategoryFilters.has(category)) {
        newCategoryFilters.delete(category);
      } else {
        newCategoryFilters.add(category);
      }
      onFilterSortChange({
        ...filterSortState,
        categoryFilters: newCategoryFilters,
      });
    },
    [filterSortState, onFilterSortChange]
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
                      onFilterSortChange({
                        sortBy: filterSortState.sortBy,
                        sourceFilters: new Set(),
                        categoryFilters: new Set(),
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
                    filterSortState.sortBy === 'most-recent'
                      ? 'bg-neutral-200 dark:bg-neutral-800 text-gray-900 dark:text-gray-100 border-neutral-300 dark:border-neutral-700'
                      : 'text-gray-700 dark:text-gray-300 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                  }`}
                >
                  Most Recent
                </button>
                <button
                  onClick={() => handleSortChange('oldest-first')}
                  className={`w-full text-left px-2 py-1.5 rounded-lg text-xs font-medium transition-colors border ${
                    filterSortState.sortBy === 'oldest-first'
                      ? 'bg-neutral-200 dark:bg-neutral-800 text-gray-900 dark:text-gray-100 border-neutral-300 dark:border-neutral-700'
                      : 'text-gray-700 dark:text-gray-300 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                  }`}
                >
                  Oldest First
                </button>
                <button
                  onClick={() => handleSortChange('longest-duration')}
                  className={`w-full text-left px-2 py-1.5 rounded-lg text-xs font-medium transition-colors border ${
                    filterSortState.sortBy === 'longest-duration'
                      ? 'bg-neutral-200 dark:bg-neutral-800 text-gray-900 dark:text-gray-100 border-neutral-300 dark:border-neutral-700'
                      : 'text-gray-700 dark:text-gray-300 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                  }`}
                >
                  Longest Duration
                </button>
                <button
                  onClick={() => handleSortChange('shortest-duration')}
                  className={`w-full text-left px-2 py-1.5 rounded-lg text-xs font-medium transition-colors border ${
                    filterSortState.sortBy === 'shortest-duration'
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
                {/* SOURCE Section */}
                <div className="space-y-1.5">
                  <h4 className="text-[10px] font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">
                    Source
                  </h4>

                  {/* Calendar */}
                  <button
                    onClick={() => handleSourceFilterToggle('calendar')}
                    className={`w-full flex items-center gap-2 px-2 py-1.5 rounded-lg transition-all border ${
                      filterSortState.sourceFilters.has('calendar')
                        ? 'bg-gray-600/20 dark:bg-gray-600/30 border-gray-600/40 dark:border-gray-600/50'
                        : 'bg-neutral-100/50 dark:bg-neutral-800/30 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                    }`}
                  >
                    <Calendar className="size-4 text-gray-700 dark:text-gray-300 flex-shrink-0" />
                    <span className="flex-1 text-left text-xs font-medium text-gray-900 dark:text-gray-100">
                      Calendar
                    </span>
                    {filterSortState.sourceFilters.has('calendar') && (
                      <Check className="size-3.5 text-gray-900 dark:text-gray-100 flex-shrink-0" />
                    )}
                  </button>

                  {/* Activity Tracker */}
                  <button
                    onClick={() => handleSourceFilterToggle('ai')}
                    className={`w-full flex items-center gap-2 px-2 py-1.5 rounded-lg transition-all border ${
                      filterSortState.sourceFilters.has('ai')
                        ? 'bg-purple-500/20 dark:bg-purple-500/25 border-purple-500/40 dark:border-purple-500/50'
                        : 'bg-neutral-100/50 dark:bg-neutral-800/30 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                    }`}
                  >
                    <Brain className="size-4 text-gray-700 dark:text-gray-300 flex-shrink-0" />
                    <span className="flex-1 text-left text-xs font-medium text-gray-900 dark:text-gray-100">
                      Activity Tracker
                    </span>
                    {filterSortState.sourceFilters.has('ai') && (
                      <Check className="size-3.5 text-gray-900 dark:text-gray-100 flex-shrink-0" />
                    )}
                  </button>
                </div>

                <Separator className="bg-neutral-200 dark:bg-neutral-700" />

                {/* CATEGORY Section */}
                <div className="space-y-1.5">
                  <h4 className="text-[10px] font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">
                    Category
                  </h4>

                  {/* Personal */}
                  <button
                    onClick={() => handleCategoryFilterToggle('personal')}
                    className={`w-full flex items-center gap-2 px-2 py-1.5 rounded-lg transition-all border ${
                      filterSortState.categoryFilters.has('personal')
                        ? 'bg-yellow-600/25 dark:bg-yellow-600/20 border-yellow-600/40 dark:border-yellow-600/50'
                        : 'bg-neutral-100/50 dark:bg-neutral-800/30 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                    }`}
                  >
                    <User className="size-4 text-gray-700 dark:text-gray-300 flex-shrink-0" />
                    <span className="flex-1 text-left text-xs font-medium text-gray-900 dark:text-gray-100">
                      Personal
                    </span>
                    {filterSortState.categoryFilters.has('personal') && (
                      <Check className="size-3.5 text-gray-900 dark:text-gray-100 flex-shrink-0" />
                    )}
                  </button>

                  {/* General */}
                  <button
                    onClick={() => handleCategoryFilterToggle('general')}
                    className={`w-full flex items-center gap-2 px-2 py-1.5 rounded-lg transition-all border ${
                      filterSortState.categoryFilters.has('general')
                        ? 'bg-blue-500/20 dark:bg-blue-500/25 border-blue-500/40 dark:border-blue-500/50'
                        : 'bg-neutral-100/50 dark:bg-neutral-800/30 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                    }`}
                  >
                    <Users className="size-4 text-gray-700 dark:text-gray-300 flex-shrink-0" />
                    <span className="flex-1 text-left text-xs font-medium text-gray-900 dark:text-gray-100">
                      General
                    </span>
                    {filterSortState.categoryFilters.has('general') && (
                      <Check className="size-3.5 text-gray-900 dark:text-gray-100 flex-shrink-0" />
                    )}
                  </button>

                  {/* Project */}
                  <button
                    onClick={() => handleCategoryFilterToggle('project')}
                    className={`w-full flex items-center gap-2 px-2 py-1.5 rounded-lg transition-all border ${
                      filterSortState.categoryFilters.has('project')
                        ? 'bg-orange-600/25 dark:bg-orange-600/20 border-orange-600/40 dark:border-orange-600/50'
                        : 'bg-neutral-100/50 dark:bg-neutral-800/30 hover:bg-neutral-100 dark:hover:bg-neutral-800/50 border-neutral-200/50 dark:border-neutral-700/50'
                    }`}
                  >
                    <Calendar className="size-4 text-gray-700 dark:text-gray-300 flex-shrink-0" />
                    <span className="flex-1 text-left text-xs font-medium text-gray-900 dark:text-gray-100">
                      Project
                    </span>
                    {filterSortState.categoryFilters.has('project') && (
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
