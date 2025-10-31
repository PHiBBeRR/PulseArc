// Phase 2: Outbox Status Component
// Displays SAP time entry outbox status with retry functionality

import { Badge } from '@/shared/components/ui/badge';
import { Button } from '@/shared/components/ui/button';
import { cn } from '@/shared/components/ui/utils';
import { SapService, type OutboxStatus } from '@/features/settings/services/sapService';
import { AlertCircle, CheckCircle, Clock, Loader2, RefreshCw } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';

export type OutboxStatusProps = {
  refreshInterval?: number; // Auto-refresh interval in ms (default: 10000)
  className?: string;
};

/**
 * Outbox Status Component
 *
 * Displays real-time status of SAP time entry outbox:
 * - Pending: Waiting to be sent
 * - Sent: Successfully posted to SAP
 * - Failed: Errors during submission (can retry)
 *
 * Features:
 * - Auto-refresh every 10 seconds
 * - Manual refresh button
 * - Retry failed entries button
 * - Status badges with color coding
 *
 * @example
 * ```tsx
 * <OutboxStatus refreshInterval={5000} />
 * ```
 */
export function OutboxStatusComponent({ refreshInterval = 10000, className }: OutboxStatusProps) {
  const [status, setStatus] = useState<OutboxStatus | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isRetrying, setIsRetrying] = useState(false);

  // Fetch outbox status
  const fetchStatus = useCallback(async () => {
    try {
      const outboxStatus = await SapService.getOutboxStatus();
      setStatus(outboxStatus);
    } catch (error) {
      console.error('Failed to fetch outbox status:', error);
      toast.error('Failed to load outbox status');
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, []);

  // Auto-refresh
  useEffect(() => {
    void fetchStatus();

    if (refreshInterval > 0) {
      const timer = setInterval(() => void fetchStatus(), refreshInterval);
      return () => clearInterval(timer);
    }
    return undefined;
  }, [fetchStatus, refreshInterval]);

  // Manual refresh
  const handleRefresh = useCallback(async () => {
    setIsRefreshing(true);
    await fetchStatus();
  }, [fetchStatus]);

  // Retry failed entries
  const handleRetry = useCallback(async () => {
    setIsRetrying(true);
    try {
      const count = await SapService.retryFailedEntries();
      toast.success(`Retrying ${count} failed ${count === 1 ? 'entry' : 'entries'}`);
      await fetchStatus(); // Refresh status after retry
    } catch (error) {
      console.error('Failed to retry entries:', error);
      toast.error('Failed to retry entries');
    } finally {
      setIsRetrying(false);
    }
  }, [fetchStatus]);

  if (isLoading) {
    return (
      <div className={cn('flex items-center gap-2 text-sm text-muted-foreground', className)}>
        <Loader2 className="h-4 w-4 animate-spin" />
        <span>Loading outbox status...</span>
      </div>
    );
  }

  if (!status) {
    return null;
  }

  const totalEntries = status.pending + status.sent + status.failed;
  const hasFailedEntries = status.failed > 0;
  const hasPendingEntries = status.pending > 0;

  return (
    <div className={cn('flex items-center gap-3', className)}>
      {/* Status Badges */}
      <div className="flex items-center gap-2">
        {hasPendingEntries && (
          <Badge variant="secondary" className="gap-1">
            <Clock className="h-3 w-3" />
            <span>{status.pending} pending</span>
          </Badge>
        )}

        {status.sent > 0 && (
          <Badge variant="default" className="gap-1 bg-green-600 hover:bg-green-700">
            <CheckCircle className="h-3 w-3" />
            <span>{status.sent} sent</span>
          </Badge>
        )}

        {hasFailedEntries && (
          <Badge variant="destructive" className="gap-1">
            <AlertCircle className="h-3 w-3" />
            <span>{status.failed} failed</span>
          </Badge>
        )}

        {totalEntries === 0 && <span className="text-sm text-muted-foreground">No entries</span>}
      </div>

      {/* Action Buttons */}
      <div className="flex items-center gap-1">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => void handleRefresh()}
          disabled={isRefreshing}
          title="Refresh status"
        >
          <RefreshCw className={cn('h-4 w-4', isRefreshing && 'animate-spin')} />
        </Button>

        {hasFailedEntries && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => void handleRetry()}
            disabled={isRetrying}
            className="gap-1"
          >
            {isRetrying ? (
              <>
                <Loader2 className="h-3 w-3 animate-spin" />
                <span>Retrying...</span>
              </>
            ) : (
              <>
                <RefreshCw className="h-3 w-3" />
                <span>Retry Failed</span>
              </>
            )}
          </Button>
        )}
      </div>
    </div>
  );
}
