import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Check, ChevronDown, RefreshCw, X } from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import type { CalendarConnectionStatus } from '@/shared/types/generated/CalendarConnectionStatus';
import { calendarService } from '../services/calendarService';

type CalendarProvider = 'google' | 'microsoft';

type CalendarProviderCardProps = {
  provider: CalendarProvider;
  status?: CalendarConnectionStatus;
  onConnect: () => void;
  onDisconnect: () => void;
  isLoading?: boolean;
};

type ProviderConfig = {
  name: string;
  iconUrl: string;
  iconAlt: string;
};

const PROVIDER_CONFIGS: Record<CalendarProvider, ProviderConfig> = {
  google: {
    name: 'Google Calendar',
    iconUrl: 'https://upload.wikimedia.org/wikipedia/commons/a/a5/Google_Calendar_icon_%282020%29.svg',
    iconAlt: 'Google Calendar',
  },
  microsoft: {
    name: 'Microsoft 365',
    iconUrl: 'https://upload.wikimedia.org/wikipedia/commons/thumb/0/0e/Microsoft_365_%282022%29.svg/512px-Microsoft_365_%282022%29.svg.png',
    iconAlt: 'Microsoft 365',
  },
};

export function CalendarProviderCard({
  provider,
  status,
  onConnect,
  onDisconnect,
  isLoading = false,
}: CalendarProviderCardProps) {
  const config = PROVIDER_CONFIGS[provider];
  const isConnected = status?.connected ?? false;
  const [isSyncing, setIsSyncing] = useState(false);

  const handleSyncCalendar = async () => {
    setIsSyncing(true);
    try {
      const count = await calendarService.syncProvider(provider);
      console.warn(`[CalendarProviderCard] ${provider} calendar synced: ${count} suggestions generated`);
    } catch (error) {
      console.error(`[CalendarProviderCard] Failed to sync ${provider} calendar:`, error);
    } finally {
      setIsSyncing(false);
    }
  };

  return (
    <div className="flex items-center justify-between bg-white/10 dark:bg-white/5 border border-white/30 dark:border-white/20 rounded-xl p-3 mb-3">
      <div className="flex items-center gap-3">
        <img src={config.iconUrl} alt={config.iconAlt} className="w-7 h-7" />
        <div>
          <div className="text-sm text-gray-900 dark:text-gray-100">{config.name}</div>
          <div className="text-xs text-gray-500 dark:text-gray-400">
            {isConnected ? (status?.email ?? 'Connected') : 'Not connected'}
          </div>
        </div>
      </div>
      {isConnected ? (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              size="sm"
              variant="outline"
              disabled={isLoading || isSyncing}
              className="text-xs h-7 bg-green-500/20 hover:bg-green-500/30 dark:bg-green-400/20 dark:hover:bg-green-400/30 border border-green-500/30 dark:border-green-400/30 text-green-400 dark:text-green-400"
            >
              <Check className="w-3 h-3 mr-1" />
              Connected
              <ChevronDown className="w-3 h-3 ml-1" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-40">
            <DropdownMenuItem
              onClick={() => void handleSyncCalendar()}
              disabled={isSyncing}
            >
              <RefreshCw className={`w-4 h-4 mr-2 ${isSyncing ? 'animate-spin' : ''}`} />
              {isSyncing ? 'Syncing...' : 'Sync Now'}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={onDisconnect}
              disabled={isLoading}
              className="text-red-600 dark:text-red-400 focus:text-red-600 dark:focus:text-red-400"
            >
              <X className="w-4 h-4 mr-2" />
              Disconnect
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      ) : (
        <Button
          size="sm"
          variant="default"
          onClick={() => {
            console.warn(`[CalendarProviderCard] Connect button clicked for ${provider}`);
            onConnect();
          }}
          disabled={isLoading}
          className="text-xs h-7 backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15 text-gray-700 dark:text-gray-300"
        >
          {isLoading ? 'Connecting...' : 'Connect'}
        </Button>
      )}
    </div>
  );
}

