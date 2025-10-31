// Phase 2 & 3: SAP Settings Component
// Manage SAP S/4HANA connection, authentication, and sync settings

import { OutboxStatusComponent } from '@/features/timer/components/OutboxStatus';
import { Badge } from '@/shared/components/ui/badge';
import { Button } from '@/shared/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/shared/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/shared/components/ui/select';
import { Separator } from '@/shared/components/ui/separator';
import { Switch } from '@/shared/components/ui/switch';
import { cn } from '@/shared/components/ui/utils';
import type { SapSyncSettings } from '@/shared/types/generated/SapSyncSettings';
import { CheckCircle, Loader2, LogIn, LogOut, RefreshCw, Trash2, XCircle } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';
import { SapService } from '../services/sapService';

export type SapSettingsProps = {
  className?: string;
};

/**
 * SAP Settings Component
 *
 * Provides UI for:
 * - View authentication status
 * - Login/logout with Auth0 OAuth
 * - View outbox status
 * - Manage forwarder (start/stop)
 *
 * @example
 * ```tsx
 * <SapSettings />
 * ```
 */
export function SapSettings({ className }: SapSettingsProps) {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [isLoggingOut, setIsLoggingOut] = useState(false);

  // Phase 3: Sync settings state
  const [syncSettings, setSyncSettings] = useState<SapSyncSettings | null>(null);
  const [isSyncing, setIsSyncing] = useState(false);
  const [isClearing, setIsClearing] = useState(false);

  // Phase 4.4: Health check and error handling state
  const [healthStatus, setHealthStatus] = useState<{
    healthy: boolean;
    latency_ms: number | null;
    last_error: string | null;
  } | null>(null);
  const [isCheckingHealth, setIsCheckingHealth] = useState(false);

  const checkAuthStatus = useCallback(async () => {
    setIsLoading(true);
    try {
      const authenticated = await SapService.isAuthenticated();
      setIsAuthenticated(authenticated);
    } catch (error) {
      console.error('Failed to check auth status:', error);
      toast.error('Failed to check SAP connection status');
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Check auth status on mount
  useEffect(() => {
    void checkAuthStatus();
  }, [checkAuthStatus]);

  // Listen for OAuth callback from backend
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      const { listen } = await import('@tauri-apps/api/event');

      unlisten = await listen<{ code: string; state: string }>('sap-oauth-callback', (event) => {
        void (async () => {
          try {
            const { code, state } = event.payload;
            await SapService.completeLogin(code, state);
            setIsAuthenticated(true);
            setIsLoggingIn(false);
            toast.success('Successfully connected to SAP');
            void checkAuthStatus();
          } catch (error) {
            console.error('OAuth callback failed:', error);
            toast.error('Failed to complete SAP login');
            setIsLoggingIn(false);
          }
        })();
      });
    };

    void setupListener();

    return () => {
      unlisten?.();
    };
  }, [checkAuthStatus]);

  const handleLogin = useCallback(async () => {
    setIsLoggingIn(true);
    try {
      await SapService.startLogin();
      toast.info('Opening browser for SAP login...');

      // OAuth callback will be handled by Tauri event listener (see useEffect above)
      // After user completes auth in browser, backend emits 'sap-oauth-callback' event
      toast.info('Complete login in browser, then return to app');
    } catch (error) {
      console.error('Login failed:', error);
      toast.error('Failed to start SAP login');
    } finally {
      setIsLoggingIn(false);
    }
  }, []);

  const handleLogout = useCallback(async () => {
    setIsLoggingOut(true);
    try {
      await SapService.logout();
      setIsAuthenticated(false);
      toast.success('Logged out from SAP');
    } catch (error) {
      console.error('Logout failed:', error);
      toast.error('Failed to logout from SAP');
    } finally {
      setIsLoggingOut(false);
    }
  }, []);

  // Phase 3: Load sync settings
  const loadSyncSettings = useCallback(async () => {
    if (!isAuthenticated) return;

    try {
      const settings = await SapService.getSyncSettings();
      setSyncSettings(settings);
    } catch (error) {
      console.error('Failed to load sync settings:', error);
    }
  }, [isAuthenticated]);

  // Load sync settings when authenticated
  useEffect(() => {
    void loadSyncSettings();
  }, [loadSyncSettings]);

  // Phase 4.4: Health check function
  const checkHealth = useCallback(async () => {
    if (!isAuthenticated) return;

    setIsCheckingHealth(true);
    try {
      const status = await SapService.checkConnectionHealth();
      setHealthStatus(status);
    } catch (error) {
      console.error('Failed to check health:', error);
      setHealthStatus({
        healthy: false,
        latency_ms: null,
        last_error: error instanceof Error ? error.message : 'Unknown error',
      });
    } finally {
      setIsCheckingHealth(false);
    }
  }, [isAuthenticated]);

  // Auto-refresh health status every 30 seconds
  useEffect(() => {
    if (!isAuthenticated) return;

    // Initial check
    void checkHealth();

    // Set up interval
    const interval = setInterval(() => {
      void checkHealth();
    }, 30000); // 30 seconds

    return () => {
      clearInterval(interval);
    };
  }, [isAuthenticated, checkHealth]);

  // Phase 3: Sync settings handlers
  const handleToggleSync = useCallback(
    (enabled: boolean) => {
      if (!syncSettings) return;

      void (async () => {
        try {
          await SapService.updateSyncSettings(enabled, syncSettings.sync_interval_hours);
          setSyncSettings({ ...syncSettings, enabled });
          toast.success(enabled ? 'Background sync enabled' : 'Background sync disabled');
        } catch (error) {
          console.error('Failed to update sync settings:', error);
          toast.error('Failed to update sync settings');
        }
      })();
    },
    [syncSettings]
  );

  const handleIntervalChange = useCallback(
    (value: string) => {
      if (!syncSettings) return;

      const interval = parseInt(value, 10);
      void (async () => {
        try {
          await SapService.updateSyncSettings(syncSettings.enabled, interval);
          setSyncSettings({ ...syncSettings, sync_interval_hours: interval });
          toast.success(`Sync interval updated to ${interval} hour${interval > 1 ? 's' : ''}`);
        } catch (error) {
          console.error('Failed to update sync interval:', error);
          toast.error('Failed to update sync interval');
        }
      })();
    },
    [syncSettings]
  );

  const handleSyncNow = useCallback(() => {
    void (async () => {
      setIsSyncing(true);
      try {
        await SapService.triggerSyncNow();
        toast.success('WBS sync started');
        // Reload settings to get updated last_sync_epoch
        setTimeout(() => void loadSyncSettings(), 2000);
      } catch (error) {
        console.error('Failed to trigger sync:', error);
        toast.error('Failed to start WBS sync');
      } finally {
        setIsSyncing(false);
      }
    })();
  }, [loadSyncSettings]);

  const handleClearCache = useCallback(() => {
    void (async () => {
      setIsClearing(true);
      try {
        const count = await SapService.clearCache();
        toast.success(`Cleared ${count} WBS code${count !== 1 ? 's' : ''} from cache`);
        void loadSyncSettings();
      } catch (error) {
        console.error('Failed to clear cache:', error);
        toast.error('Failed to clear WBS cache');
      } finally {
        setIsClearing(false);
      }
    })();
  }, [loadSyncSettings]);

  // Phase 4.4: Retry sync handler
  const handleRetrySync = useCallback(() => {
    void (async () => {
      setIsSyncing(true);
      try {
        const result = await SapService.retrySyncNow();
        if (result.success) {
          toast.success(`Synced ${result.elements_synced} WBS codes`);
          void loadSyncSettings();
        } else {
          toast.error(result.error || 'Sync failed');
        }
      } catch (error) {
        console.error('Failed to retry sync:', error);
        toast.error('Failed to retry sync');
      } finally {
        setIsSyncing(false);
      }
    })();
  }, [loadSyncSettings]);

  return (
    <div className={cn('space-y-6', className)}>
      <Card>
        <CardHeader>
          <CardTitle>SAP S/4HANA Integration</CardTitle>
          <CardDescription>
            Manage your connection to SAP for WBS code management and time entry posting
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Authentication Status */}
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <div className="flex items-center gap-2">
                <span className="font-medium">Connection Status</span>
                {isLoading ? (
                  <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                ) : isAuthenticated ? (
                  <Badge variant="default" className="gap-1 bg-green-600">
                    <CheckCircle className="h-3 w-3" />
                    Connected
                  </Badge>
                ) : (
                  <Badge variant="secondary" className="gap-1">
                    <XCircle className="h-3 w-3" />
                    Disconnected
                  </Badge>
                )}
              </div>
              <p className="text-sm text-muted-foreground">
                {isAuthenticated
                  ? 'You are connected to SAP S/4HANA'
                  : 'Connect to enable WBS code search and time entry posting'}
              </p>
            </div>

            {/* Login/Logout Button */}
            <div>
              {isAuthenticated ? (
                <Button
                  variant="outline"
                  onClick={() => void handleLogout()}
                  disabled={isLoggingOut}
                  className="gap-2"
                >
                  {isLoggingOut ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      <span>Logging out...</span>
                    </>
                  ) : (
                    <>
                      <LogOut className="h-4 w-4" />
                      <span>Disconnect</span>
                    </>
                  )}
                </Button>
              ) : (
                <Button onClick={() => void handleLogin()} disabled={isLoggingIn} className="gap-2">
                  {isLoggingIn ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      <span>Connecting...</span>
                    </>
                  ) : (
                    <>
                      <LogIn className="h-4 w-4" />
                      <span>Connect to SAP</span>
                    </>
                  )}
                </Button>
              )}
            </div>
          </div>

          <Separator />

          {/* Outbox Status (only show when authenticated) */}
          {isAuthenticated && (
            <div className="space-y-2">
              <div className="font-medium">Time Entry Outbox</div>
              <p className="text-sm text-muted-foreground">
                Time entries are queued locally and synced to SAP every 10 seconds
              </p>
              <OutboxStatusComponent />
            </div>
          )}

          {/* Info Section */}
          {!isAuthenticated && (
            <div className="rounded-lg bg-muted p-4">
              <h4 className="font-medium mb-2">How it works</h4>
              <ul className="space-y-1 text-sm text-muted-foreground">
                <li>• Securely authenticate with Auth0 OAuth</li>
                <li>• Search WBS codes from your SAP system</li>
                <li>• Time entries are synced automatically</li>
                <li>• Tokens are stored securely in macOS Keychain</li>
              </ul>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Phase 3: Sync Settings */}
      {isAuthenticated && syncSettings && (
        <Card>
          <CardHeader>
            <CardTitle>WBS Sync Settings</CardTitle>
            <CardDescription>Configure background WBS code synchronization</CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            {/* Background Sync Toggle */}
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <div className="font-medium">Background Sync</div>
                <p className="text-sm text-muted-foreground">
                  Automatically sync WBS codes from SAP
                </p>
              </div>
              <Switch checked={syncSettings.enabled} onCheckedChange={handleToggleSync} />
            </div>

            <Separator />

            {/* Sync Interval */}
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <div className="font-medium">Sync Interval</div>
                <p className="text-sm text-muted-foreground">
                  How often to fetch updated WBS codes
                </p>
              </div>
              <Select
                value={String(syncSettings.sync_interval_hours)}
                onValueChange={handleIntervalChange}
                disabled={!syncSettings.enabled}
              >
                <SelectTrigger className="w-32">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="1">1 hour</SelectItem>
                  <SelectItem value="3">3 hours</SelectItem>
                  <SelectItem value="6">6 hours</SelectItem>
                  <SelectItem value="12">12 hours</SelectItem>
                  <SelectItem value="24">24 hours</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <Separator />

            {/* Phase 4.4: Connection Health Status */}
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <div className="font-medium">Connection Health</div>
                {healthStatus ? (
                  <div className="flex items-center gap-2">
                    {healthStatus.healthy ? (
                      <>
                        <Badge variant="default" className="bg-green-600">
                          Healthy
                        </Badge>
                        {healthStatus.latency_ms && (
                          <span className="text-xs text-muted-foreground">
                            {healthStatus.latency_ms}ms
                          </span>
                        )}
                      </>
                    ) : (
                      <>
                        <Badge variant="destructive">Unhealthy</Badge>
                        {healthStatus.last_error && (
                          <span className="text-xs text-destructive">
                            {healthStatus.last_error}
                          </span>
                        )}
                      </>
                    )}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground">Checking...</p>
                )}
              </div>
              {isCheckingHealth && (
                <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
              )}
            </div>

            {/* Offline Indicator */}
            {healthStatus &&
              !healthStatus.healthy &&
              healthStatus.last_error?.toLowerCase().includes('offline') && (
                <div className="rounded-lg bg-destructive/10 p-4 border border-destructive/20">
                  <div className="flex items-center gap-2">
                    <XCircle className="h-4 w-4 text-destructive" />
                    <span className="font-medium text-destructive">Offline</span>
                  </div>
                  <p className="text-sm text-muted-foreground mt-1">
                    No internet connection. Time entries will be queued locally.
                  </p>
                </div>
              )}

            <Separator />

            {/* Last Sync Status */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <div className="space-y-1">
                  <div className="font-medium">Last Sync</div>
                  <p className="text-sm text-muted-foreground">
                    {syncSettings.last_sync_epoch
                      ? new Date(Number(syncSettings.last_sync_epoch) * 1000).toLocaleString()
                      : 'Never'}
                  </p>
                </div>
                {syncSettings.last_sync_status && (
                  <Badge
                    variant={
                      syncSettings.last_sync_status.toLowerCase().includes('success')
                        ? 'default'
                        : 'destructive'
                    }
                  >
                    {syncSettings.last_sync_status}
                  </Badge>
                )}
              </div>

              {/* Error Message Display with Retry Button */}
              {syncSettings.last_sync_status &&
                !syncSettings.last_sync_status.toLowerCase().includes('success') &&
                syncSettings.last_sync_status.length > 0 && (
                  <div className="rounded-lg bg-destructive/10 p-4 border border-destructive/20">
                    <p className="text-sm text-destructive">{syncSettings.last_sync_status}</p>
                    {/* Show retry button for network errors (not validation errors) */}
                    {!syncSettings.last_sync_status.toLowerCase().includes('invalid') &&
                      !syncSettings.last_sync_status.toLowerCase().includes('validation') && (
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => void handleRetrySync()}
                          disabled={isSyncing}
                          className="mt-2"
                        >
                          {isSyncing ? (
                            <>
                              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                              Retrying...
                            </>
                          ) : (
                            <>
                              <RefreshCw className="mr-2 h-4 w-4" />
                              Retry Sync
                            </>
                          )}
                        </Button>
                      )}
                  </div>
                )}
            </div>

            <Separator />

            {/* Manual Actions */}
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => void handleSyncNow()}
                disabled={isSyncing}
                className="flex-1"
              >
                {isSyncing ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Syncing...
                  </>
                ) : (
                  <>
                    <RefreshCw className="mr-2 h-4 w-4" />
                    Sync Now
                  </>
                )}
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => void handleClearCache()}
                disabled={isClearing}
                className="flex-1"
              >
                {isClearing ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Clearing...
                  </>
                ) : (
                  <>
                    <Trash2 className="mr-2 h-4 w-4" />
                    Clear Cache
                  </>
                )}
              </Button>
            </div>

            {/* Info Section */}
            <div className="rounded-lg bg-muted p-4">
              <p className="text-sm text-muted-foreground">
                WBS codes are cached locally for faster search. Background sync keeps the cache up
                to date with SAP. Cache expires after 24 hours.
              </p>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
