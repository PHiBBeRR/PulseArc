import { useState } from 'react';
import { Sparkles, Bell, Zap } from 'lucide-react';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Slider } from '@/components/ui/slider';
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import { Separator } from '@/components/ui/separator';
import type { SettingsPanelProps } from '../types';

export function SettingsPanel({ isOpen, onClose }: SettingsPanelProps) {
  const [autoApply, setAutoApply] = useState(true);
  const [notifications, setNotifications] = useState(true);
  const [confidence, setConfidence] = useState([75]);

  return (
    <Sheet open={isOpen} onOpenChange={onClose}>
      <SheetContent
        side="right"
        className="w-full sm:max-w-md backdrop-blur-3xl bg-white/90 dark:bg-gray-900/90 border-gray-200/30 dark:border-gray-700/30"
      >
        <SheetHeader>
          <SheetTitle className="text-gray-900 dark:text-gray-100">Settings</SheetTitle>
        </SheetHeader>

        <div className="mt-6 space-y-6">
          {/* ML Settings */}
          <div>
            <div className="flex items-center gap-2 mb-4">
              <Sparkles className="w-4 h-4 text-blue-500 dark:text-blue-400" />
              <h3 className="text-sm text-gray-900 dark:text-gray-100">ML Suggestions</h3>
            </div>

            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex-1">
                  <Label htmlFor="auto-apply" className="text-sm text-gray-700 dark:text-gray-300">
                    Auto-apply suggestions
                  </Label>
                  <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                    Automatically accept high-confidence suggestions
                  </p>
                </div>
                <Switch id="auto-apply" checked={autoApply} onCheckedChange={setAutoApply} />
              </div>

              <div>
                <div className="flex items-center justify-between mb-2">
                  <Label className="text-sm text-gray-700 dark:text-gray-300">Confidence threshold</Label>
                  <span className="text-sm text-gray-900 dark:text-gray-100">{confidence[0]}%</span>
                </div>
                <Slider
                  value={confidence}
                  onValueChange={setConfidence}
                  min={50}
                  max={100}
                  step={5}
                  className="w-full"
                />
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-1.5">
                  Only auto-apply suggestions above this confidence level
                </p>
              </div>
            </div>
          </div>

          <Separator className="bg-gray-200/50 dark:bg-gray-700/50" />

          {/* Notifications */}
          <div>
            <div className="flex items-center gap-2 mb-4">
              <Bell className="w-4 h-4 text-blue-500 dark:text-blue-400" />
              <h3 className="text-sm text-gray-900 dark:text-gray-100">Notifications</h3>
            </div>

            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex-1">
                  <Label htmlFor="notifications" className="text-sm text-gray-700 dark:text-gray-300">
                    Enable notifications
                  </Label>
                  <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                    Get notified about pending suggestions
                  </p>
                </div>
                <Switch id="notifications" checked={notifications} onCheckedChange={setNotifications} />
              </div>
            </div>
          </div>

          <Separator className="bg-gray-200/50 dark:bg-gray-700/50" />

          {/* Keyboard Shortcuts */}
          <div>
            <div className="flex items-center gap-2 mb-4">
              <Zap className="w-4 h-4 text-blue-500 dark:text-blue-400" />
              <h3 className="text-sm text-gray-900 dark:text-gray-100">Keyboard Shortcuts</h3>
            </div>

            <div className="space-y-2 text-xs">
              <div className="flex justify-between">
                <span className="text-gray-600 dark:text-gray-400">Start/Pause timer</span>
                <kbd className="px-2 py-0.5 rounded bg-gray-200 dark:bg-gray-800 text-gray-900 dark:text-gray-100">
                  Space
                </kbd>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600 dark:text-gray-400">New entry</span>
                <kbd className="px-2 py-0.5 rounded bg-gray-200 dark:bg-gray-800 text-gray-900 dark:text-gray-100">
                  ⌘N
                </kbd>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600 dark:text-gray-400">View entries</span>
                <kbd className="px-2 py-0.5 rounded bg-gray-200 dark:bg-gray-800 text-gray-900 dark:text-gray-100">
                  ⌘E
                </kbd>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600 dark:text-gray-400">Quick switcher</span>
                <kbd className="px-2 py-0.5 rounded bg-gray-200 dark:bg-gray-800 text-gray-900 dark:text-gray-100">
                  ⌘K
                </kbd>
              </div>
            </div>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}
