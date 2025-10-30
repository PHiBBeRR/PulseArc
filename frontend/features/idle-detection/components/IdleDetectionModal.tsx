import { AlertCircle, Clock } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { Button } from '@/components/ui/button';
import type { IdleDetectionModalProps } from '../types';

export function IdleDetectionModal({ isOpen, onKeepTime, onDiscardTime, idleMinutes }: IdleDetectionModalProps) {
  return (
    <AnimatePresence>
      {isOpen && (
        <>
          {/* Backdrop overlay */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-40 bg-black/20"
          />

          {/* Modal content */}
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.15 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-6 pointer-events-none"
          >
            <div className="w-full max-w-sm bg-black/[0.925] dark:bg-black/[0.925] border-2 border-white/20 dark:border-white/10 rounded-[40px] p-5 shadow-[0_8px_32px_0_rgba(0,0,0,0.2),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_8px_32px_0_rgba(0,0,0,0.4),0_0_0_1px_rgba(255,255,255,0.1)_inset] pointer-events-auto">
              <div className="flex items-center gap-2 mb-3 pb-2 border-b border-white/10 dark:border-white/10">
                <AlertCircle className="w-3.5 h-3.5 text-gray-400 dark:text-gray-400" />
                <span className="text-sm text-gray-100 dark:text-gray-100 font-semibold">Idle Time Detected</span>
              </div>
              <p className="text-gray-400 dark:text-gray-400 text-sm mb-3">
                You've been away for {idleMinutes} minutes. What would you like to do with this time?
              </p>

              <div className="p-3 rounded-xl bg-white/10 dark:bg-white/10 border border-white/20 dark:border-white/20">
                <div className="flex items-center gap-2 text-sm">
                  <Clock className="w-4 h-4 text-gray-400 dark:text-gray-400" />
                  <span className="text-gray-300 dark:text-gray-300">Idle duration:</span>
                  <span className="text-gray-100 dark:text-gray-100">{idleMinutes}m</span>
                </div>
              </div>

              {/* Warning for long idle periods (Phase 0 limitation) */}
              {idleMinutes >= 60 && (
                <div className="mt-3 p-2 bg-yellow-500/10 border border-yellow-500/20 rounded-lg">
                  <p className="text-xs text-yellow-300">
                    Note: Long idle periods may include system sleep time.
                    Future versions will detect sleep/wake cycles automatically.
                  </p>
                </div>
              )}

              <div className="flex gap-2 mt-4">
                <Button
                  onClick={onDiscardTime}
                  variant="ghost"
                  className="flex-1 h-8 bg-white/10 hover:bg-white/20 dark:bg-white/10 dark:hover:bg-white/20 text-gray-300 dark:text-gray-300 text-xs"
                >
                  Discard
                </Button>
                <Button
                  onClick={onKeepTime}
                  className="flex-1 h-8 bg-blue-500/20 hover:bg-blue-500/30 dark:bg-blue-400/20 dark:hover:bg-blue-400/30 text-blue-300 dark:text-blue-300 text-xs"
                >
                  Keep Time
                </Button>
              </div>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
