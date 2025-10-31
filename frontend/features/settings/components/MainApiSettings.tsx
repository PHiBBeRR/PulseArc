/**
 * Main API Settings Component - FEATURE-016 Phase 3
 *
 * Manage connection to Main Pulsarc API for time entry sync.
 */

import { Badge } from '@/shared/components/ui/badge';
import { Button } from '@/shared/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/shared/components/ui/card';
import { Separator } from '@/shared/components/ui/separator';
import { cn } from '@/shared/components/ui/utils';
import { CheckCircle, Clock, Loader2, LogIn, LogOut, XCircle } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';
import { WebApiService, type OutboxStatus } from '../services/WebApiService';

export type MainApiSettingsProps = {
  className?: string;
};

/**
 * Main API Settings Component
 *
 * Provides UI for:
 * - View authentication status
 * - Login/logout with Auth0 OAuth
 * - View outbox status for main_api target
 * - Scheduler status
 *
 * @example
 * ```tsx
 * <MainApiSettings />
 * ```
 */
export function MainApiSettings({ className }: MainApiSettingsProps) {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [isLoggingOut, setIsLoggingOut] = useState(false);
  const [userEmail, setUserEmail] = useState<string | undefined>();
  const [outboxStatus, setOutboxStatus] = useState<OutboxStatus | null>(null);

  const checkAuthStatus = useCallback(async () => {
    setIsLoading(true);
    try {
      const status = await WebApiService.getAuthStatus();
      setIsAuthenticated(status.authenticated);
      setUserEmail(status.userEmail);
    } catch (error) {
      console.error('Failed to check auth status:', error);
      toast.error('Failed to check Main API connection status');
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Check auth status on mount
  useEffect(() => {
    void checkAuthStatus();
  }, [checkAuthStatus]);

  // Poll for auth status after OAuth flow starts
  // Note: OAuth callback is handled automatically by backend loopback server
  // We just need to detect when auth status changes from false → true

  // Load outbox status
  const loadOutboxStatus = useCallback(async () => {
    if (!isAuthenticated) return;

    try {
      const status = await WebApiService.getOutboxStatus();
      setOutboxStatus(status);
    } catch (error) {
      console.error('Failed to load outbox status:', error);
    }
  }, [isAuthenticated]);

  // Auto-refresh outbox status every 10 seconds when authenticated
  useEffect(() => {
    if (!isAuthenticated) return;

    // Initial load
    void loadOutboxStatus();

    // Set up interval
    const interval = setInterval(() => {
      void loadOutboxStatus();
    }, 10000); // 10 seconds

    return () => {
      clearInterval(interval);
    };
  }, [isAuthenticated, loadOutboxStatus]);

  const handleLogin = useCallback(async () => {
    setIsLoggingIn(true);
    try {
      await WebApiService.startLogin();
      toast.info('Opening browser for Pulsarc login...');

      // OAuth callback is handled automatically by backend loopback server
      // Poll for auth status change
      const pollInterval = setInterval(() => {
        void (async () => {
          const authenticated = await WebApiService.isAuthenticated();
          if (authenticated) {
            clearInterval(pollInterval);
            setIsAuthenticated(true);
            setIsLoggingIn(false);
            toast.success('Successfully connected to Pulsarc API');
            void checkAuthStatus();
          }
        })();
      }, 1000); // Poll every second

      // Timeout after 5 minutes
      setTimeout(() => {
        clearInterval(pollInterval);
        setIsLoggingIn(false);
      }, 300000);
    } catch (error) {
      console.error('Login failed:', error);
      toast.error('Failed to start Main API login');
      setIsLoggingIn(false);
    }
  }, [checkAuthStatus]);

  const handleLogout = useCallback(async () => {
    setIsLoggingOut(true);
    try {
      await WebApiService.logout();
      setIsAuthenticated(false);
      setUserEmail(undefined);
      setOutboxStatus(null);
      toast.success('Logged out from Pulsarc API');
    } catch (error) {
      console.error('Logout failed:', error);
      toast.error('Failed to logout from Main API');
    } finally {
      setIsLoggingOut(false);
    }
  }, []);

  const totalEntries = outboxStatus
    ? outboxStatus.pending + outboxStatus.sent + outboxStatus.failed
    : 0;
  const hasPendingEntries = (outboxStatus?.pending ?? 0) > 0;
  const hasFailedEntries = (outboxStatus?.failed ?? 0) > 0;

  return (
    <div className={cn('space-y-6', className)}>
      <Card>
        <CardHeader>
          <CardTitle>Pulsarc API Integration</CardTitle>
          <CardDescription>
            Manage your connection to Pulsarc for time entry submission
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
                  ? userEmail
                    ? `Connected as ${userEmail}`
                    : 'You are connected to Pulsarc API'
                  : 'Connect to enable automatic time entry sync'}
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
                      <span>Connect to Pulsarc</span>
                    </>
                  )}
                </Button>
              )}
            </div>
          </div>

          <Separator />

          {/* Outbox Status (only show when authenticated) */}
          {isAuthenticated && outboxStatus && (
            <div className="space-y-2">
              <div className="font-medium">Time Entry Outbox</div>
              <p className="text-sm text-muted-foreground">
                Time entries are queued locally and synced to Pulsarc every 10 seconds
              </p>

              {/* Status Badges */}
              <div className="flex items-center gap-2">
                {hasPendingEntries && (
                  <Badge variant="secondary" className="gap-1">
                    <Clock className="h-3 w-3" />
                    <span>{outboxStatus.pending} pending</span>
                  </Badge>
                )}

                {outboxStatus.sent > 0 && (
                  <Badge variant="default" className="gap-1 bg-green-600 hover:bg-green-700">
                    <CheckCircle className="h-3 w-3" />
                    <span>{outboxStatus.sent} sent</span>
                  </Badge>
                )}

                {hasFailedEntries && (
                  <Badge variant="destructive" className="gap-1">
                    <XCircle className="h-3 w-3" />
                    <span>{outboxStatus.failed} failed</span>
                  </Badge>
                )}

                {totalEntries === 0 && (
                  <span className="text-sm text-muted-foreground">No entries</span>
                )}
              </div>
            </div>
          )}

          {/* Info Section */}
          {!isAuthenticated && (
            <div className="rounded-lg bg-muted p-4">
              <h4 className="font-medium mb-2">How it works</h4>
              <ul className="space-y-1 text-sm text-muted-foreground">
                <li>• Securely authenticate with Auth0 OAuth</li>
                <li>• Time entries are synced automatically</li>
                <li>• Works offline - entries queued until connection restored</li>
                <li>• Tokens are stored securely in macOS Keychain</li>
              </ul>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
