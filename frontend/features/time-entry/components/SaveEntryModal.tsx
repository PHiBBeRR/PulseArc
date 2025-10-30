import { useState, useEffect } from 'react';
import { Check, X, Edit2, Plus, Minus } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { haptic, celebrateWithConfetti } from '@/shared/utils';
import { getProjectColor } from '../../project/utils/projectColors';
import { entryService } from '../services';
import type { SaveEntryModalProps } from '../types';

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

export function SaveEntryModal({ isOpen, onClose, onAccept, onReject, duration, elapsedSeconds, activityContext }: SaveEntryModalProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [aiSuggestion, setAiSuggestion] = useState(entryService.getAISuggestion(elapsedSeconds));
  const [editedProject, setEditedProject] = useState('');
  const [editedTask, setEditedTask] = useState('');
  const [editedDuration, setEditedDuration] = useState('');
  const [editedDescription, setEditedDescription] = useState('');

  // Update AI suggestion when modal opens
  useEffect(() => {
    if (isOpen) {
      let suggestion;

      // Use activity context if available, otherwise fall back to time-based AI suggestion
      if (activityContext) {
        suggestion = {
          project: activityContext.suggested_client ?? activityContext.active_app.app_name ?? 'Unknown Project',
          task: activityContext.detected_activity ?? activityContext.active_app.window_title ?? 'Work session',
          confidence: Math.round(activityContext.billable_confidence * 100),
          reason: 'Based on your recent activity',
        };
      } else {
        suggestion = entryService.getAISuggestion(elapsedSeconds);
      }

      setAiSuggestion(suggestion);
      setEditedProject(suggestion.project);
      setEditedTask(suggestion.task);
      setEditedDuration(duration);
      setEditedDescription('');
      setIsEditing(false);
    }
  }, [isOpen, elapsedSeconds, duration, activityContext]);

  const handleAccept = () => {
    haptic.success();
    celebrateWithConfetti({ particleCount: 60, spread: 55 });
    onAccept({
      project: editedProject,
      task: editedTask,
      duration,
    });
    onClose();
  };

  const handleReject = () => {
    haptic.light();
    onReject();
    onClose();
  };

  const handleCorrect = () => {
    setIsEditing(true);
    haptic.light();
  };

  const handleSaveCorrection = () => {
    haptic.success();
    celebrateWithConfetti({ particleCount: 50, spread: 50 });
    onAccept({
      project: editedProject,
      task: editedTask,
      duration: editedDuration,
    });
    onClose();
  };

  const projectColor = getProjectColor(aiSuggestion.project);

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
              {!isEditing ? (
                /* View Mode */
                <div className="space-y-3">
                  {/* Header */}
                  <div className="flex items-center gap-2 mb-3 pb-2 border-b border-neutral-200 dark:border-neutral-700">
                    <span className="text-sm text-gray-900 dark:text-gray-100 font-semibold">Save Time Entry</span>
                  </div>

                  {/* Project */}
                  <div className="flex items-center gap-2">
                    <div className={`w-2 h-2 rounded-full ${projectColor.dot}`} />
                    <span className={`text-sm font-medium ${projectColor.text}`}>{aiSuggestion.project}</span>
                    <span className="text-xs text-gray-500 dark:text-gray-400 ml-auto">{duration}</span>
                  </div>

                  {/* Task */}
                  <p className="text-sm text-gray-700 dark:text-gray-300 leading-relaxed">{aiSuggestion.task}</p>

                  {/* Actions */}
                  <div className="flex gap-1.5 pt-1">
                    <Button
                      variant="ghost"
                      onClick={handleReject}
                      className="flex-1 h-8 bg-neutral-200 hover:bg-neutral-300 dark:bg-neutral-800 dark:hover:bg-neutral-700 text-gray-700 dark:text-gray-300 text-xs px-2"
                    >
                      <X className="w-3.5 h-3.5 mr-1" />
                      Reject
                    </Button>
                    <Button
                      variant="ghost"
                      onClick={handleCorrect}
                      className="flex-1 h-8 bg-neutral-200 hover:bg-neutral-300 dark:bg-neutral-800 dark:hover:bg-neutral-700 text-gray-700 dark:text-gray-300 text-xs px-2"
                    >
                      <Edit2 className="w-3.5 h-3.5 mr-1" />
                      Edit
                    </Button>
                    <Button
                      onClick={handleAccept}
                      className="flex-1 h-8 bg-blue-500 hover:bg-blue-600 dark:bg-blue-600 dark:hover:bg-blue-700 text-white text-xs px-2"
                    >
                      <Check className="w-3.5 h-3.5 mr-1" />
                      Save
                    </Button>
                  </div>
                </div>
              ) : (
                /* Edit Mode */
                <>
                  <div className="flex items-center gap-2 mb-3 pb-2 border-b border-neutral-200 dark:border-neutral-700">
                    <Edit2 className="w-3.5 h-3.5 text-gray-600 dark:text-gray-400" />
                    <span className="text-sm text-gray-900 dark:text-gray-100 font-semibold">Edit Entry</span>
                  </div>

                  <div className="space-y-2.5">
                    <div className="relative">
                      <Input
                        list="project-suggestions-edit"
                        value={editedProject}
                        onChange={(e) => setEditedProject(e.target.value)}
                        className="h-8 bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-600 text-gray-900 dark:text-gray-100 text-sm"
                        placeholder="Project name"
                      />
                      <datalist id="project-suggestions-edit">
                        <option value="Project Alpha" />
                        <option value="Project Beta" />
                        <option value="Internal" />
                        <option value="Meetings" />
                        <option value="Admin" />
                      </datalist>
                    </div>
                    <div>
                      <Input
                        list="task-suggestions-edit"
                        value={editedTask}
                        onChange={(e) => setEditedTask(e.target.value)}
                        className="h-8 bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-600 text-gray-900 dark:text-gray-100 text-sm"
                        placeholder="What did you work on?"
                      />
                      <datalist id="task-suggestions-edit">
                        <option value="Feature development" />
                        <option value="Bug fixing" />
                        <option value="Code review" />
                        <option value="Meeting" />
                        <option value="Documentation" />
                        <option value="Planning" />
                        <option value="Testing" />
                      </datalist>
                    </div>
                    <div>
                      <div className="flex items-center gap-1.5">
                        <Button
                          type="button"
                          variant="ghost"
                          onClick={() => {
                            const minutes = parseDuration(editedDuration);
                            if (minutes > 15) {
                              setEditedDuration(formatDuration(minutes - 15));
                            }
                          }}
                          className="h-8 w-8 p-0 bg-neutral-200 hover:bg-neutral-300 dark:bg-neutral-800 dark:hover:bg-neutral-700 text-gray-700 dark:text-gray-300"
                        >
                          <Minus className="w-3.5 h-3.5" />
                        </Button>
                        <Input
                          value={editedDuration}
                          onChange={(e) => setEditedDuration(e.target.value)}
                          className="h-8 flex-1 bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-600 text-gray-900 dark:text-gray-100 text-sm text-center"
                          placeholder="e.g., 1h 30m"
                        />
                        <Button
                          type="button"
                          variant="ghost"
                          onClick={() => {
                            const minutes = parseDuration(editedDuration);
                            setEditedDuration(formatDuration(minutes + 15));
                          }}
                          className="h-8 w-8 p-0 bg-neutral-200 hover:bg-neutral-300 dark:bg-neutral-800 dark:hover:bg-neutral-700 text-gray-700 dark:text-gray-300"
                        >
                          <Plus className="w-3.5 h-3.5" />
                        </Button>
                      </div>
                    </div>
                    <div>
                      <Textarea
                        value={editedDescription}
                        onChange={(e) => setEditedDescription(e.target.value)}
                        className="bg-white dark:bg-neutral-800 border-neutral-300 dark:border-neutral-600 text-gray-900 dark:text-gray-100 text-sm min-h-[60px]"
                        placeholder="Add notes..."
                      />
                    </div>
                  </div>

                  {/* Edit Actions */}
                  <div className="flex gap-1.5 pt-3">
                    <Button
                      variant="ghost"
                      onClick={() => setIsEditing(false)}
                      className="flex-1 h-8 bg-neutral-200 hover:bg-neutral-300 dark:bg-neutral-800 dark:hover:bg-neutral-700 text-gray-700 dark:text-gray-300 text-xs"
                    >
                      Cancel
                    </Button>
                    <Button
                      onClick={handleSaveCorrection}
                      disabled={!editedProject || !editedTask}
                      className="flex-1 h-8 bg-blue-500 hover:bg-blue-600 dark:bg-blue-600 dark:hover:bg-blue-700 text-white disabled:opacity-50 disabled:cursor-not-allowed text-xs"
                    >
                      Save
                    </Button>
                  </div>
                </>
              )}
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
