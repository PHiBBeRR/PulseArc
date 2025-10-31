'use client';

import { ChevronLeft, ChevronRight } from 'lucide-react';
import { DayPicker, type DayPickerProps } from 'react-day-picker';

import { cn } from './utils';

export type CalendarProps = DayPickerProps;

function Calendar({ className, classNames, showOutsideDays = true, ...props }: CalendarProps) {
  return (
    <DayPicker
      showOutsideDays={showOutsideDays}
      className={cn('p-4', className)}
      classNames={{
        month_caption: 'flex justify-center pt-1 relative items-center mb-1',
        caption_label: 'text-sm font-semibold text-gray-900 dark:text-gray-100',
        nav: 'absolute inset-x-0 top-0 flex items-center justify-between px-1',
        button_previous:
          'h-9 w-9 bg-transparent p-0 opacity-50 hover:opacity-100 hover:bg-neutral-200 dark:hover:bg-neutral-800 rounded-md transition-all inline-flex items-center justify-center cursor-pointer',
        button_next:
          'h-9 w-9 bg-transparent p-0 opacity-50 hover:opacity-100 hover:bg-neutral-200 dark:hover:bg-neutral-800 rounded-md transition-all inline-flex items-center justify-center cursor-pointer',
        month_grid: 'w-full border-collapse mt-4',
        weekdays: 'flex',
        weekday: 'text-gray-600 dark:text-gray-400 w-9 font-normal text-xs text-center',
        week: 'flex w-full mt-2',
        day: 'h-9 w-9 p-0 font-normal text-gray-900 dark:text-gray-100 hover:bg-neutral-200 dark:hover:bg-neutral-800 rounded-md inline-flex items-center justify-center text-sm transition-all',
        day_button: 'h-9 w-9',
        selected:
          'bg-blue-500 text-white hover:bg-blue-600 dark:bg-blue-600 dark:hover:bg-blue-700 font-medium',
        today: 'bg-neutral-200 dark:bg-neutral-800 text-gray-900 dark:text-gray-100 font-semibold',
        outside: 'text-gray-500 dark:text-gray-500 opacity-50',
        disabled: 'text-gray-500 dark:text-gray-500 opacity-50',
        hidden: 'invisible',
        ...classNames,
      }}
      components={{
        Chevron: (chevronProps) => {
          const Icon = chevronProps.orientation === 'left' ? ChevronLeft : ChevronRight;
          return <Icon className="h-4 w-4 text-gray-600 dark:text-gray-400" />;
        },
      }}
      {...props}
    />
  );
}
Calendar.displayName = 'Calendar';

export { Calendar };
