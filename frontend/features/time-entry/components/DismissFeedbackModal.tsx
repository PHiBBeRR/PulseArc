/**
 * Dismiss Feedback Modal
 * Modal for collecting user feedback when dismissing suggested entries
 * Used to help train the AI model
 */

import { Button } from '@/components/ui/button';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Textarea } from '@/components/ui/textarea';
import { cn } from '@/components/ui/utils';
import { haptic } from '@/shared/utils';
import { Check, ChevronsUpDown, MessageSquare, Send } from 'lucide-react';
import { useState } from 'react';
import type { TimeEntry } from '../types';

interface DismissFeedbackModalProps {
  entry: TimeEntry | null;
  isOpen: boolean;
  onClose: () => void;
  onDismiss: (_entry: TimeEntry, _reason: string) => Promise<void>;
}

// Pre-defined quick feedback options
const QUICK_FEEDBACK_OPTIONS = [
  { value: 'wrong-project', label: 'Wrong project' },
  { value: 'incorrect-time', label: 'Incorrect time/duration' },
  { value: 'duplicate', label: 'Duplicate entry' },
  { value: 'not-relevant', label: 'Not relevant to my work' },
  { value: 'wrong-task', label: 'Wrong task description' },
  { value: 'personal', label: 'Personal time (not work)' },
  { value: 'other', label: 'Other (please specify)' },
];

// Helper function to get card colors based on entry source/category
const getEntryCardColors = (entry: TimeEntry | null) => {
  if (!entry)
    return 'bg-neutral-100 dark:bg-neutral-800 border-neutral-300 dark:border-neutral-700';

  // Calendar = orange
  if (entry.source === 'calendar') {
    return 'bg-gradient-to-br from-orange-500/10 to-amber-500/10 dark:from-orange-400/10 dark:to-amber-400/10 border border-orange-500/20 dark:border-orange-400/20';
  }

  // AI = purple
  if (entry.source === 'ai' || entry.category === 'ai') {
    return 'bg-gradient-to-br from-purple-500/10 to-indigo-500/10 dark:from-purple-400/10 dark:to-indigo-400/10 border border-purple-500/20 dark:border-purple-400/20';
  }

  // Personal = yellow
  if (entry.category === 'personal') {
    return 'bg-gradient-to-br from-yellow-500/10 to-amber-500/10 dark:from-yellow-400/10 dark:to-amber-400/10 border border-yellow-500/20 dark:border-yellow-400/20';
  }

  // General = blue
  if (entry.category === 'general') {
    return 'bg-gradient-to-br from-blue-500/10 to-cyan-500/10 dark:from-blue-400/10 dark:to-cyan-400/10 border border-blue-500/20 dark:border-blue-400/20';
  }

  // Default = grey
  return 'bg-neutral-100 dark:bg-neutral-800 border-neutral-300 dark:border-neutral-700';
};

