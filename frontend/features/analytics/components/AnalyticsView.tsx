import { useState, useEffect } from 'react';
import { ArrowLeft, TrendingUp, Clock, DollarSign, BarChart3, GripHorizontal, Activity, PauseCircle, AlertCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { AnalyticsChartSkeleton, StatCardSkeleton } from '@/shared/components/feedback';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import {
  BarChart,
  Bar,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  AreaChart,
  Area,
} from 'recharts';
import { analyticsService } from '../services/analyticsService';
import type { TimeEntryAnalytics } from '@/shared/types/generated';
import type { AnalyticsViewProps, TimePeriod, DailyIdleSummary } from '../types';

export function AnalyticsView({ onBack }: AnalyticsViewProps) {
  const [period, setPeriod] = useState<TimePeriod>('week');
  const [isLoading, setIsLoading] = useState(true);
  const [idleSummaries, setIdleSummaries] = useState<DailyIdleSummary[]>([]);
  const [timeEntryAnalytics, setTimeEntryAnalytics] = useState<TimeEntryAnalytics[]>([]);

  // Fetch data when period changes
  useEffect(() => {
    const fetchData = async () => {
      setIsLoading(true);
      try {
        const { startDate, endDate } = analyticsService.getDateRangeForPeriod(period);
        
        // Fetch both idle summaries and time entry analytics
        const [summaries, analytics] = await Promise.all([
          analyticsService.fetchIdleSummariesForRange(startDate, endDate),
          analyticsService.fetchTimeEntryAnalytics(startDate, endDate),
        ]);
        
        setIdleSummaries(summaries);
        setTimeEntryAnalytics(analytics);
      } catch (error) {
        console.error('Failed to fetch analytics data:', error);
        setIdleSummaries([]);
        setTimeEntryAnalytics([]);
      } finally {
        setIsLoading(false);
      }
    };

    void fetchData();
  }, [period]);

  // Dynamically resize window on mount (like EntriesView does)
  useEffect(() => {
    const resizeWindow = async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const { LogicalSize } = await import('@tauri-apps/api/window');
        const currentWindow = getCurrentWindow();

        const targetWidth = 580;
        const targetHeight = 1025;

        await currentWindow.setSize(new LogicalSize(targetWidth, targetHeight));
        await currentWindow.setResizable(false);

        // Lock the size for analytics view
        await currentWindow.setMinSize(new LogicalSize(targetWidth, targetHeight));
        await currentWindow.setMaxSize(new LogicalSize(targetWidth, targetHeight));
      } catch (error) {
        console.error('[AnalyticsView] Failed to resize window:', error);
      }
    };

    void resizeWindow();
  }, []); // Run once on mount

  // Convert real time entry data to chart format
  const data = analyticsService.convertAnalyticsToTimeData(timeEntryAnalytics, period);

  // Calculate stats from real data only
  const stats = analyticsService.calculateStats(data, idleSummaries, timeEntryAnalytics);

  // Check if we have any data to display
  const hasData = timeEntryAnalytics.length > 0 || idleSummaries.length > 0;
  const pieData = analyticsService.getPieChartData(stats);
  const xAxisKey = analyticsService.getXAxisKey(period);
  
  // Calculate idle time percentages
  const totalTrackedTime = (stats.totalActive ?? 0) + (stats.totalIdle ?? 0);
  const activePercentage = totalTrackedTime > 0 ? Math.round(((stats.totalActive ?? 0) / totalTrackedTime) * 100) : 0;
  
  // Check if there are pending idle periods that need review
  const hasPendingIdle = (stats.totalIdlePending ?? 0) > 0;
  const pendingIdleHours = (stats.totalIdlePending ?? 0).toFixed(1);
  
  // Show adjusted vs raw billable if we have adjustments
  const hasAdjustments = stats.adjustedBillable !== undefined && stats.adjustedBillable !== stats.totalBillable;

  return (
    <div className="backdrop-blur-[24px] overflow-hidden h-full flex flex-col">
      {/* Drag handle bar */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-center py-2 cursor-move rounded-t-[40px] select-none"
      >
        <GripHorizontal className="w-8 h-3 text-gray-400/50 dark:text-gray-500/50 pointer-events-none" />
      </div>

      {/* Header */}
      <div className="p-4 pt-2 border-b border-white/10 dark:border-white/5">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="icon"
              onClick={onBack}
              className="h-7 w-7 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10"
            >
              <ArrowLeft className="w-3.5 h-3.5" />
            </Button>
            <h2 className="text-sm text-gray-900 dark:text-gray-100">Analytics</h2>
          </div>

          <Select value={period} onValueChange={(value) => setPeriod(value as TimePeriod)}>
            <SelectTrigger className="w-40 h-7 text-xs backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20">
              <BarChart3 className="w-3 h-3 mr-1" />
              <SelectValue />
            </SelectTrigger>
            <SelectContent className="backdrop-blur-[60px] bg-white/80 dark:bg-black/80 border-2 border-white/20 dark:border-white/10 shadow-[0_8px_32px_0_rgba(0,0,0,0.2)]">
              <SelectItem value="week">Past Week</SelectItem>
              <SelectItem value="month">Past Month</SelectItem>
              <SelectItem value="3months">3 Months</SelectItem>
              <SelectItem value="6months">6 Months</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      <ScrollArea className="flex-1 rounded-b-[2.5rem] overflow-hidden">
        <div className="p-4 space-y-4">
          {isLoading ? (
            <>
              {/* Loading Stats Cards */}
              <div className="grid grid-cols-3 gap-2">
                <StatCardSkeleton />
                <StatCardSkeleton />
                <StatCardSkeleton />
              </div>

              {/* Loading Charts */}
              <AnalyticsChartSkeleton />
              <AnalyticsChartSkeleton />
            </>
          ) : !hasData ? (
            <div className="flex flex-col items-center justify-center py-16 px-4">
              <BarChart3 className="w-16 h-16 text-gray-400 dark:text-gray-500 mb-4" />
              <h3 className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-2">No Analytics Data</h3>
              <p className="text-xs text-gray-500 dark:text-gray-400 text-center max-w-xs">
                Start tracking your time to see analytics for the selected period. Your time entries will appear here once you create them.
              </p>
            </div>
          ) : (
            <>
              {/* Pending Idle Warning */}
              {hasPendingIdle && (
                <div className="backdrop-blur-xl bg-amber-500/10 dark:bg-amber-500/5 border border-amber-500/30 dark:border-amber-500/20 rounded-2xl p-3 shadow-[0_4px_16px_0_rgba(0,0,0,0.08)]">
                  <div className="flex items-center gap-2">
                    <AlertCircle className="w-4 h-4 text-amber-600 dark:text-amber-400" />
                    <div className="flex-1">
                      <div className="text-xs font-medium text-amber-900 dark:text-amber-100">
                        {pendingIdleHours}h pending review
                      </div>
                      <div className="text-xs text-amber-700 dark:text-amber-300 mt-0.5">
                        Review idle periods to ensure accurate billable calculations
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {/* Stats Cards - Top Row */}
              <div className="grid grid-cols-3 gap-2">
                <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-3 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
                  <div className="flex items-center gap-1.5 mb-1">
                    <Clock className="w-3 h-3 text-gray-500 dark:text-gray-400" />
                    <span className="text-xs text-gray-500 dark:text-gray-400">Total{hasAdjustments ? ' (Raw)' : ''}</span>
                  </div>
                  <div className="text-gray-900 dark:text-gray-100">{stats.total.toFixed(1)}h</div>
                </div>

                <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-3 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
                  <div className="flex items-center gap-1.5 mb-1">
                    <DollarSign className="w-3 h-3 text-blue-500 dark:text-blue-400" />
                    <span className="text-xs text-gray-500 dark:text-gray-400">Billable{hasAdjustments ? ' (Raw)' : ''}</span>
                  </div>
                  <div className="text-gray-900 dark:text-gray-100">{stats.totalBillable.toFixed(1)}h</div>
                  {hasAdjustments && (
                    <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                      Adj: {stats.adjustedBillable?.toFixed(1)}h
                    </div>
                  )}
                </div>

                <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-3 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
                  <div className="flex items-center gap-1.5 mb-1">
                    <TrendingUp className="w-3 h-3 text-blue-500 dark:text-blue-400" />
                    <span className="text-xs text-gray-500 dark:text-gray-400">Rate{hasAdjustments ? ' (Raw)' : ''}</span>
                  </div>
                  <div className="text-gray-900 dark:text-gray-100">{stats.billablePercentage}%</div>
                  {hasAdjustments && stats.adjustedBillablePercentage && (
                    <div className="text-xs text-blue-600 dark:text-blue-400 mt-0.5">
                      Adj: {stats.adjustedBillablePercentage}%
                    </div>
                  )}
                </div>
              </div>

              {/* Idle Time Stats Cards - Second Row */}
              <div className="grid grid-cols-3 gap-2">
                <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-3 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
                  <div className="flex items-center gap-1.5 mb-1">
                    <Activity className="w-3 h-3 text-green-500 dark:text-green-400" />
                    <span className="text-xs text-gray-500 dark:text-gray-400">Active</span>
                  </div>
                  <div className="text-gray-900 dark:text-gray-100">{(stats.totalActive ?? 0).toFixed(1)}h</div>
                </div>

                <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-3 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
                  <div className="flex items-center gap-1.5 mb-1">
                    <PauseCircle className="w-3 h-3 text-amber-500 dark:text-amber-400" />
                    <span className="text-xs text-gray-500 dark:text-gray-400">Idle</span>
                  </div>
                  <div className="text-gray-900 dark:text-gray-100">{(stats.totalIdle ?? 0).toFixed(1)}h</div>
                </div>

                <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-3 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
                  <div className="flex items-center gap-1.5 mb-1">
                    <Activity className="w-3 h-3 text-green-500 dark:text-green-400" />
                    <span className="text-xs text-gray-500 dark:text-gray-400">Active %</span>
                  </div>
                  <div className="text-gray-900 dark:text-gray-100">{activePercentage}%</div>
                </div>
              </div>

              {/* Trend Chart - Area */}
              <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-4 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
                <h3 className="text-xs text-gray-700 dark:text-gray-300 mb-3">Hours Trend</h3>
                <ResponsiveContainer width="100%" height={200}>
                  <AreaChart data={data} margin={{ top: 5, right: 5, left: -20, bottom: 5 }}>
                    <defs>
                      <linearGradient id="fillBillable" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3} />
                        <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
                      </linearGradient>
                      <linearGradient id="fillNonBillable" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="#94a3b8" stopOpacity={0.3} />
                        <stop offset="95%" stopColor="#94a3b8" stopOpacity={0} />
                      </linearGradient>
                    </defs>
                    <CartesianGrid strokeDasharray="3 3" stroke="rgba(148,163,184,0.2)" vertical={false} />
                    <XAxis
                      dataKey={xAxisKey}
                      tickLine={false}
                      axisLine={false}
                      tickMargin={8}
                      tick={{ fill: 'currentColor', fontSize: 10 }}
                      stroke="rgba(148,163,184,0.3)"
                    />
                    <YAxis
                      tickLine={false}
                      axisLine={false}
                      tickMargin={8}
                      tick={{ fill: 'currentColor', fontSize: 10 }}
                      stroke="rgba(148,163,184,0.3)"
                    />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: 'rgba(255, 255, 255, 0.95)',
                        backdropFilter: 'blur(20px)',
                        border: '1px solid rgba(255, 255, 255, 0.3)',
                        borderRadius: '12px',
                        fontSize: '11px',
                        padding: '8px 12px',
                      }}
                      labelStyle={{ color: '#171717', fontWeight: 600 }}
                    />
                    <Legend wrapperStyle={{ fontSize: '11px', paddingTop: '12px' }} iconType="circle" />
                    <Area
                      type="monotone"
                      dataKey="billable"
                      name="Billable"
                      stroke="#3b82f6"
                      strokeWidth={2}
                      fillOpacity={1}
                      fill="url(#fillBillable)"
                    />
                    <Area
                      type="monotone"
                      dataKey="nonBillable"
                      name="Non-Billable"
                      stroke="#94a3b8"
                      strokeWidth={2}
                      fillOpacity={1}
                      fill="url(#fillNonBillable)"
                    />
                  </AreaChart>
                </ResponsiveContainer>
              </div>

              {/* Bar Chart */}
              <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-4 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
                <h3 className="text-xs text-gray-700 dark:text-gray-300 mb-3">Hours Breakdown</h3>
                <ResponsiveContainer width="100%" height={200}>
                  <BarChart data={data} margin={{ top: 5, right: 5, left: -20, bottom: 5 }}>
                    <CartesianGrid strokeDasharray="3 3" stroke="rgba(148,163,184,0.2)" vertical={false} />
                    <XAxis
                      dataKey={xAxisKey}
                      tickLine={false}
                      axisLine={false}
                      tickMargin={8}
                      tick={{ fill: 'currentColor', fontSize: 10 }}
                      stroke="rgba(148,163,184,0.3)"
                    />
                    <YAxis
                      tickLine={false}
                      axisLine={false}
                      tickMargin={8}
                      tick={{ fill: 'currentColor', fontSize: 10 }}
                      stroke="rgba(148,163,184,0.3)"
                    />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: 'rgba(255, 255, 255, 0.95)',
                        backdropFilter: 'blur(20px)',
                        border: '1px solid rgba(255, 255, 255, 0.3)',
                        borderRadius: '12px',
                        fontSize: '11px',
                        padding: '8px 12px',
                      }}
                      labelStyle={{ color: '#171717', fontWeight: 600 }}
                    />
                    <Legend wrapperStyle={{ fontSize: '11px', paddingTop: '12px' }} iconType="circle" />
                    <Bar dataKey="billable" name="Billable" fill="#3b82f6" radius={[6, 6, 0, 0]} maxBarSize={60} />
                    <Bar
                      dataKey="nonBillable"
                      name="Non-Billable"
                      fill="#94a3b8"
                      radius={[6, 6, 0, 0]}
                      maxBarSize={60}
                    />
                  </BarChart>
                </ResponsiveContainer>
              </div>

              {/* Active vs Idle Time Chart */}
              {idleSummaries.length > 0 && (
                <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-4 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
                  <h3 className="text-xs text-gray-700 dark:text-gray-300 mb-3">Active vs Idle Time</h3>
                  <ResponsiveContainer width="100%" height={200}>
                    <AreaChart 
                      data={idleSummaries.map(s => ({
                        date: new Date(s.date).toLocaleDateString('en-US', { month: 'short', day: 'numeric' }),
                        active: (s.totalActiveSecs / 3600).toFixed(1),
                        idle: (s.totalIdleSecs / 3600).toFixed(1),
                      }))} 
                      margin={{ top: 5, right: 5, left: -20, bottom: 5 }}
                    >
                      <defs>
                        <linearGradient id="fillActive" x1="0" y1="0" x2="0" y2="1">
                          <stop offset="5%" stopColor="#10b981" stopOpacity={0.3} />
                          <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
                        </linearGradient>
                        <linearGradient id="fillIdle" x1="0" y1="0" x2="0" y2="1">
                          <stop offset="5%" stopColor="#f59e0b" stopOpacity={0.3} />
                          <stop offset="95%" stopColor="#f59e0b" stopOpacity={0} />
                        </linearGradient>
                      </defs>
                      <CartesianGrid strokeDasharray="3 3" stroke="rgba(148,163,184,0.2)" vertical={false} />
                      <XAxis
                        dataKey="date"
                        tickLine={false}
                        axisLine={false}
                        tickMargin={8}
                        tick={{ fill: 'currentColor', fontSize: 10 }}
                        stroke="rgba(148,163,184,0.3)"
                      />
                      <YAxis
                        tickLine={false}
                        axisLine={false}
                        tickMargin={8}
                        tick={{ fill: 'currentColor', fontSize: 10 }}
                        stroke="rgba(148,163,184,0.3)"
                        label={{ value: 'Hours', angle: -90, position: 'insideLeft', style: { fontSize: 10 } }}
                      />
                      <Tooltip
                        contentStyle={{
                          backgroundColor: 'rgba(255, 255, 255, 0.95)',
                          backdropFilter: 'blur(20px)',
                          border: '1px solid rgba(255, 255, 255, 0.3)',
                          borderRadius: '12px',
                          fontSize: '11px',
                          padding: '8px 12px',
                        }}
                        labelStyle={{ color: '#171717', fontWeight: 600 }}
                      />
                      <Legend wrapperStyle={{ fontSize: '11px', paddingTop: '12px' }} iconType="circle" />
                      <Area
                        type="monotone"
                        dataKey="active"
                        name="Active Time"
                        stroke="#10b981"
                        strokeWidth={2}
                        fillOpacity={1}
                        fill="url(#fillActive)"
                      />
                      <Area
                        type="monotone"
                        dataKey="idle"
                        name="Idle Time"
                        stroke="#f59e0b"
                        strokeWidth={2}
                        fillOpacity={1}
                        fill="url(#fillIdle)"
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                </div>
              )}

              {/* Distribution & Summary Combined */}
              <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-4 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
                <h3 className="text-xs text-gray-700 dark:text-gray-300 mb-3">Distribution & Summary</h3>

                <div className="flex items-center gap-6">
                  {/* Pie Chart - Left Side */}
                  <div className="flex-shrink-0" style={{ width: '180px', height: '180px' }}>
                    <ResponsiveContainer width="100%" height="100%">
                      <PieChart>
                        <Pie
                          data={pieData}
                          cx="50%"
                          cy="50%"
                          innerRadius={45}
                          outerRadius={70}
                          paddingAngle={3}
                          dataKey="value"
                          strokeWidth={2}
                          stroke="rgba(255,255,255,0.2)"
                          label={false}
                        >
                          {pieData.map((entry, index) => (
                            <Cell
                              key={`cell-${index}`}
                              fill={entry.color}
                              className="transition-opacity hover:opacity-80"
                            />
                          ))}
                        </Pie>
                        <Tooltip
                          contentStyle={{
                            backgroundColor: 'rgba(255, 255, 255, 0.95)',
                            backdropFilter: 'blur(20px)',
                            border: '1px solid rgba(255, 255, 255, 0.3)',
                            borderRadius: '12px',
                            fontSize: '11px',
                            padding: '8px 12px',
                          }}
                          itemStyle={{ color: '#171717' }}
                        />
                      </PieChart>
                    </ResponsiveContainer>
                  </div>

                  {/* Summary Stats - Right Side */}
                  <div className="flex-1 space-y-3 text-xs">
                    <div className="flex items-center justify-between py-2 px-3 rounded-xl bg-white/10 dark:bg-white/5">
                      <div className="flex items-center gap-2">
                        <div className="w-2 h-2 rounded-full bg-blue-500" />
                        <span className="text-gray-600 dark:text-gray-400">Billable hours</span>
                      </div>
                      <span className="text-gray-900 dark:text-gray-100">{stats.totalBillable}h</span>
                    </div>

                    <div className="flex items-center justify-between py-2 px-3 rounded-xl bg-white/10 dark:bg-white/5">
                      <div className="flex items-center gap-2">
                        <div className="w-2 h-2 rounded-full bg-slate-400" />
                        <span className="text-gray-600 dark:text-gray-400">Non-billable hours</span>
                      </div>
                      <span className="text-gray-900 dark:text-gray-100">{stats.totalNonBillable}h</span>
                    </div>

                    <div className="flex items-center justify-between py-2 px-3 rounded-xl bg-blue-500/10 dark:bg-blue-500/5 border border-blue-500/20">
                      <span className="text-gray-700 dark:text-gray-300">Billable rate</span>
                      <span className="text-blue-600 dark:text-blue-400">{stats.billablePercentage}%</span>
                    </div>
                  </div>
                </div>
              </div>

              {/* Idle Time Breakdown */}
              {idleSummaries.length > 0 && (stats.totalIdle ?? 0) > 0 && (
                <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-4 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
                  <h3 className="text-xs text-gray-700 dark:text-gray-300 mb-3">Idle Time Breakdown</h3>
                  <div className="space-y-2 text-xs">
                    <div className="flex items-center justify-between py-2 px-3 rounded-xl bg-white/10 dark:bg-white/5">
                      <div className="flex items-center gap-2">
                        <div className="w-2 h-2 rounded-full bg-green-500" />
                        <span className="text-gray-600 dark:text-gray-400">Kept (counted as work)</span>
                      </div>
                      <span className="text-gray-900 dark:text-gray-100">{(stats.totalIdleKept ?? 0).toFixed(1)}h</span>
                    </div>

                    <div className="flex items-center justify-between py-2 px-3 rounded-xl bg-white/10 dark:bg-white/5">
                      <div className="flex items-center gap-2">
                        <div className="w-2 h-2 rounded-full bg-red-500" />
                        <span className="text-gray-600 dark:text-gray-400">Discarded (excluded)</span>
                      </div>
                      <span className="text-gray-900 dark:text-gray-100">{(stats.totalIdleDiscarded ?? 0).toFixed(1)}h</span>
                    </div>

                    <div className="flex items-center justify-between py-2 px-3 rounded-xl bg-amber-500/10 dark:bg-amber-500/5 border border-amber-500/20">
                      <div className="flex items-center gap-2">
                        <div className="w-2 h-2 rounded-full bg-amber-500" />
                        <span className="text-gray-600 dark:text-gray-400">Pending review</span>
                      </div>
                      <span className="text-amber-600 dark:text-amber-400">{(stats.totalIdlePending ?? 0).toFixed(1)}h</span>
                    </div>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
