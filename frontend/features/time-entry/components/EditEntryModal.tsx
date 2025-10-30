/**
 * Edit Entry Modal
 * Modal for editing suggested time entries before accepting
 * FEATURE-019 Phase 3
 */

import { useState, useEffect } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Pencil, Clock, Send } from 'lucide-react';
import { WbsAutocomplete } from '@/features/timer/components/WbsAutocomplete';
import type { TimeEntry } from '../types';
import type { AcceptPatch, WbsElement } from '@/shared/types/generated';
import { haptic } from '@/shared/utils';

interface EditEntryModalProps {
  entry: TimeEntry | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (_entry: TimeEntry, _patch: AcceptPatch) => Promise<void>;
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

export function EditEntryModal({ entry, isOpen, onClose, onSave }: EditEntryModalProps) {
  const [wbsCode, setWbsCode] = useState('');
  const [selectedWbs, setSelectedWbs] = useState<WbsElement | undefined>();
  const [task, setTask] = useState('');
  const [duration, setDuration] = useState(0); // in seconds
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Pre-fill form when entry changes
  useEffect(() => {
    if (entry && isOpen) {
      setTask(entry.task || '');
      setDuration(entry.durationSeconds ?? 0);
      setWbsCode(entry.wbsCode || '');
      // Clear selected WBS element to force fresh lookup
      setSelectedWbs(undefined);
    }
  }, [entry, isOpen]);

  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
  };

  const handleSave = async () => {
    if (!entry || !wbsCode) return;

    haptic.success();
    setIsSubmitting(true);

    try {
      // Extract project name from WBS element or keep original
      const projectName = selectedWbs?.project_name || entry.project;

      const patch: AcceptPatch = {
        title: task !== entry.task ? task : null,
        project: projectName !== entry.project ? projectName : null,
        wbs_code: wbsCode !== entry.wbsCode ? wbsCode : null,
        duration_sec: duration !== entry.durationSeconds ? duration : undefined,
        entry_date: null, // Keep original date
      };

      await onSave(entry, patch);
      handleClose();
    } catch (error) {
      console.error('Failed to save entry:', error);
      setIsSubmitting(false);
    }
  };

  const handleClose = () => {
    setTask('');
    setDuration(0);
    setWbsCode('');
    setSelectedWbs(undefined);
    setIsSubmitting(false);
    onClose();
  };

  if (!entry) return null;

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent hideClose hideOverlay className="bg-neutral-100 dark:bg-neutral-900 border-2 border-neutral-200 dark:border-neutral-700 shadow-xl sm:max-w-[360px] p-5 gap-4">
        <DialogHeader>
          <DialogTitle className="text-gray-900 dark:text-gray-100 text-sm flex items-center gap-2">
            <Pencil className="w-4 h-4" />
            Edit Suggestion
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {/* Entry Info Header */}
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
              <Clock className="w-3 h-3 inline" />
              {' '}{formatDuration(duration)}
            </div>
          </div>

          {/* WBS Code Search (Standardized across all entry points) */}
          <div className="space-y-2">
            <p className="text-xs text-gray-600 dark:text-gray-400">
              Search WBS code or project name
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
              buttonClassName="w-full bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-600 h-9 text-sm"
              popoverClassName="w-[310px] bg-white dark:bg-neutral-800 border-neutral-200 dark:border-neutral-700"
            />
          </div>

          {/* Task Description */}
          <Input
            value={task}
            onChange={(e) => setTask(e.target.value)}
            placeholder="Task description"
            disabled={isSubmitting}
            className="bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-600 h-9 text-sm"
          />
        </div>

        {/* Action Buttons */}
        <div className="flex gap-2 pt-1">
          <Button
            type="button"
            variant="ghost"
            onClick={handleClose}
            disabled={isSubmitting}
            className="flex-1 bg-neutral-200 dark:bg-neutral-700 hover:bg-neutral-300 dark:hover:bg-neutral-600 h-9 text-xs"
          >
            Cancel
          </Button>
          <Button
            onClick={() => void handleSave()}
            disabled={!wbsCode || isSubmitting}
            className="flex-1 bg-blue-500/20 hover:bg-blue-500/30 dark:bg-blue-500/20 dark:hover:bg-blue-500/30 text-blue-600 dark:text-blue-400 disabled:opacity-50 disabled:cursor-not-allowed h-9 text-xs"
          >
            {isSubmitting ? 'Saving...' : 'Submit'}
            <Send className="w-3 h-3 ml-1.5" />
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
