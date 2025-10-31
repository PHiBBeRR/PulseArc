import { Button } from '@/shared/components/ui/button';
import { Input } from '@/shared/components/ui/input';
import type { WbsElement } from '@/shared/types/generated';
import { AnimatePresence, motion } from 'framer-motion';
import { ArrowRight, Loader2, Minus, Plus } from 'lucide-react';
import { useState } from 'react';
import { WbsAutocomplete } from '../../timer/components/WbsAutocomplete';
import type { CompactQuickEntryProps, EntryData } from '../types';

// Helper functions for duration parsing and formatting
const parseDuration = (duration: string): number => {
  if (!duration) return 0;
  const hourMatch = duration.match(/(\d+)\s*h/);
  const minuteMatch = duration.match(/(\d+)\s*m/);
  const hours = hourMatch?.[1] ? parseInt(hourMatch[1]) : 0;
  const minutes = minuteMatch?.[1] ? parseInt(minuteMatch[1]) : 0;
  return hours * 60 + minutes;
};

const formatDuration = (minutes: number): string => {
  if (minutes === 0) return '0m';
  const hours = Math.floor(minutes / 60);
  const mins = minutes % 60;
  if (hours === 0) return `${mins}m`;
  if (mins === 0) return `${hours}h`;
  return `${hours}h ${mins}m`;
};

