import { ErrorMessage, LoadingSpinner } from '@/shared/components';
import { haptic } from '@/shared/utils';
import { AnimatePresence, motion } from 'framer-motion';
import { Zap } from 'lucide-react';
import { useEffect } from 'react';
import { useProjectStore } from '../stores';
import type { QuickProjectSwitcherProps } from '../types';
import { getProjectColor } from '../utils/projectColors';

export function QuickProjectSwitcher({ isOpen, onClose, onSelect }: QuickProjectSwitcherProps) {
  const { recentProjects, loading, error, fetchRecentProjects } = useProjectStore();

  // Fetch recent projects when component mounts or modal opens
  useEffect(() => {
    if (isOpen && recentProjects.length === 0 && !loading) {
      void fetchRecentProjects();
    }
  }, [isOpen, recentProjects.length, loading, fetchRecentProjects]);

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
                <Zap className="w-3.5 h-3.5 text-gray-600 dark:text-gray-400" />
                <span className="text-sm text-gray-900 dark:text-gray-100 font-semibold">
                  Quick Start
                </span>
              </div>
              {error ? (
                <ErrorMessage
                  message={error}
                  onRetry={() => void fetchRecentProjects()}
                  className="py-8"
                />
              ) : loading ? (
                <div className="py-8">
                  <LoadingSpinner size="md" text="Loading projects..." />
                </div>
              ) : recentProjects.length === 0 ? (
                <div className="py-8 text-center">
                  <p className="text-xs text-gray-600 dark:text-gray-400">No recent projects</p>
                </div>
              ) : (
                <div className="space-y-1">
                  {recentProjects.map((project) => {
                    const projectColor = getProjectColor(project.project);
                    return (
                      <button
                        key={project.id}
                        onClick={() => {
                          haptic.light();
                          onSelect(project);
                          onClose();
                        }}
                        className="w-full text-left px-3 py-2 rounded-xl hover:bg-neutral-200 dark:hover:bg-neutral-800 transition-colors group"
                      >
                        <div className="flex items-center gap-2">
                          <div className={`w-2 h-2 rounded-full ${projectColor.dot}`} />
                          <div className="flex-1">
                            <div className="text-sm text-gray-900 dark:text-gray-100 mb-0.5 group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors">
                              {project.task}
                            </div>
                            <div className={`text-xs ${projectColor.text}`}>{project.project}</div>
                          </div>
                        </div>
                      </button>
                    );
                  })}
                </div>
              )}
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
