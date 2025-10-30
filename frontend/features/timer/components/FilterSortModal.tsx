import { useCallback } from 'react';
import { Filter, SortAsc } from 'lucide-react';
import { Dialog, DialogContent } from '@/components/ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Checkbox } from '@/components/ui/checkbox';
import { Separator } from '@/components/ui/separator';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

export type SortOption = 'most-recent' | 'oldest-first' | 'longest-duration' | 'shortest-duration';
export type SourceFilter = 'calendar' | 'ai';
export type CategoryFilter = 'personal' | 'general' | 'project' | 'ai';

export interface FilterSortState {
  sortBy: SortOption;
  sourceFilters: Set<SourceFilter>;
  categoryFilters: Set<CategoryFilter>;
}

export interface FilterSortModalProps {
  isOpen: boolean;
  onClose: () => void;
  filterSortState: FilterSortState;
  onFilterSortChange: (state: FilterSortState) => void;
  activeFilterCount: number;
}

export function FilterSortModal({
  isOpen,
  onClose,
  filterSortState,
  onFilterSortChange,
  activeFilterCount,
}: FilterSortModalProps) {
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

  const handleClearAll = useCallback(() => {
    onFilterSortChange({
      sortBy: 'most-recent',
      sourceFilters: new Set(),
      categoryFilters: new Set(),
    });
  }, [onFilterSortChange]);

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[340px] p-0 pt-4">
        <Tabs defaultValue="sort" className="w-full">
          <div className="px-4">
            <TabsList className="w-full">
              <TabsTrigger value="sort" className="flex-1 gap-1 text-xs">
                <SortAsc className="size-3.5" />
                Sort
              </TabsTrigger>
              <TabsTrigger value="filter" className="flex-1 gap-1 text-xs">
                <Filter className="size-3.5" />
                Filter
                {activeFilterCount > 0 && (
                  <Badge variant="secondary" className="ml-1 h-4 min-w-4 px-1 text-[10px]">
                    {activeFilterCount}
                  </Badge>
                )}
              </TabsTrigger>
            </TabsList>
          </div>

          <TabsContent value="sort" className="mt-0 px-4 pb-4 pt-3">
            <RadioGroup value={filterSortState.sortBy} onValueChange={handleSortChange}>
              <div className="space-y-2.5">
                <label className="flex items-center gap-3 cursor-pointer">
                  <RadioGroupItem value="most-recent" id="most-recent" />
                  <span className="text-sm font-medium">Most Recent</span>
                </label>
                <label className="flex items-center gap-3 cursor-pointer">
                  <RadioGroupItem value="oldest-first" id="oldest-first" />
                  <span className="text-sm font-medium">Oldest First</span>
                </label>
                <label className="flex items-center gap-3 cursor-pointer">
                  <RadioGroupItem value="longest-duration" id="longest-duration" />
                  <span className="text-sm font-medium">Longest Duration</span>
                </label>
                <label className="flex items-center gap-3 cursor-pointer">
                  <RadioGroupItem value="shortest-duration" id="shortest-duration" />
                  <span className="text-sm font-medium">Shortest Duration</span>
                </label>
              </div>
            </RadioGroup>
          </TabsContent>

          <TabsContent value="filter" className="mt-0 px-4 pb-4 pt-3">
            <div className="space-y-3">
              {/* Source Section */}
              <div className="space-y-2">
                <h4 className="text-xs font-semibold text-foreground">Source</h4>
                <div className="space-y-2.5">
                  <label className="flex items-center gap-2.5 cursor-pointer">
                    <Checkbox
                      id="calendar-filter"
                      checked={filterSortState.sourceFilters.has('calendar')}
                      onCheckedChange={() => handleSourceFilterToggle('calendar')}
                    />
                    <span className="text-sm font-medium">Calendar</span>
                  </label>
                  <label className="flex items-center gap-2.5 cursor-pointer">
                    <Checkbox
                      id="ai-source-filter"
                      checked={filterSortState.sourceFilters.has('ai')}
                      onCheckedChange={() => handleSourceFilterToggle('ai')}
                    />
                    <span className="text-sm font-medium">AI Detected</span>
                  </label>
                </div>
              </div>

              <Separator />

              {/* Category Section */}
              <div className="space-y-2">
                <h4 className="text-xs font-semibold text-foreground">Category</h4>
                <div className="space-y-2.5">
                  <label className="flex items-center gap-2.5 cursor-pointer">
                    <Checkbox
                      id="personal-filter"
                      checked={filterSortState.categoryFilters.has('personal')}
                      onCheckedChange={() => handleCategoryFilterToggle('personal')}
                    />
                    <span className="text-sm font-medium">Personal</span>
                  </label>
                  <label className="flex items-center gap-2.5 cursor-pointer">
                    <Checkbox
                      id="general-filter"
                      checked={filterSortState.categoryFilters.has('general')}
                      onCheckedChange={() => handleCategoryFilterToggle('general')}
                    />
                    <span className="text-sm font-medium">General</span>
                  </label>
                  <label className="flex items-center gap-2.5 cursor-pointer">
                    <Checkbox
                      id="project-filter"
                      checked={filterSortState.categoryFilters.has('project')}
                      onCheckedChange={() => handleCategoryFilterToggle('project')}
                    />
                    <span className="text-sm font-medium">Project</span>
                  </label>
                  <label className="flex items-center gap-2.5 cursor-pointer">
                    <Checkbox
                      id="ai-category-filter"
                      checked={filterSortState.categoryFilters.has('ai')}
                      onCheckedChange={() => handleCategoryFilterToggle('ai')}
                    />
                    <span className="text-sm font-medium">AI Activity</span>
                  </label>
                </div>
              </div>

              {/* Clear All Button */}
              {activeFilterCount > 0 && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleClearAll}
                  className="w-full mt-2"
                >
                  Clear all ({activeFilterCount})
                </Button>
              )}
            </div>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