export function CompactQuickEntry({
  isOpen,
  onClose,
  onSave,
  isLoading = false,
  showValidationErrors = false,
}: CompactQuickEntryProps) {
  const [wbsCode, setWbsCode] = useState('');
  const [selectedWbs, setSelectedWbs] = useState<WbsElement | undefined>();

  const [formData, setFormData] = useState<EntryData>({
    project: '',
    task: '',
    duration: '',
    description: '',
  });

  const [errors, setErrors] = useState({
    project: false,
    task: false,
    duration: false,
  });

  const handleSave = () => {
    // Validate required fields
    const newErrors = {
      project: !wbsCode && !selectedWbs,
      task: !formData.task,
      duration: !formData.duration,
    };

    setErrors(newErrors);

    // Show validation errors if needed
    if (showValidationErrors || Object.values(newErrors).some((error) => error)) {
      return;
    }

    // Update formData with WBS info
    const dataToSave = {
      ...formData,
      project: selectedWbs?.project_name ?? wbsCode,
    };

    // Parent component will show notification
    onSave?.(dataToSave);
    onClose();

    // Reset form
    setFormData({
      project: '',
      task: '',
      duration: '',
      description: '',
    });
    setWbsCode('');
    setSelectedWbs(undefined);
    setErrors({
      project: false,
      task: false,
      duration: false,
    });
  };

  return (
    <AnimatePresence>
      {isOpen && (
        <>
          {/* Backdrop overlay */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="absolute inset-0 z-40"
            onClick={onClose}
          />

          {/* Modal content */}
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.15 }}
            className="absolute inset-0 z-50 flex items-center justify-center p-6 pointer-events-none"
          >
            <div className="w-full max-w-xs bg-neutral-100 dark:bg-neutral-900 border-2 border-neutral-200 dark:border-neutral-700 rounded-[40px] p-5 shadow-xl pointer-events-auto">
              <div className="flex items-center gap-2 mb-3 pb-2 border-b border-neutral-200 dark:border-neutral-700">
                <Plus className="w-3.5 h-3.5 text-gray-600 dark:text-gray-400" />
                <span className="text-sm text-gray-900 dark:text-gray-100 font-semibold">
                  Quick Entry
                </span>
              </div>

              <div className="space-y-2.5">
                {/* SAP WBS Code Search (FEATURE-020 Phase 2) */}
                <div>
                  <WbsAutocomplete
                    value={wbsCode}
                    onChange={(code, element) => {
                      setWbsCode(code);
                      setSelectedWbs(element);
                    }}
                    placeholder="Search SAP WBS code..."
                    disabled={isLoading}
                    buttonClassName={`bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-600 h-8 text-sm ${errors.project ? 'border-red-500' : ''}`}
                    popoverClassName="w-[280px] bg-white dark:bg-neutral-800 border-neutral-200 dark:border-neutral-700"
                  />
                  {errors.project && (
                    <p className="text-xs text-red-500 mt-1">WBS code is required</p>
                  )}
                </div>

                <div>
                  <Input
                    list="task-suggestions-quick"
                    id="task"
                    value={formData.task}
                    onChange={(e) => setFormData((prev) => ({ ...prev, task: e.target.value }))}
                    className={`h-8 bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-600 text-gray-900 dark:text-gray-100 text-sm ${errors.task ? 'border-red-500' : ''}`}
                    placeholder="What did you work on?"
                  />
                  <datalist id="task-suggestions-quick">
                    <option value="Feature development" />
                    <option value="Bug fixing" />
                    <option value="Code review" />
                    <option value="Meeting" />
                    <option value="Documentation" />
                    <option value="Planning" />
                    <option value="Testing" />
                  </datalist>
                  {errors.task && <p className="text-xs text-red-500 mt-1">Task is required</p>}
                </div>

                <div>
                  <div className="flex items-center gap-1.5">
                    <Button
                      type="button"
                      variant="ghost"
                      onClick={() => {
                        const minutes = parseDuration(formData.duration);
                        if (minutes > 15) {
                          setFormData((prev) => ({
                            ...prev,
                            duration: formatDuration(minutes - 15),
                          }));
                        }
                      }}
                      className="h-8 w-8 p-0 bg-neutral-200 hover:bg-neutral-300 dark:bg-neutral-800 dark:hover:bg-neutral-700 text-gray-700 dark:text-gray-300"
                    >
                      <Minus className="w-3.5 h-3.5" />
                    </Button>
                    <Input
                      id="duration"
                      value={formData.duration}
                      onChange={(e) =>
                        setFormData((prev) => ({ ...prev, duration: e.target.value }))
                      }
                      className={`h-8 flex-1 bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-600 text-gray-900 dark:text-gray-100 text-sm text-center ${errors.duration ? 'border-red-500' : ''}`}
                      placeholder="e.g., 1h 30m"
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      onClick={() => {
                        const minutes = parseDuration(formData.duration);
                        setFormData((prev) => ({
                          ...prev,
                          duration: formatDuration(minutes + 15),
                        }));
                      }}
                      className="h-8 w-8 p-0 bg-neutral-200 hover:bg-neutral-300 dark:bg-neutral-800 dark:hover:bg-neutral-700 text-gray-700 dark:text-gray-300"
                    >
                      <Plus className="w-3.5 h-3.5" />
                    </Button>
                  </div>
                  {errors.duration && (
                    <p className="text-xs text-red-500 mt-1">Duration is required</p>
                  )}
                </div>
              </div>

              <div className="flex gap-1.5 pt-3">
                <Button
                  variant="ghost"
                  onClick={onClose}
                  className="flex-1 h-8 bg-neutral-200 hover:bg-neutral-300 dark:bg-neutral-800 dark:hover:bg-neutral-700 text-gray-700 dark:text-gray-300 text-xs"
                  disabled={isLoading}
                >
                  Cancel
                </Button>
                <Button
                  onClick={handleSave}
                  className="flex-1 h-8 bg-blue-500 hover:bg-blue-600 dark:bg-blue-600 dark:hover:bg-blue-700 text-white text-xs disabled:opacity-50 disabled:cursor-not-allowed"
                  disabled={isLoading || !wbsCode || !formData.task}
                >
                  {isLoading ? (
                    <>
                      <Loader2 className="w-3.5 h-3.5 mr-1 animate-spin" />
                      Submitting...
                    </>
                  ) : (
                    <>
                      Submit
                      <ArrowRight className="w-3.5 h-3.5 ml-1" />
                    </>
                  )}
                </Button>
              </div>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