export function DismissFeedbackModal({
  entry,
  isOpen,
  onClose,
  onDismiss,
}: DismissFeedbackModalProps) {
  const [open, setOpen] = useState(false);
  const [selectedReason, setSelectedReason] = useState<(typeof QUICK_FEEDBACK_OPTIONS)[0] | null>(
    null
  );
  const [feedback, setFeedback] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleReasonSelect = (option: (typeof QUICK_FEEDBACK_OPTIONS)[0]) => {
    setSelectedReason(option);
    setOpen(false);
    haptic.light();

    // Auto-fill feedback based on selection (except "other")
    if (option.value !== 'other') {
      setFeedback(option.label);
    } else {
      setFeedback(''); // Clear for custom input
    }
  };

  const handleSubmit = async () => {
    if (!entry) return;

    haptic.light();
    setIsSubmitting(true);

    try {
      // Use feedback text (either auto-filled from dropdown or custom)
      await onDismiss(entry, feedback);
      setFeedback('');
      setSelectedReason(null);
      onClose();
    } catch (error) {
      console.error('Failed to dismiss entry:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleSkip = async () => {
    if (!entry) return;

    haptic.light();
    setIsSubmitting(true);

    try {
      // Submit with empty feedback
      await onDismiss(entry, '');
      setFeedback('');
      setSelectedReason(null);
      onClose();
    } catch (error) {
      console.error('Failed to dismiss entry:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!entry) return null;

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        hideClose
        hideOverlay
        className="bg-neutral-100 dark:bg-neutral-900 border-2 border-neutral-200 dark:border-neutral-700 shadow-xl sm:max-w-[360px] p-5 gap-4"
      >
        <DialogHeader className="text-left">
          <DialogTitle className="text-gray-900 dark:text-gray-100 text-sm flex items-center gap-2">
            <MessageSquare className="w-4 h-4" />
            Help improve suggestions
          </DialogTitle>
        </DialogHeader>

        {/* Entry Preview */}
        <div className={`${getEntryCardColors(entry)} rounded-lg p-3`}>
          <div className="text-sm text-gray-900 dark:text-gray-50 mb-1">
            {entry.project ?? entry.category ?? 'Unallocated'}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400 mb-1">{entry.task}</div>
          <div className="text-xs text-gray-600 dark:text-gray-400">
            {entry.shortDate ?? entry.time}
            {(entry.shortDate ?? entry.time) && ' • '}
            {entry.duration}
          </div>
        </div>

        {/* Quick Reason Selection */}
        <div className="space-y-2">
          <p className="text-xs text-gray-600 dark:text-gray-400">
            Why isn't this suggestion helpful?
          </p>

          <Popover open={open} onOpenChange={setOpen}>
            <PopoverTrigger asChild>
              <Button
                variant="outline"
                role="combobox"
                aria-expanded={open}
                disabled={isSubmitting}
                className="w-full justify-between bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-600 h-9 text-sm"
              >
                {selectedReason ? (
                  <span className="truncate">{selectedReason.label}</span>
                ) : (
                  <span className="text-gray-500 dark:text-gray-400">Choose a reason...</span>
                )}
                <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
              </Button>
            </PopoverTrigger>
            <PopoverContent className="w-[310px] p-0 bg-white dark:bg-neutral-800 border-neutral-200 dark:border-neutral-700">
              <Command>
                <CommandInput placeholder="Search reasons..." className="h-9" />
                <CommandList className="max-h-[170px] overflow-y-auto scrollbar-hide">
                  <CommandEmpty>No reason found.</CommandEmpty>
                  <CommandGroup>
                    {QUICK_FEEDBACK_OPTIONS.map((option) => (
                      <CommandItem
                        key={option.value}
                        value={option.label}
                        onSelect={() => handleReasonSelect(option)}
                      >
                        <Check
                          className={cn(
                            'mr-2 h-4 w-4',
                            selectedReason?.value === option.value ? 'opacity-100' : 'opacity-0'
                          )}
                        />
                        <span className="text-sm">{option.label}</span>
                      </CommandItem>
                    ))}
                  </CommandGroup>
                </CommandList>
              </Command>
            </PopoverContent>
          </Popover>
        </div>

        {/* Feedback Input */}
        <Textarea
          value={feedback}
          onChange={(e) => setFeedback(e.target.value)}
          placeholder="Optional: Add more details or write a custom reason..."
          className="bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-600 rounded-lg min-h-[80px] resize-none text-sm"
          disabled={isSubmitting}
        />

        {/* Action Buttons */}
        <div className="flex gap-2">
          <Button
            onClick={() => void handleSkip()}
            disabled={isSubmitting}
            variant="ghost"
            className="flex-1 bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600 h-9 text-xs"
          >
            Skip
          </Button>
          <Button
            onClick={() => void handleSubmit()}
            disabled={isSubmitting}
            className="flex-1 bg-blue-500/20 hover:bg-blue-500/30 dark:bg-blue-500/20 dark:hover:bg-blue-500/30 text-blue-600 dark:text-blue-400 h-9 text-xs"
          >
            Submit
            <Send className="w-3 h-3 ml-1.5" />
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
