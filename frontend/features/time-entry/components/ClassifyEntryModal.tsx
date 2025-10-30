/**
 * Classify Entry Modal
 * Modal for assigning unallocated entries to projects
 * Used to categorize time entries as billable or G&A
 */

import { useState, useEffect } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { ArrowRight, FolderKanban } from 'lucide-react';
import { WbsAutocomplete } from '@/features/timer/components/WbsAutocomplete';
import type { TimeEntry } from '../types';
import type { AcceptPatch, WbsElement } from '@/shared/types/generated';
import { haptic } from '@/shared/utils';

interface ClassifyEntryModalProps {
  entry: TimeEntry | null;
  isOpen: boolean;
  onClose: () => void;
  onClassify: (_entry: TimeEntry, _patch: AcceptPatch) => Promise<void>;
}

// Helper function to get card colors based on entry source/category
const getEntryCardColors = (entry: TimeEntry | null) => {
  if (!entry) return 'bg-neutral-100 dark:bg-neutral-800 border-neutral-300 dark:border-neutral-700';

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

export function ClassifyEntryModal({ entry, isOpen, onClose, onClassify }: ClassifyEntryModalProps) {
  const [wbsCode, setWbsCode] = useState('');
  const [selectedWbs, setSelectedWbs] = useState<WbsElement | undefined>();
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setWbsCode('');
      setSelectedWbs(undefined);
    }
  }, [isOpen]);

  const handleSubmit = async () => {
    if (!entry || !wbsCode) return;

    haptic.light();
    setIsSubmitting(true);

    try {
      // Extract project name from WBS element or use default
      const projectName = selectedWbs?.project_name || entry.project || 'Unallocated';

      const patch: AcceptPatch = {
        title: null,
        project: projectName,
        wbs_code: wbsCode,
        duration_sec: entry.durationSeconds ?? 0,
        entry_date: null,
      };

      await onClassify(entry, patch);
      setWbsCode('');
      setSelectedWbs(undefined);
      onClose();
    } catch (error) {
      console.error('Failed to classify entry:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCancel = () => {
    haptic.light();
    setWbsCode('');
    setSelectedWbs(undefined);
    onClose();
  };

  if (!entry) return null;

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && handleCancel()}>
      <DialogContent hideClose hideOverlay className="bg-neutral-100 dark:bg-neutral-900 border-2 border-neutral-200 dark:border-neutral-700 shadow-xl sm:max-w-[360px] p-5 gap-4">
        <DialogHeader>
          <DialogTitle className="text-gray-900 dark:text-gray-100 text-sm flex items-center gap-2">
            <FolderKanban className="w-4 h-4" />
            Classify time entry
          </DialogTitle>
        </DialogHeader>

        {/* Entry Preview */}
        <div className={`${getEntryCardColors(entry)} rounded-lg p-3`}>
          <div className="text-sm text-gray-900 dark:text-gray-50 mb-1">
            {entry.project ?? entry.category ?? 'Unallocated'}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400 mb-1">
            {entry.task}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">
            {entry.shortDate ?? entry.time}
            {(entry.shortDate ?? entry.time) && ' • '}
            {entry.duration}
          </div>
        </div>

        {/* WBS Code Search (Standardized) */}
        <div className="space-y-2">
          <p className="text-xs text-gray-600 dark:text-gray-400">
            Assign this entry to a project via WBS code
          </p>
          <WbsAutocomplete
            value={wbsCode}
            onChange={(code, element) => {
              setWbsCode(code);
              setSelectedWbs(element);
              haptic.light();
            }}
            placeholder="Search WBS code..."
            disabled={isSubmitting}
            buttonClassName="w-full bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-600 rounded-lg h-9 text-sm"
            popoverClassName="w-[310px] bg-white dark:bg-neutral-800 border-neutral-200 dark:border-neutral-700"
          />
        </div>

        {/* Action Buttons */}
        <div className="flex gap-2">
          <Button
            variant="ghost"
            onClick={handleCancel}
            disabled={isSubmitting}
            className="flex-1 bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600 h-9 text-xs"
          >
            Cancel
          </Button>
          <Button
            onClick={() => void handleSubmit()}
            disabled={!wbsCode || isSubmitting}
            className="flex-1 bg-blue-500/20 hover:bg-blue-500/30 dark:bg-blue-500/20 dark:hover:bg-blue-500/30 text-blue-600 dark:text-blue-400 disabled:opacity-50 disabled:cursor-not-allowed h-9 text-xs"
          >
            Submit
            <ArrowRight className="w-3 h-3 ml-1.5" />
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
